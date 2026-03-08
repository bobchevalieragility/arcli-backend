use clap::{Parser, Subcommand};
use std;
use std::convert::From;
use std::path::PathBuf;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use crate::models::goals::Goal;
use crate::models::log_level::LogLevel;

// This constant must be kept in sync with its usage in the #[arg] attributes below
pub const PROMPT: &str = "PROMPT";

#[derive(Parser, Clone, Debug, PartialEq, Eq, Hash)]
#[command(author, version, about = "CLI Tool for Arc Backend")]
pub struct CliArgs {
    #[arg(
        short,
        long,
        global = true,
        hide = true,
        help = "Print to std_out (useful when calling `backend` from scripts)"
    )]
    pub(crate) raw_output: bool,

    #[command(subcommand)]
    pub(crate) command: CliCommand,
}

impl CliArgs {
    pub(crate) fn to_goals(self) -> Vec<Goal> {
        match self.command {
            CliCommand::Argo { pull_request} => vec![
                Goal::terminal_argo(pull_request)
            ],
            CliCommand::Completions => vec![Goal::terminal_tab_completions()],
            CliCommand::Influx { action } => {
                match action {
                    InfluxAction::Ui { aws_profile } => vec![
                        Goal::terminal_influx_launched(aws_profile)
                    ],
                    InfluxAction::Dump { day, start, end, output_dir, file_per_measurement, aws_profile } => vec![
                        Goal::terminal_influx_dump_completed(day, start, end, output_dir, file_per_measurement, aws_profile)
                    ],
                }
            },
            CliCommand::Logging { action } => {
                match action {
                    LoggingAction::Get { service, package, kube_context } => vec![
                        Goal::terminal_log_level_known(service, package, kube_context)
                    ],
                    LoggingAction::Set { service, package, level, kube_context } => vec![
                        Goal::terminal_log_level_set(service, package, level, kube_context)
                    ],
                }
            },
            CliCommand::Pgcli { aws_profile } => vec![Goal::terminal_pgcli_running(aws_profile)],
            CliCommand::PortForward { namespace, service, port, group, kube_context } => vec![
                Goal::terminal_port_forward_established(namespace, service, port, group, kube_context)
            ],
            CliCommand::Secret { store } => {
                match store {
                    SecretStore::Aws { name, aws_profile } => vec![
                        Goal::terminal_aws_secret_known(name, aws_profile)
                    ],
                    SecretStore::Vault { path, field, aws_profile } => vec![
                        Goal::terminal_vault_secret_known(path, field, aws_profile)
                    ],
                }
            },
            CliCommand::Switch { aws_profile, kube_context } => {
                // Use global parameters to determine which prompts are needed, if any
                match (aws_profile, kube_context) {
                    (None, None) => vec![
                        Goal::terminal_kube_context_selected(PROMPT),
                        Goal::terminal_aws_profile_selected(PROMPT)
                    ],
                    (Some(p), Some(k)) => vec![
                        Goal::terminal_kube_context_selected(k.clone()),
                        Goal::terminal_aws_profile_selected(p.clone())
                    ],
                    (Some(p), None) => vec![
                        Goal::terminal_aws_profile_selected(p.clone())
                    ],
                    (None, Some(k)) => vec![
                        Goal::terminal_kube_context_selected(k.clone())
                    ],
                }
            },
        }
    }
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CliCommand {
    #[command(about = "Monitor ArgoCD application statuses")]
    Argo {
        #[arg(
            short, long,
            help = "PR number, from services-gitops, used to infer which apps to monitor",
            num_args = 0..=1,
            default_missing_value = "0",
        )]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        pull_request: Option<u32>,
    },
    #[command(about = "Generate a shell completion script")]
    Completions,
    #[command(about = "Interact with InfluxDB")]
    Influx {
        #[command(subcommand)]
        action: InfluxAction,
    },
    #[command(about = "View or set the log level for a Java Spring Boot service")]
    Logging {
        #[command(subcommand)]
        action: LoggingAction,
    },
    #[command(about = "Launch pgcli to interact with a Postgres RDS instance")]
    Pgcli {
        #[arg(short = 'a', long, help = "Use AWS profile", num_args = 0..=1, default_missing_value = "PROMPT")]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        aws_profile: Option<String>,
    },
    #[command(about = "Start port-forwarding to one or more Kubernetes service(s)")]
    PortForward {
        #[arg(short, long, help = "Cluster namespace, e.g. 'development' (if omitted, will prompt)", conflicts_with = "group")]
        namespace: Option<String>,

        #[arg(short, long, help = "Service name, e.g. 'metrics' (if omitted, will prompt)", conflicts_with = "group")]
        service: Option<String>,

        #[arg(short, long, help = "Local port (defaults to random, unused port)", conflicts_with = "group")]
        port: Option<u16>,

        #[arg(
            short, long,
            help = "Group of services to forward to, if blank you'll be prompted to select a group",
            num_args = 0..=1,
            default_missing_value = "PROMPT",
            conflicts_with = "service"
        )]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        group: Option<String>,

        #[arg(short = 'k', long, help = "Use K8 context", num_args = 0..=1, default_missing_value = "PROMPT")]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        kube_context: Option<String>,
    },
    #[command(about = "Retrieve a secret value from AWS Secrets Manager or Vault")]
    Secret {
        #[command(subcommand)]
        store: SecretStore,
    },
    #[command(about = "Switch AWS profile and/or Kubernetes context")]
    Switch {
        #[arg(short = 'a', long, help = "Use AWS profile", num_args = 0..=1, default_missing_value = "PROMPT")]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        aws_profile: Option<String>,

        #[arg(short = 'k', long, help = "Use K8 context", num_args = 0..=1, default_missing_value = "PROMPT")]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        kube_context: Option<String>,
    },
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SecretStore {
    #[command(about = "Retrieve a secret from AWS Secrets Manager")]
    Aws {
        #[arg(short, long, help = "Name of the secret to retrieve (if omitted, will prompt)")]
        name: Option<String>,

        #[arg(short = 'a', long, help = "Use AWS profile", num_args = 0..=1, default_missing_value = "PROMPT")]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        aws_profile: Option<String>,
    },
    #[command(about = "Retrieve a secret from Vault")]
    Vault {
        #[arg(short, long, help = "Path to secret to retrieve (if omitted, will prompt)")]
        path: Option<String>,

        #[arg(short, long, help = "Field within secret to retrieve (defaults to entire secret)")]
        field: Option<String>,

        #[arg(short = 'a', long, help = "Use AWS profile", num_args = 0..=1, default_missing_value = "PROMPT")]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        aws_profile: Option<String>,
    },
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LoggingAction {
    #[command(about = "Get the current log level for a service")]
    Get {
        #[arg(short, long, help = "Service name, e.g. 'metrics' (if omitted, will prompt)")]
        service: Option<String>,

        #[arg(
            short, long,
            help = "Package, e.g. 'com.agilityrobotics.metrics' (defaults to ROOT)",
            default_value = "ROOT"
        )]
        package: String,

        #[arg(short = 'k', long, help = "Use K8 context", num_args = 0..=1, default_missing_value = "PROMPT")]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        kube_context: Option<String>,
    },
    #[command(about = "Set the log level for a service")]
    Set {
        #[arg(short, long, help = "Service name, e.g. 'metrics' (if omitted, will prompt)")]
        service: Option<String>,

        #[arg(
            short, long,
            help = "Package, e.g. 'com.agilityrobotics.metrics' (defaults to ROOT)",
            default_value = "ROOT"
        )]
        package: String,

        #[arg(short, long, help = "Desired log level (if omitted, will prompt)")]
        level: Option<LogLevel>,

        #[arg(short = 'k', long, help = "Use K8 context", num_args = 0..=1, default_missing_value = "PROMPT")]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        kube_context: Option<String>,
    },
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq, Hash)]
pub enum InfluxAction {
    #[command(about = "Launch the InfluxDB UI")]
    Ui {
        #[arg(short = 'a', long, help = "Use AWS profile", num_args = 0..=1, default_missing_value = "PROMPT")]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        aws_profile: Option<String>,
    },
    #[command(about = "Query InfluxDB and dump results to a CSV file")]
    Dump {
        #[arg(
            short, long, help = "Query for all records on this day (e.g., '2026-01-19')",
            conflicts_with = "start"
        )]
        day: Option<NaiveDate>,

        #[arg(
            short, long,
            help = "Start time as either RFC3339 or ms since epoch (e.g. '2026-01-01T00:00:00Z')",
            value_parser = parse_datetime,
            conflicts_with = "day"
        )]
        start: Option<DateTime<Utc>>,

        #[arg(
            short, long,
            help = "End time as either RFC3339 or ms since epoch. Defaults to NOW. (e.g. '2025-01-19T00:00:00Z')",
            value_parser = parse_datetime,
            requires = "start",
            conflicts_with = "day"
        )]
        end: Option<DateTime<Utc>>,

        #[arg(short, long, help = "Path to output file", default_value = ".")]
        output_dir: PathBuf,

        #[arg(short, long, default_value = "false", help = "Create separate files for each measurement type")]
        file_per_measurement: bool,

        #[arg(short = 'a', long, help = "Use AWS profile", num_args = 0..=1, default_missing_value = "PROMPT")]
        // Will be PROMPT if the user included the flag without a value, None if they didn't include the flag at all
        aws_profile: Option<String>,
    },
}

fn parse_datetime(input: &str) -> Result<DateTime<Utc>, String> {
    // Try parsing as milliseconds since epoch
    if let Ok(millis) = input.parse::<i64>() {
        // Convert to seconds and nanoseconds
        let seconds = millis / 1000;
        let nanoseconds = (millis % 1000) * 1_000_000;

        return Utc.timestamp_opt(seconds, nanoseconds as u32)
            .single()
            .ok_or_else(|| format!("Milliseconds since epoch '{}' is out of range", input));
    }

    // Try parsing as RFC3339 string
    DateTime::parse_from_rfc3339(input)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("Invalid datetime format '{}': {}", input, e))
}
