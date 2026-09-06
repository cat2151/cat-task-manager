use super::*;

#[test]
fn free_time_key_holds_in_progress_tasks_and_selects_free_time_tab() {
    let mut app = App::new(
        vec![
            task_list("0730", vec![task("a", 1, 1)]),
            task_list(FREE_TIME_TAB_LABEL, vec![task(FREE_TIME_TASK_NAME, 1, 1)]),
        ],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );
    let keybindings = KeyBindings::from_config(KeyBindingsConfig::default()).unwrap();

    app.advance_selected();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()),
        &keybindings,
    );

    assert!(app.free_time_active());
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::OnHold);
    assert_eq!(app.tabs[0].tasks[0].pauses.len(), 1);
    assert!(app.tabs[0].tasks[0].pauses[0].resumed_at.is_none());
    assert_eq!(app.current_tab_label(), FREE_TIME_TAB_LABEL);
    assert_eq!(
        app.selected_visible_task().unwrap().1.name,
        FREE_TIME_TASK_NAME
    );
}

#[test]
fn free_time_key_resumes_first_on_hold_task_and_selects_its_tab() {
    let mut app = App::new(
        vec![
            task_list("0730", vec![task("a", 1, 1)]),
            task_list("0800", vec![task("b", 1, 1)]),
            task_list(FREE_TIME_TAB_LABEL, vec![task(FREE_TIME_TASK_NAME, 1, 1)]),
        ],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );
    let keybindings = KeyBindings::from_config(KeyBindingsConfig::default()).unwrap();
    app.tabs[0].tasks[0].state = TaskState::InProgress;
    app.tabs[0].tasks[0].started_at = Some(timestamp());
    app.tabs[1].tasks[0].state = TaskState::InProgress;
    app.tabs[1].tasks[0].started_at = Some(timestamp());

    app.handle_key(
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()),
        &keybindings,
    );
    app.handle_key(
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()),
        &keybindings,
    );

    assert!(!app.free_time_active());
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::InProgress);
    assert!(app.tabs[0].tasks[0].pauses[0].resumed_at.is_some());
    assert_eq!(app.tabs[1].tasks[0].state, TaskState::OnHold);
    assert!(app.tabs[1].tasks[0].pauses[0].resumed_at.is_none());
    assert_eq!(app.current_tab_label(), "0730");
}

#[test]
fn starting_a_task_auto_stops_free_time() {
    let mut app = App::new(
        vec![
            task_list("0730", vec![task("a", 1, 1)]),
            task_list(FREE_TIME_TAB_LABEL, vec![task(FREE_TIME_TASK_NAME, 1, 1)]),
        ],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );
    let keybindings = KeyBindings::from_config(KeyBindingsConfig::default()).unwrap();

    app.handle_key(
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()),
        &keybindings,
    );
    assert!(app.free_time_active());

    app.select_previous_tab();
    app.advance_selected();

    assert!(!app.free_time_active());
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::InProgress);
    assert!(app.message().contains("free timeを停止しました"));
}

#[test]
fn releasing_hold_auto_stops_free_time() {
    let mut app = App::new(
        vec![
            task_list("0730", vec![task("a", 1, 1)]),
            task_list(FREE_TIME_TAB_LABEL, vec![task(FREE_TIME_TASK_NAME, 1, 1)]),
        ],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );
    let keybindings = KeyBindings::from_config(KeyBindingsConfig::default()).unwrap();

    // task aを実施中にしてからfree timeを開始すると、aは保留になる。
    app.advance_selected();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()),
        &keybindings,
    );
    assert!(app.free_time_active());
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::OnHold);

    // 保留を解除すると矛盾するため、free timeは自動停止する。
    app.select_previous_tab();
    app.toggle_hold_selected();

    assert!(!app.free_time_active());
    assert_eq!(app.tabs[0].tasks[0].state, TaskState::InProgress);
    assert!(app.tabs[0].tasks[0].pauses[0].resumed_at.is_some());
    assert!(app.message().contains("free timeを停止しました"));
}
