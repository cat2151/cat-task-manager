use std::path::Path;

use chrono::{DateTime, Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    event::KeyBindings,
    storage::{self, Task, TaskStatus},
};

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

#[derive(Debug)]
pub struct App {
    pub tasks: Vec<DailyTask>,
    pub current_date: NaiveDate,
    view_mode: ViewMode,
    selected_visible: usize,
    show_help: bool,
    message: String,
}

impl App {
    pub fn new(tasks: Vec<Task>, current_date: NaiveDate) -> Self {
        Self {
            tasks: tasks
                .into_iter()
                .map(|task| DailyTask {
                    name: task.name,
                    order: task.order,
                    source_line: task.source_line,
                    state: TaskState::NotStarted,
                    started_at: None,
                    completed_at: None,
                })
                .collect(),
            current_date,
            view_mode: ViewMode::OneLine,
            selected_visible: 0,
            show_help: false,
            message: "待機中".to_string(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, keybindings: &KeyBindings) {
        if keybindings.help.matches(&key) {
            self.toggle_help();
        } else if self.show_help {
            if key.code == KeyCode::Esc {
                self.show_help = false;
                self.message = "ヘルプを閉じました".to_string();
            }
        } else if keybindings.next.matches(&key) {
            self.select_next();
        } else if keybindings.previous.matches(&key) {
            self.select_previous();
        } else if keybindings.advance.matches(&key) {
            self.advance_selected();
        } else if keybindings.hold.matches(&key) {
            self.toggle_hold_selected();
        } else if keybindings.toggle_view.matches(&key) {
            self.toggle_view_mode();
        }
    }

    pub fn visible_tasks(&self) -> Vec<(usize, &DailyTask)> {
        self.tasks
            .iter()
            .enumerate()
            .filter(|(_, task)| task.state.visible())
            .collect()
    }

    pub fn selected_visible(&self) -> usize {
        self.selected_visible
    }

    pub fn selected_visible_task(&self) -> Option<(usize, &DailyTask)> {
        self.visible_tasks().get(self.selected_visible).copied()
    }

    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    pub fn show_help(&self) -> bool {
        self.show_help
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    pub fn replace_tasks(&mut self, tasks: Vec<Task>) {
        self.tasks = tasks
            .into_iter()
            .map(|task| DailyTask {
                name: task.name,
                order: task.order,
                source_line: task.source_line,
                state: TaskState::NotStarted,
                started_at: None,
                completed_at: None,
            })
            .collect();
        self.clamp_selection();
    }

    pub fn apply_statuses(&mut self, statuses: &[TaskStatus]) {
        for (task, status) in self.tasks.iter_mut().zip(statuses) {
            task.state = status.state.clone();
            task.started_at = status.started_at;
            task.completed_at = status.completed_at;
        }
        self.clamp_selection();
    }

    pub fn complete_day(&mut self, records_dir: impl AsRef<Path>, new_date: NaiveDate) {
        match storage::write_day_record(records_dir, self.current_date, &self.tasks) {
            Ok(path) => {
                self.reset_for_new_day(new_date);
                self.message = format!("記録を書き出しました: {}", path.display());
            }
            Err(err) => {
                self.message = err;
            }
        }
    }

    fn select_next(&mut self) {
        let visible_len = self.visible_count();
        if visible_len == 0 {
            self.selected_visible = 0;
            return;
        }
        self.selected_visible = (self.selected_visible + 1).min(visible_len - 1);
    }

    fn select_previous(&mut self) {
        self.selected_visible = self.selected_visible.saturating_sub(1);
    }

    fn advance_selected(&mut self) {
        let Some(index) = self.selected_task_index() else {
            self.message = "表示対象のタスクがありません".to_string();
            return;
        };

        match self.tasks[index].state {
            TaskState::NotStarted => {
                if self.previous_task_is_done(index) {
                    let now = Local::now();
                    self.tasks[index].state = TaskState::InProgress;
                    self.tasks[index].started_at = Some(now);
                    self.tasks[index].completed_at = None;
                    self.message = format!("開始しました: {}", self.tasks[index].name);
                } else {
                    self.message = "前のタスクが完了していません".to_string();
                }
            }
            TaskState::InProgress => {
                let now = Local::now();
                self.tasks[index].state = TaskState::Done;
                if self.tasks[index].started_at.is_none() {
                    self.tasks[index].started_at = Some(now);
                }
                self.tasks[index].completed_at = Some(now);
                self.message = format!("完了しました: {}", self.tasks[index].name);
                self.clamp_selection();
            }
            TaskState::OnHold => {
                self.message = "進める前に保留を解除してください".to_string();
            }
            TaskState::Done | TaskState::TimeOut => {}
        }
    }

    fn toggle_hold_selected(&mut self) {
        let Some(index) = self.selected_task_index() else {
            self.message = "表示対象のタスクがありません".to_string();
            return;
        };

        match self.tasks[index].state {
            TaskState::InProgress => {
                self.tasks[index].state = TaskState::OnHold;
                self.message = format!("保留しました: {}", self.tasks[index].name);
            }
            TaskState::OnHold => {
                self.tasks[index].state = TaskState::InProgress;
                self.message = format!("再開しました: {}", self.tasks[index].name);
            }
            _ => {
                self.message = "保留できるのは実施中のタスクだけです".to_string();
            }
        }
    }

    fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::OneLine => ViewMode::Incomplete,
            ViewMode::Incomplete => ViewMode::All,
            ViewMode::All => ViewMode::OneLine,
        };
        self.message = format!("表示モード: {}", self.view_mode.label());
    }

    fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        self.message = if self.show_help {
            "ヘルプを表示しています".to_string()
        } else {
            "ヘルプを閉じました".to_string()
        };
    }

    pub fn selected_task_index(&self) -> Option<usize> {
        self.visible_tasks()
            .get(self.selected_visible)
            .map(|(index, _)| *index)
    }

    fn visible_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| task.state.visible())
            .count()
    }

    fn clamp_selection(&mut self) {
        let visible_len = self.visible_count();
        if visible_len == 0 {
            self.selected_visible = 0;
        } else {
            self.selected_visible = self.selected_visible.min(visible_len - 1);
        }
    }

    fn previous_task_is_done(&self, index: usize) -> bool {
        index == 0 || self.tasks[index - 1].state == TaskState::Done
    }

    fn reset_for_new_day(&mut self, new_date: NaiveDate) {
        for task in &mut self.tasks {
            task.state = TaskState::NotStarted;
            task.started_at = None;
            task.completed_at = None;
        }
        self.current_date = new_date;
        self.selected_visible = 0;
        self.show_help = false;
    }
}

#[cfg(test)]
mod tests;
