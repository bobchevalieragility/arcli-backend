use cliclack::{intro, select};
use async_trait::async_trait;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, GoalParams};
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::organization::Organization;
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct SelectOrganizationTask;

#[async_trait]
impl Task for SelectOrganizationTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Select Organization")?;
        Ok(())
    }

    async fn execute(
        &self,
        _params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        _state: &State
    ) -> Result<GoalStatus, ArcError> {
        let available_orgs = Organization::all();

        // Prompt user to select organization
        let mut menu = select("Select Organization");
        for org in &available_orgs {
            menu = menu.item(org.name(), org.name(), "");
        }
        let org_name = menu.interact()?;

        // Convert selected name to an Organization
        let org = Organization::from(org_name);

        Ok(GoalStatus::Completed(TaskResult::Organization(org), OutroText::None))
    }
}