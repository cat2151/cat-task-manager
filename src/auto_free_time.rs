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

    pub fn tick(
        &mut self,
        app: &mut App,
        now: DateTime<FixedOffset>,
        allow_auto_start: bool,
    ) -> bool {
        if !self.config.is_active_at(now.time()) {
            return self.enforce_active_hours(app, now);
        }

        if !self.config.enabled
            || !allow_auto_start
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
        app.start_free_time_automatically(self.config.idle_seconds, now)
    }

    pub fn enforce_active_hours(&mut self, app: &mut App, now: DateTime<FixedOffset>) -> bool {
        if self.config.is_active_at(now.time()) {
            return false;
        }

        self.idle_since = None;
        self.stop_at_active_hours_end(app, now)
    }

    pub fn toggle_manually(&mut self, app: &mut App, now: DateTime<FixedOffset>) -> bool {
        self.idle_since = None;
        if !self.config.is_active_at(now.time()) {
            if app.free_time_active() {
                return self.stop_at_active_hours_end(app, now);
            }
            app.set_message("active_hours外ではfree timeを開始できません");
            return false;
        }

        app.toggle_free_time_at(now);
        true
    }

    fn stop_at_active_hours_end(&self, app: &mut App, now: DateTime<FixedOffset>) -> bool {
        app.stop_free_time_at_active_hours_end(self.config.active_end_at_or_before(now))
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

        assert!(!tracker.tick(&mut app, now("2026-06-23T09:00:00+09:00"), true));
        assert!(!tracker.tick(&mut app, now("2026-06-23T09:00:59+09:00"), true));
        assert!(tracker.tick(&mut app, now("2026-06-23T09:01:00+09:00"), true));
        assert!(app.free_time_active());
        assert_eq!(app.current_tab_label(), FREE_TIME_TAB_LABEL);
    }

    #[test]
    fn in_progress_task_resets_idle_timer() {
        let mut app = app();
        let mut tracker = tracker("09:00-17:00");

        tracker.tick(&mut app, now("2026-06-23T09:00:00+09:00"), true);
        app.tabs[0].tasks[0].state = TaskState::InProgress;
        tracker.tick(&mut app, now("2026-06-23T09:00:30+09:00"), true);
        app.tabs[0].tasks[0].state = TaskState::Done;

        assert!(!tracker.tick(&mut app, now("2026-06-23T09:01:00+09:00"), true));
        assert!(!tracker.tick(&mut app, now("2026-06-23T09:01:59+09:00"), true));
        assert!(tracker.tick(&mut app, now("2026-06-23T09:02:00+09:00"), true));
    }

    #[test]
    fn outside_active_hours_resets_idle_timer() {
        let mut app = app();
        let mut tracker = tracker("09:00-17:00");

        tracker.tick(&mut app, now("2026-06-23T16:59:30+09:00"), true);
        assert!(!tracker.tick(&mut app, now("2026-06-23T17:00:30+09:00"), true));
        assert!(!app.free_time_active());
    }

    #[test]
    fn leaving_active_hours_stops_at_end_boundary() {
        let mut app = app();
        let mut tracker = tracker("09:00-17:00");

        tracker.tick(&mut app, now("2026-06-23T16:58:00+09:00"), true);
        assert!(tracker.tick(&mut app, now("2026-06-23T16:59:00+09:00"), true));
        assert!(tracker.tick(&mut app, now("2026-06-23T18:30:00+09:00"), true));

        assert!(!app.free_time_active());
        assert_eq!(app.tabs[1].tasks[0].free_time_seconds, Some(60));
    }

    #[test]
    fn cross_midnight_active_hours_stop_at_end_boundary() {
        let mut app = app();
        let mut tracker = tracker("22:00-02:00");

        tracker.tick(&mut app, now("2026-06-23T23:58:00+09:00"), true);
        assert!(tracker.tick(&mut app, now("2026-06-23T23:59:00+09:00"), true));
        assert!(tracker.tick(&mut app, now("2026-06-24T03:00:00+09:00"), true));

        assert!(!app.free_time_active());
        assert_eq!(
            app.tabs[1].tasks[0].free_time_seconds,
            Some(2 * 60 * 60 + 60)
        );
    }

    #[test]
    fn background_work_prevents_start_but_not_boundary_stop() {
        let mut app = app();
        let mut tracker = tracker("09:00-17:00");

        tracker.tick(&mut app, now("2026-06-23T16:58:00+09:00"), true);
        assert!(tracker.tick(&mut app, now("2026-06-23T16:59:00+09:00"), true));
        assert!(tracker.tick(&mut app, now("2026-06-23T17:30:00+09:00"), false));

        assert!(!app.free_time_active());
        assert_eq!(app.tabs[1].tasks[0].free_time_seconds, Some(60));
    }

    #[test]
    fn manual_free_time_cannot_start_outside_active_hours() {
        let mut app = app();
        let mut tracker = tracker("09:00-17:00");

        assert!(!tracker.toggle_manually(&mut app, now("2026-06-23T08:00:00+09:00")));

        assert!(!app.free_time_active());
        assert!(app.message().contains("active_hours外"));
    }
}
