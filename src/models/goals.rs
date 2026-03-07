use std;
use std::convert::From;
use chrono::{DateTime, NaiveDate, Utc};
use crate::models::args::PROMPT;
use crate::models::aws_profile::AwsAccount;
use crate::models::log_level::LogLevel;
use crate::tasks::Task;
use crate::tasks::create_tab_completions::CreateTabCompletionsTask;
use crate::tasks::get_aws_secret::GetAwsSecretTask;
use crate::tasks::get_vault_secret::GetVaultSecretTask;
use crate::tasks::launch_influx::LaunchInfluxTask;
use crate::tasks::get_argo_app_statuses::GetArgoAppStatusesTask;
use crate::tasks::get_github_pr_files::GetGithubPrFilesTask;
use crate::tasks::perform_sso::PerformSsoTask;
use crate::tasks::port_forward::PortForwardTask;
use crate::tasks::influx_dump::InfluxDumpTask;
use crate::tasks::run_pgcli::RunPgcliTask;
use crate::tasks::select_actuator_service::SelectActuatorServiceTask;
use crate::tasks::select_aws_profile::SelectAwsProfileTask;
use crate::tasks::select_influx_instance::SelectInfluxInstanceTask;
use crate::tasks::select_kube_context::SelectKubeContextTask;
use crate::tasks::select_organization::SelectOrganizationTask;
use crate::tasks::select_rds_instance::SelectRdsInstanceTask;
use crate::tasks::logging::LoggingTask;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Goal {
    pub goal_type: GoalType,
    pub params: GoalParams,
    pub is_terminal_goal: bool,
}

impl Goal {
    fn new(goal_type: GoalType, params: GoalParams) -> Self {
        Goal { goal_type, params, is_terminal_goal: false }
    }

    fn new_terminal(goal_type: GoalType, params: GoalParams) -> Self {
        Goal { goal_type, params, is_terminal_goal: true }
    }

    pub fn actuator_service_selected() -> Self {
        Goal::new(GoalType::ActuatorServiceSelected, GoalParams::None)
    }

    pub fn aws_profile_selected(aws_profile: Option<String>) -> Self {
        let params = match aws_profile {
            Some(p) => GoalParams::AwsProfileSelected { profile: p.clone(), use_current: false },
            None => GoalParams::AwsProfileSelected { profile: PROMPT.to_string(), use_current: true },
        };
        Goal::new(GoalType::AwsProfileSelected, params)
    }

    pub fn terminal_aws_profile_selected(profile: impl Into<String>) -> Self {
        let params = GoalParams::AwsProfileSelected { profile: profile.into(), use_current: false };
        Goal::new_terminal(GoalType::AwsProfileSelected, params)
    }

    pub fn aws_secret_known(secret_name: String, aws_profile: Option<String>) -> Self {
        let params = GoalParams::AwsSecretKnown { name: Some(secret_name), aws_profile };
        Goal::new(GoalType::AwsSecretKnown, params)
    }

    pub fn terminal_aws_secret_known(name: Option<String>, aws_profile: Option<String>) -> Self {
        let params = GoalParams::AwsSecretKnown { name, aws_profile };
        Goal::new_terminal(GoalType::AwsSecretKnown, params)
    }

    pub fn github_pr_files_known(
        repo: String,
        pull_request: Option<u32>,
        lookback_duration: Option<std::time::Duration>,
    ) -> Self {
        let params = GoalParams::GithubPrFilesKnown { repo, pull_request, lookback_duration };
        Goal::new(GoalType::GithubPrFilesKnown, params)
    }

    pub fn influx_instance_selected(aws_profile: Option<String>) -> Self {
        let params = GoalParams::InfluxInstanceSelected { aws_profile };
        Goal::new(GoalType::InfluxInstanceSelected, params)
    }

    pub fn terminal_influx_launched(aws_profile: Option<String>) -> Self {
        let params = GoalParams::InfluxLaunched { aws_profile };
        Goal::new_terminal(GoalType::InfluxLaunched, params)
    }

