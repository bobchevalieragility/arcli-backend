use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_secretsmanager::Client;
use aws_types::region::Region;
use cliclack::{intro, select};
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, Goal, GoalParams, GoalType};
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct GetAwsSecretTask;

#[async_trait]
impl Task for GetAwsSecretTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Get AWS Secret")?;
        Ok(())
    }

    async fn execute(
        &self,
        params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Ensure that SSO token has not expired
        let sso_goal = Goal::sso_token_valid();
        if !state.contains(&sso_goal) {
            return Ok(GoalStatus::Needs(sso_goal));
        }

        // Extract aws_profile arg from params
        let aws_profile = match params {
            GoalParams::AwsSecretKnown { aws_profile, .. } => aws_profile.clone(),
            _ => None,
        };

        // If AWS profile info is not available, we need to wait for that goal to complete
        let profile_goal = Goal::aws_profile_selected(aws_profile);
        if !state.contains(&profile_goal) {
            return Ok(GoalStatus::Needs(profile_goal));
        }

        // Retrieve info about the selected AWS profile from state
        let profile_info = state.get_aws_profile_info(&profile_goal)?;

        // Create AWS Secrets Manager client with the selected profile
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(profile_info.region.clone()))
            .profile_name(&profile_info.name)
            .load()
            .await;
        let client = Client::new(&aws_config);

        // Determine which secret to retrieve, prompting user if necessary
        let secret_name = match params {
            GoalParams::AwsSecretKnown{ name: Some(x), .. } => x.clone(),
            GoalParams::AwsSecretKnown{ name: None, .. } => prompt_for_aws_secret(&client).await?,
            _ => return Err(ArcError::invalid_goal_params(GoalType::AwsSecretKnown, params)),
        };

        // Retrieve the secret value
        let resp = client.get_secret_value()
            .secret_id(&secret_name)
            .send()
            .await;
        let secret_value = resp?.secret_string
            .ok_or_else(|| ArcError::UnparseableSecret(secret_name))?;

        let key = "Secret Value".to_string();
        let outro_text = OutroText::single(key, secret_value.clone());
        Ok(GoalStatus::Completed(TaskResult::AwsSecret(secret_value), outro_text))
    }
}

async fn prompt_for_aws_secret(client: &Client) -> Result<String, ArcError> {
    let available_secrets = get_available_secrets(client).await?;

    let mut menu = select("Select a secret to retrieve?");
    for secret in &available_secrets {
        menu = menu.item(secret, secret, "");
    }

    Ok(menu.interact()?.to_string())
}

async fn get_available_secrets(client: &Client) -> Result<Vec<String>, ArcError> {
    // List secrets asynchronously
    let paginator = client.list_secrets().into_paginator();
    let pages: Vec<_> = paginator.send().collect::<Vec<_>>().await;

    // Process the results
    let mut all_secrets: Vec<String> = Vec::new();
    for page_result in pages {
        let page = page_result?;
        let secrets: Vec<String> = page.secret_list()
            .iter()
            .filter_map(|e| e.name.clone())
            .collect();
        all_secrets.extend(secrets);
    }

    if all_secrets.is_empty() {
        panic!("No AWS secrets found");
    }

    all_secrets.sort();
    Ok(all_secrets)
}