use std::convert::From;
use std::path::PathBuf;
use crate::models::args::PROMPT;

#[derive(Debug)]
pub enum KubeCluster {
    Dev,
    Prod,
    Sandbox,
    Stage,
    Untracked,
}

impl From<&str> for KubeCluster {
    fn from(cluster_name: &str) -> Self {
        match cluster_name {
            "tailscale-operator-platform-dev-uw2.tail5a6c.ts.net" => KubeCluster::Dev,
            // "arn:models:eks:us-west-2:983257951706:cluster/platform-dev-uw2" => EksCluster::Dev,
            "tailscale-operator-platform-prod-uw2.tail5a6c.ts.net" => KubeCluster::Prod,
            "tailscale-operator-platform-stage-uw2.tail5a6c.ts.net" => KubeCluster::Stage,
            "tailscale-operator-sandbox-uw2.tail5a6c.ts.net" => KubeCluster::Sandbox,
            _ => KubeCluster::Untracked,
        }
    }
}

impl KubeCluster {
    pub fn namespace(&self) -> &str {
        match self {
            KubeCluster::Dev => "development",
            KubeCluster::Prod => "production",
            KubeCluster::Stage => "staging",
            KubeCluster::Sandbox => "sandbox",
            KubeCluster::Untracked => PROMPT,
        }
    }
}

#[derive(Debug)]
pub struct KubeContextInfo {
    pub name: String,
    pub cluster: KubeCluster,
    pub kubeconfig: PathBuf,
}

impl KubeContextInfo {
    pub fn new(name: String, cluster: KubeCluster, kubeconfig: PathBuf) -> KubeContextInfo {
        KubeContextInfo { name, cluster, kubeconfig }
    }
}
