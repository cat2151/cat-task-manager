use crate::app::TaskState;

pub const APP_NAME: &str = "cat-task-manager";

mod config;
mod paths;
mod records;
mod tasks;

pub use config::{load_config_file, KeyBindingsConfig};
pub use paths::{app_paths, ensure_app_storage, AppPaths};
pub use records::write_day_record;
pub use tasks::{
    load_task_files, write_task_file_status, Task, TaskFile, TaskFileStatus, TaskStatus,
};

impl TaskState {
    pub fn record_value(&self) -> &'static str {
        match self {
            TaskState::NotStarted => "not_started",
            TaskState::InProgress => "in_progress",
            TaskState::Done => "done",
            TaskState::TimeOut => "timeout",
            TaskState::OnHold => "on_hold",
            TaskState::Deferred => "deferred",
        }
    }

    pub fn from_record_value(value: &str) -> Option<Self> {
        match value {
            "not_started" => Some(TaskState::NotStarted),
            "in_progress" => Some(TaskState::InProgress),
            "done" => Some(TaskState::Done),
            "timeout" => Some(TaskState::TimeOut),
            "on_hold" => Some(TaskState::OnHold),
            "deferred" => Some(TaskState::Deferred),
            _ => None,
        }
    }
}
