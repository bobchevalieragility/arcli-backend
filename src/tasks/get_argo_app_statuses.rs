use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use cliclack::{intro, multi_progress, spinner, ProgressBar};
use async_trait::async_trait;
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, Goal, GoalParams, GoalType};
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};
use crate::models::argo::{AppInfo, ArgoCdInstance};
use crate::clients::argo_client::ArgoClient;
use crate::models::github::GithubPrFile;

#[derive(Debug)]
pub struct GetArgoAppStatusesTask;

#[async_trait]
impl Task for GetArgoAppStatusesTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Get ArgoCD app statuses")?;
        Ok(())
    }

    async fn execute(
        &self,
        params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Determine which ArgoCD instance to query and optionally which apps to filter
        let (argo_instance, target_versions) = match params {
            GoalParams::ArgoStatusesKnown { pull_request: Some(pr), aws_profile, .. } => {
                // Construct params for GitHub goal
                let repo = "services-gitops".to_string();
                let (pull_request, lookback_duration) =  if *pr == 0u32 {
                    (None, Some(Duration::from_mins(10)))
                } else {
                    (Some(*pr), None)
                };
                let github_goal = Goal::github_pr_files_known(
                    repo, pull_request, lookback_duration, aws_profile.clone()
                );

                // If we haven't obtained GitHub PR files yet, we need to wait for that goal to complete
                if !state.contains(&github_goal) {
                    return Ok(GoalStatus::Needs(github_goal));
                }

                // Retrieve GitHub PR's changed files from state
                let pr_files = state.get_github_pr_files(&github_goal)?;
                parse_github_pr_files(pr_files).await?
            },
            GoalParams::ArgoStatusesKnown { pull_request: None, aws_profile: Some(profile), .. } => {
                // If AWS profile info is not available, we need to wait for that goal to complete
                let profile_goal = Goal::aws_profile_selected(Some(profile.clone()));
                if !state.contains(&profile_goal) {
                    return Ok(GoalStatus::Needs(profile_goal));
                }

                // Retrieve info about the desired AWS profile from state
                let profile_info = state.get_aws_profile_info(&profile_goal)?;
                (ArgoCdInstance::from(profile_info), HashMap::new())
            },
            GoalParams::ArgoStatusesKnown { pull_request: None, aws_profile: None, .. } => {
                // If an Argo instance has not yet been selected, we need to wait for that goal to complete
                let argo_selection_goal = Goal::argo_instance_selected();
                if !state.contains(&argo_selection_goal) {
                    return Ok(GoalStatus::Needs(argo_selection_goal));
                }

                // Retrieve selected Argo instance from state
                let argo_instance = state.get_argo_instance(&argo_selection_goal)?;
                (argo_instance.clone(), HashMap::new())
            },
            _ => return Err(ArcError::invalid_goal_params(GoalType::ArgoStatusKnown, params)),
        };

        // Create session guard to handle token renewal
        let argo_client = ArgoClient::new(argo_instance.clone())?;

        // Retrieve the initial status of all apps
        let apps = argo_client.fetch_apps("arc", &target_versions).await?;

        // Extract snapshot goal parameter
        let snapshot = match params {
            GoalParams::ArgoStatusesKnown { snapshot, .. } => *snapshot,
            _ => return Err(ArcError::invalid_goal_params(GoalType::ArgoStatusKnown, params)),
        };

        if !snapshot {
            // Continually update the app statuses until all apps are synced
            let multi = multi_progress(format!("Waiting for ArgoCD ({}) applications to sync...", argo_instance.name()));

            // Create a progress spinner for each app
            let mut spinners: HashMap<String, ProgressBar> = apps.values()
                .map(|app| {
                    let item = multi.add(spinner());
                    (app.name.clone(), item)
                })
                .collect();

            // Wait until all spinners have been added before starting any of them, just to be safe
            for (app, spinner) in apps.values().zip(spinners.values()) {
                spinner.start(format!(" {}", app.minimal_text()));
            }

            // Loop until all apps are synced and corresponding spinners are stopped
            spinners = update_progress(&apps, &target_versions, &spinners)?;
            while !spinners.is_empty() {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let apps = argo_client.fetch_apps("arc", &target_versions).await?;
                spinners = update_progress(&apps, &target_versions, &spinners)?;
            }

            multi.stop();
            Ok(GoalStatus::Completed(TaskResult::ArgoAppStatuses(apps), OutroText::None))
        } else {
            // Just show a single snapshot of the current status as a table
            let mut rows = Vec::new();
            rows.push(AppInfo::header());
            rows.push("-".repeat(105));
            for (_name, app) in &apps {
                rows.push(app.to_string());
            }

            let status_msg = rows.join("\n");
            let prompt = format!("ArgoCD Application Status ({})", argo_instance.name());
            let outro_text = OutroText::multi(prompt, status_msg);

            Ok(GoalStatus::Completed(TaskResult::ArgoAppStatuses(apps), outro_text))
        }
    }
}

