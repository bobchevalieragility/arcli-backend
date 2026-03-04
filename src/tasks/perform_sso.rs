use std::path::PathBuf;
use async_trait::async_trait;
use aws_runtime::env_config::section::EnvConfigSections;
use cliclack::{confirm, intro};
use sha1::{Sha1, Digest};
use aws_sdk_ssooidc as ssooidc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tiny_http::{Server, Response};
use openidconnect::{CsrfToken, PkceCodeChallenge};
use url::Url;
use crate::{GoalStatus, OutroText};
use crate::models::get_env_configs;
use crate::models::config::CliConfig;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, GoalParams};
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct PerformSsoTask;

#[async_trait]
impl Task for PerformSsoTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Perform AWS SSO")?;
        Ok(())
    }

    async fn execute(
        &self,
        _params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        _state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Load AWS config to determine the name of the SSO session for currently selected profile
        let env_configs = get_env_configs().await?;
        let sso_session_name = get_sso_session_name(&env_configs).await?;

        // Infer the expected SSO token file path from the SSO session name
        let sso_token_path = get_hashed_cache_path(&sso_session_name)?;

        if !is_sso_session_valid(&sso_token_path)? {
            // First, try to refresh the token if we have a refresh token
            let sso_session = env_configs.sso_session(&sso_session_name)
                .ok_or_else(|| ArcError::AwsSsoError(format!("SSO session '{}' not found", sso_session_name)))?;
            let sso_region = sso_session.get("sso_region")
                .ok_or_else(|| ArcError::AwsSsoError("sso_region not found in sso_session".to_string()))?
                .to_string();

            if try_refresh_token(&sso_token_path, &sso_region).await? {
                // Successfully refreshed token
                return Ok(GoalStatus::Completed(TaskResult::SsoSessionValid, OutroText::None));
            }

            // Refresh token failed or unavailable, fall back to full OAuth flow
            let prompt = "SSO session is expired. Should I initiate login?";
            let should_continue: bool = confirm(prompt).interact()?;

            if should_continue {
                let sso_start_url = sso_session.get("sso_start_url")
                    .ok_or_else(|| ArcError::AwsSsoError("sso_start_url not found in sso_session".to_string()))?
                    .to_string();

                // Start local HTTP server to receive OAuth callback
                let redirect_host = "127.0.0.1:0";
                let http_server = Server::http(redirect_host)
                    .map_err(|e| ArcError::AwsSsoError(format!("Failed to start HTTP server: {}", e)))?;

                let port = match http_server.server_addr() {
                    tiny_http::ListenAddr::IP(socket_addr) => socket_addr.port(),
                    _ => return Err(ArcError::AwsSsoError("Unexpected server address type".to_string())),
                };
                let redirect_uri = format!("http://127.0.0.1:{}/oauth/callback", port);

                // Build client name following AWS CLI pattern
                let client_name = format!("botocore-client-{}", sso_session_name);

                // Check for cached client registration
                let registration_path = get_hashed_cache_path(&client_name)?;

                let registration = if registration_path.exists() && is_registration_valid(&registration_path)? {
                    read_cached_registration(&registration_path)?
                } else {
                    // Create AWS config with no credentials for anonymous SSO OIDC calls
                    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                        .region(aws_config::Region::new(sso_region.clone()))
                        .no_credentials()
                        .load()
                        .await;
                    let ssooidc_client = ssooidc::Client::new(&aws_config);

                    let register_response = ssooidc_client
                        .register_client()
                        .client_name(&client_name)
                        .client_type("public")
                        .grant_types("authorization_code")
                        .grant_types("refresh_token")
                        .redirect_uris(&redirect_uri)
                        .issuer_url(&sso_start_url)
                        .scopes("sso:account:access")
                        .send()
                        .await?;

                    let client_id = register_response.client_id()
                        .ok_or_else(|| ArcError::AwsSsoError("Missing client_id".to_string()))?
                        .to_string();
                    let client_secret = register_response.client_secret()
                        .ok_or_else(|| ArcError::AwsSsoError("Missing client_secret".to_string()))?
                        .to_string();
                    let expires_at_secs = register_response.client_secret_expires_at();
                    let registration_expires_at_dt = chrono::DateTime::from_timestamp(expires_at_secs, 0)
                        .ok_or_else(|| ArcError::AwsSsoError("Invalid expiration timestamp".to_string()))?;

                    // Format with Z suffix instead of timezone offset for Smithy compatibility
                    let registration_expires_at = registration_expires_at_dt
                        .format("%Y-%m-%dT%H:%M:%SZ")
                        .to_string();

                    let reg = ClientRegistrationCache {
                        client_id,
                        client_secret,
                        expires_at: registration_expires_at,
                        scopes: vec!["sso:account:access".to_string()],
                        grant_types: vec!["authorization_code".to_string(), "refresh_token".to_string()],
                    };

                    // Save registration cache
                    save_registration(&reg, &registration_path)?;
                    reg
                };

                // Create PKCE challenge (openidconnect library handles this)
                let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

                // Generate CSRF token
                let csrf_token = CsrfToken::new_random();

                // Manually build authorization URL since AWS SSO doesn't support OIDC discovery
                let auth_url = build_aws_sso_authorization_url(
                    &sso_region,
                    &registration.client_id,
                    &redirect_uri,
                    csrf_token.secret(),
                    &pkce_challenge,
                )?;

                // Automatically open the URL in the default browser
                if let Err(e) = webbrowser::open(&auth_url) {
                    eprintln!("Warning: Could not automatically open browser: {}", e);
                    eprintln!("Please manually execute 'models sso login'");
                }

                // Wait for OAuth callback with authorization code
                let (auth_code, returned_state) = wait_for_oauth_callback(http_server)?;

                // Verify state parameter
                if &returned_state != csrf_token.secret() {
                    return Err(ArcError::AwsSsoError("State parameter mismatch".to_string()));
                }

                // Exchange authorization code for tokens using AWS SDK
                // CreateToken is also an unauthenticated endpoint
                let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(aws_config::Region::new(sso_region.clone()))
                    .no_credentials()
                    .load()
                    .await;
                let ssooidc_client = ssooidc::Client::new(&aws_config);

                let sso_cache = exchange_code_for_token(
                    &ssooidc_client,
                    &registration,
                    &auth_code,
                    pkce_verifier.secret(),
                    &redirect_uri,
                    &sso_start_url,
                    &sso_region,
                ).await?;

                // Cache token using session name hash (AWS CLI Python behavior)
                save_token(&sso_cache, &sso_token_path).await?;
            } else {
                return Err(ArcError::AwsSsoExpired)
            }
        }

        Ok(GoalStatus::Completed(TaskResult::SsoSessionValid, OutroText::None))
    }
}

