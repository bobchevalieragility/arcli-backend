use cliclack::{intro, select};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crate::{GoalStatus, OutroText};
use crate::models::github::{get_github_app_token, get_installation_id};
use crate::models::github::GithubPr;
use crate::models::github::GithubPrFile;
use crate::models::config::CliConfig;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, Goal, GoalParams, GoalType};
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};
use reqwest::header::{AUTHORIZATION, USER_AGENT};

pub const SECRET_PATH: &str = "mp/arcli-backend-services-gitops";
pub const APP_ID_FIELD: &str = "APP_ID";
pub const PRIVATE_KEY_FIELD: &str = "PRIVATE_KEY";

#[derive(Debug)]
pub struct GetGithubPrFilesTask;

#[async_trait]
impl Task for GetGithubPrFilesTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Get GitHub PR files")?;
        Ok(())
    }

    async fn execute(
        &self,
        params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Extract aws_profile arg from params
        let aws_profile = match params {
            GoalParams::InfluxInstanceSelected { aws_profile, .. } => aws_profile.clone(),
            _ => None,
        };

        // Fetch GitHub AppID from Vault
        let app_id_goal = Goal::vault_secret_known(
            SECRET_PATH.to_string(), Some(APP_ID_FIELD.to_string()), aws_profile.clone()
        );
        if !state.contains(&app_id_goal) {
            return Ok(GoalStatus::Needs(app_id_goal));
        }

        // Fetch GitHub private key from Vault
        let private_key_goal = Goal::vault_secret_known(
            SECRET_PATH.to_string(), Some(PRIVATE_KEY_FIELD.to_string()), aws_profile
        );
        if !state.contains(&private_key_goal) {
            return Ok(GoalStatus::Needs(private_key_goal));
        }

        // Retrieve GitHub AppID and private key from state
        let app_id = state.get_vault_secret(&app_id_goal)?;
        let private_key = state.get_vault_secret(&private_key_goal)?;

        let client = reqwest::Client::new();

        // Fetch installation ID for the GitHub App (assuming single installation for org app)
        let installation_id = get_installation_id(&client, &app_id, &private_key).await?;

        // Fetch GitHub API access token for the app installation
        let token = get_github_app_token(&client, &app_id, &private_key, &installation_id).await?;

        // Extract goal parameters
        let (repo, pr) = match params {
            GoalParams::GithubPrFilesKnown { repo, pull_request: Some(pr), .. } => (repo, *pr),
            GoalParams::GithubPrFilesKnown { repo, pull_request: _, lookback_duration: Some(duration), .. } => {
                // Prompt user to select a PR that was opened within the specified window duration
                let selected_pr = prompt_to_select_recently_opened_pr(&client, repo, &token, duration).await?;
                (repo, selected_pr)
            },
            _ => return Err(ArcError::invalid_goal_params(GoalType::GithubPrFilesKnown, params)),
        };

        // Construct the GitHub API URL for listing files changed in the PR
        let api_url = format!("https://api.github.com/repos/agilityrobotics/{repo}/pulls/{pr}/files");

        // Query GitHub API for list of files changed in the PR
        let req = client.get(api_url)
            .header(USER_AGENT, "rust-github-pr-list")
            .header("Accept", "application/vnd.github.v3+json")
            .header(AUTHORIZATION, format!("Bearer {}", token));

        let files: Vec<GithubPrFile> = req.send().await?.json().await?;
        Ok(GoalStatus::Completed(TaskResult::GithubPrFiles(files), OutroText::None))
    }
}

async fn prompt_to_select_recently_opened_pr(
    client: &reqwest::Client,
    repo: &str,
    token: &str,
    duration: &std::time::Duration
) -> Result<u32, ArcError> {
    // Query GitHub API for all open PRs created within the past window_duration
    let minutes = duration.as_secs() / 60;
    let cutoff_time = Utc::now() - chrono::Duration::minutes(minutes as i64);
    let prs = query_recent_prs(client, repo, token, cutoff_time).await?;

    if prs.is_empty() {
        return Err(ArcError::UserInputError(format!("No open PRs found in the last {} minutes", minutes)));
    }

    let mut menu = select("Select a pull request");

    for pr in prs {
        let label = format!("#{} - {} (by @{})", pr.number, pr.title, pr.user.login);
        menu = menu.item(pr.number, label, "");
    }

    let selected = menu.interact()?;
    Ok(selected as u32)
}

async fn query_recent_prs(
    client: &reqwest::Client,
    repo: &str,
    token: &str,
    cutoff_time: DateTime<Utc>,
) -> Result<Vec<GithubPr>, ArcError> {
    let api_url = format!("https://api.github.com/repos/agilityrobotics/{repo}/pulls?state=open&sort=created&direction=desc&per_page=100");

    let response = client.get(&api_url)
        .header(USER_AGENT, "rust-github-pr-list")
        .header("Accept", "application/vnd.github.v3+json")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await?;

    let all_prs: Vec<GithubPr> = response.json().await?;

    // Filter PRs created after cutoff time
    let recent_prs: Vec<GithubPr> = all_prs.into_iter()
        .filter(|pr| pr.created_at > cutoff_time)
        .collect();

    Ok(recent_prs)
}