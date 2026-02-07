use async_trait::async_trait;
use cliclack::intro;
use std::fs;
use url::Url;
use vaultrs::auth::oidc;
use vaultrs::token;
use crate::{config_dir, GoalStatus, OutroText};
use crate::aws::vault;
use crate::aws::vault::VaultInstance;
use crate::config::CliConfig;
use crate::errors::ArcError;
use crate::goals::{GlobalParams, Goal, GoalParams};
use crate::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct LoginToVaultTask;

#[async_trait]
impl Task for LoginToVaultTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Login to Vault")?;
        Ok(())
    }

    async fn execute(
        &self,
        _params: &GoalParams,
        _config: &CliConfig,
        global_params: &GlobalParams,
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // If AWS profile info is not available, we need to wait for that goal to complete
        let profile_goal = Goal::aws_profile_selected(global_params);
        if !state.contains(&profile_goal) {
            return Ok(GoalStatus::Needs(profile_goal));
        }

        // Retrieve info about the desired AWS profile from state
        let profile_info = state.get_aws_profile_info(&profile_goal)?;

        // Check for existing local Vault token
        let vault_instance = profile_info.account.vault_instance();
        if let Some(token) = read_token_file() {
            // A local Vault token already exists, let's add it to a client to see if it is expired
            let client = vault::create_client(
                vault_instance.address(),
                None,
                Some(token.clone())
            );

            // We use the lookup_self endpoint to check token validity
            if let Ok(token_info) = token::lookup_self(&client).await {
                if token_info.ttl > 0 {
                    // Existing token is still valid, so let's use it
                    cliclack::log::info("Using existing Vault token")?;
                    return Ok(GoalStatus::Completed(TaskResult::VaultToken(token), OutroText::None));
                }
            }
        }

        // If we made it this far, then we need to re-login to Vault via OIDC
        let token = vault_login(&vault_instance).await?;
        save_token_file(&token)?;

        cliclack::log::info("Successfully logged into Vault")?;
        Ok(GoalStatus::Completed(TaskResult::VaultToken(token), OutroText::None))
    }
}

fn vault_token_path() -> Result<std::path::PathBuf, ArcError> {
    let mut path = config_dir()?;
    path.push("vault_token");
    Ok(path)
}

fn read_token_file() -> Option<String> {
    let token_path = vault_token_path();
    match token_path {
        Ok(path) => {
            match fs::read_to_string(path) {
                Ok(token) => Some(token.trim().to_string()),
                Err(_) => None,
            }
        },
        Err(_) => return None,
    }
}

fn save_token_file(token: &str) -> Result<(), ArcError> {
    let token_path = vault_token_path()?;
    fs::write(token_path, token)?;
    Ok(())
}

async fn vault_login(vault_instance: &VaultInstance) -> Result<String, ArcError> {
    // Start a local HTTP server to listen for the OIDC callback
    let redirect_host = "localhost:8250";
    let redirect_uri = format!("http://{}/oidc/callback", redirect_host);
    let server = tiny_http::Server::http(redirect_host)?;

    // Retrieve the OIDC auth URL from Vault
    let client = vault::create_client(
        vault_instance.address(),
        vault_instance.oidc_namespace(),
        None
    );
    let auth_response = oidc::auth(
        &client,
        "oidc", // mount path
        &redirect_uri,
        vault_instance.oidc_role(),
    ).await?;

    // Extract nonce from auth URL
    let url = Url::parse(&auth_response.auth_url)?;
    let nonce = extract_query_param(&url, "nonce")?;

    // Open the user's default web browser to the auth URL
    webbrowser::open(&auth_response.auth_url)?;

    // Wait for the OIDC callback request
    let request = server.recv()?;

    // The request URL is relative, so we need to construct the absolute URL
    let base_url_str = format!("http://{}/", redirect_host);
    let base_url = Url::parse(&base_url_str)?;
    let absolute_request_url = base_url.join(request.url())?;

    // Extract code and state from the request URL
    let code = extract_query_param(&absolute_request_url, "code")?;
    let state = extract_query_param(&absolute_request_url, "state")?;

    // Respond to the user in the browser
    let response = tiny_http::Response::from_string(
        "Authentication successful! You can close this tab."
    );
    request.respond(response)?;

    // Complete the login with Vault using the captured parameters
    let token_auth = oidc::callback(
        &client,
        "oidc",
        state.as_str(),
        nonce.as_str(),
        code.as_str()
    ).await?;

    Ok(token_auth.client_token)
}

fn extract_query_param(url: &Url, key: &str) -> Result<String, ArcError> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| ArcError::UrlQueryParamError(url.clone(), key.to_string()))
}
