use aws_config::profile;
use aws_runtime::env_config::file::EnvConfigFiles;
use aws_runtime::env_config::section::EnvConfigSections;
use aws_types::os_shim_internal::{Env, Fs};
use errors::ArcError;

pub mod influx;
pub mod kube_service;
pub mod rds;
pub mod vault;
pub mod argo;
pub mod github;
pub mod organization;
pub mod state;
pub mod goals;
pub mod args;
pub mod config;
pub mod errors;
pub mod aws_profile;
pub mod kube_context;

pub(crate) async fn get_env_configs() -> Result<EnvConfigSections, ArcError> {
    // Use real filesystem and environment access
    let fs = Fs::real();
    let env = Env::real();

    // Load default profile files (~/.aws/config and ~/.aws/credentials)
    let config_files = EnvConfigFiles::default();

    // Load env config sections asynchronously
    let env_config_sections = profile::load(&fs, &env, &config_files, None).await?;
    Ok(env_config_sections)
}