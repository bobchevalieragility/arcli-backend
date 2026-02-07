use cliclack::{intro, select};
use async_trait::async_trait;
use std::{env, fs};
use std::path::PathBuf;
use kube::config::Kubeconfig;
use crate::aws::eks_cluster::EksCluster;
use crate::errors::ArcError;
use crate::{GoalStatus, OutroText};
use crate::args::PROMPT;
use crate::config::CliConfig;
use crate::goals::{GlobalParams, GoalParams, GoalType};
use crate::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct SelectKubeContextTask;

#[async_trait]
impl Task for SelectKubeContextTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Switch Kube Context")?;
        Ok(())
    }

    async fn execute(
        &self,
        params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        _state: &State
    ) -> Result<GoalStatus, ArcError> {
        if let GoalParams::KubeContextSelected{ use_current: true, .. } = params {
            if let Ok(current_kubeconfig) = env::var("KUBECONFIG") {
                let kube_path = PathBuf::from(current_kubeconfig);
                let config = Kubeconfig::read_from(&kube_path)?;
                let current_context = config.current_context
                    .as_ref()
                    .ok_or_else(|| ArcError::kube_context_error("Current context not set"))?;

                // Find the cluster associated with the current context
                let eks_cluster = get_cluster(current_context, &config)?;
                let info = KubeContextInfo::new(eks_cluster, kube_path);
                let task_result = TaskResult::KubeContext{ context: info, updated: false };
                let key = "Using current Kube Context".to_string();
                let outro_text = OutroText::single(key, current_context.clone());
                return Ok(GoalStatus::Completed(task_result, outro_text))
            }
        }

        // Read the master kubeconfig file
        let kube_path = default_kube_path().ok_or_else(|| ArcError::HomeDirError)?;
        let mut config = Kubeconfig::read_from(kube_path)?;

        // Extract context arg from params
        let context: String = match params {
            GoalParams::KubeContextSelected{ context: c, .. } => c.to_string(),
            _ => Err(ArcError::invalid_goal_params(GoalType::KubeContextSelected, params))?,
        };

        // Determine the name of the K8 context to use
        let selected_kube_context = if context == PROMPT {
            // Prompt user to select a K8 context
            prompt_for_kube_context(&config)?
        } else {
            // An explicit context was provided so let's validate that it exists in the K8 config
            if config.contexts.iter().any(|ctx| ctx.name == context) {
                context
            } else {
                let available_contexts: Vec<String> = config.contexts
                    .iter()
                    .map(|ctx| ctx.name.clone())
                    .collect();
                return Err(ArcError::KubeContextError(format!(
                    "Context '{}' not found. Available contexts: {}",
                    context,
                    available_contexts.join(", ")
                )));
            }
        };

        // Set outro content
        let key = "Switched to Kube context".to_string();
        let outro_text = OutroText::single(key, selected_kube_context.clone());

        // Find the cluster associated with the selected context
        let eks_cluster = get_cluster(&selected_kube_context, &config)?;

        // Modify the current context in the in-memory config
        config.current_context = Some(selected_kube_context.clone());

        // Create a unique, terminal-specific kubeconfig file in the tmp dir
        let timestamp = chrono::Local::now().format("%Y%m%dT%H%M%S");
        let tmp_kube_path = env::temp_dir()
            .join(format!("arc_kubeconfig_{}", timestamp));

        // Save the in-memory config to the new kubeconfig file
        let yaml_data = serde_yaml::to_string(&config)?;
        fs::write(&tmp_kube_path, yaml_data)?;

        // Export the KUBECONFIG environment variable so that it can be used by dependent tasks
        unsafe { env::set_var("KUBECONFIG", &tmp_kube_path); }

        // Create task result
        let info = KubeContextInfo::new(eks_cluster, tmp_kube_path);
        let task_result = TaskResult::KubeContext{ context: info, updated: true };

        Ok(GoalStatus::Completed(task_result, outro_text))
    }
}

#[derive(Debug)]
pub struct KubeContextInfo {
    pub cluster: EksCluster,
    pub kubeconfig: PathBuf,
}

impl KubeContextInfo {
    pub fn new(cluster: EksCluster, kubeconfig: PathBuf) -> KubeContextInfo {
        KubeContextInfo { cluster, kubeconfig }
    }
}

fn default_kube_path() -> Option<PathBuf> {
    Some(home::home_dir()?.join(".kube").join("config"))
}

fn get_cluster(context_name: &str, config: &Kubeconfig) -> Result<EksCluster, ArcError> {
    let named_context = config.contexts.iter()
        .find(|ctx| ctx.name == context_name)
        .ok_or_else(|| ArcError::KubeContextError(
            format!("Unable to find named context: {}", context_name)
        ))?;

    let context = named_context.context.as_ref().ok_or_else(|| ArcError::KubeContextError(
        format!("Unable to find context info for: {}", context_name)
    ))?;

    Ok(EksCluster::from(context.cluster.as_str()))
}

fn prompt_for_kube_context(config: &Kubeconfig) -> Result<String, ArcError> {
    let mut menu = select("Select a Kubernetes Context");

    let available_contexts: Vec<String> = config.contexts
        .iter()
        .map(|ctx| ctx.name.clone())
        .collect();

    for ctx in &available_contexts {
        menu = menu.item(ctx, ctx, "");
    }

    Ok(menu.interact()?.to_string())
}