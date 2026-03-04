use cliclack::{intro, select};
use async_trait::async_trait;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, GoalParams};
use crate::{GoalStatus, OutroText};
use crate::models::argo::ArgoCdInstance;
use crate::models::config::CliConfig;
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct SelectArgoInstanceTask;

#[async_trait]
impl Task for SelectArgoInstanceTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Select ArgoCD Instance")?;
        Ok(())
    }

    async fn execute(
        &self,
        _params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        _state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Get a list of all available ArgoCD instances
        let available_argo_instances = ArgoCdInstance::all();

        // Prompt user to select ArgoCD instance only if there are multiple options
        let (argo_instance, outro_text) = match available_argo_instances.len() {
            1 => {
                let instance = available_argo_instances[0];
                let key = "Inferred ArgoCD instance".to_string();
                (instance, OutroText::single(key, instance.name().to_string()))
            },
            _ => (prompt_for_argo_instance(available_argo_instances).await?, OutroText::None)
        };

        Ok(GoalStatus::Completed(TaskResult::ArgoInstance(argo_instance), outro_text))
    }
}

async fn prompt_for_argo_instance(available_argo_instances: Vec<ArgoCdInstance>) -> Result<ArgoCdInstance, ArcError> {
    let mut menu = select("Select ArgoCD instance");
    for argo in &available_argo_instances {
        menu = menu.item(argo.name(), argo.name(), "");
    }

    let argo_name = menu.interact()?.to_string();
    Ok(ArgoCdInstance::from(argo_name.as_str()))
}