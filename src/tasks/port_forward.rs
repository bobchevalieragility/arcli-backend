use std::collections::HashMap;
use async_trait::async_trait;
use kube::{Api, Client};
use kube::api::ListParams;
use cliclack::{intro, outro_note, select, spinner};
use console::style;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use k8s_openapi::api::core::v1::{Namespace, Pod, Service, ServiceSpec};
use kube::config::Kubeconfig;
use tokio::task::AbortHandle;
use crate::models::kube_service::KubeService;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, Goal, GoalParams, GoalType};
use crate::{GoalStatus, OutroText};
use crate::models::args::PROMPT;
use crate::models::config::CliConfig;
use crate::models::kube_context::KubeCluster;
use crate::models::state::State;
use crate::tasks::{sleep_indicator, Task, TaskResult};

#[derive(Debug)]
pub struct PortForwardTask;

#[async_trait]
impl Task for PortForwardTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Port Forward")?;
        Ok(())
    }

    async fn execute(
        &self,
        params: &GoalParams,
        config: &CliConfig,
        _global_params: &GlobalParams,
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Ensure that SSO token has not expired
        let sso_goal = Goal::sso_token_valid();
        if !state.contains(&sso_goal) {
            return Ok(GoalStatus::Needs(sso_goal));
        }

        // Extract kube_context arg from params
        let kube_context = match params {
            GoalParams::PortForwardEstablished { kube_context, .. } => kube_context.clone(),
            _ => None,
        };

        // If Kube context has not been selected, we need to wait for that goal to complete
        let context_goal = Goal::kube_context_selected(kube_context);
        if !state.contains(&context_goal) {
            return Ok(GoalStatus::Needs(context_goal));
        }

        // Retrieve info about the desired Kube context from state
        let context_info = state.get_kube_context_info(&context_goal)?;

        // Create a Kubernetes client using the KUBECONFIG path from state
        let spinner = spinner();
        spinner.start("Creating Kubernetes client...");
        let kubeconfig = Kubeconfig::read_from(&context_info.kubeconfig)?;
        let client = Client::try_from(kubeconfig)?;
        spinner.stop("Kubernetes client created");

        // Determine which service(s) and port(s) to forward to, prompting user if necessary
        let targets: Vec<TargetService> = get_target_services(params, config, &context_info.cluster, &client).await?;

        let mut service_apis: HashMap<String, Api<Service>> = HashMap::new();
        let mut pod_apis: HashMap<String, Api<Pod>> = HashMap::new();

        let mut port_forward_infos = Vec::new();
        for target in &targets {
            let service_name = target.service.name.clone();
            let remote_port = target.service.port;
            let local_port = target.local_port;

            let service_api = service_apis.entry(target.service.namespace.clone())
                .or_insert_with(|| Api::namespaced(client.clone(), &target.service.namespace));
            let pod_api = pod_apis.entry(target.service.namespace.clone())
                .or_insert_with(|| Api::namespaced(client.clone(), &target.service.namespace));

            // Find pods and start port forwarding for each target service
            let pod = get_service_pod(&service_name, &service_api, &pod_api).await?;
            let pod_api_clone = pod_api.clone();
            let handle = tokio::spawn(async move {
                if let Err(e) = port_forward(&pod, local_port, remote_port, &pod_api_clone).await {
                    eprintln!("Port-forward error for {}: {}", service_name, e);
                }
            });

            let info = PortForwardInfo::new(target.clone(), handle.abort_handle());
            port_forward_infos.push(info);
        }

        let summary_msg = targets.iter().map(|t| {
            format!(
                "{}{}{}{}",
                style("Service(").dim(),
                &t.service.name,
                style(") listening on 127.0.0.1:").dim(),
                t.local_port
            )
        }).collect::<Vec<_>>().join("\n");

        if let GoalParams::PortForwardEstablished { tear_down: true, .. } = params {
            // Give port-forwards time to establish with a progress indicator
            sleep_indicator(
                2,
                "Establishing port-forward(s)...",
                &summary_msg
            ).await;
        } else {
            // Give port-forwards time to establish with a progress indicator
            sleep_indicator(
                2,
                "Establishing port-forward(s)...",
                "Port-Forward session(s) established"
            ).await;

            let prompt = "Press Ctrl+C to terminate port-forwarding";
            outro_note(style(prompt).green(), summary_msg)?;

            // Wait indefinitely - tasks will run until user interrupts (Ctrl+C)
            tokio::signal::ctrl_c().await?;
        }

        Ok(GoalStatus::Completed(TaskResult::PortForward(port_forward_infos), OutroText::None))
    }
}

#[derive(Clone, Debug)]
pub struct TargetService {
    pub service: KubeService,
    pub local_port: u16,
}

#[derive(Debug)]
pub struct PortForwardInfo {
    pub service: TargetService,
    pub handle: AbortHandle,
}

impl PortForwardInfo {
    pub fn new(service: TargetService, handle: AbortHandle) -> PortForwardInfo {
        PortForwardInfo { service, handle }
    }
}

