mod models;
mod clients;
mod keyrings;
mod tasks;

// Re-export Args for use in main.rs
pub use models::args::CliArgs;

use std::collections::HashSet;
use cliclack::{outro, outro_note};
use console::style;
use models::errors::ArcError;
use std;
use models::config::CliConfig;
use models::goals::{GlobalParams, Goal};
use models::state::State;
use crate::tasks::TaskResult;

pub async fn run(args: CliArgs) -> Result<(), ArcError> {
    // Load CLI configuration from file, or use defaults if file does not exist
    let config_file = config_file()?;
    let config: CliConfig = if config_file.exists() {
        let toml_content = std::fs::read_to_string(config_file)?;
        toml::from_str(&toml_content)?
    } else {
        CliConfig::default()
    };

    // Extract top-level global parameters from the CLI args
    let global_params = args.global_params();

    // A single ArcCommand may map to multiple goals
    // (e.g., Switch may require both AWS profile and Kube context selection)
    let terminal_goals = args.to_goals();

    // Execute each goal, including any dependent goals
    execute_goals(terminal_goals, config, global_params).await
}

async fn execute_goals(
    terminal_goals: Vec<Goal>,
    config: CliConfig,
    global_params: GlobalParams,
) -> Result<(), ArcError> {
    let mut goals = terminal_goals.clone();
    let mut eval_string = String::new();
    let mut state = State::new();
    let mut intros: HashSet<Goal> = HashSet::new();

    // Process goals until there are none left, peeking and processing before popping
    while let Some(next_goal) = goals.last() {
        let Goal { goal_type, params, is_terminal_goal } = next_goal;

        // Check to see if the goal has already been completed. While unlikely,
        // it's possible if multiple goals depend on the same sub-goal.
        if state.contains(next_goal) {
            goals.pop();
            continue;
        }

        // Instantiate a task for the current goal
        let task = goal_type.to_task();

        // Determine if this is one of the original, user-requested goals
        if *is_terminal_goal && !intros.contains(next_goal) {
            task.print_intro()?;
            intros.insert(next_goal.clone());
        }

        // Attempt to complete the next goal on the stack
        let goal_result = task.execute(
            params,
            &config,
            &global_params,
            &state,
        ).await;

        // If next goal indicates that it needs the result of a dependent goal, then add the
        // dependent goal onto the stack, leaving the original goal to be executed at a later time.
        // Otherwise, pop the goal from the stack and store its result in the state.
        match goal_result? {
            GoalStatus::Needs(dependent_goal) => goals.push(dependent_goal),
            GoalStatus::Completed(result, outro_text) => {
                if *is_terminal_goal {
                    // Print outro message (to std_err)
                    let raw_value = match outro_text {
                        OutroText::SingleLine{ key, value } => {
                            let text = format!("{}: {}", style(&key).green(), style(&value).dim());
                            outro(text)?;
                            value
                        },
                        OutroText::MultiLine{ key, value } => {
                            let prompt = style(&key).green();
                            let message = style(&value).dim();
                            outro_note(prompt, message)?;
                            value
                        },
                        OutroText::None => String::new(),
                    };

                    // Print value (to std_out) if --raw flag is provided
                    // This is useful when calling `arc` from scripts
                    if global_params.raw_output {
                        println!("{raw_value}");
                    }
                }

                // Collect any text that needs to be eval'd in the parent shell
                if let Some(s) = result.eval_string() {
                    eval_string.push_str(&s);
                }

                // Pop the completed goal and store its result in state
                let goal = goals.pop().unwrap();
                state.insert(goal, result);
            },
        }
    }

    if !global_params.raw_output {
        // This is the final output that the parent shell should eval (unless called from a script)
        // All other program outputs are sent to stderr (i.e. clickack interactive menus, outros, etc).
        println!("__EVAL__{eval_string}");
    }

    Ok(())
}

pub enum GoalStatus {
    Completed(TaskResult, OutroText),
    Needs(Goal),
}

pub enum OutroText {
    SingleLine{ key: String, value: String },
    MultiLine{ key: String, value: String },
    None,
}

impl OutroText {
    pub fn single(key: String, value: String) -> OutroText {
        OutroText::SingleLine { key, value }
    }
    pub fn multi(key: String, value: String) -> OutroText {
        OutroText::MultiLine { key, value }
    }
}

fn config_dir() -> Result<std::path::PathBuf, ArcError> {
    //TODO .arc-cli path should be configurable
    let mut path = home::home_dir().ok_or_else(|| ArcError::HomeDirError)?;
    path.push(".arc-cli");

    // Create the config directory if it doesn't already exist
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

fn config_file() -> Result<std::path::PathBuf, ArcError> {
    let mut path = config_dir()?;
    path.push("config.toml");
    Ok(path)
}
