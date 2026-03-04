use chrono::{DateTime, Utc};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use crate::models::argo::ArgoCdInstance;
use crate::models::errors::ArcError;

const KEYRING_SERVICE: &str = "arc-backend-argo";

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ArgoCredentials {
    pub id_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
}

/// Client that wraps access to operating system keyring's such as Keychain Access on MacOS
pub struct ArgoKeyring {
    service: String,
    user: String,
}

impl ArgoKeyring {
    pub fn new(instance: &ArgoCdInstance) -> Self {
        Self {
            service: KEYRING_SERVICE.to_string(),
            user: instance.name().to_string()
        }
    }

    pub fn get_credentials(&self) -> Result<ArgoCredentials, ArcError> {
        let entry = Entry::new(&self.service, &self.user)?;
        let keyring_data = entry.get_password()?;
        Ok(serde_json::from_str::<ArgoCredentials>(&keyring_data)?)
    }

    pub fn save_credentials(
        &self,
        id_token: &str,
        refresh_token: Option<String>,
        expires_in: Option<u64>,
    ) -> Result<(), ArcError> {
        let expires_at = Utc::now() + chrono::Duration::seconds(expires_in.unwrap_or(3600) as i64);
        let credentials = ArgoCredentials { id_token: id_token.to_string(), refresh_token, expires_at };
        let credentials_json = serde_json::to_string(&credentials)?;

        let entry = Entry::new(&self.service, &self.user)?;
        Ok(entry.set_password(&credentials_json)?)
    }
}