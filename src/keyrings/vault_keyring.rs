use chrono::{DateTime, Utc};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use crate::models::vault::VaultInstance;
use crate::models::errors::ArcError;

const KEYRING_SERVICE: &str = "arc-backend-vault";

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct VaultCredentials {
    pub client_token: String,
    pub expires_at: DateTime<Utc>,
    pub renewable: bool,
}

/// Client that wraps access to operating system keyring's such as Keychain Access on MacOS
pub struct VaultKeyring {
    service: String,
    user: String,
}

impl VaultKeyring {
    pub fn new(instance: &VaultInstance) -> Self {
        Self {
            service: KEYRING_SERVICE.to_string(),
            user: instance.name().to_string()
        }
    }

    pub fn get_credentials(&self) -> Result<VaultCredentials, ArcError> {
        let entry = Entry::new(&self.service, &self.user)?;
        let keyring_data = entry.get_password()?;
        Ok(serde_json::from_str::<VaultCredentials>(&keyring_data)?)
    }

    pub fn save_credentials(
        &self,
        client_token: &str,
        expires_in: u64,
        renewable: bool,
    ) -> Result<(), ArcError> {
        let expires_at = Utc::now() + chrono::Duration::seconds(expires_in as i64);
        let credentials = VaultCredentials { client_token: client_token.to_string(), expires_at, renewable };
        let credentials_json = serde_json::to_string(&credentials)?;

        let entry = Entry::new(&self.service, &self.user)?;
        Ok(entry.set_password(&credentials_json)?)
    }
}