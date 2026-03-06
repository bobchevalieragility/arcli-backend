use std::convert::From;
use clap::ValueEnum;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Off,
    Inherit,
}

impl LogLevel {
    pub fn name(&self) -> &str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Off => "OFF",
            LogLevel::Inherit => "INHERIT",
        }
    }

    pub fn value(&self) -> Value {
        match self {
            LogLevel::Trace => Value::String("trace".to_string()),
            LogLevel::Debug => Value::String("debug".to_string()),
            LogLevel::Info => Value::String("info".to_string()),
            LogLevel::Warn => Value::String("warn".to_string()),
            LogLevel::Error => Value::String("error".to_string()),
            LogLevel::Off => Value::String("off".to_string()),
            LogLevel::Inherit => Value::Null,
        }
    }

    pub(crate) fn all() -> Vec<LogLevel> {
        vec![
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
            LogLevel::Off,
            LogLevel::Inherit,
        ]
    }
}

impl From<&str> for LogLevel {
    fn from(level_name: &str) -> Self {
        match level_name {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            "OFF" => LogLevel::Off,
            "INHERIT" => LogLevel::Inherit,
            _ => panic!("Unknown log Level: {level_name}"),
        }
    }
}