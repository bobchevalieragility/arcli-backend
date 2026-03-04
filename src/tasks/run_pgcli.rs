use async_trait::async_trait;
use cliclack::intro;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, Goal, GoalParams};
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct RunPgcliTask;

#[async_trait]
impl Task for RunPgcliTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Run pgcli")?;
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

        // Extract aws_profile arg from params
        let aws_profile = match params {
            GoalParams::PgcliRunning { aws_profile } => aws_profile.clone(),
            _ => None,
        };

        // If an RDS instance has not yet been selected, we need to wait for that goal to complete
        let rds_selection_goal = Goal::rds_instance_selected(aws_profile.clone());
        if !state.contains(&rds_selection_goal) {
            return Ok(GoalStatus::Needs(rds_selection_goal));
        }

        // Retrieve selected RDS instance from state
        let rds_instance = state.get_rds_instance(&rds_selection_goal)?;
        let rds_secret_name = rds_instance.secret_id().to_string();

        // If the password for this RDS instance has not yet been retrieved, we need to wait for that goal to complete
        let secret_goal = Goal::aws_secret_known(rds_secret_name, aws_profile);
        if !state.contains(&secret_goal) {
            return Ok(GoalStatus::Needs(secret_goal));
        }

        // Retrieve secret value as JSON from state
        let secret_value = state.get_aws_secret(&secret_goal)?;

        let username = secret_value["username"]
            .as_str()
            .ok_or_else(|| ArcError::invalid_secret("username"))?;

        let cmd = format!(
            "export PGPASSWORD={}\npgcli -h {} -U {}",
            secret_value["password"], // Don't unwrap to string because we want to retain the quotes
            rds_instance.host(),
            username,
        );

        let outro_text = OutroText::single("Launching pgcli".to_string(), String::new());
        Ok(GoalStatus::Completed(TaskResult::PgcliCommand(cmd), outro_text))
    }
}