    pub fn terminal_influx_dump_completed(
        day: Option<NaiveDate>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        output_dir: std::path::PathBuf,
        file_per_measurement: bool,
        aws_profile: Option<String>,
    ) -> Self {
        let params = GoalParams::InfluxDumpCompleted { day, start, end, output_dir, file_per_measurement, aws_profile };
        Goal::new_terminal(GoalType::InfluxDumpCompleted, params)
    }

    pub fn kube_context_selected(kube_context: Option<String>) -> Self {
        let params = match kube_context {
            Some(c) => GoalParams::KubeContextSelected { context: c.clone(), use_current: false },
            None => GoalParams::KubeContextSelected { context: PROMPT.to_string(), use_current: true },
        };
        Goal::new(GoalType::KubeContextSelected, params)
    }

    pub fn terminal_kube_context_selected(context: impl Into<String>) -> Self {
        let params = GoalParams::KubeContextSelected { context: context.into(), use_current: false };
        Goal::new_terminal(GoalType::KubeContextSelected, params)
    }

    pub fn terminal_log_level_known(
        service: Option<String>,
        package: String,
        kube_context: Option<String>,
    ) -> Self {
        let params = GoalParams::LogLevelKnown { service, package, kube_context };
        Goal::new_terminal(GoalType::LogLevelKnown, params)
    }

    pub fn terminal_log_level_set(
        service: Option<String>,
        package: String,
        level: Option<LogLevel>,
        kube_context: Option<String>,
    ) -> Self {
        let params = GoalParams::LogLevelSet { service, package, level, kube_context };
        Goal::new_terminal(GoalType::LogLevelSet, params)
    }

    pub fn organization_selected() -> Self {
        Goal::new(GoalType::OrganizationSelected, GoalParams::None)
    }

    pub fn terminal_pgcli_running(aws_profile: Option<String>) -> Self {
        let params = GoalParams::PgcliRunning { aws_profile };
        Goal::new_terminal(GoalType::PgcliRunning, params)
    }

    pub fn port_forward_established(service: String, kube_context: Option<String>) -> Self {
        let params = GoalParams::PortForwardEstablished {
            namespace: None,
            service: Some(service),
            port: None,
            group: None,
            tear_down: true,
            kube_context,
        };
        Goal::new(GoalType::PortForwardEstablished, params)
    }

    pub fn terminal_port_forward_established(
        namespace: Option<String>,
        service: Option<String>,
        port: Option<u16>,
        group: Option<String>,
        kube_context: Option<String>
    ) -> Self {
        let params = GoalParams::PortForwardEstablished { namespace, service, port, group, tear_down: false, kube_context };
        Goal::new_terminal(GoalType::PortForwardEstablished, params)
    }

    pub fn rds_instance_selected(aws_profile: Option<String>) -> Self {
        let params = GoalParams::RdsInstanceSelected { aws_profile };
        Goal::new(GoalType::RdsInstanceSelected, params)
    }

    pub fn sso_token_valid() -> Self {
        Goal::new(GoalType::SsoTokenValid, GoalParams::None)
    }

    pub fn terminal_tab_completions() -> Self {
        Goal::new_terminal(GoalType::TabCompletionsExist, GoalParams::None)
    }

    pub fn terminal_argo(pull_request: Option<u32>) -> Self {
        let params = GoalParams::ArgoStatusesKnown { pull_request };
        Goal::new_terminal(GoalType::ArgoStatusKnown, params)
    }

    pub fn vault_secret_known(secret_path: String, field: Option<String>, aws_account: Option<AwsAccount>, aws_profile: Option<String>) -> Self {
        let params = GoalParams::VaultSecretKnown {
            path: Some(secret_path),
            field,
            aws_account,
            aws_profile,
        };
        Goal::new(GoalType::VaultSecretKnown, params)
    }

    pub fn terminal_vault_secret_known(path: Option<String>, field: Option<String>, aws_profile: Option<String>) -> Self {
        let params = GoalParams::VaultSecretKnown { path, field, aws_account: None, aws_profile };
        Goal::new_terminal(GoalType::VaultSecretKnown, params)
    }
}

