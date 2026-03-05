use crate::models::aws_profile::AwsAccount;

const VAULT_NON_PROD_NAME: &str = "non-prod";
const VAULT_PROD_NAME: &str = "prod";

pub enum VaultInstance {
    NonProd,
    Prod,
}

impl VaultInstance {
    pub fn name(&self) -> &str {
        match self {
            VaultInstance::NonProd => VAULT_NON_PROD_NAME,
            VaultInstance::Prod => VAULT_PROD_NAME,
        }
    }

    pub fn address(&self) -> &str {
        match self {
            VaultInstance::NonProd => "https://nonprod-public-vault-b4ed83ad.91d9045d.z1.hashicorp.cloud:8200",
            VaultInstance::Prod => "https://prod-public-vault-752e7a3c.c39279c9.z1.hashicorp.cloud:8200",
        }
    }

    pub fn oidc_namespace(&self) -> Option<String> {
        match self {
            VaultInstance::NonProd => Some("admin".to_string()),
            VaultInstance::Prod => Some("admin".to_string()),
        }
    }

    pub fn secrets_namespace(&self, account: &AwsAccount) -> Option<String> {
        match self {
            VaultInstance::NonProd => {
                match account {
                    AwsAccount::Dev => Some("admin/dev".to_string()),
                    AwsAccount::Stage => Some("admin/stage".to_string()),
                    _ => None,
                }
            },
            VaultInstance::Prod => {
                match account {
                    AwsAccount::Prod => Some("admin/prod".to_string()),
                    _ => None,
                }
            },
        }
    }

    pub fn oidc_role(&self) -> Option<String> {
        match self {
            //TODO these should be configurable
            VaultInstance::NonProd => Some("arc-backend-developer".to_string()),
            VaultInstance::Prod => Some("arc-backend-developer".to_string()),
        }
    }
}

impl From<&str> for VaultInstance {
    fn from(name: &str) -> Self {
        match name {
            VAULT_NON_PROD_NAME => VaultInstance::NonProd,
            VAULT_PROD_NAME => VaultInstance::Prod,
            _ => panic!("Unknown Vault name: {}", name),
        }
    }
}
