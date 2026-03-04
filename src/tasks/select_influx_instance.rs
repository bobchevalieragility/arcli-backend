use cliclack::{intro, select};
use async_trait::async_trait;
use crate::models::influx::InfluxInstance;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, Goal, GoalParams};
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct SelectInfluxInstanceTask;

#[async_trait]
impl Task for SelectInfluxInstanceTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Select InfluxDB Instance")?;
        Ok(())
    }

    async fn execute(
        &self,
        params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Extract aws_profile arg from params
        let aws_profile = match params {
            GoalParams::InfluxInstanceSelected { aws_profile, .. } => aws_profile.clone(),
            _ => None,
        };

        // If AWS profile info is not available, we need to wait for that goal to complete
        let profile_goal = Goal::aws_profile_selected(aws_profile);
        if !state.contains(&profile_goal) {
            return Ok(GoalStatus::Needs(profile_goal));
        }

        // Retrieve info about the selected AWS profile from state
        let profile_info = state.get_aws_profile_info(&profile_goal)?;

        // Get a list of all available Influx instances for this account
        let available_influx_instances = profile_info.account.influx_instances();

        // Prompt user to select an Influx instance only if there are multiple options
        let (influx_instance, outro_text) = match available_influx_instances.len() {
            1 => {
                let instance = available_influx_instances[0];
                let key = "Inferred Influx instance".to_string();
                (instance, OutroText::single(key, instance.name().to_string()))
            },
            _ => (prompt_for_influx_instance(available_influx_instances).await?, OutroText::None)
        };

        Ok(GoalStatus::Completed(TaskResult::InfluxInstance(influx_instance), outro_text))
    }
}

async fn prompt_for_influx_instance(
    available_influx_instances: Vec<InfluxInstance>
) -> Result<InfluxInstance, ArcError> {
    let mut menu = select("Select InfluxDB instance");
    for influx in &available_influx_instances {
        menu = menu.item(influx.name(), influx.name(), "");
    }

    let influx_name = menu.interact()?.to_string();
    Ok(InfluxInstance::from(influx_name.as_str()))
}