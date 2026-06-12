use super::*;

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
fn estimate_blink_context_requires_not_started_selected_task() {
    let mut app = app();
    app.select_next_tab();

    assert!(app.estimate_blink_context());

    app.toggle_view_mode();
    assert!(!app.estimate_blink_context());
    app.toggle_view_mode();
    app.toggle_view_mode();
    assert!(app.estimate_blink_context());

    app.advance_selected();
    assert!(!app.estimate_blink_context());

    app.toggle_hold_selected();
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::OnHold);
    assert!(!app.estimate_blink_context());
}
