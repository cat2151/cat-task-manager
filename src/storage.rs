use crate::app::TaskState;

pub const APP_NAME: &str = "cat-task-manager";

mod config;
mod paths;
mod tasks;

pub use config::{
    load_config_file, AutoFreeTimeConfig, KeyBindingsConfig, MonokaiColorName, UiConfig,
};
pub use paths::{app_paths, ensure_app_storage, AppPaths};
pub use tasks::{
    is_task_file_path, load_task_file_content, load_task_files, write_task_file_status, Task,
    TaskFile, TaskFileStatus, TaskStatus,
};

impl TaskState {
    pub fn status_value(&self) -> &'static str {
        match self {
            TaskState::NotStarted => "not_started",
            TaskState::InProgress => "in_progress",
            TaskState::Done => "done",
            TaskState::OnHold => "on_hold",
            TaskState::Deferred => "deferred",
        }
    }

    pub fn from_status_value(value: &str) -> Option<Self> {
        match value {
            "not_started" => Some(TaskState::NotStarted),
            "in_progress" => Some(TaskState::InProgress),
            "done" => Some(TaskState::Done),
            "on_hold" => Some(TaskState::OnHold),
            "deferred" => Some(TaskState::Deferred),
            _ => None,
        }
    }
}
