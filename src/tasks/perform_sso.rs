use async_trait::async_trait;
use cliclack::{confirm, intro};
use sha1::{Sha1, Digest};
use std::process::Command;
use chrono::{DateTime, Utc};
use serde_json::Value;
use crate::{aws, GoalStatus, OutroText};
use crate::config::CliConfig;
use crate::errors::ArcError;
use crate::goals::{GlobalParams, GoalParams};
use crate::state::State;
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
        // Infer the expected SSO cache file path from the AWS config
        let cache_path = get_sso_cache_path().await?;

        if !is_sso_session_valid(&cache_path)? {
            let prompt = "SSO session is expired. Should I initiate login?";
            let should_continue: bool = confirm(prompt).interact()?;

            if should_continue {
                let status = Command::new("aws")
                    .arg("sso")
                    .arg("login")
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .status()?;
                if !status.success() {
                    return Err(ArcError::AwsSsoError("Failed to login to AWS SSO".to_string()));
                }
            } else {
                return Err(ArcError::AwsSsoExpired)
            }
        }

        Ok(GoalStatus::Completed(TaskResult::SsoSessionValid, OutroText::None))
    }
}

async fn get_sso_cache_path() -> Result<std::path::PathBuf, ArcError> {
    // Load AWS config from file
    let config_sections = aws::get_env_configs().await?;

    // Assume that there is only one SSO session configured
    let sso_session = config_sections.sso_sessions().next()
        .ok_or_else(|| ArcError::AwsSsoError("No SSO sessions found in config".to_string()))?;

    // Hash the URL using SHA1 to discover the cache file name
    let mut hasher = Sha1::new();
    hasher.update(sso_session.as_bytes());
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);

    // Add the filename to the path
    let mut path = cache_dir()?;
    path.push(hash_hex);
    path.set_extension("json");

    Ok(path)
}

fn cache_dir() -> Result<std::path::PathBuf, ArcError> {
    let mut cache_path = home::home_dir().ok_or_else(|| ArcError::HomeDirError)?;
    cache_path.push(".aws");
    cache_path.push("sso");
    cache_path.push("cache");

    Ok(cache_path)
}

fn is_sso_session_valid(cache_path: &std::path::PathBuf) -> Result<bool, ArcError> {
    if cache_path.exists() {
        // Read cache file and extract expiresAt field
        let data = std::fs::read_to_string(cache_path)?;
        let session: Value = serde_json::from_str(&data)?;
        let expiration_str = session["expiresAt"]
            .as_str()
            .ok_or_else(|| ArcError::AwsSsoError("Cache missing 'expiresAt' field".to_string()))?;

        // Convert expiration string to DateTime and evaluate
        let expiration = DateTime::parse_from_rfc3339(expiration_str)?
            .with_timezone(&Utc);
        if expiration > Utc::now() {
            return Ok(true);
        }
    }

    Ok(false)
}
