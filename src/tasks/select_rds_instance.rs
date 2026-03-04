use cliclack::{intro, select};
use async_trait::async_trait;
use crate::models::rds::RdsInstance;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, Goal, GoalParams};
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct SelectRdsInstanceTask;

#[async_trait]
impl Task for SelectRdsInstanceTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Select RDS Instance")?;
        Ok(())
    }

    async fn execute(
        &self,
        params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Extract aws_profile from params
        let aws_profile = match params {
            GoalParams::RdsInstanceSelected { aws_profile } => aws_profile.clone(),
            _ => None,
        };

        // If AWS profile info is not available, we need to wait for that goal to complete
        let profile_goal = Goal::aws_profile_selected(aws_profile);
        if !state.contains(&profile_goal) {
            return Ok(GoalStatus::Needs(profile_goal));
        }

        // Retrieve the desired AWS account ID from state
        let profile_info = state.get_aws_profile_info(&profile_goal)?;

        // Get a list of all available RDS instances for this account
        let available_rds_instances = profile_info.account.rds_instances();

        // Prompt user to select RDS instance only if there are multiple options
        let (rds_instance, outro_text) = match available_rds_instances.len() {
            1 => {
                let instance = available_rds_instances[0];
                let key = "Inferred RDS instance".to_string();
                (instance, OutroText::single(key, instance.name().to_string()))
            },
            _ => (prompt_for_rds_instance(available_rds_instances).await?, OutroText::None)
        };

        Ok(GoalStatus::Completed(TaskResult::RdsInstance(rds_instance), outro_text))
    }
}

async fn prompt_for_rds_instance(available_rds_instances: Vec<RdsInstance>) -> Result<RdsInstance, ArcError> {
    let mut menu = select("Select RDS instance");
    for rds in &available_rds_instances {
        menu = menu.item(rds.name(), rds.name(), "");
    }

    let rds_name = menu.interact()?.to_string();
    Ok(RdsInstance::from(rds_name.as_str()))
}