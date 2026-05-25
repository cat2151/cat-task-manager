use std::path::Path;

use chrono::{Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    event::{KeyAction, KeyBindings},
    storage::{self, TaskStatus},
};

mod model;
pub use model::{DailyTask, TaskList, TaskState, TaskTab, ViewMode};

const ALL_TAB_LABEL: &str = "all";

#[derive(Debug, Clone, Copy)]
struct TaskLocation {
    tab_index: usize,
    task_index: usize,
}

#[derive(Debug)]
pub struct App {
    pub tabs: Vec<TaskTab>,
    pub current_date: NaiveDate,
    view_mode: ViewMode,
    selected_tab: usize,
    selected_visible: usize,
    show_help: bool,
    background_message: Option<String>,
    spinner_frame: usize,
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
            background_message: None,
            spinner_frame: 0,
            message: "待機中".to_string(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, keybindings: &KeyBindings) {
        let action = keybindings.action_for(&key);
        if action == Some(KeyAction::Help) {
            self.toggle_help();
        } else if self.show_help {
            if key.code == KeyCode::Esc {
                self.show_help = false;
                self.message = "ヘルプを閉じました".to_string();
            }
        } else if let Some(action) = action {
            match action {
                KeyAction::Next => self.select_next(),
                KeyAction::Previous => self.select_previous(),
                KeyAction::NextTab => self.select_next_tab(),
                KeyAction::PreviousTab => self.select_previous_tab(),
                KeyAction::Advance => self.advance_selected(),
                KeyAction::Hold => self.toggle_hold_selected(),
                KeyAction::ToggleView => self.toggle_view_mode(),
                KeyAction::Quit | KeyAction::Edit | KeyAction::Help => {}
            }
        }
    }

