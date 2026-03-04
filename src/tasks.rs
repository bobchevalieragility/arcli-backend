pub mod create_tab_completions;
pub mod get_aws_secret;
pub mod get_vault_secret;
pub mod launch_influx;
pub mod perform_sso;
pub mod port_forward;
pub mod influx_dump;
pub mod run_pgcli;
pub mod select_actuator_service;
pub mod select_aws_profile;
pub mod select_influx_instance;
pub mod select_kube_context;
pub mod select_organization;
pub mod select_rds_instance;
pub mod set_log_level;
pub mod get_argo_app_statuses;
pub mod get_github_pr_files;
pub mod select_argo_instance;

use async_trait::async_trait;
use cliclack::progress_bar;
use std::collections::BTreeMap;
use crate::{GoalStatus, State};
use crate::models::influx::InfluxInstance;
use crate::models::argo::{AppInfo, ArgoCdInstance};
use crate::models::aws_profile::AwsProfileInfo;
use crate::models::github::GithubPrFile;
use crate::models::rds::RdsInstance;
use crate::models::config::CliConfig;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, GoalParams};
use crate::models::organization::Organization;
use crate::tasks::port_forward::PortForwardInfo;
use crate::tasks::select_actuator_service::ActuatorService;
use crate::models::kube_context::KubeContextInfo;

#[async_trait]
pub trait Task: Send + Sync {
    fn print_intro(&self) -> Result<(), ArcError>;
    async fn execute(
        &self,
        params: &GoalParams,
        config: &CliConfig,
        global_params: &GlobalParams,
        state: &State
    ) -> Result<GoalStatus, ArcError>;
}

#[derive(Debug)]
pub enum TaskResult {
    ActuatorService(ActuatorService),
    ArgoAppStatuses(BTreeMap<String, AppInfo>),
    ArgoInstance(ArgoCdInstance),
    AwsProfile{ profile: AwsProfileInfo, updated: bool },
    AwsSecret(String),
    GithubPrFiles(Vec<GithubPrFile>),
    InfluxCommand,
    InfluxInstance(InfluxInstance),
    InfluxDumpCompleted,
    KubeContext{ context: KubeContextInfo, updated: bool },
    LogLevel,
    Organization(Organization),
    PgcliCommand(String),
    PortForward(Vec<PortForwardInfo>),
    RdsInstance(RdsInstance),
    SsoSessionValid,
    TabCompletionsCreated,
    VaultSecret(String),
}

impl TaskResult {
    pub fn eval_string(&self) -> Option<String> {
        match self {
            TaskResult::AwsProfile{ profile: AwsProfileInfo { name, .. }, updated: true } => {
                Some(format!("export AWS_PROFILE={name}\n"))
            },
            TaskResult::KubeContext{ context: KubeContextInfo { kubeconfig, .. }, updated: true } => {
                let path = kubeconfig.to_string_lossy();
                Some(format!("export KUBECONFIG={path}\n"))
            },
            TaskResult::PgcliCommand(cmd) => {
                Some(format!("{cmd}\n"))
            },
            _ => None,
        }
    }
}

impl From<&TaskResult> for String {
    fn from(result: &TaskResult) -> Self {
        format!("{:?}", result)
    }
}

pub async fn sleep_indicator(seconds: u64, start_msg: &str, end_msg: &str) {
    let progress = progress_bar(seconds).with_spinner_template();
    progress.start(start_msg);

    let sleep_duration = tokio::time::Duration::from_secs(2);
    let steps = 100;
    let step_duration = sleep_duration / steps;

    for i in 0..=steps {
        progress.inc(1);
        if i < steps {
            tokio::time::sleep(step_duration).await;
        }
    }

    progress.stop(end_msg);
}
