use async_trait::async_trait;
use chrono::NaiveTime;
use chrono::Utc;
use cliclack::intro;
use reqwest;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, ACCEPT};
use crate::models::errors::ArcError;
use crate::models::goals::{Goal, GoalParams, GoalType};
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::influx::InfluxInstance;
use crate::models::organization::Organization;
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

const DROPPED_COLS: [&str; 2] = ["_start", "_stop"];

const ALL_TABLES: [(&str, &str); 5] = [
    ("intervention", "intervention_event"),
    ("move", "move_item_event"),
    ("shift", "shift_event"),
    ("skill", "skill_event"),
    ("work", "work_event"),
];

#[derive(Debug)]
pub struct InfluxDumpTask;

#[async_trait]
impl Task for InfluxDumpTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("InfluxDB Dump")?;
        Ok(())
    }

    async fn execute(
        &self,
        params: &GoalParams,
        _config: &CliConfig,
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Ensure that SSO token has not expired
        let sso_goal = Goal::sso_token_valid();
        if !state.contains(&sso_goal) {
            return Ok(GoalStatus::Needs(sso_goal));
        }

        // Extract parameters
        let (day, start, end, output_dir, file_per_measurement, aws_profile) = match params {
            GoalParams::InfluxDumpCompleted { day, start, end, output_dir, file_per_measurement, aws_profile } => (day, start, end, output_dir, *file_per_measurement, aws_profile.clone()),
            _ => return Err(ArcError::invalid_goal_params(GoalType::InfluxDumpCompleted, params)),
        };

        // If an Influx instance has not yet been selected, we need to wait for that goal to complete
        let influx_selection_goal = Goal::influx_instance_selected(aws_profile.clone());
        if !state.contains(&influx_selection_goal) {
            return Ok(GoalStatus::Needs(influx_selection_goal));
        }

        // Retrieve selected Influx instance from state
        let influx_instance = state.get_influx_instance(&influx_selection_goal)?;

        // If the token for this Influx instance has not yet been retrieved, we need to wait for that goal to complete
        let (path, field) = influx_instance.cli_secret_info();
        let secret_goal = Goal::vault_secret_known(path.to_string(), Some(field.to_string()), None, aws_profile);
        if !state.contains(&secret_goal) {
            return Ok(GoalStatus::Needs(secret_goal));
        }

        // Retrieve secret token from state
        let token = state.get_vault_secret(&secret_goal)?;

        // If an org has not yet been selected, we need to wait for that goal to complete
        let org_selection_goal = Goal::organization_selected();
        if !state.contains(&org_selection_goal) {
            return Ok(GoalStatus::Needs(org_selection_goal));
        }

        // Retrieve organization from state
        let org = state.get_organization(&org_selection_goal)?;

        // Infer the start and end of the time range
        let (range_begin, range_end) = if let Some(start_time) = start {
            (*start_time, end.unwrap_or_else(Utc::now))
        } else if let Some(day_str) = day {
            let start = day_str
                .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .and_utc();
            let end = day_str
                .and_time(NaiveTime::from_hms_opt(11, 59, 59).unwrap())
                .and_utc();
            (start, end)
        } else {
            return Err(ArcError::invalid_goal_params(GoalType::InfluxDumpCompleted, "Missing 'day' or 'start'"));
        };

        // Apply Zulu format to the time range
        let range_begin = range_begin.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let range_end = range_end.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let dropped_cols = format!(r#"["{}"]"#, DROPPED_COLS.join(r#"", ""#));
        let client = reqwest::Client::new();

        // Create the output directory if it doesn't exist
        std::fs::create_dir_all(output_dir)?;

        if file_per_measurement {
            // Create a separate CSV file for each non-rollup table in InfluxDB
            let mut outputs = Vec::new();
            for (short_name, measurement) in ALL_TABLES.iter() {
                let fetched_data = fetch_influx_data(
                    &client,
                    &token,
                    &influx_instance,
                    org,
                    &dropped_cols,
                    &range_begin,
                    &range_end,
                    vec![measurement]
                ).await?;

                // Write the fetched data to a CSV file
                let filename = format!("{}.csv", short_name);
                let output_path = output_dir.join(filename);
                std::fs::write(&output_path, &fetched_data)?;
                outputs.push(output_path.display().to_string());
            }

            let all_outputs = outputs.join("\n  ");
            let msg = format!(
                "Start : {}\nEnd   : {}\nOrg   : {}\nOutputs:\n  {}",
                range_begin, range_end, org.id(), all_outputs
            );
            let outro_text = OutroText::multi("Influx Query Params".to_string(), msg);

            Ok(GoalStatus::Completed(TaskResult::InfluxDumpCompleted, outro_text))
        } else {
            // Create a single CSV file with data from all non-rollup tables in InfluxDB
            let tables: Vec<&str> = ALL_TABLES.iter().map(|(_, measurement)| *measurement).collect();

            let fetched_data = fetch_influx_data(
                &client,
                &token,
                &influx_instance,
                org,
                &dropped_cols,
                &range_begin,
                &range_end,
                tables
            ).await?;

            // Write the fetched data to a CSV file
            let output_path = output_dir.join("combined.csv");
            std::fs::write(&output_path, &fetched_data)?;

            let msg = format!(
                "Start : {}\nEnd   : {}\nOrg   : {}\nOutput: {}",
                range_begin, range_end, org.id(), output_path.display()
            );
            let outro_text = OutroText::multi("Influx Query Params".to_string(), msg);

            Ok(GoalStatus::Completed(TaskResult::InfluxDumpCompleted, outro_text))
        }
    }
}

async fn fetch_influx_data(
    client: &reqwest::Client,
    token: &str,
    influx_instance: &InfluxInstance,
    org: &Organization,
    dropped_cols: &str,
    range_begin: &str,
    range_end: &str,
    table_names: Vec<&str>
) -> Result<String, ArcError> {
    // Build up InfluxDB filters from the provided table names and organization
    let measurement_filters = table_names.iter()
        .map(|name| format!(r#"r["_measurement"] == "{}""#, name))
        .collect::<Vec<String>>().join(" or ");
    let filters = format!(r#"fn: (r) => r["org_id"] == "{}" and {}"#, org.id(), measurement_filters);

    // Define the Flux query
    let flux_query = format!(
        r#"from(bucket: "metrics")
              |> range(start: {}, stop: {})
              |> filter({})
              |> group()
              |> sort(columns: ["_time"])
              |> drop(columns: {})"#,
        range_begin, range_end, filters, dropped_cols
    );

    // Explicitly specify a dialect so that we get an "annotated" CSV (with extra header lines)
    let payload = serde_json::json!({
        "query": flux_query,
        "dialect": {
            "annotations": ["datatype", "group", "default"],
            "header": true,
            "delimiter": ","
        }
    });

    let url = format!("{}/api/v2/query?org=agility", influx_instance.url());

    let response = client
        .post(&url)
        .header(AUTHORIZATION, format!("Token {}", token))
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/csv")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(ArcError::influx_query_error(error_text));
    }

    // Update the #datatype line to mark columns as ignored
    let result_text = response.text().await?;
    let modified_text = ignore_datatypes(&result_text);

    Ok(modified_text)
}

fn ignore_datatypes(csv_text: &str) -> String {
    let mut lines: Vec<String> = csv_text.lines().map(|line| line.to_string()).collect();

    // Find and update the line that starts with "#datatype"
    for line in lines.iter_mut() {
        if line.starts_with("#datatype") {
            *line = line.replace("#datatype,string,long", "#datatype,ignored,ignored");
            break;
        }
    }

    lines.join("\n")
}