async fn get_sso_session_name(env_configs: &EnvConfigSections) -> Result<String, ArcError> {
    let selected = env_configs.selected_profile();
    let sso_session_name = env_configs.get_profile(selected)
        .ok_or_else(|| ArcError::AwsSsoError(format!("Profile '{}' not found", selected)))?
        .get("sso_session")
        .ok_or_else(|| ArcError::AwsProfileError("sso_session not found in profile".to_string()))?
        .to_string();

    Ok(sso_session_name)
}

fn get_hashed_cache_path(hash_item: &str) -> Result<PathBuf, ArcError> {
    // Hash the URL using SHA1 to discover the cache file name
    let mut hasher = Sha1::new();
    hasher.update(hash_item.as_bytes());
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);

    // Add the filename to the path
    let mut path = cache_dir()?;
    path.push(hash_hex);
    path.set_extension("json");

    Ok(path)
}

fn cache_dir() -> Result<PathBuf, ArcError> {
    let mut cache_path = home::home_dir().ok_or_else(|| ArcError::HomeDirError)?;
    cache_path.push(".aws");
    cache_path.push("sso");
    cache_path.push("cache");

    Ok(cache_path)
}

fn is_sso_session_valid(token_path: &PathBuf) -> Result<bool, ArcError> {
    if token_path.exists() {
        // Read token cache file and deserialize into SsoTokenCache struct
        let data = std::fs::read_to_string(token_path)?;
        let session: SsoTokenCache = serde_json::from_str(&data)?;

        // Parse datetime (handles both Z suffix and timezone offset formats)
        let expiration = DateTime::parse_from_rfc3339(&session.expires_at)?
            .with_timezone(&Utc);

        if expiration > Utc::now() {
            return Ok(true);
        }
    }

    Ok(false)
}

