use cliclack::{intro, select};
use async_trait::async_trait;
use crate::models::errors::ArcError;
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::goals::{GlobalParams, GoalParams};
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct SelectActuatorServiceTask;

#[async_trait]
impl Task for SelectActuatorServiceTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Select Actuator Service")?;
        Ok(())
    }

    async fn execute(
        &self,
        _params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        _state: &State
    ) -> Result<GoalStatus, ArcError> {
        let services = ActuatorService::all();

        // Prompt user to select a service that supports actuator functionality
        let mut menu = select("Select a service");
        for svc in &services {
            let name = svc.name();
            menu = menu.item(name, name, "");
        }

        // Convert selected service name to an ActuatorService
        let svc_name = menu.interact()?;
        let service = ActuatorService::from(svc_name);

        Ok(GoalStatus::Completed(TaskResult::ActuatorService(service), OutroText::None))
    }
}

#[derive(Debug)]
pub enum ActuatorService {
    BlockManagment,
    DeviceManager,
    EventLog,
    EventResourceManagement,
    FleetStatusManager,
    Metrics,
    Scheduler,
    UserManagement,
    WebhookIntegration,
    WorkcellMonolith,
}

impl ActuatorService {
    pub fn name(&self) -> &str {
        match self {
            ActuatorService::BlockManagment => "block-management",
            ActuatorService::DeviceManager => "device-manager",
            ActuatorService::EventLog => "event-log",
            ActuatorService::EventResourceManagement => "event-resource-management",
            ActuatorService::FleetStatusManager => "fleet-status-manager",
            ActuatorService::Metrics => "metrics",
            ActuatorService::Scheduler => "scheduler",
            ActuatorService::UserManagement => "user-management",
            ActuatorService::WebhookIntegration => "webhook-integration",
            ActuatorService::WorkcellMonolith => "workcell-monolith",
        }
    }

    fn all() -> Vec<ActuatorService> {
        vec![
            ActuatorService::BlockManagment,
            ActuatorService::DeviceManager,
            ActuatorService::EventLog,
            ActuatorService::EventResourceManagement,
            ActuatorService::FleetStatusManager,
            ActuatorService::Metrics,
            ActuatorService::Scheduler,
            ActuatorService::UserManagement,
            ActuatorService::WebhookIntegration,
            ActuatorService::WorkcellMonolith,
        ]
    }
}

impl From<&str> for ActuatorService {
    fn from(svc_name: &str) -> Self {
        match svc_name {
            "block-management" => ActuatorService::BlockManagment,
            "device-manager" => ActuatorService::DeviceManager,
            "event-log" => ActuatorService::EventLog,
            "event-resource-management" => ActuatorService::EventResourceManagement,
            "fleet-status-manager" => ActuatorService::FleetStatusManager,
            "metrics" => ActuatorService::Metrics,
            "scheduler" => ActuatorService::Scheduler,
            "user-management" => ActuatorService::UserManagement,
            "webhook-integration" => ActuatorService::WebhookIntegration,
            "workcell-monolith" => ActuatorService::WorkcellMonolith,
            _ => panic!("Unknown service name: {svc_name}"),
        }
    }
}