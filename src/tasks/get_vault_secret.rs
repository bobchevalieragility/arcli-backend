use async_trait::async_trait;
use cliclack::{intro, select};
use crate::tasks::{Task, TaskResult};
use crate::clients::vault_client::VaultClient;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, Goal, GoalParams, GoalType};
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::state::State;

#[derive(Debug)]
pub struct GetVaultSecretTask;

#[async_trait]
impl Task for GetVaultSecretTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Get Vault Secret")?;
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
            GoalParams::VaultSecretKnown { aws_profile, .. } => aws_profile.clone(),
            _ => None,
        };

        // If AWS profile info is not available, we need to wait for that goal to complete
        let profile_goal = Goal::aws_profile_selected(aws_profile);
        if !state.contains(&profile_goal) {
            return Ok(GoalStatus::Needs(profile_goal));
        }

        // Retrieve info about the desired AWS profile from state
        let profile_info = state.get_aws_profile_info(&profile_goal)?;

        // Create client for interacting with Vault
        let client = VaultClient::new(&profile_info.account);

        // Determine which secret to retrieve, prompting user if necessary
        let secret_path = match params {
            GoalParams::VaultSecretKnown{ path: Some(x), .. } => x.clone(),
            GoalParams::VaultSecretKnown{ path: None, .. } => prompt_for_secret_path(&client).await?,
            _ => return Err(ArcError::invalid_goal_params(GoalType::VaultSecretKnown, params)),
        };

        // Retrieve secret from Vault
        let (secret_value, outro_text) = match params {
            GoalParams::VaultSecretKnown{ field: Some(f), .. } => {
                // Extract a specific secret field
                let secret_field = client.guarded_read_secret_field(&secret_path, f).await?;
                let outro_msg = OutroText::multi(f.clone(), secret_field.clone());
                (secret_field, outro_msg)
            },
            GoalParams::VaultSecretKnown{ field: None, .. } => {
                // Concatenate k: v pairs into a single, newline-delimited string
                let all_fields = client.guarded_read_secret(&secret_path).await?;
                let prompt = "Secret Value".to_string();
                let outro_msg = OutroText::multi(prompt, all_fields.clone());
                (all_fields, outro_msg)
            },
            _ => return Err(ArcError::invalid_goal_params(GoalType::VaultSecretKnown, params)),
        };


        Ok(GoalStatus::Completed(TaskResult::VaultSecret(secret_value), outro_text))
    }
}

async fn prompt_for_secret_path(client: &VaultClient) -> Result<String, ArcError> {
    let mut current_path = String::new();

    while current_path.is_empty() || current_path.ends_with('/') {
        // Collect all available sub-paths
        let available_paths = client.guarded_list_paths(&current_path).await?;

        // Prompt user to select a path
        let mut menu = select("Select a secret path");
        for path in &available_paths {
            menu = menu.item(path, path, "");
        }
        current_path = menu.interact()?.to_string();
    }

    Ok(current_path.to_string())
}