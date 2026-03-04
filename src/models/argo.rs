use std::collections::HashMap;
use serde::Deserialize;
use unicode_width::UnicodeWidthStr;
use crate::models::aws_profile::{AwsAccount, AwsProfileInfo};

const ARGO_DEV_NAME: &str = "dev";
const ARGO_STAGE_NAME: &str = "stage";
const ARGO_PROD_NAME: &str = "prod";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArgoCdInstance {
    Dev,
    Stage,
    Prod
}

impl ArgoCdInstance {
    pub fn base_url(&self) -> &str {
        match self {
            ArgoCdInstance::Dev => "https://cd.dev.agilityrobotics.com",
            ArgoCdInstance::Stage => "https://cd.stage.agilityrobotics.com",
            ArgoCdInstance::Prod => "https://cd.prod.agilityrobotics.com",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ArgoCdInstance::Dev => ARGO_DEV_NAME,
            ArgoCdInstance::Stage => ARGO_STAGE_NAME,
            ArgoCdInstance::Prod => ARGO_PROD_NAME,
        }
    }

    pub fn all() -> Vec<ArgoCdInstance> {
        vec![ArgoCdInstance::Dev, ArgoCdInstance::Stage, ArgoCdInstance::Prod]
    }
}

impl From<&str> for ArgoCdInstance {
    fn from(name: &str) -> Self {
        match name {
            ARGO_DEV_NAME => ArgoCdInstance::Dev,
            ARGO_STAGE_NAME => ArgoCdInstance::Stage,
            ARGO_PROD_NAME => ArgoCdInstance::Prod,
            _ => panic!("Unknown ArgoCD name: {}", name),
        }
    }
}

impl From<&AwsProfileInfo> for ArgoCdInstance {
    fn from(aws_profile_info: &AwsProfileInfo) -> Self {
        match aws_profile_info.account {
            AwsAccount::Dev => ArgoCdInstance::Dev,
            AwsAccount::Stage => ArgoCdInstance::Stage,
            AwsAccount::Prod => ArgoCdInstance::Prod,
            _ => panic!("No known ArgoCD instance exists for AWS profile: {}", aws_profile_info.name),
        }
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct ArgoApplicationList {
    pub(crate) items: Vec<ArgoApplication>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ArgoApplication {
    pub(crate) metadata: ArgoMetadata,
    pub(crate) status: ArgoStatus,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ArgoMetadata {
    pub(crate) name: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ArgoStatus {
    sync: ArgoSyncStatus,
    #[serde(rename = "operationState")]
    operation_state: ArgoOperationState,
    summary: Option<ArgoSummary>,
}

#[derive(Deserialize, Debug)]
struct ArgoSummary {
    images: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct ArgoSyncStatus {
    status: String,
}

#[derive(Deserialize, Debug)]
struct ArgoOperationState {
    #[serde(rename = "finishedAt")]
    finished_at: Option<String>,
    #[serde(rename = "syncResult")]
    sync_result: Option<ArgoSyncResult>,
}

#[derive(Deserialize, Debug)]
struct ArgoSyncResult {
    resources: Vec<ArgoResource>,
}

#[derive(Deserialize, Debug)]
struct ArgoResource {
    group: String,
    kind: String,
    name: String,
    #[serde(default)]
    images: Vec<String>,
}

#[derive(Debug)]
pub struct AppInfo {
    pub(crate) name: String,
    pub(crate) sync_status: String,
    pub(crate) finished_at: Option<String>,
    pub(crate) image_tag: String,
}

impl From<ArgoApplication> for AppInfo {
    fn from(argo_app: ArgoApplication) -> Self {
        // Lookup the resource identifiers for this app
        let app_name = &argo_app.metadata.name;
        let (group, kind, name) = k8_resource_identity(app_name);

        // Attempt to find the image tag in the resources section of the response
        // Fallback to searching the list of images in the summary section of the response
        let image_tag = extract_resource_image_tag(&argo_app, group, kind, name)
            .unwrap_or_else(|| {
                let image_map = extract_summary_image_tags(&argo_app);
                image_map.get(name)
                    .map(|tag| tag.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            });

        AppInfo {
            name: argo_app.metadata.name,
            sync_status: argo_app.status.sync.status,
            finished_at: argo_app.status.operation_state.finished_at,
            image_tag,
        }
    }
}

fn k8_resource_identity(app_name: &str) -> (&str, &str, &str) {
    // Technically a resource is uniquely identified by (Group + Kind + namespace + name + version)
    // but for our purposes I think that we can get away with just using the (Group + Kind + name)
    match app_name {
        "apollo-server" => ("apps", "Deployment", "agility-graph"),
        "database-migration" => ("batch", "Job", "flyway"),
        "web-bff" => ("apps", "Deployment", "backend"),
        "webhook-integ" => ("apps", "Deployment", "webhook-integration"),
        _ => ("apps", "Deployment", app_name)
    }
}

fn extract_resource_image_tag(argo_app: &ArgoApplication, group: &str, kind: &str, repo_name: &str) -> Option<String> {
    argo_app.status.operation_state.sync_result
        .as_ref()
        .and_then(|sr| sr.resources.iter()
            .find(|r| r.group == group && r.kind == kind && r.name == repo_name)
            .and_then(|r| r.images.first())
            .and_then(|i: &String| i.split(':').nth(1).map(|s| s.to_string())))
}

fn extract_summary_image_tags(argo_app: &ArgoApplication) -> HashMap<&str, &str> {
    // Parse all discovered app images into a HashMap of docker repositories to tags
    argo_app.status.summary
        .as_ref()
        .and_then(|s| s.images.as_ref())
        .map(|images| {
            images.iter()
                .filter_map(|image| {
                    if let Some((name_part, tag)) = image.split_once(':') {
                        // Split on "/" and take the last part as the service name
                        let service_name = name_part.split('/').last()?;
                        Some((service_name, tag))
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

impl AppInfo {
    pub(crate) fn header() -> String {
        format!("{:<30} {:<8} {:<23} {:<40}", "Application", "Status", "Last Synced", "Version")
    }

    pub(crate) fn minimal_text(&self) -> String {
        format!("{:<30} {:<23} {:<40}", self.name, self.finished_at.as_deref().unwrap_or("unknown"), self.image_tag)
    }

    pub(crate) fn is_synced_to_target(&self, target_version: Option<&str>) -> bool {
        match target_version {
            Some(version) => self.sync_status == "Synced" && self.image_tag == version,
            None => self.sync_status == "Synced"
        }
    }
}

impl std::fmt::Display for AppInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let emoji = if self.sync_status == "Synced" { "✅" } else { "❌" };
        let emoji_width = emoji.width();
        let emoji_padding = " ".repeat(8_usize.saturating_sub(emoji_width));
        write!(
            f,
            "{:<30} {}{} {:<23} {:<40}",
            self.name,
            emoji,
            emoji_padding,
            self.finished_at.as_deref().unwrap_or("unknown"),
            self.image_tag
        )
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct ArgoTokenResponse {
    pub(crate) id_token: Option<String>,
    pub(crate) refresh_token: Option<String>,
    pub(crate) expires_in: Option<u64>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ArgocdSettings {
    #[serde(rename = "oidcConfig")]
    pub(crate) oidc_config: Option<OidcConfig>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct OidcConfig {
    pub(crate) issuer: Option<String>,

    #[serde(rename = "cliClientID")]
    pub(crate) cli_client_id: Option<String>,
}
