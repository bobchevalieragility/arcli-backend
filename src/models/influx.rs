use std::convert::From;

const METRICS_DEV_NAME: &str = "metrics (dev)";
const METRICS_STAGE_NAME: &str = "metrics (stage)";
const METRICS_PROD_NAME: &str = "metrics (prod)";

const VAULT_PATH: &str = "mp/metrics";
const VAULT_FIELD: &str = "INFLUXDB_CLI_TOKEN";

#[derive(Debug, Clone, Copy)]
pub enum InfluxInstance {
    MetricsDev,
    MetricsStage,
    MetricsProd,
}

impl InfluxInstance {
    pub fn name(&self) -> &str {
        match self {
            InfluxInstance::MetricsDev => METRICS_DEV_NAME,
            InfluxInstance::MetricsStage => METRICS_STAGE_NAME,
            InfluxInstance::MetricsProd => METRICS_PROD_NAME,
        }
    }

    pub fn cli_secret_info(&self) -> (&str, &str) {
        (VAULT_PATH, VAULT_FIELD)
    }

    pub fn ui_secret_id(&self) -> &str {
        match self {
            InfluxInstance::MetricsDev => "READONLY-InfluxDB-auth-parameters-n6ih7p944s",
            InfluxInstance::MetricsStage => "READONLY-InfluxDB-auth-parameters-sh2akvmz04",
            InfluxInstance::MetricsProd => "READONLY-InfluxDB-auth-parameters-cmdvhpm2dy",
        }
    }

    pub fn url(&self) -> &str {
        match self {
            InfluxInstance::MetricsDev => "https://n6ih7p944s-uc4ycq7jkw3e35.timestream-influxdb.us-west-2.on.aws:8086",
            InfluxInstance::MetricsStage => "https://sh2akvmz04-pl56nlv6if3nks.timestream-influxdb.us-west-2.on.aws:8086",
            InfluxInstance::MetricsProd => "https://cmdvhpm2dy-4vxnbwogyuwmgu.timestream-influxdb.us-west-2.on.aws:8086",
        }
    }
}

impl From<&str> for InfluxInstance {
    fn from(rds_name: &str) -> Self {
        match rds_name {
            METRICS_DEV_NAME => InfluxInstance::MetricsDev,
            METRICS_STAGE_NAME => InfluxInstance::MetricsStage,
            METRICS_PROD_NAME => InfluxInstance::MetricsProd,
            _ => panic!("Unknown InfluxDB name: {rds_name}"),
        }
    }
}