use std::path::{Path, PathBuf};

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

#[derive(Debug)]
pub struct App {
    pub tabs: Vec<TaskTab>,
    pub current_date: NaiveDate,
    view_mode: ViewMode,
    selected_tab: usize,
    selected_visible: usize,
    show_help: bool,
    message: String,
}

impl App {
    pub fn new(task_lists: Vec<TaskList>, current_date: NaiveDate) -> Self {
        Self {
            tabs: task_lists.into_iter().map(task_tab_from_list).collect(),
            current_date,
            view_mode: ViewMode::OneLine,
            selected_tab: 0,
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
        } else if keybindings.next_tab.matches(&key) {
            self.select_next_tab();
        } else if keybindings.previous_tab.matches(&key) {
            self.select_previous_tab();
        } else if keybindings.advance.matches(&key) {
            self.advance_selected();
        } else if keybindings.hold.matches(&key) {
            self.toggle_hold_selected();
        } else if keybindings.toggle_view.matches(&key) {
            self.toggle_view_mode();
        }
    }

    pub fn visible_tasks(&self) -> Vec<(usize, &DailyTask)> {
        self.current_tasks()
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

    pub fn tabs(&self) -> &[TaskTab] {
        &self.tabs
    }

    pub fn selected_tab(&self) -> usize {
        self.selected_tab
    }

    pub fn current_tab(&self) -> &TaskTab {
        &self.tabs[self.selected_tab]
    }

    pub fn current_tab_label(&self) -> &str {
        &self.current_tab().label
    }

    pub fn current_tab_path(&self) -> &Path {
        &self.current_tab().path
    }

    pub fn current_tasks(&self) -> &[DailyTask] {
        &self.current_tab().tasks
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

    pub fn replace_tabs(&mut self, task_lists: Vec<TaskList>) {
        let selected_path = self.tabs.get(self.selected_tab).map(|tab| tab.path.clone());
        self.tabs = task_lists.into_iter().map(task_tab_from_list).collect();
        self.selected_tab = selected_path
            .and_then(|selected_path| self.tabs.iter().position(|tab| tab.path == selected_path))
            .unwrap_or_else(|| self.selected_tab.min(self.tabs.len().saturating_sub(1)));
        self.clamp_selection();
    }

    pub fn apply_statuses(&mut self, tab_index: usize, statuses: &[TaskStatus]) {
        let Some(tab) = self.tabs.get_mut(tab_index) else {
            return;
        };

        for (task, status) in tab.tasks.iter_mut().zip(statuses) {
            task.state = status.state.clone();
            task.started_at = status.started_at;
            task.completed_at = status.completed_at;
        }
        self.clamp_selection();
    }

    pub fn complete_day(&mut self, records_dir: impl AsRef<Path>, new_date: NaiveDate) {
        match storage::write_day_record(records_dir, self.current_date, &self.tabs) {
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

    fn select_next_tab(&mut self) {
        if self.tabs.is_empty() {
            self.selected_tab = 0;
            self.selected_visible = 0;
            return;
        }

        self.selected_tab = (self.selected_tab + 1) % self.tabs.len();
        self.selected_visible = 0;
        self.message = format!("タブ: {}", self.current_tab_label());
    }

    fn select_previous_tab(&mut self) {
        if self.tabs.is_empty() {
            self.selected_tab = 0;
            self.selected_visible = 0;
            return;
        }

        self.selected_tab = self
            .selected_tab
            .checked_sub(1)
            .unwrap_or_else(|| self.tabs.len() - 1);
        self.selected_visible = 0;
        self.message = format!("タブ: {}", self.current_tab_label());
    }

    fn advance_selected(&mut self) {
        let Some(index) = self.selected_task_index() else {
            self.message = "表示対象のタスクがありません".to_string();
            return;
        };

        match self.current_tasks()[index].state {
            TaskState::NotStarted => {
                if self.previous_task_is_done(index) {
                    let now = Local::now();
                    let task = &mut self.tabs[self.selected_tab].tasks[index];
                    task.state = TaskState::InProgress;
                    task.started_at = Some(now);
                    task.completed_at = None;
                    self.message = format!("開始しました: {}", task.name);
                } else {
                    self.message = "前のタスクが完了していません".to_string();
                }
            }
            TaskState::InProgress => {
                let now = Local::now();
                let task = &mut self.tabs[self.selected_tab].tasks[index];
                task.state = TaskState::Done;
                if task.started_at.is_none() {
                    task.started_at = Some(now);
                }
                task.completed_at = Some(now);
                self.message = format!("完了しました: {}", task.name);
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

        let task = &mut self.tabs[self.selected_tab].tasks[index];
        match task.state {
            TaskState::InProgress => {
                task.state = TaskState::OnHold;
                self.message = format!("保留しました: {}", task.name);
            }
            TaskState::OnHold => {
                task.state = TaskState::InProgress;
                self.message = format!("再開しました: {}", task.name);
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
        self.current_tasks()
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
        index == 0 || self.current_tasks()[index - 1].state == TaskState::Done
    }

    fn reset_for_new_day(&mut self, new_date: NaiveDate) {
        for tab in &mut self.tabs {
            for task in &mut tab.tasks {
                task.state = TaskState::NotStarted;
                task.started_at = None;
                task.completed_at = None;
            }
        }
        self.current_date = new_date;
        self.selected_visible = 0;
        self.show_help = false;
    }
}

fn task_tab_from_list(task_list: TaskList) -> TaskTab {
    TaskTab {
        label: task_list.label,
        path: task_list.path,
        tasks: task_list
            .tasks
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
    }
}

#[cfg(test)]
mod tests;
