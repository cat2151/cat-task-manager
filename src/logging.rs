use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};

use crate::{
    app::{DailyTask, TaskState, TaskTab},
    clock,
    storage::AppPaths,
};

const LOGS_DIR_NAME: &str = "logs";
const APP_LOG_FILE_NAME: &str = "app.log";

#[derive(Debug)]
pub struct AppLogger {
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    tab: String,
    name: String,
    order: u32,
    source_line: u32,
    state: TaskState,
    started_at: Option<DateTime<Local>>,
    completed_at: Option<DateTime<Local>>,
}

#[derive(Debug, Clone, Copy)]
pub enum TaskChangeCause {
    KeyAdvance,
    KeyHold,
    KeyOther,
    TaskFileRead,
    DayChanged,
}

pub struct AppRunLog<'a> {
    logger: &'a AppLogger,
}

impl AppLogger {
    pub fn new(root_dir: impl AsRef<Path>) -> Result<Self, String> {
        let logs_dir = root_dir.as_ref().join(LOGS_DIR_NAME);
        fs::create_dir_all(&logs_dir).map_err(|err| {
            format!(
                "log directory を作成できませんでした: {} ({err})",
                logs_dir.display()
            )
        })?;

        Ok(Self {
            path: logs_dir.join(APP_LOG_FILE_NAME),
        })
    }

    pub fn log_app_start(&self, paths: &AppPaths) -> Result<(), String> {
        self.write_event(format!(
            "アプリを開始しました: pid={} config={} tasks_dir={} log={}",
            std::process::id(),
            quoted_path(&paths.config_path),
            quoted_path(&paths.tasks_dir),
            quoted_path(&self.path)
        ))
    }

    pub fn log_app_exit(&self) -> Result<(), String> {
        self.write_event(format!("アプリを終了しました: pid={}", std::process::id()))
    }

    pub fn log_task_file_status_read(
        &self,
        path: impl AsRef<Path>,
        outcome: TaskFileStatusReadOutcome,
    ) -> Result<(), String> {
        self.write_event(format!(
            "tasks fileの状態を読みました: path={} 結果={}{}",
            quoted_path(path.as_ref()),
            outcome.label(),
            outcome.detail_field()
        ))
    }

    pub fn log_task_changes(
        &self,
        before: &[TaskSnapshot],
        after: &[TaskTab],
        cause: TaskChangeCause,
    ) -> Result<(), String> {
        for tab in after {
            for task in &tab.tasks {
                let before = before.iter().find(|before| {
                    before.tab == tab.label
                        && before.source_line == task.source_line
                        && before.name == task.name
                });
                if !task_changed(before, task) {
                    continue;
                }
                self.log_task_change(tab.label.as_str(), before, task, cause)?;
            }
        }

        Ok(())
    }

    fn log_task_change(
        &self,
        tab: &str,
        before: Option<&TaskSnapshot>,
        after: &DailyTask,
        cause: TaskChangeCause,
    ) -> Result<(), String> {
        let from_state = before.map(|before| before.state.label()).unwrap_or("なし");
        let from_started_at = before.and_then(|before| before.started_at.as_ref());
        let from_completed_at = before.and_then(|before| before.completed_at.as_ref());
        let start_fields = start_fields(before.map(|before| &before.state), after, cause);

        self.write_event(format!(
            "タスク状態を変更しました: tab={} task={} line={} order={} 変更前={} 変更後={} 開始前={} 開始後={} 完了前={} 完了後={} 原因={} 分類={}{}",
            quoted(tab),
            quoted(&after.name),
            before
                .map(|before| before.source_line)
                .unwrap_or(after.source_line),
            before.map(|before| before.order).unwrap_or(after.order),
            from_state,
            after.state.label(),
            optional_time(from_started_at),
            optional_time(after.started_at.as_ref()),
            optional_time(from_completed_at),
            optional_time(after.completed_at.as_ref()),
            cause.label(),
            cause.group_label(),
            start_fields
        ))
    }

