use chrono::{DateTime, FixedOffset};

use crate::{app::App, storage::AutoFreeTimeConfig};

#[derive(Debug)]
pub struct AutoFreeTimeTracker {
    config: AutoFreeTimeConfig,
    idle_since: Option<DateTime<FixedOffset>>,
}

impl AutoFreeTimeTracker {
    pub fn new(config: AutoFreeTimeConfig) -> Self {
        Self {
            config,
            idle_since: None,
        }
    }

    pub fn update_config(&mut self, config: AutoFreeTimeConfig) {
        self.config = config;
        self.idle_since = None;
    }

    pub fn tick(&mut self, app: &mut App, now: DateTime<FixedOffset>) -> bool {
        if !self.config.enabled
            || !self.config.is_active_at(now.time())
            || app.free_time_active()
            || app.has_in_progress_task()
        {
            self.idle_since = None;
            return false;
        }

        let Some(idle_since) = self.idle_since else {
            self.idle_since = Some(now);
            return false;
        };
        if (now - idle_since).num_seconds() < self.config.idle_seconds as i64 {
            return false;
        }

        self.idle_since = None;
        app.start_free_time_automatically(self.config.idle_seconds)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::DateTime;

    use super::*;
    use crate::{
        app::{TaskList, TaskState, FREE_TIME_TAB_LABEL, FREE_TIME_TASK_NAME},
        storage::Task,
    };

    fn now(raw: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(raw).unwrap()
    }

    fn tracker(active_hours: &str) -> AutoFreeTimeTracker {
        let config = toml::from_str(&format!(
            r#"
enabled = true
idle_seconds = 60
active_hours = "{active_hours}"
"#
        ))
        .unwrap();
        AutoFreeTimeTracker::new(config)
    }

    fn app() -> App {
        App::new(
            vec![
                task_list("0900", "work"),
                task_list(FREE_TIME_TAB_LABEL, FREE_TIME_TASK_NAME),
            ],
            now("2026-06-23T09:00:00+09:00").date_naive(),
        )
    }

    fn task_list(label: &str, name: &str) -> TaskList {
        TaskList {
            label: label.to_string(),
            path: PathBuf::from(format!("{label}.md")),
            tasks: vec![Task {
                name: name.to_string(),
                order: 1,
                source_line: 1,
            }],
        }
    }

    #[test]
    fn starts_free_time_after_sixty_idle_seconds() {
        let mut app = app();
        let mut tracker = tracker("09:00-17:00");

        assert!(!tracker.tick(&mut app, now("2026-06-23T09:00:00+09:00")));
        assert!(!tracker.tick(&mut app, now("2026-06-23T09:00:59+09:00")));
        assert!(tracker.tick(&mut app, now("2026-06-23T09:01:00+09:00")));
        assert!(app.free_time_active());
        assert_eq!(app.current_tab_label(), FREE_TIME_TAB_LABEL);
    }

    #[test]
    fn in_progress_task_resets_idle_timer() {
        let mut app = app();
        let mut tracker = tracker("09:00-17:00");

        tracker.tick(&mut app, now("2026-06-23T09:00:00+09:00"));
        app.tabs[0].tasks[0].state = TaskState::InProgress;
        tracker.tick(&mut app, now("2026-06-23T09:00:30+09:00"));
        app.tabs[0].tasks[0].state = TaskState::Done;

        assert!(!tracker.tick(&mut app, now("2026-06-23T09:01:00+09:00")));
        assert!(!tracker.tick(&mut app, now("2026-06-23T09:01:59+09:00")));
        assert!(tracker.tick(&mut app, now("2026-06-23T09:02:00+09:00")));
    }

    #[test]
    fn outside_active_hours_resets_idle_timer() {
        let mut app = app();
        let mut tracker = tracker("09:00-17:00");

        tracker.tick(&mut app, now("2026-06-23T16:59:30+09:00"));
        assert!(!tracker.tick(&mut app, now("2026-06-23T17:00:30+09:00")));
        assert!(!app.free_time_active());
    }
}
