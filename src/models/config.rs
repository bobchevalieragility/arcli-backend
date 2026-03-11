use serde::Deserialize;
use std::path::PathBuf;
use crate::models::errors::ArcError;

#[derive(Debug, Deserialize)]
pub struct CliConfig {
    #[serde(default)]
    pub(crate) bazel: BazelConfig,

    #[serde(default, rename = "port-forward")]
    pub(crate) port_forward: PortForwardConfig,
}

impl Default for CliConfig {
    fn default() -> Self {
        CliConfig {
            bazel: BazelConfig::default(),
            port_forward: PortForwardConfig { groups: Vec::new() },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct BazelConfig {
    agility_software_repo: Option<String>,
}

impl BazelConfig {
    /// Returns the absolute path to the agility-software repository.
    /// Expands tilde (~) if present and validates the configuration.
    pub fn agility_software_repo(&self) -> Result<PathBuf, ArcError> {
        let repo_path = self.agility_software_repo
            .as_ref()
            .ok_or_else(|| ArcError::invalid_config_error("bazel.agility_software_repo is not configured"))?;

        // Expand tilde if present
        let path = if repo_path.starts_with("~/") {
            let home_dir = home::home_dir().ok_or_else(|| ArcError::HomeDirError)?;
            home_dir.join(&repo_path[2..])
        } else {
            PathBuf::from(repo_path)
        };

        Ok(path)
    }
}

impl Default for BazelConfig {
    fn default() -> Self {
        BazelConfig {
            agility_software_repo: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PortForwardConfig {
    pub(crate) groups: Vec<ServiceGroup>,
}

impl Default for PortForwardConfig {
    fn default() -> Self {
        PortForwardConfig { groups: Vec::new() }
    }
}

#[derive(Debug, Deserialize)]
pub struct ServiceGroup {
    pub(crate) name: String,
    pub(crate) services: Vec<Service>,
}

#[derive(Debug, Deserialize)]
pub struct Service {
    pub(crate) name: String,
    pub(crate) namespace: String,
    pub(crate) local_port: u16,
}