impl Drop for PortForwardInfo {
    // Ensure graceful cleanup of the spawned port-forward task
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn get_target_services(
    params: &GoalParams,
    config: &CliConfig,
    cluster: &KubeCluster,
    client: &Client,
) -> Result<Vec<TargetService>, ArcError> {
    if let GoalParams::PortForwardEstablished { group: Some(group_name), .. } = params {
        // Port-forward to a group of services
        let group_name_str = if group_name != PROMPT {
            group_name.as_str()
        } else {
            &prompt_for_group_name(config)?
        };

        // Find the ServiceGroup whose name matches group_name
        let service_group = config.port_forward.groups
            .iter()
            .find(|group| group.name == group_name_str)
            .ok_or_else(|| ArcError::invalid_config_error(&format!("Port-forward group '{}' not found in config", group_name_str)))?;

        // Convert the services to TargetService objects
        let mut service_apis: HashMap<String, Api<Service>> = HashMap::new();
        let mut targets = Vec::new();
        for s in &service_group.services {
            let namespace = s.namespace.clone();
            let api = service_apis.entry(namespace)
                .or_insert_with(|| Api::namespaced(client.clone(), &s.namespace));
            let remote_port = get_remote_port(&api, &s.name).await?;
            let svc = KubeService::new(s.namespace.clone(), s.name.clone(), remote_port);
            targets.push(TargetService { service: svc, local_port: s.local_port });
        }
        return Ok(targets)
    };

    // Determine service's namespace
    let namespace = match params {
        GoalParams::PortForwardEstablished { namespace: Some(ns), .. } => ns.clone(),
        _ => match cluster.namespace() {
            // Infer namespace from cluster unless it's PROMPT
            PROMPT => {
                let namespace_api: Api<Namespace> = Api::all(client.clone());
                prompt_for_namespace(&namespace_api).await?
            },
            _ => cluster.namespace().to_string(),
        }
    };

    let service_api: Api<Service> = Api::namespaced(client.clone(), &namespace);

    match params {
        GoalParams::PortForwardEstablished { service: Some(s), port: Some(p), .. } => {
            // Single service and local port specified
            let remote_port = get_remote_port(&service_api, s).await?;
            let svc = KubeService::new(namespace, s.clone(), remote_port);
            Ok(vec![TargetService { service: svc, local_port: *p }])
        },
        GoalParams::PortForwardEstablished { service: Some(s), port: None, .. } => {
            // Single service specified
            let remote_port = get_remote_port(&service_api, s).await?;
            let svc = KubeService::new(namespace, s.clone(), remote_port);
            let local_port = find_available_port().await?;
            Ok(vec![TargetService { service: svc, local_port }])
        },
        GoalParams::PortForwardEstablished { service: None, port: Some(p), .. } => {
            // Single local port specified
            let svc = prompt_for_service(&namespace, &service_api).await?;
            Ok(vec![TargetService { service: svc, local_port: *p }])
        },
        GoalParams::PortForwardEstablished { service: None, port: None, group: None, .. } => {
            // Single port forward desired, but neither service nor port specified
            let svc = prompt_for_service(&namespace, &service_api).await?;
            let local_port = find_available_port().await?;
            Ok(vec![TargetService { service: svc, local_port }])
        },
        _ => Err(ArcError::invalid_goal_params(GoalType::PortForwardEstablished, params)),
    }
}

fn prompt_for_group_name(config: &CliConfig) -> Result<String, ArcError> {
    let group_name = match config.port_forward.groups.len() {
        0 => return Err(ArcError::invalid_config_error("No port-forward groups defined in config")),
        1 => {
            let name = &config.port_forward.groups[0].name;
            cliclack::log::info(format!(
                "Selecting only group found in {}: {}",
                style(crate::config_file()?.display()).dim(),
                style(name).blue()
            ))?;
            name.clone()
        },
        _ => {
            // Prompt user to select a group
            let mut menu = select("Select port-forward group");
            for group in &config.port_forward.groups {
                menu = menu.item(&group.name, &group.name, "");
            }
            menu.interact()?.to_string()
        }
    };
    Ok(group_name)
}

async fn get_app_services(namespace: &str, service_api: &Api<Service>) -> Result<Vec<KubeService>, ArcError> {
    // Retrieve ALL services for the given namespace
    let list_params = ListParams::default();
    let svc_list = service_api.list(&list_params).await?;

    // Filter out services that don't contain "app" in their selector
    let kube_services = svc_list.items.into_iter()
        .filter(|svc| {
            svc.spec.as_ref()
                .and_then(|spec| spec.selector.as_ref())
                .map_or(false, |selector| selector.contains_key("app"))
        }).map(|svc| {
            let name = svc.metadata.name.unwrap();
            let remote_port = extract_port(svc.spec)?;
            Ok(KubeService::new(namespace.to_string(), name, remote_port))
        }).collect::<Result<Vec<_>, ArcError>>()?;

    Ok(kube_services)
}

async fn get_namespaces(namespace_api: &Api<Namespace>) -> Result<Vec<String>, ArcError> {
    // Retrieve ALL namespaces
    let list_params = ListParams::default();
    let ns_list = namespace_api.list(&list_params).await?;
    let namespaces: Vec<String> = ns_list.items
        .into_iter()
        .filter_map(|ns| ns.metadata.name)
        .collect();

    Ok(namespaces)
}

async fn prompt_for_namespace(namespace_api: &Api<Namespace>) -> Result<String, ArcError> {
    let available_namespaces = get_namespaces(&namespace_api).await?;

    let mut menu = select("Select the service's namespace");
    for ns in &available_namespaces {
        menu = menu.item(ns, ns, "");
    }
    let selected_namespace = menu.interact()?;

    Ok(selected_namespace.clone())
}

async fn prompt_for_service(namespace: &str, service_api: &Api<Service>) -> Result<KubeService, ArcError> {
    let available_services = get_app_services(namespace, &service_api).await?;

    let mut menu = select("Select a service for port-forwarding");
    for svc in &available_services {
        menu = menu.item(&svc.name, &svc.name, "");
    }

    let selected_name = menu.interact()?;

    // Find the KubeService that matches the selected name
    let kube_service = available_services
        .iter()
        .find(|svc| &svc.name == selected_name)
        .expect("Selected service not found in available services")
        .clone();

    Ok(kube_service)
}

async fn get_remote_port(service_api: &Api<Service>, service_name: &str) -> Result<u16, ArcError> {
    let svc = service_api.get(service_name).await?;
    extract_port(svc.spec)
}

fn extract_port(spec: Option<ServiceSpec>) -> Result<u16, ArcError> {
    Ok(spec.as_ref()
        .and_then(|spec| spec.ports.as_ref())
        .and_then(|ports| ports.first())
        .map_or(0, |port| port.port as u16))
}

async fn get_service_pod(service_name: &str, service_api: &Api<Service>, pod_api: &Api<Pod>) -> Result<String, ArcError> {
    // Get the selector label for the given service so that we can find its pods
    let selector_label = get_selector_label(service_name, service_api).await?;

    // List pods matching the service selector
    //TODO return Selector from get_selector_label and then call labels_from(Selector)
    let list_params = ListParams::default().labels(&selector_label);
    let pod_list = pod_api.list(&list_params).await?;

    // Return the name of the first pod found
    pod_list.items.first()
        .and_then(|pod| pod.metadata.name.clone())
        .ok_or_else(|| ArcError::KubePodError(selector_label))
}

async fn get_selector_label(service_name: &str, service_api: &Api<Service>) -> Result<String, ArcError> {
    let service = service_api.get(service_name).await?;

    // Extract selector labels from the service
    let selector = service.spec
        .and_then(|spec| spec.selector)
        .ok_or_else(|| ArcError::KubeServiceSpecError(service_name.to_string()))?;

    // Return label selector string (e.g., "app=metrics")
    let selector_label = selector
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(",");

    Ok(selector_label)
}

async fn find_available_port() -> Result<u16, ArcError> {
    // Bind to port 0, which lets the OS assign an available port
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();

    // Drop the listener to free the port
    drop(listener);
    Ok(port)
}

async fn port_forward(
    pod_name: &str,
    local_port: u16,
    remote_port: u16,
    pod_api: &Api<Pod>,
) -> Result<(), ArcError> {
    // Bind local TCP listener
    let listener = TcpListener::bind(("127.0.0.1", local_port)).await?;

    loop {
        let (local_stream, _) = listener.accept().await?;
        let pod_api = pod_api.clone();
        let pod_name = pod_name.to_string();

        tokio::spawn(async move {
            // Create port-forward connection to the pod
            let port_forward_stream = match pod_api
                .portforward(&pod_name, &[remote_port])
                .await
            {
                Ok(mut pf) => match pf.take_stream(remote_port) {
                    Some(stream) => stream,
                    None => {
                        eprintln!("Port {} not available", remote_port);
                        return;
                    }
                },
                Err(e) => {
                    eprintln!("Failed to establish port-forward: {}", e);
                    return;
                }
            };

            // Bidirectional copy between local connection and port-forward stream
            let (mut local_read, mut local_write) = tokio::io::split(local_stream);
            let (mut remote_read, mut remote_write) = tokio::io::split(port_forward_stream);

            //TODO pull this duplicate code into a reusable function
            let client_to_server = async {
                let mut buf = vec![0u8; 8192];
                loop {
                    match local_read.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if remote_write.write_all(&buf[..n]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            };

            let server_to_client = async {
                let mut buf = vec![0u8; 8192];
                loop {
                    match remote_read.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if local_write.write_all(&buf[..n]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            };

            // Run both directions concurrently
            tokio::select! {
                _ = client_to_server => {},
                _ = server_to_client => {},
            }
        });
    }
}

