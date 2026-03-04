use cliclack::{intro, select};
use async_trait::async_trait;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use crate::models::args::CliArgs;
use crate::{config_dir, GoalStatus, OutroText};
use crate::models::config::CliConfig;
use crate::models::errors::ArcError;
use crate::models::goals::{GlobalParams, GoalParams};
use crate::models::state::State;
use crate::tasks::{Task, TaskResult};

#[derive(Debug)]
pub struct CreateTabCompletionsTask;

#[async_trait]
impl Task for CreateTabCompletionsTask {
    fn print_intro(&self) -> Result<(), ArcError> {
        intro("Creating tab completions file")?;
        Ok(())
    }

    async fn execute(
        &self,
        _params: &GoalParams,
        _config: &CliConfig,
        _global_params: &GlobalParams,
        _state: &State
    ) -> Result<GoalStatus, ArcError> {
        // Get a list of all available RDS instances for this account
        // let available_rds_instances = profile_info.account.rds_instances();
        let shell = prompt_for_shell()?;

        // Create a file to store the completions
        let path = completions_path(shell.to_string().to_lowercase())?;
        let mut file = std::fs::File::create(&path)?;

        // Generate the completion file
        let mut cmd = CliArgs::command();
        generate(shell, &mut cmd, "arc", &mut file);

        let prompt = "Tab completions file generated".to_string();
        let msg = format!("Completions file: {}\nSource this file from your startup script (i.e. ~/.zshrc) to enable.", path.display());
        let outro_text = OutroText::multi(prompt, msg.to_string());

        Ok(GoalStatus::Completed(TaskResult::TabCompletionsCreated, outro_text))
    }
}

fn prompt_for_shell() -> Result<Shell, ArcError> {
    let available_shells = vec!["bash", "zsh", "fish", "powershell", "elvish"];
    let mut menu = select("Select shell");
    for shell in &available_shells {
        menu = menu.item(shell, shell, "");
    }

    let shell_name = menu.interact()?.to_string();

    match shell_name.as_str() {
        "bash" => Ok(Shell::Bash),
        "zsh" => Ok(Shell::Zsh),
        "fish" => Ok(Shell::Fish),
        "powershell" => Ok(Shell::PowerShell),
        "elvish" => Ok(Shell::Elvish),
        _ => Err(ArcError::UserInputError(format!("Unsupported shell: {shell_name}"))),
    }
}

fn completions_path(shell: impl Into<String>) -> Result<std::path::PathBuf, ArcError> {
    let mut path = config_dir()?;
    path.push(format!("arc-completions-{}", shell.into()));
    Ok(path)
}
