use super::*;
use crate::storage::KeyBindingsConfig;
use crate::storage::Task;
use chrono::DateTime;
use crossterm::event::KeyModifiers;
use std::path::PathBuf;

fn task(name: &str, order: u32, source_line: u32) -> Task {
    Task {
        name: name.to_string(),
        order,
        source_line,
    }
}

fn app() -> App {
    App::new(
        vec![task_list("0730", vec![task("a", 1, 1), task("b", 2, 2)])],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    )
}

fn task_list(label: &str, tasks: Vec<Task>) -> TaskList {
    TaskList {
        label: label.to_string(),
        path: PathBuf::from(format!("{label}.txt")),
        tasks,
    }
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
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::InProgress);
    app.select_next();
    app.advance_selected();
    assert_eq!(app.tabs[0].tasks[1].state, TaskState::NotStarted);
    app.select_previous();
    app.advance_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::Done);
    assert!(app.tabs[0].tasks[0].completed_at.is_some());
    app.select_next();
    app.advance_selected();
    assert_eq!(app.tabs[0].tasks[1].state, TaskState::InProgress);
    assert!(app.tabs[0].tasks[1].started_at.is_some());
}

#[test]
fn hold_toggles_only_in_progress_tasks() {
    let mut app = app();
    app.toggle_hold_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::NotStarted);
    app.advance_selected();
    app.toggle_hold_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::OnHold);
    assert!(app.tabs[0].tasks[0].started_at.is_some());
    app.toggle_hold_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::InProgress);
}

#[test]
fn day_change_keeps_done_and_times_out_other_states_for_records() {
    assert_eq!(TaskState::Done.on_day_changed(), TaskState::Done);
    assert_eq!(TaskState::NotStarted.on_day_changed(), TaskState::TimeOut);
    assert_eq!(TaskState::OnHold.on_day_changed(), TaskState::TimeOut);
}

#[test]
fn replace_tabs_resets_state_to_tasks_file_content() {
    let mut app = app();
    app.advance_selected();

    app.replace_tabs(vec![task_list(
        "0730",
        vec![task("a", 1, 1), task("c", 2, 3)],
    )]);

    assert_eq!(app.tabs[0].tasks[0].state, TaskState::NotStarted);
    assert_eq!(app.tabs[0].tasks[1].state, TaskState::NotStarted);
}

#[test]
fn apply_statuses_matches_by_tasks_file_order() {
    let mut app = App::new(
        vec![task_list(
            "0730",
            vec![task("renamed", 1, 1), task("repeat", 2, 2)],
        )],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );

    app.apply_statuses(
        0,
        &[
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
        ],
    );

    assert_eq!(app.tabs[0].tasks[0].state, TaskState::Done);
    assert_eq!(app.tabs[0].tasks[1].state, TaskState::InProgress);
}

#[test]
fn extra_tasks_without_status_stay_not_started() {
    let mut app = App::new(
        vec![task_list(
            "0730",
            vec![task("repeat", 1, 1), task("repeat", 2, 2)],
        )],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );

    app.apply_statuses(
        0,
        &[TaskStatus {
            state: TaskState::Done,
            started_at: Some(timestamp()),
            completed_at: Some(timestamp()),
        }],
    );

    assert_eq!(app.tabs[0].tasks[0].state, TaskState::Done);
    assert_eq!(app.tabs[0].tasks[1].state, TaskState::NotStarted);
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
fn all_tab_is_leftmost_and_lists_tasks_in_tab_order() {
    let app = App::new(
        vec![
            task_list("0730", vec![task("a", 1, 1), task("b", 2, 2)]),
            task_list("0800", vec![task("c", 1, 1)]),
        ],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );

    let names: Vec<&str> = app
        .current_tasks()
        .into_iter()
        .map(|task| task.name.as_str())
        .collect();

    assert_eq!(app.current_tab_label(), "all");
    assert!(app.current_tab_is_all());
    assert_eq!(app.current_tab_path(), None);
    assert_eq!(app.display_tab_label(0), Some("all"));
    assert_eq!(app.display_tab_label(1), Some("0730"));
    assert_eq!(names, vec!["a", "b", "c"]);
}

#[test]
fn tab_keys_switch_current_tab_from_all_tab() {
    let mut app = App::new(
        vec![
            task_list("0730", vec![task("a", 1, 1)]),
            task_list("0800", vec![task("b", 1, 1)]),
        ],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );
    let keybindings = KeyBindings::from_config(KeyBindingsConfig::default()).unwrap();

    assert_eq!(app.current_tab_label(), "all");
    app.handle_key(
        KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty()),
        &keybindings,
    );
    assert_eq!(app.current_tab_label(), "0730");
    app.handle_key(
        KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty()),
        &keybindings,
    );
    assert_eq!(app.current_tab_label(), "0800");
    app.handle_key(
        KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty()),
        &keybindings,
    );
    assert_eq!(app.current_tab_label(), "0730");
}

#[test]
fn all_tab_advances_tasks_across_file_tabs() {
    let mut app = App::new(
        vec![
            task_list("0730", vec![task("a", 1, 1)]),
            task_list("0800", vec![task("b", 1, 1)]),
        ],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );

    app.advance_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::InProgress);

    app.select_next();
    app.advance_selected();
    assert_eq!(app.tabs[1].tasks[0].state, TaskState::NotStarted);
    assert_eq!(app.message(), "前のタスクが完了していません");

    app.select_previous();
    app.advance_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::Done);

    app.advance_selected();
    assert_eq!(app.tabs[1].tasks[0].state, TaskState::InProgress);
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
