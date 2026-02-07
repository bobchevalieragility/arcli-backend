use std;
use std::convert::From;
use chrono::{DateTime, NaiveDate, Utc};
use crate::args::PROMPT;
use crate::tasks::Task;
use crate::tasks::create_tab_completions::CreateTabCompletionsTask;
use crate::tasks::get_aws_secret::GetAwsSecretTask;
use crate::tasks::get_vault_secret::GetVaultSecretTask;
use crate::tasks::launch_influx::LaunchInfluxTask;
use crate::tasks::login_to_vault::LoginToVaultTask;
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
use crate::tasks::set_log_level::{Level, SetLogLevelTask};

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

    pub fn aws_profile_selected(global_params: &GlobalParams) -> Self {
        let params = match &global_params.aws_profile {
            Some(p) => GoalParams::AwsProfileSelected { profile: p.clone(), use_current: false },
            None => GoalParams::AwsProfileSelected { profile: PROMPT.to_string(), use_current: true },
        };
        Goal::new(GoalType::AwsProfileSelected, params)
    }

    pub fn terminal_aws_profile_selected(profile: impl Into<String>) -> Self {
        let params = GoalParams::AwsProfileSelected { profile: profile.into(), use_current: false };
        Goal::new_terminal(GoalType::AwsProfileSelected, params)
    }

    pub fn aws_secret_known(secret_name: String) -> Self {
        let params = GoalParams::AwsSecretKnown { name: Some(secret_name) };
        Goal::new(GoalType::AwsSecretKnown, params)
    }

    pub fn terminal_aws_secret_known(name: Option<String>) -> Self {
        let params = GoalParams::AwsSecretKnown { name };
        Goal::new_terminal(GoalType::AwsSecretKnown, params)
    }

    pub fn influx_instance_selected() -> Self {
        Goal::new(GoalType::InfluxInstanceSelected, GoalParams::None)
    }

    pub fn terminal_influx_launched() -> Self {
        Goal::new_terminal(GoalType::InfluxLaunched, GoalParams::None)
    }

    pub fn terminal_influx_dump_completed(
        day: Option<NaiveDate>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        output: std::path::PathBuf
    ) -> Self {
        let params = GoalParams::InfluxDumpCompleted { day, start, end, output };
        Goal::new_terminal(GoalType::InfluxDumpCompleted, params)
    }

    pub fn kube_context_selected(global_params: &GlobalParams) -> Self {
        let params = match &global_params.kube_context {
            Some(c) => GoalParams::KubeContextSelected { context: c.clone(), use_current: false },
            None => GoalParams::KubeContextSelected { context: PROMPT.to_string(), use_current: true },
        };
        Goal::new(GoalType::KubeContextSelected, params)
    }

    pub fn terminal_kube_context_selected(context: impl Into<String>) -> Self {
        let params = GoalParams::KubeContextSelected { context: context.into(), use_current: false };
        Goal::new_terminal(GoalType::KubeContextSelected, params)
    }

    pub fn terminal_log_level_set(
        service: Option<String>,
        package: String,
        level: Option<Level>,
        display_only: bool
    ) -> Self {
        let params = GoalParams::LogLevelSet { service, package, level, display_only };
        Goal::new_terminal(GoalType::LogLevelSet, params)
    }

    pub fn organization_selected() -> Self {
        Goal::new(GoalType::OrganizationSelected, GoalParams::None)
    }

    pub fn terminal_pgcli_running() -> Self {
        Goal::new_terminal(GoalType::PgcliRunning, GoalParams::None)
    }

    pub fn port_forward_established(service: String) -> Self {
        let params = GoalParams::PortForwardEstablished {
            service: Some(service),
            port: None,
            group: None,
            tear_down: true
        };
        Goal::new(GoalType::PortForwardEstablished, params)
    }

    pub fn terminal_port_forward_established(service: Option<String>, port: Option<u16>, group: Option<String>) -> Self {
        let params = GoalParams::PortForwardEstablished { service, port, group, tear_down: false };
        Goal::new_terminal(GoalType::PortForwardEstablished, params)
    }

    pub fn rds_instance_selected() -> Self {
        Goal::new(GoalType::RdsInstanceSelected, GoalParams::None)
    }

    pub fn sso_token_valid() -> Self {
        Goal::new(GoalType::SsoTokenValid, GoalParams::None)
    }

    pub fn terminal_tab_completions() -> Self {
        Goal::new_terminal(GoalType::TabCompletionsExist, GoalParams::None)
    }

    pub fn vault_secret_known(secret_path: String, secret_field: String) -> Self {
        let params = GoalParams::VaultSecretKnown {
            path: Some(secret_path),
            field: Some(secret_field)
        };
        Goal::new(GoalType::VaultSecretKnown, params)
    }

    pub fn vault_token_valid() -> Self {
        Goal::new(GoalType::VaultTokenValid, GoalParams::None)
    }

    pub fn terminal_vault_secret_known(path: Option<String>, field: Option<String>) -> Self {
        let params = GoalParams::VaultSecretKnown { path, field };
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
    AwsProfileSelected,
    AwsSecretKnown,
    InfluxInstanceSelected,
    InfluxLaunched,
    InfluxDumpCompleted,
    KubeContextSelected,
    LogLevelSet,
    OrganizationSelected,
    PgcliRunning,
    PortForwardEstablished,
    RdsInstanceSelected,
    SsoTokenValid,
    TabCompletionsExist,
    VaultSecretKnown,
    VaultTokenValid,
}