    fn write_event(&self, message: String) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|err| {
                format!(
                    "log fileを開けませんでした: {} ({err})",
                    self.path.display()
                )
            })?;
        writeln!(file, "{} {message}", clock::now_jst().to_rfc3339()).map_err(|err| {
            format!(
                "log fileを書き込めませんでした: {} ({err})",
                self.path.display()
            )
        })
    }
}

impl<'a> AppRunLog<'a> {
    pub fn start(logger: &'a AppLogger, paths: &AppPaths) -> Result<Self, String> {
        logger.log_app_start(paths)?;
        Ok(Self { logger })
    }
}

impl Drop for AppRunLog<'_> {
    fn drop(&mut self) {
        let _ = self.logger.log_app_exit();
    }
}

impl TaskChangeCause {
    fn label(self) -> &'static str {
        match self {
            TaskChangeCause::KeyAdvance => "advance key",
            TaskChangeCause::KeyHold => "hold key",
            TaskChangeCause::KeyOther => "その他のkey",
            TaskChangeCause::TaskFileRead => "tasks file読み込み",
            TaskChangeCause::DayChanged => "日付変更",
        }
    }

    fn group_label(self) -> &'static str {
        match self {
            TaskChangeCause::KeyAdvance | TaskChangeCause::KeyHold | TaskChangeCause::KeyOther => {
                "key操作"
            }
            TaskChangeCause::TaskFileRead => "tasks file",
            TaskChangeCause::DayChanged => "その他",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TaskFileStatusReadOutcome {
    Missing,
    Loaded,
    DateMismatch { status_date: chrono::NaiveDate },
}

impl TaskFileStatusReadOutcome {
    fn label(self) -> &'static str {
        match self {
            TaskFileStatusReadOutcome::Missing => "なし",
            TaskFileStatusReadOutcome::Loaded => "読み込み済み",
            TaskFileStatusReadOutcome::DateMismatch { .. } => "日付不一致",
        }
    }

    fn detail_field(self) -> String {
        match self {
            TaskFileStatusReadOutcome::DateMismatch { status_date } => {
                format!(" task fileの状態日付={status_date}")
            }
            TaskFileStatusReadOutcome::Missing | TaskFileStatusReadOutcome::Loaded => String::new(),
        }
    }
}

pub fn task_snapshots(tabs: &[TaskTab]) -> Vec<TaskSnapshot> {
    tabs.iter()
        .flat_map(|tab| {
            tab.tasks.iter().map(|task| TaskSnapshot {
                tab: tab.label.clone(),
                name: task.name.clone(),
                order: task.order,
                source_line: task.source_line,
                state: task.state.clone(),
                started_at: task.started_at,
                completed_at: task.completed_at,
            })
        })
        .collect()
}

fn task_changed(before: Option<&TaskSnapshot>, after: &DailyTask) -> bool {
    match before {
        Some(before) => {
            before.state != after.state
                || before.started_at != after.started_at
                || before.completed_at != after.completed_at
        }
        None => {
            after.state != TaskState::NotStarted
                || after.started_at.is_some()
                || after.completed_at.is_some()
        }
    }
}

fn start_fields(
    before_state: Option<&TaskState>,
    after: &DailyTask,
    cause: TaskChangeCause,
) -> String {
    if after.state != TaskState::InProgress
        || before_state.is_some_and(|before| before == &TaskState::InProgress)
    {
        return String::new();
    }

    format!(" 開始元={} 開始要因={}", cause.group_label(), cause.label())
}

fn optional_time(time: Option<&DateTime<Local>>) -> String {
    time.map(clock::format_rfc3339_jst)
        .unwrap_or_else(|| "なし".to_string())
}

fn quoted_path(path: &Path) -> String {
    quoted(&path.display().to_string())
}

fn quoted(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "\\r")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}
