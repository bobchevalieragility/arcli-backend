use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CliConfig {
    #[serde(rename = "port-forward")]
    pub(crate) port_forward: PortForwardConfig,
}

impl Default for CliConfig {
    fn default() -> Self {
        CliConfig {
            port_forward: PortForwardConfig {
                groups: Vec::new(),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PortForwardConfig {
    pub(crate) groups: Vec<ServiceGroup>,
}

#[derive(Debug, Deserialize)]
pub struct ServiceGroup {
    pub(crate) name: String,
    pub(crate) services: Vec<Service>,
}

#[derive(Debug, Deserialize)]
pub struct Service {
    pub(crate) name: String,
    pub(crate) namespace: String,
    pub(crate) local_port: u16,
}