impl From<&Goal> for String {
    fn from(goal: &Goal) -> Self {
        format!("{:?}", goal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GoalType {
    ActuatorServiceSelected,
    ArgoStatusKnown,
    AwsProfileSelected,
    AwsSecretKnown,
    GithubPrFilesKnown,
    InfluxInstanceSelected,
    InfluxLaunched,
    InfluxDumpCompleted,
    KubeContextSelected,
    LogLevelKnown,
    LogLevelSet,
    OrganizationSelected,
    PgcliRunning,
    PortForwardEstablished,
    RdsInstanceSelected,
    SsoTokenValid,
    TabCompletionsExist,
    VaultSecretKnown,
}

impl GoalType {
    pub fn to_task(&self) -> Box<dyn Task> {
        match self {
            GoalType::ActuatorServiceSelected => Box::new(SelectActuatorServiceTask),
            GoalType::ArgoStatusKnown => Box::new(GetArgoAppStatusesTask),
            GoalType::AwsProfileSelected => Box::new(SelectAwsProfileTask),
            GoalType::AwsSecretKnown => Box::new(GetAwsSecretTask),
            GoalType::GithubPrFilesKnown => Box::new(GetGithubPrFilesTask),
            GoalType::InfluxInstanceSelected => Box::new(SelectInfluxInstanceTask),
            GoalType::InfluxLaunched => Box::new(LaunchInfluxTask),
            GoalType::InfluxDumpCompleted => Box::new(InfluxDumpTask),
            GoalType::KubeContextSelected => Box::new(SelectKubeContextTask),
            GoalType::LogLevelKnown => Box::new(LoggingTask),
            GoalType::LogLevelSet => Box::new(LoggingTask),
            GoalType::OrganizationSelected => Box::new(SelectOrganizationTask),
            GoalType::PgcliRunning => Box::new(RunPgcliTask),
            GoalType::PortForwardEstablished => Box::new(PortForwardTask),
            GoalType::RdsInstanceSelected => Box::new(SelectRdsInstanceTask),
            GoalType::SsoTokenValid => Box::new(PerformSsoTask),
            GoalType::TabCompletionsExist => Box::new(CreateTabCompletionsTask),
            GoalType::VaultSecretKnown => Box::new(GetVaultSecretTask),
        }
    }
}

impl From<GoalType> for String {
    fn from(goal_type: GoalType) -> Self {
        format!("{:?}", goal_type)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum GoalParams {
    ArgoStatusesKnown {
        pull_request: Option<u32>,
    },
    AwsProfileSelected {
        profile: String,
        use_current: bool,
    },
    AwsSecretKnown {
        name: Option<String>,
        aws_profile: Option<String>,
    },
    GithubPrFilesKnown {
        repo: String,
        pull_request: Option<u32>,
        lookback_duration: Option<std::time::Duration>,
    },
    InfluxDumpCompleted {
        day: Option<NaiveDate>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        output_dir: std::path::PathBuf,
        file_per_measurement: bool,
        aws_profile: Option<String>,
    },
    InfluxInstanceSelected {
        aws_profile: Option<String>,
    },
    InfluxLaunched {
        aws_profile: Option<String>,
    },
    KubeContextSelected {
        context: String,
        use_current: bool,
    },
    LogLevelKnown {
        service: Option<String>,
        package: String,
        kube_context: Option<String>,
    },
    LogLevelSet {
        service: Option<String>,
        package: String,
        level: Option<LogLevel>,
        kube_context: Option<String>,
    },
    None,
    PgcliRunning {
        aws_profile: Option<String>,
    },
    PortForwardEstablished {
        namespace: Option<String>,
        service: Option<String>,
        port: Option<u16>,
        group: Option<String>,
        tear_down: bool,
        kube_context: Option<String>,
    },
    RdsInstanceSelected {
        aws_profile: Option<String>,
    },
    VaultSecretKnown {
        path: Option<String>,
        field: Option<String>,
        aws_account: Option<AwsAccount>,
        aws_profile: Option<String>,
    },
}

impl From<&GoalParams> for String {
    fn from(params: &GoalParams) -> Self {
        format!("{:?}", params)
    }
}
