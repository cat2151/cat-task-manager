use super::*;

fn time(raw: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(raw)
        .unwrap()
        .with_timezone(&Local)
}

#[test]
fn hold_toggles_only_in_progress_tasks() {
    let mut app = app();
    app.select_next_tab();
    app.toggle_hold_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::NotStarted);
    app.advance_selected();
    app.toggle_hold_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::OnHold);
    assert!(app.tabs[0].tasks[0].started_at.is_some());
    assert_eq!(app.tabs[0].tasks[0].pauses.len(), 1);
    assert!(app.tabs[0].tasks[0].pauses[0].resumed_at.is_none());
    app.toggle_hold_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::InProgress);
    assert!(app.tabs[0].tasks[0].pauses[0].resumed_at.is_some());
}

#[test]
fn on_hold_task_still_blocks_next_task() {
    let mut app = app();

    app.select_next_tab();
    app.advance_selected();
    app.toggle_hold_selected();
    app.select_next();
    app.advance_selected();

    assert_eq!(app.tabs[0].tasks[1].state, TaskState::NotStarted);
    assert_eq!(app.message(), "前のタスクが完了していません");
}

#[test]
fn deferred_task_does_not_block_next_task() {
    let mut app = app();

    app.defer_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::Deferred);
    assert!(app.tabs[0].tasks[0].started_at.is_none());
    assert!(app.tabs[0].tasks[0].pauses.is_empty());
    assert_eq!(app.selected_visible_task().unwrap().1.name, "b");

    app.advance_selected();

    assert_eq!(app.tabs[0].tasks[1].state, TaskState::InProgress);
}

#[test]
fn advance_resumes_deferred_task() {
    let mut app = app();

    app.advance_selected();
    app.defer_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::Deferred);
    assert_eq!(app.tabs[0].tasks[0].pauses.len(), 1);

    app.toggle_view_mode();
    app.advance_selected();

    assert_eq!(app.tabs[0].tasks[0].state, TaskState::InProgress);
    assert!(app.tabs[0].tasks[0].started_at.is_some());
    assert!(app.tabs[0].tasks[0].completed_at.is_none());
    assert!(app.tabs[0].tasks[0].pauses[0].resumed_at.is_some());
}

#[test]
fn hold_and_defer_append_separate_pauses() {
    let mut app = app();
    app.select_next_tab();
    app.tabs[0].tasks[0].state = TaskState::InProgress;
    app.tabs[0].tasks[0].started_at = Some(time("2026-05-18T09:00:00+09:00"));

    app.toggle_hold_selected_at(time("2026-05-18T09:10:00+09:00"));
    app.toggle_hold_selected_at(time("2026-05-18T09:20:00+09:00"));
    app.toggle_view_mode();
    app.defer_selected_at(time("2026-05-18T09:30:00+09:00"));
    app.defer_selected_at(time("2026-05-18T09:40:00+09:00"));

    let pauses = &app.tabs[0].tasks[0].pauses;
    assert_eq!(pauses.len(), 2);
    assert_eq!(
        pauses[0].resumed_at,
        Some(time("2026-05-18T09:20:00+09:00"))
    );
    assert_eq!(
        pauses[1].resumed_at,
        Some(time("2026-05-18T09:40:00+09:00"))
    );
}

#[test]
fn day_change_clears_pauses() {
    let mut app = app();
    app.tabs[0].tasks[0].state = TaskState::InProgress;
    app.tabs[0].tasks[0].started_at = Some(time("2026-05-18T09:00:00+09:00"));
    app.tabs[0].tasks[0].pause_at(TaskState::OnHold, time("2026-05-18T09:10:00+09:00"));

    app.complete_day(NaiveDate::from_ymd_opt(2026, 5, 19).unwrap());

    assert_eq!(app.tabs[0].tasks[0].state, TaskState::NotStarted);
    assert!(app.tabs[0].tasks[0].pauses.is_empty());
}
