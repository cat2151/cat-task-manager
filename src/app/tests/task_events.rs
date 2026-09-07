use super::*;

#[test]
fn starting_and_completing_queue_lifecycle_events() {
    let mut app = app();

    app.advance_selected();
    let started = app.drain_task_events().collect::<Vec<_>>();
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].kind, TaskEventKind::Started);
    assert_eq!(started[0].task_name, "a");
    assert_eq!(started[0].task_file, PathBuf::from("0730.md"));
    assert_eq!(started[0].source_line, 1);

    app.advance_selected();
    let completed = app.drain_task_events().collect::<Vec<_>>();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].kind, TaskEventKind::Completed);
    assert_eq!(completed[0].task_name, "a");
}

#[test]
fn resuming_an_already_started_task_does_not_queue_another_start() {
    let mut app = app();
    app.advance_selected();
    app.drain_task_events().for_each(drop);
    app.defer_selected();
    app.toggle_view_mode();

    app.advance_selected();

    assert!(app.drain_task_events().next().is_none());
}

#[test]
fn first_start_from_deferred_queues_a_start() {
    let mut app = app();
    app.tabs[0].tasks[0].state = TaskState::Deferred;
    app.toggle_view_mode();

    app.advance_selected();

    let events = app.drain_task_events().collect::<Vec<_>>();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, TaskEventKind::Started);
}