fn update_progress(
    apps: &BTreeMap<String, AppInfo>,
    target_versions: &HashMap<String, String>,
    spinners: &HashMap<String, ProgressBar>
) -> Result<HashMap<String, ProgressBar>, ArcError> {
    let mut unsynced_app_spinners: HashMap<String, ProgressBar> = HashMap::new();

    for (name, spinner) in spinners {
        let app = apps.get(name)
            .ok_or_else(|| ArcError::UserInputError(format!("Missing AppInfo for: {name}.")))?;
        let target_version = target_versions.get(name).map(|s| s.as_str());

        // Update spinner's progress
        if app.is_synced_to_target(target_version) {
            spinner.stop(format!("✅ {}", app.minimal_text()));
        } else {
            unsynced_app_spinners.insert(name.to_string(), spinner.clone());
        }
    }

    Ok(unsynced_app_spinners)
}


async fn parse_github_pr_files(files: &Vec<GithubPrFile>) -> Result<(ArgoCdInstance, HashMap<String, String>), ArcError> {
    // Grab the first changed file in the PR
    let first_filename = &files.first()
        .ok_or_else(|| ArcError::UserInputError("No files found in PR".to_string()))?
        .filename;

    // Extract environment name from the filename, between the second-to-last and last slashes
    // e.g., "charts/arc/arc-example/envs/models/us-west-2/stage/version.yaml" -> "stage"
    let parts: Vec<&str> = first_filename.split('/').collect();
    let argo_env = if parts.len() >= 2 {
        parts[parts.len() - 2]
    } else {
        return Err(ArcError::UserInputError(format!("Unable to parse environment from filename: {}", first_filename)));
    };

    // Infer an ArgoCD instance from the environment name
    let argo_instance = ArgoCdInstance::from(argo_env);

    // Extract list of app names and their target versions from each changed file in the PR
    let target_versions: HashMap<String, String> = files.iter()
        .filter_map(|file| {
            let app_name = extract_arc_app_name(file)?;
            let target_version = extract_app_target_version(file)?;
            Some((app_name, target_version))
        })
        .collect();

    Ok((argo_instance, target_versions))
}

fn extract_arc_app_name(pr_file: &GithubPrFile) -> Option<String> {
    const PREFIX: &str = "charts/arc/";

    if !pr_file.filename.starts_with(PREFIX) {
        return None;
    }

    // Get the portion after "charts/arc/"
    let after_prefix = &pr_file.filename[PREFIX.len()..];

    // Find the next "/" and extract the service name
    after_prefix.split('/').next().map(|s| s.to_string())
}

fn extract_app_target_version(pr_file: &GithubPrFile) -> Option<String> {
    let patch = pr_file.patch.as_ref()?;

    // Search for lines that start with "+  tag: " in the patch
    for line in patch.lines() {
        if line.starts_with("+  tag: ") {
            // Extract the version after "+  tag: "
            let version = line["+  tag: ".len()..].trim();
            // Remove quotes if present
            let version = version.trim_matches('"').trim_matches('\'');
            return Some(version.to_string());
        }
    }

    None
}
