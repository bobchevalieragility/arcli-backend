use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use openidconnect::http::header::{AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use crate::models::errors::ArcError;

#[derive(Debug, Deserialize)]
pub(crate) struct Installation {
    pub(crate) id: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct InstallationToken {
    pub(crate) token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Claims {
    pub(crate) iat: u64,
    pub(crate) exp: u64,
    pub(crate) iss: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GithubPr {
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) user: GithubUser,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GithubUser {
    pub(crate) login: String,
}

#[derive(Deserialize, Debug)]
pub struct GithubPrFile {
    pub(crate) filename: String,
    
    #[serde(default)]
    pub(crate) patch: Option<String>,
}

pub async fn get_installation_id(
    client: &reqwest::Client,
    app_id: &str,
    private_key: &str,
) -> Result<String, ArcError> {
    // Generate JWT (same as in get_github_app_token)
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let claims = Claims {
        iat: now,
        exp: now + 600,
        iss: app_id.to_string(),
    };

    let key = EncodingKey::from_rsa_pem(private_key.as_bytes())?;
    let jwt = encode(&Header::new(Algorithm::RS256), &claims, &key)?;

    // Get installations for this app
    let url = "https://api.github.com/app/installations";

    let response = client
        .get(url)
        .header(USER_AGENT, "rust-github-pr-list")
        .header("Accept", "application/vnd.github.v3+json")
        .header(AUTHORIZATION, format!("Bearer {}", jwt))
        .send()
        .await?;

    let installations: Vec<Installation> = response.json().await?;

    // Use the first installation (typically there's only one for org apps)
    installations
        .first()
        .map(|i| i.id.to_string())
        .ok_or_else(|| ArcError::UserInputError("No GitHub App installations found".to_string()))
}

pub async fn get_github_app_token(
    client: &reqwest::Client,
    app_id: &str,
    private_key: &str,
    installation_id: &str,
) -> Result<String, ArcError> {
    // Generate JWT
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let claims = Claims {
        iat: now,
        exp: now + 600, // JWT expires in 10 minutes
        iss: app_id.to_string(),
    };

    let key = EncodingKey::from_rsa_pem(private_key.as_bytes())?;
    let jwt = encode(&Header::new(Algorithm::RS256), &claims, &key)?;

    // Exchange JWT for installation access token
    let url = format!(
        "https://api.github.com/app/installations/{}/access_tokens",
        installation_id
    );

    let response = client
        .post(&url)
        .header(USER_AGENT, "rust-github-pr-list")
        .header("Accept", "application/vnd.github.v3+json")
        .header(AUTHORIZATION, format!("Bearer {}", jwt))
        .send()
        .await?;

    let token_response: InstallationToken = response.json().await?;
    Ok(token_response.token)
}