async fn try_refresh_token(
    sso_token_path: &PathBuf,
    sso_region: &str,
) -> Result<bool, ArcError> {
    // Check if the token cache file exists
    if !sso_token_path.exists() {
        return Ok(false);
    }

    // Read the cached token
    let data = std::fs::read_to_string(sso_token_path)?;
    let token_cache: SsoTokenCache = serde_json::from_str(&data)?;

    // Check if we have a refresh token
    let refresh_token = match token_cache.refresh_token {
        Some(rt) => rt,
        None => return Ok(false), // No refresh token available
    };

    // Check if the registration is still valid
    let registration_expires_at = DateTime::parse_from_rfc3339(&token_cache.registration_expires_at)?
        .with_timezone(&Utc);

    if registration_expires_at <= Utc::now() {
        return Ok(false); // Registration expired, need full re-auth
    }

    // Create AWS config with no credentials for anonymous SSO OIDC calls
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(sso_region.to_string()))
        .no_credentials()
        .load()
        .await;
    let ssooidc_client = ssooidc::Client::new(&aws_config);

    // Attempt to refresh the token
    let token_response = match ssooidc_client
        .create_token()
        .client_id(&token_cache.client_id)
        .client_secret(&token_cache.client_secret)
        .grant_type("refresh_token")
        .refresh_token(&refresh_token)
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return Ok(false), // Refresh failed, need full re-auth
    };

    // Extract new tokens from response
    let access_token = token_response.access_token()
        .ok_or_else(|| ArcError::AwsSsoError("Missing access_token in refresh response".to_string()))?
        .to_string();

    let new_refresh_token = token_response.refresh_token()
        .map(|s| s.to_string())
        .or(Some(refresh_token)); // Keep old refresh token if new one not provided

    // Calculate new expiration time
    let expires_in_secs = token_response.expires_in();
    let access_token_expires_at = Utc::now() + chrono::Duration::seconds(expires_in_secs as i64);
    let expires_at = access_token_expires_at
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    // Create updated token cache
    let updated_cache = SsoTokenCache {
        start_url: token_cache.start_url,
        region: token_cache.region,
        access_token,
        expires_at,
        client_id: token_cache.client_id,
        client_secret: token_cache.client_secret,
        registration_expires_at: token_cache.registration_expires_at,
        refresh_token: new_refresh_token,
    };

    // Save the updated token cache
    save_token(&updated_cache, sso_token_path).await?;

    Ok(true)
}

//TODO move this into a shared utils module
fn extract_query_param(url: &Url, key: &str) -> Result<String, ArcError> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| ArcError::UrlQueryParamError(url.clone(), key.to_string()))
}

fn build_aws_sso_authorization_url(
    sso_region: &str,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    pkce_challenge: &PkceCodeChallenge,
) -> Result<String, ArcError> {
    // Build OIDC endpoint URL - AWS SSO uses oidc.<region>.amazonaws.com
    let base_url = format!("https://oidc.{}.amazonaws.com/authorize", sso_region);

    // Get the code challenge as a string (without the trailing =)
    let code_challenge_str = pkce_challenge.as_str().trim_end_matches('=');

    // Build the full authorization URL with all required parameters
    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&state={}&code_challenge_method=S256&scope=sso%3Aaccount%3Aaccess&code_challenge={}",
        base_url,
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(state),
        urlencoding::encode(code_challenge_str)
    );

    Ok(auth_url)
}

