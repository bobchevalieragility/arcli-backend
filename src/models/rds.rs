use std::convert::From;

const WORKCELL_DEV_NAME: &str = "workcell (dev)";
const WORKCELL_STAGE_NAME: &str = "workcell (stage)";
const WORKCELL_PROD_NAME: &str = "workcell (prod)";
const EVENTLOG_DEV_NAME: &str = "event-log (dev)";
const EVENTLOG_STAGE_NAME: &str = "event-log (stage)";
const EVENTLOG_PROD_NAME: &str = "event-log (prod)";

#[derive(Debug, Clone, Copy)]
pub enum RdsInstance {
    WorkcellDev,
    WorkcellStage,
    WorkcellProd,
    EventLogDev,
    EventLogStage,
    EventLogProd,
}

impl RdsInstance {
    pub fn name(&self) -> &str {
        match self {
            RdsInstance::WorkcellDev => WORKCELL_DEV_NAME,
            RdsInstance::WorkcellStage => WORKCELL_STAGE_NAME,
            RdsInstance::WorkcellProd => WORKCELL_PROD_NAME,
            RdsInstance::EventLogDev => EVENTLOG_DEV_NAME,
            RdsInstance::EventLogStage => EVENTLOG_STAGE_NAME,
            RdsInstance::EventLogProd => EVENTLOG_PROD_NAME,
        }
    }

    pub fn host(&self) -> &str {
        match self {
            RdsInstance::WorkcellDev => "development-sws-postgres-db.tail5a6c.ts.net",
            RdsInstance::WorkcellStage => "staging-sws-postgres-db.tail5a6c.ts.net",
            RdsInstance::WorkcellProd => "production-sws-postgres-db.tail5a6c.ts.net",
            RdsInstance::EventLogDev => "development-event-log-postgres-db.tail5a6c.ts.net",
            RdsInstance::EventLogStage => "staging-event-log-postgres-db.tail5a6c.ts.net",
            RdsInstance::EventLogProd => "production-event-log-postgres-db.tail5a6c.ts.net",
        }
    }

    pub fn secret_id(&self) -> &str {
        match self {
            RdsInstance::WorkcellDev => "rds!db-cf31b504-504a-46e8-a906-f1240bbfd059",
            RdsInstance::WorkcellStage => "rds!db-755654c2-511c-4e2a-b85f-e87a62f712f6",
            RdsInstance::WorkcellProd => "rds!db-efd88cd6-b1a2-4cad-8bf3-6debd6143f14",
            RdsInstance::EventLogDev => "rds!db-aa7bfb33-f024-4379-99b0-ccd4368fcd3f",
            RdsInstance::EventLogStage => "rds!db-ac2c815c-c945-4b31-97ef-0904b67fc4dd",
            RdsInstance::EventLogProd => "rds!db-c7f4b05a-420c-4f7f-8393-fb3a633531ff",
        }
    }
}

impl From<&str> for RdsInstance {
    fn from(rds_name: &str) -> Self {
        match rds_name {
            WORKCELL_DEV_NAME => RdsInstance::WorkcellDev,
            WORKCELL_STAGE_NAME => RdsInstance::WorkcellStage,
            WORKCELL_PROD_NAME => RdsInstance::WorkcellProd,
            EVENTLOG_DEV_NAME => RdsInstance::EventLogDev,
            EVENTLOG_STAGE_NAME => RdsInstance::EventLogStage,
            EVENTLOG_PROD_NAME => RdsInstance::EventLogProd,
            _ => panic!("Unknown RDS name: {rds_name}"),
        }
    }
}