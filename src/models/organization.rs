use std::convert::From;

const AGILITY_NAME: &str = "Agility (AGILITY)";
const AMAZON_NAME: &str = "Amazon (CAYENNE)";
const GXO_NAME: &str = "GXO (POBLANO)";
const SCHAEFFLER_NAME: &str = "Schaeffler (SHISHITO)";
const TOYOTA_NAME: &str = "Toyota (TOYOTA)";
const TRADESHOW_NAME: &str = "Tradeshow (TRADE)";

#[derive(Debug)]
pub enum Organization {
    Agility,
    Amazon,
    GXO,
    Schaeffler,
    Toyota,
    Tradeshow,
}

impl Organization {
    pub fn name(&self) -> &str {
        match self {
            Organization::Agility => AGILITY_NAME,
            Organization::Amazon => AMAZON_NAME,
            Organization::GXO => GXO_NAME,
            Organization::Schaeffler => SCHAEFFLER_NAME,
            Organization::Toyota => TOYOTA_NAME,
            Organization::Tradeshow => TRADESHOW_NAME,
        }
    }

    pub fn id(&self) -> String {
        match self {
            Organization::Agility => "org_J19Lhq3IBNnh3OcP".to_string(),
            Organization::Amazon => "org_Ck0uzc4qIPTeO7O7".to_string(),
            Organization::GXO => "org_gj8LdW9nH8oiBtb5".to_string(),
            Organization::Schaeffler => "org_UoWXghySmCCdsjlV".to_string(),
            Organization::Toyota => "org_JOXByhHmJjGoyuAE".to_string(),
            Organization::Tradeshow => "org_BLpcFbt7MJKWHicz".to_string(),
        }
    }

    pub fn all() -> Vec<Organization> {
        vec![
            Organization::Agility,
            Organization::Amazon,
            Organization::GXO,
            Organization::Schaeffler,
            Organization::Toyota,
            Organization::Tradeshow,
        ]
    }
}

impl From<&str> for Organization {
    fn from(org_name: &str) -> Self {
        match org_name {
            AGILITY_NAME => Organization::Agility,
            AMAZON_NAME => Organization::Amazon,
            GXO_NAME => Organization::GXO,
            SCHAEFFLER_NAME => Organization::Schaeffler,
            TOYOTA_NAME => Organization::Toyota,
            TRADESHOW_NAME => Organization::Tradeshow,
            _ => panic!("Unknown Organization name: {}", org_name),
        }
    }
}
