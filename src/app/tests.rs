use super::*;
use crate::storage::KeyBindingsConfig;
use crossterm::event::KeyModifiers;

fn task(name: &str, order: u32, source_line: u32) -> Task {
    Task {
        name: name.to_string(),
        order,
        source_line,
    }
}

fn app() -> App {
    App::new(
        vec![task("a", 1, 1), task("b", 2, 2)],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    )
}

fn timestamp() -> DateTime<Local> {
    DateTime::parse_from_rfc3339("2026-05-18T09:12:00+09:00")
        .unwrap()
        .with_timezone(&Local)
}

#[test]
fn advances_tasks_in_order() {
    let mut app = app();
    app.advance_selected();
    assert_eq!(app.tasks[0].state, TaskState::InProgress);
    app.select_next();
    app.advance_selected();
    assert_eq!(app.tasks[1].state, TaskState::NotStarted);
    app.select_previous();
    app.advance_selected();
    assert_eq!(app.tasks[0].state, TaskState::Done);
    assert!(app.tasks[0].completed_at.is_some());
    app.select_next();
    app.advance_selected();
    assert_eq!(app.tasks[1].state, TaskState::InProgress);
    assert!(app.tasks[1].started_at.is_some());
}

#[test]
fn hold_toggles_only_in_progress_tasks() {
    let mut app = app();
    app.toggle_hold_selected();
    assert_eq!(app.tasks[0].state, TaskState::NotStarted);
    app.advance_selected();
    app.toggle_hold_selected();
    assert_eq!(app.tasks[0].state, TaskState::OnHold);
    assert!(app.tasks[0].started_at.is_some());
    app.toggle_hold_selected();
    assert_eq!(app.tasks[0].state, TaskState::InProgress);
}

#[test]
fn day_change_keeps_done_and_times_out_other_states_for_records() {
    assert_eq!(TaskState::Done.on_day_changed(), TaskState::Done);
    assert_eq!(TaskState::NotStarted.on_day_changed(), TaskState::TimeOut);
    assert_eq!(TaskState::OnHold.on_day_changed(), TaskState::TimeOut);
}

#[test]
fn replace_tasks_resets_state_to_tasks_file_content() {
    let mut app = app();
    app.advance_selected();

    app.replace_tasks(vec![task("a", 1, 1), task("c", 2, 3)]);

    assert_eq!(app.tasks[0].state, TaskState::NotStarted);
    assert_eq!(app.tasks[1].state, TaskState::NotStarted);
}

#[test]
fn apply_statuses_matches_by_tasks_file_order() {
    let mut app = App::new(
        vec![task("renamed", 1, 1), task("repeat", 2, 2)],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );

    app.apply_statuses(&[
        TaskStatus {
            state: TaskState::Done,
            started_at: Some(timestamp()),
            completed_at: Some(timestamp()),
        },
        TaskStatus {
            state: TaskState::InProgress,
            started_at: None,
            completed_at: None,
        },
    ]);

    assert_eq!(app.tasks[0].state, TaskState::Done);
    assert_eq!(app.tasks[1].state, TaskState::InProgress);
}

#[test]
fn extra_tasks_without_status_stay_not_started() {
    let mut app = App::new(
        vec![task("repeat", 1, 1), task("repeat", 2, 2)],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );

    app.apply_statuses(&[TaskStatus {
        state: TaskState::Done,
        started_at: Some(timestamp()),
        completed_at: Some(timestamp()),
    }]);

    assert_eq!(app.tasks[0].state, TaskState::Done);
    assert_eq!(app.tasks[1].state, TaskState::NotStarted);
}

#[test]
fn starts_in_one_line_mode_and_toggles_view_mode() {
    let mut app = app();
    assert_eq!(app.view_mode(), ViewMode::OneLine);
    app.toggle_view_mode();
    assert_eq!(app.view_mode(), ViewMode::Incomplete);
    app.toggle_view_mode();
    assert_eq!(app.view_mode(), ViewMode::All);
    app.toggle_view_mode();
    assert_eq!(app.view_mode(), ViewMode::OneLine);
}

#[test]
fn question_mark_toggles_help() {
    let mut app = app();
    let keybindings = KeyBindings::from_config(KeyBindingsConfig::default()).unwrap();

    assert!(!app.show_help());
    app.handle_key(
        KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT),
        &keybindings,
    );
    assert!(app.show_help());
    app.handle_key(
        KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT),
        &keybindings,
    );
    assert!(!app.show_help());
}
