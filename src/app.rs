use std::path::Path;

use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    event::{KeyAction, KeyBindings},
    storage::TaskStatus,
};

mod actions;
mod model;
pub use model::{DailyTask, TaskList, TaskState, TaskTab, ViewMode};

const ALL_TAB_LABEL: &str = "all";
const EMPTY_VISIBLE_TASKS_MESSAGE: &str = "表示対象のタスクはありません";
const ALL_DONE_VISIBLE_TASKS_MESSAGE: &str = "このタブのタスクはすべて完了済みです";

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
                KeyAction::Defer => self.defer_selected(),
                KeyAction::ToggleView => self.toggle_view_mode(),
                KeyAction::Quit | KeyAction::Edit | KeyAction::Help => {}
            }
        }
    }

    pub fn visible_tasks(&self) -> Vec<(usize, &DailyTask)> {
        self.current_task_entries()
            .into_iter()
            .filter(|(_, location, task)| self.task_is_visible(*location, task))
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

    pub fn empty_visible_tasks_message(&self) -> &'static str {
        if self.current_tab_tasks_are_all_done() {
            ALL_DONE_VISIBLE_TASKS_MESSAGE
        } else {
            EMPTY_VISIBLE_TASKS_MESSAGE
        }
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    pub fn start_background_work(&mut self, message: impl Into<String>) {
        self.background_message = Some(message.into());
        self.spinner_frame = 0;
    }

    pub fn set_background_message(&mut self, message: impl Into<String>) {
        if self.background_message.is_some() {
            self.background_message = Some(message.into());
        } else {
            self.set_message(message);
        }
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

    pub fn complete_day(&mut self, _records_dir: impl AsRef<Path>, new_date: NaiveDate) {
        // records output is frozen while validating the date-change snapshot flow.
        self.reset_for_new_day(new_date);
        self.message = format!("日付を更新しました: {new_date}");
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

    fn previous_task_allows_start(&self, display_index: usize, location: TaskLocation) -> bool {
        if display_index == 0 {
            return true;
        }

        self.current_task_entries()
            .into_iter()
            .take(display_index)
            .all(|(_, previous_location, task)| {
                !self.task_blocks_start_of(previous_location, task, location)
            })
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
            .filter(|(_, location, task)| self.task_is_visible(*location, task))
            .nth(self.selected_visible)
            .map(|(display_index, location, _)| (display_index, location))
    }

    fn task_is_visible(&self, location: TaskLocation, task: &DailyTask) -> bool {
        task.state.visible() && !self.hides_from_one_line_task(location, task)
    }

    fn current_tab_tasks_are_all_done(&self) -> bool {
        let tasks = self.current_tasks();
        !tasks.is_empty() && tasks.iter().all(|task| task.state == TaskState::Done)
    }

    fn hides_from_one_line_task(&self, location: TaskLocation, task: &DailyTask) -> bool {
        self.view_mode == ViewMode::OneLine
            && (task.state == TaskState::Deferred
                || (self.current_tab_is_all()
                    && (task.state == TaskState::OnHold
                        || self.same_tab_has_prior_on_hold(location))))
    }

    fn same_tab_has_prior_on_hold(&self, location: TaskLocation) -> bool {
        self.tabs[location.tab_index].tasks[..location.task_index]
            .iter()
            .any(|task| task.state == TaskState::OnHold)
    }

    fn task_blocks_start_of(
        &self,
        previous_location: TaskLocation,
        previous_task: &DailyTask,
        location: TaskLocation,
    ) -> bool {
        if previous_task.state.allows_next_task() {
            return false;
        }

        !(self.current_tab_is_all()
            && previous_location.tab_index != location.tab_index
            && self.tab_has_on_hold(previous_location.tab_index))
    }

    fn tab_has_on_hold(&self, tab_index: usize) -> bool {
        self.tabs[tab_index]
            .tasks
            .iter()
            .any(|task| task.state == TaskState::OnHold)
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
