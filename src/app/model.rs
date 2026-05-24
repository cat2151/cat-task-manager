use std::path::PathBuf;

use chrono::{DateTime, Local};

use crate::storage::Task;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    NotStarted,
    InProgress,
    Done,
    TimeOut,
    OnHold,
}

impl TaskState {
    pub fn on_day_changed(&self) -> Self {
        match self {
            TaskState::Done => TaskState::Done,
            _ => TaskState::TimeOut,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TaskState::NotStarted => "未着手",
            TaskState::InProgress => "実施中",
            TaskState::Done => "完了",
            TaskState::TimeOut => "時間切れ",
            TaskState::OnHold => "保留",
        }
    }

    pub fn visible(&self) -> bool {
        matches!(
            self,
            TaskState::NotStarted | TaskState::InProgress | TaskState::OnHold
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    OneLine,
    Incomplete,
    All,
}

impl ViewMode {
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::OneLine => "1行",
            ViewMode::Incomplete => "未完了",
            ViewMode::All => "全体表示",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DailyTask {
    pub name: String,
    pub order: u32,
    pub source_line: u32,
    pub state: TaskState,
    pub started_at: Option<DateTime<Local>>,
    pub completed_at: Option<DateTime<Local>>,
}

#[derive(Debug, Clone)]
pub struct TaskList {
    pub label: String,
    pub path: PathBuf,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone)]
pub struct TaskTab {
    pub label: String,
    pub path: PathBuf,
    pub tasks: Vec<DailyTask>,
}
