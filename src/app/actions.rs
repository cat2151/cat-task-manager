use chrono::Local;

use super::{App, TaskState, ViewMode};

impl App {
    pub(super) fn select_next(&mut self) {
        let visible_len = self.visible_count();
        if visible_len == 0 {
            self.selected_visible = 0;
            return;
        }
        self.selected_visible = (self.selected_visible + 1).min(visible_len - 1);
    }

    pub(super) fn select_previous(&mut self) {
        self.selected_visible = self.selected_visible.saturating_sub(1);
    }

    pub(super) fn select_next_tab(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % self.display_tab_count();
        self.selected_visible = 0;
        self.message = format!("タブ: {}", self.current_tab_label());
    }

    pub(super) fn select_previous_tab(&mut self) {
        self.selected_tab = self
            .selected_tab
            .checked_sub(1)
            .unwrap_or_else(|| self.display_tab_count() - 1);
        self.selected_visible = 0;
        self.message = format!("タブ: {}", self.current_tab_label());
    }

    pub(super) fn advance_selected(&mut self) {
        let Some((display_index, location)) = self.selected_task_location() else {
            self.message = self.empty_visible_tasks_message().to_string();
            return;
        };

        match self.task_at(location).state.clone() {
            TaskState::NotStarted => {
                if self.previous_task_allows_start(display_index, location) {
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
            TaskState::Deferred => {
                let now = Local::now();
                let task = self.task_at_mut(location);
                task.state = TaskState::InProgress;
                if task.started_at.is_none() {
                    task.started_at = Some(now);
                }
                task.completed_at = None;
                self.message = format!("再開しました: {}", task.name);
            }
            TaskState::OnHold => {
                self.message = "進める前に保留を解除してください".to_string();
            }
            TaskState::Done => {}
        }
    }

    pub(super) fn toggle_hold_selected(&mut self) {
        let Some((_, location)) = self.selected_task_location() else {
            self.message = self.empty_visible_tasks_message().to_string();
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

    pub(super) fn defer_selected(&mut self) {
        let Some((_, location)) = self.selected_task_location() else {
            self.message = self.empty_visible_tasks_message().to_string();
            return;
        };

        let task = self.task_at_mut(location);
        match task.state {
            TaskState::NotStarted | TaskState::InProgress => {
                task.state = TaskState::Deferred;
                task.completed_at = None;
                self.message = format!("後回しにしました: {}", task.name);
                self.clamp_selection();
            }
            TaskState::Deferred => {
                task.state = if task.started_at.is_some() {
                    TaskState::InProgress
                } else {
                    TaskState::NotStarted
                };
                self.message = format!("後回しを解除しました: {}", task.name);
            }
            _ => {
                self.message = "後回しにできるのは未着手か実施中のタスクだけです".to_string();
            }
        }
    }

    pub(super) fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::OneLine => ViewMode::Incomplete,
            ViewMode::Incomplete => ViewMode::All,
            ViewMode::All => ViewMode::OneLine,
        };
        self.clamp_selection();
        self.message = format!("表示モード: {}", self.view_mode.label());
    }

    pub(super) fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        self.message = if self.show_help {
            "ヘルプを表示しています".to_string()
        } else {
            "ヘルプを閉じました".to_string()
        };
    }
}
