use std::collections::HashMap;
use url::Url;
use vaultrs::auth::oidc;
use vaultrs::client::VaultClientSettingsBuilder;
use vaultrs::kv2;
use crate::models::errors::ArcError;
use crate::models::vault::VaultInstance;
use crate::clients::{auth_success_response, extract_query_param};
use crate::keyrings::vault_keyring::VaultKeyring;
use crate::models::aws_profile::AwsAccount;

/// Client that wraps Vault API calls and handles token expiration
pub struct VaultClient {
    vault_instance: VaultInstance,
    secrets_namespace: Option<String>,
    keyring: VaultKeyring,
}

impl VaultClient {
    pub fn new(account: &AwsAccount) -> Self {
        let vault_instance = account.vault_instance();
        let secrets_namespace = vault_instance.secrets_namespace(account);
        let keyring = VaultKeyring::new(&vault_instance);

        Self { vault_instance, secrets_namespace, keyring  }
    }

    pub async fn guarded_list_paths(&self, parent_path: &str) -> Result<Vec<String>, ArcError> {
        let token = self.get_cached_token().await?;

        match self.list_paths(parent_path, &token).await {
            Ok(paths) => Ok(paths),
            Err(_) => {
                // Assume error is due to token being expired and re-login
                cliclack::log::warning("Cached Vault credentials expired or invalid. Initiating login flow...")?;
                let new_token = self.login().await?;

                // Retry the request with the new token
                self.list_paths(parent_path, &new_token).await
            }
        }
    }

    pub async fn guarded_read_secret(&self, path: &str) -> Result<String, ArcError> {
        let token = self.get_cached_token().await?;

        match self.read_secret(path, &token).await {
            Ok(secrets) => Ok(secrets),
            Err(_) => {
                // Assume error is due to token being expired and re-login
                cliclack::log::warning("Cached Vault credentials expired or invalid. Initiating login flow...")?;
                let new_token = self.login().await?;

                // Retry the request with the new token
                self.read_secret(path, &new_token).await
            }
        }
    }

    pub async fn guarded_read_secret_field(&self, path: &str, field: &str) -> Result<String, ArcError> {
        let token = self.get_cached_token().await?;

        match self.read_secret_field(path, field, &token).await {
            Ok(secret_field) => Ok(secret_field),
            Err(_) => {
                // Assume error is due to token being expired and re-login
                cliclack::log::warning("Cached Vault credentials expired or invalid. Initiating login flow...")?;
                let new_token = self.login().await?;

                // Retry the request with the new token
                self.read_secret_field(path, field, &new_token).await
            }
        }
    }

    async fn get_cached_token(&self) -> Result<String, ArcError> {
        match self.keyring.get_credentials() {
            Ok(cached_credentials) => {
                cliclack::log::info("Attempting to use cached Vault credentials.")?;
                Ok(cached_credentials.client_token)
            },
            Err(_) => {
                // Either no token in cache or it couldn't be deserialized
                cliclack::log::warning("Vault credentials not cached. Initiating login flow...")?;
                self.login().await
            }
        }
    }

    async fn list_paths(&self, parent_path: &str, token: &str) -> Result<Vec<String>, ArcError> {
        let client = create_vault_client(
            self.vault_instance.address(),
            self.secrets_namespace.clone(),
            Some(token.to_string()),
        );

        let items = kv2::list(&client, "kv-v2", parent_path).await?;

        // Collect all available sub-paths
        let child_paths: Vec<String> = items
            .iter()
            .map(|i|  format!("{}{}", parent_path, i))
            .collect();

        Ok(child_paths)
    }

    async fn read_secret(&self, path: &str, token: &str) -> Result<String, ArcError> {
        let client = create_vault_client(
            self.vault_instance.address(),
            self.secrets_namespace.clone(),
            Some(token.to_string()),
        );

        let secrets: HashMap<String, String> = kv2::read(&client, "kv-v2", path).await?;

        let all_fields = secrets.iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<String>>()
            .join("\n");

        Ok(all_fields)
    }

    async fn read_secret_field(&self, path: &str, field: &str, token: &str) -> Result<String, ArcError> {
        let client = create_vault_client(
            self.vault_instance.address(),
            self.secrets_namespace.clone(),
            Some(token.to_string()),
        );

        let secrets: HashMap<String, String> = kv2::read(&client, "kv-v2", path).await?;

        let secret_field = secrets.get(field)
            .ok_or_else(|| ArcError::UserInputError(format!("Field '{}' not found in secret '{}'", field, path)))?
            .to_string();

        Ok(secret_field)
    }

    async fn login(&self) -> Result<String, ArcError> {
        // Start a local HTTP server to listen for the OIDC callback
        let redirect_host = "localhost:8250";
        let redirect_uri = format!("http://{}/oidc/callback", redirect_host);
        let server = tiny_http::Server::http(redirect_host)?;

        // Retrieve the OIDC auth URL from Vault
        let client = create_vault_client(
            self.vault_instance.address(),
            self.vault_instance.oidc_namespace(),
            None
        );
        let auth_response = oidc::auth(
            &client,
            "oidc", // mount path
            &redirect_uri,
            self.vault_instance.oidc_role(),
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
        let response = auth_success_response("Vault")?;
        request.respond(response)?;

        // Complete the login with Vault using the captured parameters
        let token_auth = oidc::callback(
            &client,
            "oidc",
            state.as_str(),
            nonce.as_str(),
            code.as_str()
        ).await?;

        let token = token_auth.client_token;

        // Save Vault credentials to the keyring
        self.keyring.save_credentials(&token, token_auth.lease_duration, token_auth.renewable)?;

        Ok(token)
    }
}

pub fn create_vault_client(
    address: &str,
    namespace: Option<String>,
    token: Option<String>
) -> vaultrs::client::VaultClient {
    let settings = VaultClientSettingsBuilder::default()
        .address(address)
        .namespace(namespace)
        .token(token.unwrap_or_default())
        .build()
        .expect("Unable to build VaultClient settings");

    vaultrs::client::VaultClient::new(settings).expect("Vault Client creation failed")
}
