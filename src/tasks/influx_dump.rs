use async_trait::async_trait;
use chrono::NaiveTime;
use chrono::Utc;
use cliclack::intro;
use reqwest;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, Goal, GoalParams, GoalType};
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

const DROPPED_COLS: [&str; 2] = ["_start", "_stop"];
const FILTERED_COLS: [&str; 2] = ["result", "table"];
const ORDERED_COLS: [&str; 4] = ["_time", "event_type", "_field", "_value"];

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
        _global_params: &GlobalParams,
        state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Ensure that SSO token has not expired
        let sso_goal = Goal::sso_token_valid();
        if !state.contains(&sso_goal) {
            return Ok(GoalStatus::Needs(sso_goal));
        }

        // Extract parameters
        let (day, start, end, output, aws_profile) = match params {
            GoalParams::InfluxDumpCompleted { day, start, end, output, aws_profile } => (day, start, end, output, aws_profile.clone()),
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
        let secret_goal = Goal::vault_secret_known(path.to_string(), Some(field.to_string()), aws_profile);
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
        let organization = state.get_organization(&org_selection_goal)?;

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
        let range_begin = range_begin.format("%Y-%m-%dT%H:%M:%SZ");
        let range_end = range_end.format("%Y-%m-%dT%H:%M:%SZ");

        // Build Flux query to get all data in the time range
        let dropped_cols = format!(r#"["{}"]"#, DROPPED_COLS.join(r#"", ""#));
        let flux_query = format!(
            r#"from(bucket: "metrics")
              |> range(start: {}, stop: {})
              |> filter(fn: (r) => r["org_id"] == "{}")
              |> group()
              |> sort(columns: ["_time"])
              |> drop(columns: {})"#,
            range_begin, range_end, organization.id(), dropped_cols
        );

        // Execute the query
        let client = reqwest::Client::new();
        let url = format!("{}/api/v2/query?org=agility", influx_instance.url());

        let response = client
            .post(&url)
            // .basic_auth(username, Some(password))
            .header("Authorization", format!("Token {}", token))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&serde_json::json!({
                "query": flux_query,
                "type": "flux"
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(ArcError::influx_query_error(error_text));
        }

        let result_text = response.text().await?;

        // Parse result_text as CSV to filter and format the output
        let filtered_csv = filter_csv_columns(&result_text)?;

        // Write the filtered CSV to the output file
        std::fs::write(output, &filtered_csv)?;

        let msg = format!(
            "Start : {}\nEnd   : {}\nOrg   : {}\nOutput: {}",
            range_begin, range_end, organization.id(), output.display()
        );
        let outro_text = OutroText::multi("Influx Query Params".to_string(), msg);

        Ok(GoalStatus::Completed(TaskResult::InfluxDumpCompleted, outro_text))
    }
}

fn filter_csv_columns(csv_text: &str) -> Result<String, ArcError> {
    let mut output_lines = Vec::new();
    let mut lines = csv_text.lines();

    // Assume the first line is the header
    let header = lines.next().ok_or_else(|| {
        ArcError::influx_query_error("Empty CSV response")
    })?;

    // Parse the header to find column indices
    // Lines begin with a comma, so the first element will be empty
    let columns: Vec<&str> = header.split(',').collect();

    // Find indices for _time and event_type
    let mut ordered_indices: Vec<usize> = ORDERED_COLS.iter()
        .filter_map(|&col_name| columns.iter().position(|&col| col.trim() == col_name))
        .collect();

    // Now add indices for all other columns (excluding filtered columns)
    for (i, col) in columns.iter().enumerate() {
        let col_name = col.trim();
        // Keep all columns except leading empty column, filtered, and ordered columns
        if !col_name.is_empty() && !FILTERED_COLS.contains(&col_name) && !ORDERED_COLS.contains(&col_name) {
            ordered_indices.push(i);
        }
    }

    // Build filtered header
    let filtered_header: Vec<String> = ordered_indices
        .iter()
        .map(|&i| columns[i].to_string())
        .collect();
    output_lines.push(filtered_header.join(","));

    // Process data rows
    for line in lines {
        if line.is_empty() {
            output_lines.push(line.to_string());
            continue;
        }

        let values: Vec<&str> = line.split(',').collect();
        let filtered_values: Vec<String> = ordered_indices
            .iter()
            .filter_map(|&i| values.get(i).map(|v| v.to_string()))
            .collect();

        output_lines.push(filtered_values.join(","));
    }

    Ok(output_lines.join("\n"))
}

