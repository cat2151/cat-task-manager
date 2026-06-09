use chrono::{DateTime, Local};

use super::{App, DailyTask, TaskLocation, TaskState, FREE_TIME_TAB_LABEL};

impl DailyTask {
    pub fn is_free_time(&self) -> bool {
        self.free_time_seconds.is_some()
    }
}

impl App {
    pub fn free_time_active(&self) -> bool {
        self.free_time_active
    }

    pub fn free_time_display_seconds(&self, task: &DailyTask) -> Option<u64> {
        let base = task.free_time_seconds?;
        let active_seconds = if self.free_time_active {
            self.free_time_active_elapsed_seconds()
        } else {
            0
        };

        Some(base.saturating_add(active_seconds))
    }

    pub fn sync_free_time_elapsed(&mut self) {
        if !self.free_time_active {
            return;
        }

        let now = Local::now();
        let elapsed_seconds = self
            .free_time_started_at
            .map(|started_at| elapsed_seconds_since(started_at, now))
            .unwrap_or(0);
        self.free_time_started_at = Some(now);

        if elapsed_seconds == 0 {
            return;
        }

        if let Some(task) = self.free_time_task_mut() {
            let base = task.free_time_seconds.unwrap_or(0);
            task.free_time_seconds = Some(base.saturating_add(elapsed_seconds));
        }
    }

    pub(super) fn toggle_free_time(&mut self) {
        if self.free_time_active {
            self.stop_free_time();
        } else {
            self.start_free_time();
        }
    }

    fn start_free_time(&mut self) {
        self.prepare_free_time_task();
        let held_count = self.hold_in_progress_tasks_except_free_time();
        self.free_time_active = true;
        self.free_time_started_at = Some(Local::now());

        if !self.select_free_time_tab() {
            self.message = "free_time tabがありません".to_string();
            return;
        }

        self.message = if held_count == 0 {
            "free timeを開始しました".to_string()
        } else {
            format!("free timeを開始しました。{held_count}件を保留しました")
        };
    }

    fn stop_free_time(&mut self) {
        self.sync_free_time_elapsed();
        self.free_time_active = false;
        self.free_time_started_at = None;
        self.prepare_free_time_task();

        if let Some(task_name) = self.resume_first_on_hold_task() {
            self.message = format!("free timeを終了して再開しました: {task_name}");
        } else {
            self.message = "free timeを終了しました。再開する保留taskはありません".to_string();
            self.clamp_selection();
        }
    }

    fn prepare_free_time_task(&mut self) {
        if let Some(task) = self.free_time_task_mut() {
            task.state = TaskState::Done;
            task.started_at = None;
            task.completed_at = None;
            task.free_time_seconds.get_or_insert(0);
        }
    }

    fn hold_in_progress_tasks_except_free_time(&mut self) -> usize {
        let mut held_count = 0;

        for tab in &mut self.tabs {
            if tab.label == FREE_TIME_TAB_LABEL {
                continue;
            }

            for task in &mut tab.tasks {
                if task.state != TaskState::InProgress {
                    continue;
                }
                task.state = TaskState::OnHold;
                held_count += 1;
            }
        }

        held_count
    }

    fn resume_first_on_hold_task(&mut self) -> Option<String> {
        let location = self.first_on_hold_task_location()?;
        let task_name = {
            let task = self.task_at_mut(location);
            task.state = TaskState::InProgress;
            task.name.clone()
        };
        self.select_task_location(location);

        Some(task_name)
    }

    fn first_on_hold_task_location(&self) -> Option<TaskLocation> {
        self.tabs
            .iter()
            .enumerate()
            .filter(|(_, tab)| tab.label != FREE_TIME_TAB_LABEL)
            .find_map(|(tab_index, tab)| {
                tab.tasks
                    .iter()
                    .position(|task| task.state == TaskState::OnHold)
                    .map(|task_index| TaskLocation {
                        tab_index,
                        task_index,
                    })
            })
    }

    fn select_task_location(&mut self, location: TaskLocation) {
        self.selected_tab = location.tab_index + 1;
        self.selected_visible = self
            .current_task_entries()
            .into_iter()
            .filter(|(_, entry_location, task)| self.task_is_visible(*entry_location, task))
            .position(|(_, entry_location, _)| entry_location == location)
            .unwrap_or(0);
        self.clamp_selection();
    }

    fn select_free_time_tab(&mut self) -> bool {
        let Some(tab_index) = self.free_time_tab_index() else {
            return false;
        };

        self.selected_tab = tab_index + 1;
        self.selected_visible = 0;
        self.clamp_selection();
        true
    }

    fn free_time_task_mut(&mut self) -> Option<&mut DailyTask> {
        let tab_index = self.free_time_tab_index()?;
        self.tabs
            .get_mut(tab_index)?
            .tasks
            .iter_mut()
            .find(|task| task.is_free_time())
    }

    fn free_time_tab_index(&self) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.label == FREE_TIME_TAB_LABEL)
    }

    fn free_time_active_elapsed_seconds(&self) -> u64 {
        self.free_time_started_at
            .map(|started_at| elapsed_seconds_since(started_at, Local::now()))
            .unwrap_or(0)
    }
}

fn elapsed_seconds_since(started_at: DateTime<Local>, now: DateTime<Local>) -> u64 {
    (now - started_at).num_seconds().try_into().unwrap_or(0)
}
