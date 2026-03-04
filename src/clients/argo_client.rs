use std::collections::BTreeMap;
use std::collections::HashMap;
use openidconnect::core::{CoreClient, CoreProviderMetadata, CoreResponseType};
use openidconnect::{AuthenticationFlow, ClientId, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge, RedirectUrl, Scope};
use reqwest::Client;
use url::Url;
use crate::models::argo::{AppInfo, ArgoCdInstance, ArgoTokenResponse, ArgoApplicationList, ArgocdSettings};
use crate::clients::{auth_success_response, extract_query_param};
use crate::models::errors::ArcError;
use crate::keyrings::argo_keyring::ArgoKeyring;

/// Client that wraps ArgoCD API calls and handles token expiration
pub struct ArgoClient {
    instance: ArgoCdInstance,
    client: Client,
    keyring: ArgoKeyring,
}

impl ArgoClient {
    pub fn new(instance: ArgoCdInstance) -> Result<Self, ArcError> {
        // Create a re-usable HTTP client that accepts invalid TLS certs since many ArgoCD instances use self-signed certs
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;
        let keyring = ArgoKeyring::new(&instance);

        Ok(Self { instance, client, keyring })
    }

    pub async fn fetch_apps(
        &self,
        project: &str,
        target_versions: &HashMap<String, String>,
    ) -> Result<BTreeMap<String, AppInfo>, ArcError> {
        let argo_api_url = format!("{}/api/v1/applications?projects={project}", self.instance.base_url());
        let resp = self.guarded_fetch(&argo_api_url).await?;

        let app_infos: BTreeMap<String, AppInfo> = serde_json::from_str::<ArgoApplicationList>(&resp.to_string())?.items
            .into_iter()
            .filter(|app| target_versions.contains_key(app.metadata.name.as_str()))
            .map(|app| {
                let app_info: AppInfo = app.into();
                (app_info.name.clone(), app_info)
            })
            .collect();

        Ok(app_infos)
    }

    async fn guarded_fetch(&self, url: &str) -> Result<serde_json::Value, ArcError> {
        let token = match self.keyring.get_credentials() {
            Ok(cached_credentials) => {
                Ok(cached_credentials.id_token)
            },
            Err(_) => {
                // Either no token in cache or it couldn't be deserialized
                cliclack::log::warning("ArgoCD credentials not cached. Initiating login flow...")?;
                self.login().await
            }
        }?;

        let response = self.fetch(url, &token).await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if status == 401 {
                // Token is expired, re-login
                cliclack::log::warning("Cached ArgoCD credentials expired. Initiating login flow...")?;
                let new_token = self.login().await?;

                // Retry the request with the new token
                let response = self.fetch(url, &new_token).await?;
                if response.status().is_success() {
                    return Ok(response.json().await?);
                }
            }

            return Err(ArcError::UserInputError(format!("ArgoCD API error {}: {}", status, body)));
        }

        Ok(response.json().await?)
    }

    async fn fetch(&self, url: &str, token: &str) -> Result<reqwest::Response, ArcError> {
        let request = self.client.get(url)
            .header(reqwest::header::USER_AGENT, "arc-backend")
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token));

        let response = request.send().await?;
        Ok(response)
    }

    async fn login(&self) -> Result<String, ArcError> {
        // Start a local HTTP server to listen for the OIDC callback
        let redirect_host = "localhost:8085";
        let redirect_uri = format!("http://{}/auth/callback", redirect_host);
        let http_server = tiny_http::Server::http(redirect_host)?;

        // Create an HTTP client that accepts self-signed certificates (--insecure)
        let http_client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        // Fetch ArgoCD's OIDC settings
        let settings_url = format!("{}/api/v1/settings", self.instance.base_url());
        let settings_response = http_client.get(&settings_url).send().await?;

        // Get the full response as text in case we want to inspect it
        let response_text = settings_response.text().await?;

        // Parse the response text as JSON to extract OIDC settings
        let settings: ArgocdSettings = serde_json::from_str(&response_text)?;

        // Extract OIDC configuration
        let oidc_config = settings.oidc_config
            .ok_or_else(|| ArcError::UserInputError("ArgoCD server does not have OIDC configured. SSO login requires OIDC to be enabled.".to_string()))?;

        let issuer_url = IssuerUrl::new(
            oidc_config
                .issuer
                .ok_or_else(|| ArcError::UserInputError("No OIDC issuer in ArgoCD settings".to_string()))?
        )?;

        let client_id = oidc_config
            .cli_client_id
            .ok_or_else(|| ArcError::UserInputError("No OIDC client ID in ArgoCD settings".to_string()))?;

        // Discover OIDC provider metadata
        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client).await?;

        let token_endpoint_url = provider_metadata
            .token_endpoint()
            .ok_or_else(|| ArcError::UserInputError("No token endpoint in provider metadata".to_string()))?
            .url()
            .to_string();

        // Create OIDC client
        let oidc_client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(client_id),
            None,
        ).set_redirect_uri(RedirectUrl::new(redirect_uri.clone())?);

        // Create PKCE challenge
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Build authorization URL
        let (auth_url, _csrf_token, _nonce) = oidc_client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("groups".to_string()))
            .add_scope(Scope::new("offline_access".to_string()))  // Request refresh token
            .set_pkce_challenge(pkce_challenge)
            .url();

        // Open the user's browser for authentication
        webbrowser::open(auth_url.as_str())?;

        // Wait for the OIDC callback
        let request = http_server.recv()?;
        let request_url = Url::parse(&format!("http://localhost{}", request.url()))?;
        let code = extract_query_param(&request_url, "code")?;

        // Exchange authorization code for tokens
        let token_params = vec![
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("client_id", oidc_client.client_id().as_str()),
            ("code_verifier", pkce_verifier.secret()),
        ];

        let token_response = http_client
            .post(&token_endpoint_url)
            .form(&token_params)
            .send()
            .await?;

        let token_data: ArgoTokenResponse = token_response.json().await?;
        let id_token = token_data.id_token
            .ok_or_else(|| ArcError::UserInputError("No ID token in response".to_string()))?;

        // Save ArgoCD credentials to the keyring
        self.keyring.save_credentials(&id_token, token_data.refresh_token, token_data.expires_in)?;

        // Respond to the browser
        let response = auth_success_response("ArgoCD")?;
        request.respond(response)?;

        Ok(id_token)
    }
}