    pub fn visible_tasks(&self) -> Vec<(usize, &DailyTask)> {
        self.current_task_entries()
            .into_iter()
            .filter(|(_, _, task)| self.task_is_visible(task))
            .map(|(display_index, _, task)| (display_index, task))
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

    pub fn display_tab_count(&self) -> usize {
        self.tabs.len() + 1
    }

    pub fn display_tab_label(&self, index: usize) -> Option<&str> {
        if index == 0 {
            Some(ALL_TAB_LABEL)
        } else {
            self.tabs.get(index - 1).map(|tab| tab.label.as_str())
        }
    }

    pub fn selected_tab(&self) -> usize {
        self.selected_tab
    }

    pub fn current_tab_is_all(&self) -> bool {
        self.selected_tab == 0
    }

    pub fn current_tab_label(&self) -> &str {
        self.display_tab_label(self.selected_tab)
            .unwrap_or(ALL_TAB_LABEL)
    }

    pub fn current_tab_path(&self) -> Option<&Path> {
        self.current_file_tab().map(|tab| tab.path.as_path())
    }

    pub fn current_tasks(&self) -> Vec<&DailyTask> {
        self.current_task_entries()
            .into_iter()
            .map(|(_, _, task)| task)
            .collect()
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

    pub fn start_background_work(&mut self, message: impl Into<String>) {
        self.background_message = Some(message.into());
        self.spinner_frame = 0;
    }

    pub fn finish_background_work(&mut self) {
        self.background_message = None;
        self.spinner_frame = 0;
    }

    pub fn tick_background_work(&mut self) {
        if self.background_message.is_some() {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
        }
    }

    pub fn has_background_work(&self) -> bool {
        self.background_message.is_some()
    }

    pub fn background_message(&self) -> Option<&str> {
        self.background_message.as_deref()
    }

    pub fn spinner_frame(&self) -> usize {
        self.spinner_frame
    }

    pub fn replace_tabs(&mut self, task_lists: Vec<TaskList>) {
        let selected_path = self.current_tab_path().map(Path::to_path_buf);
        self.tabs = task_lists.into_iter().map(task_tab_from_list).collect();
        self.selected_tab = selected_path
            .and_then(|selected_path| {
                self.tabs
                    .iter()
                    .position(|tab| tab.path == selected_path)
                    .map(|index| index + 1)
            })
            .unwrap_or_else(|| {
                self.selected_tab
                    .min(self.display_tab_count().saturating_sub(1))
            });
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
        self.selected_tab = (self.selected_tab + 1) % self.display_tab_count();
        self.selected_visible = 0;
        self.message = format!("タブ: {}", self.current_tab_label());
    }

    fn select_previous_tab(&mut self) {
        self.selected_tab = self
            .selected_tab
            .checked_sub(1)
            .unwrap_or_else(|| self.display_tab_count() - 1);
        self.selected_visible = 0;
        self.message = format!("タブ: {}", self.current_tab_label());
    }

    fn advance_selected(&mut self) {
        let Some((display_index, location)) = self.selected_task_location() else {
            self.message = "表示対象のタスクがありません".to_string();
            return;
        };

        match self.task_at(location).state.clone() {
            TaskState::NotStarted => {
                if self.previous_task_is_done(display_index) {
                    let now = Local::now();
                    let task = self.task_at_mut(location);
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
                let task = self.task_at_mut(location);
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
        let Some((_, location)) = self.selected_task_location() else {
            self.message = "表示対象のタスクがありません".to_string();
            return;
        };

        let task = self.task_at_mut(location);
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
        self.clamp_selection();
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
        self.visible_tasks().len()
    }

    fn clamp_selection(&mut self) {
        let visible_len = self.visible_count();
        if visible_len == 0 {
            self.selected_visible = 0;
        } else {
            self.selected_visible = self.selected_visible.min(visible_len - 1);
        }
    }

    fn previous_task_is_done(&self, display_index: usize) -> bool {
        if display_index == 0 {
            return true;
        }

        self.current_task_entries()
            .get(display_index - 1)
            .is_some_and(|(_, _, task)| task.state == TaskState::Done)
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

    fn current_file_tab(&self) -> Option<&TaskTab> {
        self.selected_tab
            .checked_sub(1)
            .and_then(|index| self.tabs.get(index))
    }

    fn current_task_locations(&self) -> Vec<TaskLocation> {
        if self.current_tab_is_all() {
            return self
                .tabs
                .iter()
                .enumerate()
                .flat_map(|(tab_index, tab)| {
                    (0..tab.tasks.len()).map(move |task_index| TaskLocation {
                        tab_index,
                        task_index,
                    })
                })
                .collect();
        }

        self.selected_tab
            .checked_sub(1)
            .and_then(|tab_index| self.tabs.get(tab_index).map(|tab| (tab_index, tab)))
            .map(|(tab_index, tab)| {
                (0..tab.tasks.len())
                    .map(|task_index| TaskLocation {
                        tab_index,
                        task_index,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn current_task_entries(&self) -> Vec<(usize, TaskLocation, &DailyTask)> {
        self.current_task_locations()
            .into_iter()
            .enumerate()
            .map(|(display_index, location)| (display_index, location, self.task_at(location)))
            .collect()
    }

    fn selected_task_location(&self) -> Option<(usize, TaskLocation)> {
        self.current_task_entries()
            .into_iter()
            .filter(|(_, _, task)| self.task_is_visible(task))
            .nth(self.selected_visible)
            .map(|(display_index, location, _)| (display_index, location))
    }

    fn task_is_visible(&self, task: &DailyTask) -> bool {
        task.state.visible() && !self.hides_on_hold_for_current_task(task)
    }

    fn hides_on_hold_for_current_task(&self, task: &DailyTask) -> bool {
        self.current_tab_is_all()
            && self.view_mode == ViewMode::OneLine
            && task.state == TaskState::OnHold
    }

    fn task_at(&self, location: TaskLocation) -> &DailyTask {
        &self.tabs[location.tab_index].tasks[location.task_index]
    }

    fn task_at_mut(&mut self, location: TaskLocation) -> &mut DailyTask {
        &mut self.tabs[location.tab_index].tasks[location.task_index]
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
