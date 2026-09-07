use std::path::PathBuf;

use chrono::{DateTime, Local};

use crate::storage::Task;

use super::{FREE_TIME_TAB_LABEL, FREE_TIME_TASK_NAME};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    NotStarted,
    InProgress,
    Done,
    OnHold,
    Deferred,
}

impl TaskState {
    pub fn label(&self) -> &'static str {
        match self {
            TaskState::NotStarted => "未着手",
            TaskState::InProgress => "実施中",
            TaskState::Done => "完了",
            TaskState::OnHold => "保留",
            TaskState::Deferred => "後回し",
        }
    }

    pub fn visible(&self) -> bool {
        matches!(
            self,
            TaskState::NotStarted | TaskState::InProgress | TaskState::OnHold | TaskState::Deferred
        )
    }

    pub fn allows_next_task(&self) -> bool {
        matches!(self, TaskState::Done | TaskState::Deferred)
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
    pub pauses: Vec<TaskPause>,
    pub free_time_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskPause {
    pub paused_at: DateTime<Local>,
    pub resumed_at: Option<DateTime<Local>>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskEventKind {
    Started,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskEvent {
    pub kind: TaskEventKind,
    pub occurred_at: DateTime<Local>,
    pub task_name: String,
    pub task_file: PathBuf,
    pub source_line: u32,
}

pub(super) fn task_tab_from_list(task_list: TaskList) -> TaskTab {
    let is_free_time_tab = task_list.label == FREE_TIME_TAB_LABEL;
    TaskTab {
        label: task_list.label,
        path: task_list.path,
        tasks: task_list
            .tasks
            .into_iter()
            .map(|task| {
                let is_free_time_task = is_free_time_tab && task.name == FREE_TIME_TASK_NAME;
                DailyTask {
                    name: task.name,
                    order: task.order,
                    source_line: task.source_line,
                    state: if is_free_time_task {
                        TaskState::Done
                    } else {
                        TaskState::NotStarted
                    },
                    started_at: None,
                    completed_at: None,
                    pauses: Vec::new(),
                    free_time_seconds: is_free_time_task.then_some(0),
                }
            })
            .collect(),
    }
}
