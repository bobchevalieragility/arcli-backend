use async_trait::async_trait;
use cliclack::{intro, select};
use std::collections::HashMap;
use vaultrs::client::VaultClient;
use vaultrs::kv2;
use crate::tasks::{Task, TaskResult};
use crate::aws::vault;
use crate::errors::ArcError;
use crate::goals::{GlobalParams, Goal, GoalParams, GoalType};
use crate::{GoalStatus, OutroText};
use crate::config::CliConfig;
use crate::state::State;

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
        global_params: &GlobalParams,
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // If AWS profile info is not available, we need to wait for that goal to complete
        let profile_goal = Goal::aws_profile_selected(global_params);
        if !state.contains(&profile_goal) {
            return Ok(GoalStatus::Needs(profile_goal));
        }

        // If we haven't obtained a valid Vault token yet, we need to wait for that goal to complete
        let login_goal = Goal::vault_token_valid();
        if !state.contains(&login_goal) {
            return Ok(GoalStatus::Needs(login_goal));
        }

        // Retrieve info about the desired AWS profile from state
        let profile_info = state.get_aws_profile_info(&profile_goal)?;

        // Retrieve validated Vault token from state
        let token = state.get_vault_token(&login_goal)?;

        // Create Vault client using the token
        let aws_account = &profile_info.account;
        let vault_instance = aws_account.vault_instance();
        let client = vault::create_client(
            vault_instance.address(),
            vault_instance.secrets_namespace(aws_account),
            Some(token)
        );

        // Determine which secret to retrieve, prompting user if necessary
        let secret_path = match params {
            GoalParams::VaultSecretKnown{ path: Some(x), .. } => x.clone(),
            GoalParams::VaultSecretKnown{ path: None, .. } => prompt_for_secret_path(&client).await?,
            _ => return Err(ArcError::invalid_goal_params(GoalType::VaultSecretKnown, params)),
        };

        // Retrieve the secret key-value pairs from Vault
        let secrets: HashMap<String, String> = kv2::read(&client, "kv-v2", &secret_path).await?;

        // Optionally extract a specific field from the secret and format for display
        let (secret_value, outro_text) = match params {
            GoalParams::VaultSecretKnown{ field: Some(f), .. } => {
                // Extract specific field
                //TODO abstract this logic into a function
                let secret_field = match secrets.get(f) {
                    Some(value) => value.to_string(),
                    None => {
                        panic!("Field '{}' not found in secret at path '{}'", f, secret_path);
                    }
                };
                let outro_msg = OutroText::multi(f.clone(), secret_field.clone());
                (secret_field, outro_msg)
            },
            GoalParams::VaultSecretKnown{ field: None, .. } => {
                // Concatenate k: v pairs into a single, newline-delimited string
                //TODO abstract this logic into a function
                let full_secret = secrets.iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<String>>()
                    .join("\n");
                let prompt = "Secret Value".to_string();
                let outro_msg = OutroText::multi(prompt, full_secret.clone());
                (full_secret, outro_msg)
            },
            _ => return Err(ArcError::invalid_goal_params(GoalType::VaultSecretKnown, params)),
        };

        Ok(GoalStatus::Completed(TaskResult::VaultSecret(secret_value), outro_text))
    }
}

async fn prompt_for_secret_path(client: &VaultClient) -> Result<String, ArcError> {
    let mut current_path = String::new();

    while current_path.is_empty() || current_path.ends_with('/') {
        let items = kv2::list(client, "kv-v2", &current_path).await?;

        // Collect all available sub-paths
        let available_paths: Vec<String> = items
            .iter()
            .map(|i|  format!("{}{}", current_path, i))
            .collect();

        // Prompt user to select a path
        let mut menu = select("Select a secret path");
        for path in &available_paths {
            menu = menu.item(path, path, "");
        }
        current_path = menu.interact()?.to_string();
    }

    Ok(current_path.to_string())
}