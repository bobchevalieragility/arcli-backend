use std::collections::HashMap;
use std::time::Duration;
use cliclack::{intro, multi_progress, spinner, ProgressBar};
use async_trait::async_trait;
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::errors::ArcError;
use crate::models::goals::{Goal, GoalParams, GoalType};
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};
use crate::models::argo::{AppInfo, ArgoCdInstance};
use crate::clients::argo_client::ArgoClient;
use crate::models::aws_profile::AwsProfileInfo;
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
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Determine which ArgoCD instance to query and optionally which apps to filter
        let (argo_instance, target_versions) = match params {
            GoalParams::ArgoStatusesKnown { pull_request: Some(pr) } => {
                // Construct params for GitHub goal
                let repo = "services-gitops".to_string();
                let (pull_request, lookback_duration) =  if *pr == 0u32 {
                    // We use a sentinel value of zero when user specifies '-pr' option without a value
                    (None, Some(Duration::from_mins(10)))
                } else {
                    (Some(*pr), None)
                };
                let github_goal = Goal::github_pr_files_known(repo, pull_request, lookback_duration);

                // If we haven't obtained GitHub PR files yet, we need to wait for that goal to complete
                if !state.contains(&github_goal) {
                    return Ok(GoalStatus::Needs(github_goal));
                }

                // Retrieve GitHub PR's changed files from state
                let pr_files = state.get_github_pr_files(&github_goal)?;
                parse_github_pr_files(pr_files).await?
            },
            GoalParams::ArgoStatusesKnown { pull_request: None } => {
                if let Some(profile) = AwsProfileInfo::current().await {
                    // An AWS profile is currently active, so use it to infer ArgoCD instance
                    (ArgoCdInstance::from(&profile), HashMap::new())
                } else {
                    // No AWS profile is currently active, prompt user to select ArgoCD instance
                    let argo_instance = prompt_for_argo_instance()?;
                    (argo_instance, HashMap::new())
                }
            },
            _ => return Err(ArcError::invalid_goal_params(GoalType::ArgoStatusKnown, params)),
        };

        // Create session guard to handle token renewal
        let argo_client = ArgoClient::new(argo_instance.clone())?;

        // Retrieve the initial status of all apps
        let apps = argo_client.fetch_apps("arc").await?;

        let mut apps_to_monitor: Vec<&str> = if target_versions.is_empty() {
            // Monitor all apps
            apps.keys().map(|s| s.as_str()).collect()
        } else {
            // Monitor only those apps that are modified in the given PR
            target_versions.keys().map(|s| s.as_str()).collect()
        };
        apps_to_monitor.sort();

        if target_versions.is_empty() {
            // Just show a single snapshot of the current status as a table
            let mut rows = Vec::new();
            rows.push(AppInfo::header());
            rows.push("-".repeat(105));
            for name in apps_to_monitor.iter() {
                let app_status = apps.get(*name)
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| format!("❓ {:<30} {:<23} {:<40}", name, "-", "-"));
                rows.push(app_status);
            }

            let status_msg = rows.join("\n");
            let prompt = format!("ArgoCD Application Status ({})", argo_instance.name());
            let outro_text = OutroText::multi(prompt, status_msg);

            Ok(GoalStatus::Completed(TaskResult::ArgoAppStatuses(apps), outro_text))
        } else {
            // Continually update the app statuses until all apps are synced
            let multi = multi_progress(format!("Waiting for ArgoCD ({}) applications to sync...", argo_instance.name()));

            // Create a progress spinner for each app
            let mut spinners: HashMap<&str, ProgressBar> = apps_to_monitor.iter()
                .map(|&name| {
                    let item = multi.add(spinner());
                    (name, item)
                })
                .collect();

            // Wait until all spinners have been added before starting any of them, just to be safe
            for (&name, spinner) in &spinners {
                match apps.get(name) {
                    Some(app) => {
                        spinner.start(format!(" {}", app.minimal_text(false)));
                    },
                    None => {
                        spinner.start(format!(" {:<30} {:<23} {:<40}", name, "-", "-"));
                    }
                }
            }

            // Loop until all apps are synced and corresponding spinners are stopped
            spinners = update_progress(&apps, &target_versions, spinners)?;
            while !spinners.is_empty() {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let apps = argo_client.fetch_apps("arc").await?;
                spinners = update_progress(&apps, &target_versions, spinners)?;
            }

            multi.stop();
            Ok(GoalStatus::Completed(TaskResult::ArgoAppStatuses(apps), OutroText::None))
        }
    }
}

fn update_progress<'a>(
    apps: &HashMap<String, AppInfo>,
    target_versions: &HashMap<String, String>,
    spinners: HashMap<&'a str, ProgressBar>
) -> Result<HashMap<&'a str, ProgressBar>, ArcError> {
    let mut unsynced_app_spinners: HashMap<&str, ProgressBar> = HashMap::new();

    for (name, spinner) in spinners {
        match apps.get(name) {
            Some(app) => {
                // let target_version = target_versions.get(name).map(|s| s.as_str());
                let target_version = target_versions.get(name)
                    .ok_or_else(|| ArcError::UserInputError(format!("Unknown target version for app: {}", name)))?;

                if app.is_version_updated(target_version) && app.is_synced() {
                    spinner.stop(format!("✅ {}", app.minimal_text(true)));
                } else if app.is_version_updated(target_version) {
                    spinner.set_message(format!(" {}", app.minimal_text(true)));
                    unsynced_app_spinners.insert(name, spinner);
                } else {
                    unsynced_app_spinners.insert(name, spinner);
                }
            },
            None => {
                spinner.stop(format!("❓ {:<30} {:<23} {:<40}", name, "-", "-"));
            }
        };
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

fn prompt_for_argo_instance() -> Result<ArgoCdInstance, ArcError> {
    // Get a list of all available ArgoCD instances
    let available_argo_instances = ArgoCdInstance::all();

    let mut menu = cliclack::select("Select ArgoCD instance");
    for argo in &available_argo_instances {
        menu = menu.item(argo.name(), argo.name(), "");
    }

    let argo_name = menu.interact()?.to_string();
    Ok(ArgoCdInstance::from(argo_name.as_str()))
}