fn wait_for_oauth_callback(server: Server) -> Result<(String, String), ArcError> {
    const TIMEOUT_SECS: u64 = 600; // 10 minutes
    let start = std::time::Instant::now();

    for request in server.incoming_requests() {
        if start.elapsed().as_secs() > TIMEOUT_SECS {
            return Err(ArcError::AwsSsoError("Timeout waiting for OAuth callback".to_string()));
        }

        let request_url = Url::parse(&format!("http://localhost{}", request.url()))?;

        // Check for errors first
        if let Ok(error) = extract_query_param(&request_url, "error") {
            // Send error response to browser
            let html = format!(r#"
                <!DOCTYPE html>
                <html>
                <head><title>AWS SSO Login</title></head>
                <body>
                    <h1>Authorization Failed</h1>
                    <p>OAuth error: {}</p>
                    <p>You may close this window and return to the CLI.</p>
                </body>
                </html>
            "#, error);

            let response = Response::from_string(html)
                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap());
            let _ = request.respond(response);

            return Err(ArcError::AwsSsoError(format!("OAuth error: {}", error)));
        }

        // Extract authorization code and state
        let code = extract_query_param(&request_url, "code")?;
        let state = extract_query_param(&request_url, "state")?;

        // Send success response to browser
        let response = crate::clients::auth_success_response("SSO")?;
        let _ = request.respond(response);

        return Ok((code, state));
    }

    Err(ArcError::AwsSsoError("Server closed without receiving auth code".to_string()))
}

async fn exchange_code_for_token(
    client: &ssooidc::Client,
    registration: &ClientRegistrationCache,
    auth_code: &str,
    code_verifier: &str,
    redirect_uri: &str,
    sso_start_url: &str,
    sso_region: &str,
) -> Result<SsoTokenCache, ArcError> {
    let token_response = client
        .create_token()
        .client_id(&registration.client_id)
        .client_secret(&registration.client_secret)
        .grant_type("authorization_code")
        .code(auth_code)
        .code_verifier(code_verifier)
        .redirect_uri(redirect_uri)
        .send()
        .await?;

    let access_token = token_response.access_token()
        .ok_or_else(|| ArcError::AwsSsoError("Missing access_token in response".to_string()))?
        .to_string();

    let refresh_token = token_response.refresh_token()
        .map(|s| s.to_string());

    // Calculate expiration times
    let expires_in_secs = token_response.expires_in();
    let access_token_expires_at = Utc::now() + chrono::Duration::seconds(expires_in_secs as i64);

    // Format datetime as RFC3339 with Z suffix (no timezone offset) for Smithy compatibility
    let expires_at = access_token_expires_at
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let sso_cache = SsoTokenCache {
        start_url: sso_start_url.to_string(),
        region: sso_region.to_string(),
        access_token,
        expires_at,
        client_id: registration.client_id.clone(),
        client_secret: registration.client_secret.clone(),
        registration_expires_at: registration.expires_at.clone(),
        refresh_token,
    };

    Ok(sso_cache)
}

fn is_registration_valid(path: &PathBuf) -> Result<bool, ArcError> {
    if !path.exists() {
        return Ok(false);
    }

    let data = std::fs::read_to_string(path)?;
    let registration: ClientRegistrationCache = serde_json::from_str(&data)?;

    let expiration = DateTime::parse_from_rfc3339(&registration.expires_at)?
        .with_timezone(&Utc);

    Ok(expiration > Utc::now())
}

fn read_cached_registration(path: &PathBuf) -> Result<ClientRegistrationCache, ArcError> {
    let data = std::fs::read_to_string(path)?;
    let registration: ClientRegistrationCache = serde_json::from_str(&data)?;
    Ok(registration)
}

fn save_registration(registration: &ClientRegistrationCache, path: &PathBuf) -> Result<(), ArcError> {
    // Create client registration cache directory if it doesn't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, serde_json::to_string(&registration)?)?;
    Ok(())
}

async fn save_token(token_cache: &SsoTokenCache, path: &PathBuf) -> Result<(), ArcError> {
    // Create token cache directory if it doesn't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, serde_json::to_string(&token_cache)?)?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct SsoTokenCache {
    #[serde(rename = "startUrl")]
    start_url: String,
    region: String,
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresAt")]
    expires_at: String,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "clientSecret")]
    client_secret: String,
    #[serde(rename = "registrationExpiresAt")]
    registration_expires_at: String,
    #[serde(rename = "refreshToken", skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ClientRegistrationCache {
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "clientSecret")]
    client_secret: String,
    #[serde(rename = "expiresAt")]
    expires_at: String,
    scopes: Vec<String>,
    #[serde(rename = "grantTypes")]
    grant_types: Vec<String>,
}
