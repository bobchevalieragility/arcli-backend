use std::convert::From;
use aws_runtime::env_config::section::EnvConfigSections;
use crate::models::influx::InfluxInstance;
use crate::models::rds::RdsInstance;
use crate::models::vault::VaultInstance;

#[derive(Debug)]
pub enum AwsAccount {
    DataPlatform,
    Dev,
    Iot,
    Prod,
    Sandbox,
    Stage,
}

impl From<&str> for AwsAccount {
    fn from(account_id: &str) -> Self {
        match account_id {
            "789472542317" => AwsAccount::DataPlatform,
            "983257951706" => AwsAccount::Dev,
            "283152483325" => AwsAccount::Iot,
            "871891271706" => AwsAccount::Prod,
            "287642671827" => AwsAccount::Sandbox,
            "975050271628" => AwsAccount::Stage,
            _ => panic!("Unknown AWS sso account id: {account_id}"),
        }
    }
}

impl AwsAccount {
    pub fn vault_instance(&self) -> VaultInstance {
        match self {
            AwsAccount::DataPlatform => VaultInstance::Prod,
            AwsAccount::Dev => VaultInstance::NonProd,
            AwsAccount::Prod => VaultInstance::Prod,
            AwsAccount::Sandbox => VaultInstance::NonProd,
            AwsAccount::Stage => VaultInstance::NonProd,
            _ => panic!("No Vault instance exists for this AWS account: {:?}", self),
        }
    }

    pub fn influx_instances(&self) -> Vec<InfluxInstance> {
        match self {
            AwsAccount::Dev => vec![InfluxInstance::MetricsDev],
            AwsAccount::Prod => vec![InfluxInstance::MetricsProd],
            AwsAccount::Stage => vec![InfluxInstance::MetricsStage],
            _ => panic!("No Influx instances exist for this AWS account: {:?}", self),
        }
    }

    pub fn rds_instances(&self) -> Vec<RdsInstance> {
        match self {
            AwsAccount::Dev => vec![RdsInstance::WorkcellDev, RdsInstance::EventLogDev],
            AwsAccount::Prod => vec![RdsInstance::WorkcellProd, RdsInstance::EventLogProd],
            AwsAccount::Stage => vec![RdsInstance::WorkcellStage, RdsInstance::EventLogStage],
            _ => panic!("No RDS instances exist for this AWS account: {:?}", self),
        }
    }
}

#[derive(Debug)]
pub struct AwsProfileInfo {
    pub name: String,
    pub account: AwsAccount,
    pub region: String,
}

impl From<(&str, &EnvConfigSections)> for AwsProfileInfo {
    fn from((profile_name, env_config): (&str, &EnvConfigSections)) -> Self {
        let profile = env_config.get_profile(profile_name)
            .unwrap_or_else(|| panic!("Profile '{}' not found", profile_name));
        let account_id = profile.get("sso_account_id")
            .expect("sso_account_id not found in profile");
        let account = AwsAccount::from(account_id);

        let region= profile.get("region").unwrap_or("us-west-2");

        AwsProfileInfo::new(profile_name.to_string(), account, region)
    }
}

impl From<(&String, &EnvConfigSections)> for AwsProfileInfo {
    fn from((profile_name, env_config): (&String, &EnvConfigSections)) -> Self {
        AwsProfileInfo::from((profile_name.as_str(), env_config))
    }
}

impl AwsProfileInfo {
    pub fn new(name: String, account: AwsAccount, region: &str) -> AwsProfileInfo {
        AwsProfileInfo { name, account, region: region.to_string() }
    }
}
