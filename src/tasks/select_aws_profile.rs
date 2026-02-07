use cliclack::{intro, select};
use async_trait::async_trait;
use std::env;
use crate::aws::aws_account::AwsAccount;
use crate::{aws, GoalStatus, OutroText};
use crate::args::PROMPT;
use crate::config::CliConfig;
use crate::errors::ArcError;
use crate::goals::{GlobalParams, GoalParams, GoalType};
use crate::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct SelectAwsProfileTask;

#[async_trait]
impl Task for SelectAwsProfileTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Switch AWS Profile")?;
        Ok(())
    }

    async fn execute(
        &self,
        params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        _state: &State
    ) -> Result<GoalStatus, ArcError> {
        if let GoalParams::AwsProfileSelected{ use_current: true, .. } = params {
            // User wants to use current AWS_PROFILE, if it's already set
            if let Ok(current_profile) = env::var("AWS_PROFILE") {
                let account = get_aws_account(&current_profile).await?;
                let info = AwsProfileInfo::new(current_profile, account);
                let key = "Using current AWS profile".to_string();
                let outro_text = OutroText::single(key, info.name.clone());
                let task_result = TaskResult::AwsProfile{ profile: info, updated: false };
                return Ok(GoalStatus::Completed(task_result, outro_text));
            }
        }

        // Extract profile arg from params
        let profile: String = match params {
            GoalParams::AwsProfileSelected{ profile: p, .. } => p.to_string(),
            _ => Err(ArcError::invalid_goal_params(GoalType::AwsProfileSelected, params))?,
        };

        // Determine the name of the AWS profile to use
        let selected_aws_profile = if profile == PROMPT {
            // Prompt user to select an AWS profile
            prompt_for_aws_profile().await?
        } else {
            // An explicit profile was provided so let's validate that it exists in the AWS config
            let available_profiles = get_available_aws_profiles().await?;
            if available_profiles.contains(&profile) {
                profile
            } else {
                return Err(ArcError::AwsProfileError(format!(
                    "Profile '{}' not found. Available profiles: {}",
                    profile,
                    available_profiles.join(", ")
                )));
            }
        };

        // Set outro content
        let key = "Switched to AWS profile".to_string();
        let outro_text = OutroText::single(key, selected_aws_profile.clone());

        // Create task result
        let account_id = get_aws_account(&selected_aws_profile).await?;
        let info = AwsProfileInfo::new(selected_aws_profile, account_id);
        let task_result = TaskResult::AwsProfile{ profile: info, updated: true };

        Ok(GoalStatus::Completed(task_result, outro_text))
    }
}

#[derive(Debug)]
pub struct AwsProfileInfo {
    pub name: String,
    pub account: AwsAccount,
}

impl AwsProfileInfo {
    pub fn new(name: String, account: AwsAccount) -> AwsProfileInfo {
        AwsProfileInfo { name, account }
    }
}

async fn prompt_for_aws_profile() -> Result<String, ArcError> {
    let available_profiles = get_available_aws_profiles().await?;

    let mut menu = select("Select an AWS Profile");
    for profile in &available_profiles {
        menu = menu.item(profile, profile, "");
    }

    Ok(menu.interact()?.to_string())
}

async fn get_available_aws_profiles() -> Result<Vec<String>, ArcError> {
    let config_sections = aws::get_env_configs().await?;

    // Extract profile names
    let mut profile_names: Vec<String> = config_sections
        .profiles()
        .map(|s| s.to_string())
        .filter(|s| s != "default")
        .collect();

    if profile_names.is_empty() {
        panic!("No AWS profiles found");
    }

    profile_names.sort();
    Ok(profile_names)
}

async fn get_aws_account(profile_name: &str) -> Result<AwsAccount, ArcError> {
    let config_sections = aws::get_env_configs().await?;

    // Extract SSO account ID
    let profile = config_sections.get_profile(profile_name)
        .ok_or_else(|| ArcError::AwsProfileError(format!("Profile '{}' not found", profile_name)))?;
    let account_id = profile.get("sso_account_id")
        .ok_or_else(|| ArcError::AwsProfileError("sso_account_id not found in profile".to_string()))?;

    Ok(AwsAccount::from(account_id))
}