impl GoalType {
    pub fn to_task(&self) -> Box<dyn Task> {
        match self {
            GoalType::ActuatorServiceSelected => Box::new(SelectActuatorServiceTask),
            GoalType::AwsProfileSelected => Box::new(SelectAwsProfileTask),
            GoalType::AwsSecretKnown => Box::new(GetAwsSecretTask),
            GoalType::InfluxInstanceSelected => Box::new(SelectInfluxInstanceTask),
            GoalType::InfluxLaunched => Box::new(LaunchInfluxTask),
            GoalType::InfluxDumpCompleted => Box::new(InfluxDumpTask),
            GoalType::KubeContextSelected => Box::new(SelectKubeContextTask),
            GoalType::LogLevelSet => Box::new(SetLogLevelTask),
            GoalType::OrganizationSelected => Box::new(SelectOrganizationTask),
            GoalType::PgcliRunning => Box::new(RunPgcliTask),
            GoalType::PortForwardEstablished => Box::new(PortForwardTask),
            GoalType::RdsInstanceSelected => Box::new(SelectRdsInstanceTask),
            GoalType::SsoTokenValid => Box::new(PerformSsoTask),
            GoalType::TabCompletionsExist => Box::new(CreateTabCompletionsTask),
            GoalType::VaultSecretKnown => Box::new(GetVaultSecretTask),
            GoalType::VaultTokenValid => Box::new(LoginToVaultTask),
        }
    }
}

impl From<GoalType> for String {
    fn from(goal_type: GoalType) -> Self {
        format!("{:?}", goal_type)
    }
}

pub struct GlobalParams {
    pub aws_profile: Option<String>,
    pub kube_context: Option<String>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum GoalParams {
    AwsProfileSelected {
        profile: String,
        use_current: bool,
    },
    AwsSecretKnown {
        name: Option<String>,
    },
    InfluxDumpCompleted {
        day: Option<NaiveDate>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        output: std::path::PathBuf,
    },
    KubeContextSelected {
        context: String,
        use_current: bool,
    },
    LogLevelSet {
        service: Option<String>,
        package: String,
        level: Option<Level>,
        display_only: bool,
    },
    None,
    PortForwardEstablished {
        service: Option<String>,
        port: Option<u16>,
        group: Option<String>,
        tear_down: bool,
    },
    VaultSecretKnown {
        path: Option<String>,
        field: Option<String>,
    },
}

impl From<&GoalParams> for String {
    fn from(params: &GoalParams) -> Self {
        format!("{:?}", params)
    }
}
