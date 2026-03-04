use std::collections::HashMap;
use serde_json::Value;
use crate::models::influx::InfluxInstance;
use crate::models::rds::RdsInstance;
use crate::models::errors::ArcError;
use crate::tasks::TaskResult;
use crate::tasks::port_forward::PortForwardInfo;
use crate::tasks::select_actuator_service::ActuatorService;
use crate::models::kube_context::KubeContextInfo;
use std;
use crate::models::argo::ArgoCdInstance;
use crate::models::aws_profile::AwsProfileInfo;
use crate::models::github::GithubPrFile;
use crate::models::goals::Goal;
use crate::models::organization::Organization;

pub struct State {
    results: HashMap<Goal, TaskResult>,
}

impl State {
    pub(crate) fn new() -> Self {
        State { results: HashMap::new() }
    }

    pub(crate) fn contains(&self, goal: &Goal) -> bool {
        self.results.contains_key(goal)
    }

    pub(crate) fn insert(&mut self, goal: Goal, result: TaskResult) {
        self.results.insert(goal, result);
    }

    fn get(&self, goal: &Goal) -> Result<&TaskResult, ArcError> {
        self.results.get(goal).ok_or_else(|| ArcError::insufficient_state(goal))
    }

    pub(crate) fn get_actuator_service(&self, goal: &Goal) -> Result<&ActuatorService, ArcError> {
        match self.get(goal)? {
            TaskResult::ActuatorService(x) => Ok(x),
            result => Err(ArcError::invalid_state(goal, "ActuatorService", result)),
        }
    }

    pub(crate) fn get_argo_instance(&self, goal: &Goal) -> Result<&ArgoCdInstance, ArcError> {
        match self.get(goal)? {
            TaskResult::ArgoInstance(x) => Ok(x),
            result => Err(ArcError::invalid_state(goal, "ArgoInstance", result)),
        }
    }

    pub(crate) fn get_aws_profile_info(&self, goal: &Goal) -> Result<&AwsProfileInfo, ArcError> {
        match self.get(goal)? {
            TaskResult::AwsProfile { profile, .. } => Ok(profile),
            result => Err(ArcError::invalid_state(goal, "AwsProfile", result)),
        }
    }

    pub(crate) fn get_aws_secret(&self, goal: &Goal) -> Result<Value, ArcError> {
        match self.get(goal)? {
            TaskResult::AwsSecret(x) => {
                let secret_json: Value = serde_json::from_str(x)?;
                Ok(secret_json)
            },
            result => Err(ArcError::invalid_state(goal, "AwsSecret", result)),
        }
    }
    
    pub(crate) fn get_github_pr_files(&self, goal: &Goal) -> Result<&Vec<GithubPrFile>, ArcError> {
        match self.get(goal)? {
            TaskResult::GithubPrFiles(files) => Ok(files),
            result => Err(ArcError::invalid_state(goal, "GithubPrFiles", result)),
        }
    }

    pub(crate) fn get_influx_instance(&self, goal: &Goal) -> Result<&InfluxInstance, ArcError> {
        match self.get(goal)? {
            TaskResult::InfluxInstance(x) => Ok(x),
            result => Err(ArcError::invalid_state(goal, "InfluxInstance", result)),
        }
    }

    pub(crate) fn get_kube_context_info(&self, goal: &Goal) -> Result<&KubeContextInfo, ArcError> {
        match self.get(goal)? {
            TaskResult::KubeContext { context, .. } => Ok(context),
            result => Err(ArcError::invalid_state(goal, "KubeContext", result)),
        }
    }

    pub(crate) fn get_organization(&self, goal: &Goal) -> Result<&Organization, ArcError> {
        match self.get(goal)? {
            TaskResult::Organization(x) => Ok(x),
            result => Err(ArcError::invalid_state(goal, "Organization", result)),
        }
    }

    pub(crate) fn get_port_forward_infos(&self, goal: &Goal) -> Result<&Vec<PortForwardInfo>, ArcError> {
        match self.get(goal)? {
            TaskResult::PortForward(infos) => Ok(infos),
            result => Err(ArcError::invalid_state(goal, "PortForward", result)),
        }
    }

    pub(crate) fn get_rds_instance(&self, goal: &Goal) -> Result<&RdsInstance, ArcError> {
        match self.get(goal)? {
            TaskResult::RdsInstance(x) => Ok(x),
            result => Err(ArcError::invalid_state(goal, "RdsInstance", result)),
        }
    }

    pub(crate) fn get_vault_secret(&self, goal: &Goal) -> Result<String, ArcError> {
        match self.get(goal)? {
            TaskResult::VaultSecret(x) => Ok(x.clone()),
            result => Err(ArcError::invalid_state(goal, "VaultSecret", result)),
        }
    }
}