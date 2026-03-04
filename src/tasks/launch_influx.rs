use async_trait::async_trait;
use cliclack::intro;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, Goal, GoalParams};
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct LaunchInfluxTask;

#[async_trait]
impl Task for LaunchInfluxTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Launch Influx UI")?;
        Ok(())
    }

    async fn execute(
        &self,
        params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Ensure that SSO token has not expired
        let sso_goal = Goal::sso_token_valid();
        if !state.contains(&sso_goal) {
            return Ok(GoalStatus::Needs(sso_goal));
        }

        // Extract aws_profile from params
        let aws_profile = match params {
            GoalParams::InfluxLaunched { aws_profile } => aws_profile.clone(),
            _ => None,
        };

        // If an Influx instance has not yet been selected, we need to wait for that goal to complete
        let influx_selection_goal = Goal::influx_instance_selected(aws_profile.clone());
        if !state.contains(&influx_selection_goal) {
            return Ok(GoalStatus::Needs(influx_selection_goal));
        }

        // Retrieve selected Influx instance from state
        let influx_instance = state.get_influx_instance(&influx_selection_goal)?;
        let influx_secret_name = influx_instance.ui_secret_id().to_string();

        // If the password for this Influx instance has not yet been retrieved, we need to wait for that goal to complete
        let secret_goal = Goal::aws_secret_known(influx_secret_name, aws_profile);
        if !state.contains(&secret_goal) {
            return Ok(GoalStatus::Needs(secret_goal));
        }

        // Retrieve secret value as JSON Value from state
        let secret_value = state.get_aws_secret(&secret_goal)?;

        // Set outro text content
        let username = secret_value["username"]
            .as_str()
            .ok_or_else(|| ArcError::invalid_secret("username"))?;
        let password = secret_value["password"]
            .as_str()
            .ok_or_else(|| ArcError::invalid_secret("password"))?;

        let outro_text = OutroText::multi(
            "Influx Credentials".to_string(),
            format!("username: {}\npassword: {}", username, password),
        );

        // Open the user's default web browser to the auth URL
        webbrowser::open(influx_instance.url())?;

        Ok(GoalStatus::Completed(TaskResult::InfluxCommand, outro_text))
    }
}