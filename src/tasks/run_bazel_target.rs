use async_trait::async_trait;
use cliclack::{intro, outro_note};
use console::style;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task::AbortHandle;
use crate::models::errors::ArcError;
use crate::models::goals::GoalParams;
use crate::{GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct RunBazelTargetTask;

#[derive(Debug)]
pub struct BazelProcessInfo {
    pub target: String,
    pub handle: AbortHandle,
}

impl BazelProcessInfo {
    pub fn new(target: String, handle: AbortHandle) -> Self {
        BazelProcessInfo { target, handle }
    }
}

impl Drop for BazelProcessInfo {
    // Ensure graceful cleanup of the spawned bazel process
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[async_trait]
impl Task for RunBazelTargetTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Run Bazel Target")?;
        Ok(())
    }

    async fn execute(
        &self,
        params: &GoalParams,
        config: &CliConfig,
        _state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Extract bazel target and tear_down flag from params
        let (target, tear_down) = match params {
            GoalParams::BazelTargetRunning { target, tear_down } => (target, *tear_down),
            _ => return Err(ArcError::invalid_goal_params(
                crate::models::goals::GoalType::BazelTargetRunning,
                params
            )),
        };

        // Validate that bazel is installed
        let bazel_check = Command::new("which")
            .arg("bazel")
            .output()
            .await;

        if bazel_check.is_err() || !bazel_check?.status.success() {
            return Err(ArcError::BazelNotFound);
        }

        // Get the bazel workspace directory from config (with tilde expansion)
        let bazel_workspace = config.bazel.agility_software_repo()?;

        // Validate that the workspace directory exists
        if !bazel_workspace.exists() {
            return Err(ArcError::CommandExecutionError(
                format!("Bazel workspace directory does not exist: {}", bazel_workspace.display())
            ));
        }

        // Spawn the bazel run command
        let qualified_target = format!("//package/{}", target);
        let mut child = Command::new("bazel")
            .arg("run")
            .arg(&qualified_target)
            .current_dir(&bazel_workspace)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ArcError::CommandExecutionError(format!("Failed to spawn bazel: {}", e)))?;

        // Get handles to stdout and stderr
        let stdout = child.stdout.take()
            .ok_or_else(|| ArcError::CommandExecutionError("Failed to capture stdout".to_string()))?;
        let stderr = child.stderr.take()
            .ok_or_else(|| ArcError::CommandExecutionError("Failed to capture stderr".to_string()))?;

        // Spawn tasks to stream output
        let short_name = target.split(':').last().unwrap_or(target);
        let short_name_stdout = short_name.to_string();
        let stdout_handle = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                eprintln!("{}:{}", style(&short_name_stdout).blue(), line);
            }
        });

        let short_name_stderr = short_name.to_string();
        let stderr_handle = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                eprintln!("{}:{}", style(&short_name_stderr).blue(), style(line).red());
            }
        });

        // Spawn a task to manage the child process
        let target_clone = target.clone();
        let process_handle = tokio::spawn(async move {
            match child.wait().await {
                Ok(status) if status.success() => {
                    eprintln!("Bazel target {} completed successfully", target_clone);
                }
                Ok(status) => {
                    eprintln!("Bazel target {} exited with status: {}", target_clone, status);
                }
                Err(e) => {
                    eprintln!("Failed to wait for bazel target {}: {}", target_clone, e);
                }
            }

            // Wait for output tasks to complete
            let _ = stdout_handle.await;
            let _ = stderr_handle.await;
        });

        let prompt = "Bazel target is running. Press Ctrl+C to terminate";
        let summary_msg = format!("{}{}", style("Target: ").dim(), style(qualified_target).cyan());
        outro_note(style(prompt).green(), summary_msg)?;

        if !tear_down {
            // Wait indefinitely - task will run until user interrupts (Ctrl+C)
            tokio::signal::ctrl_c().await?;
        }

        // Return the process handle for later cleanup
        let info = BazelProcessInfo::new(target.clone(), process_handle.abort_handle());
        Ok(GoalStatus::Completed(TaskResult::BazelProcess(info), OutroText::None))
    }
}
