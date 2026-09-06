use super::*;

#[test]
fn checked_markdown_task_lines_are_done() {
    let parsed = parser::parse_task_file_content("- [ ] a\n- [x] b\n", status_date()).unwrap();
    let status = parsed.status.unwrap();

    assert_eq!(status.date, status_date());
    assert_eq!(status.states[0].state, TaskState::NotStarted);
    assert_eq!(status.states[1].state, TaskState::Done);
    assert!(status.states[1].started_at.is_some());
    assert!(status.states[1].completed_at.is_some());
}

#[test]
fn checked_task_without_line_end_json_is_persisted_with_completion_times() {
    let path = temp_tasks_path("checked-task-without-json");
    fs::write(&path, "- [x] a\n").unwrap();

    let loaded = load_task_file(&path).unwrap();
    let status = loaded.status.unwrap();
    let tasks = loaded
        .task
        .iter()
        .zip(&status.states)
        .map(|(task, status)| DailyTask {
            name: task.name.clone(),
            order: task.order,
            source_line: task.source_line,
            state: status.state.clone(),
            started_at: status.started_at,
            completed_at: status.completed_at,
            pauses: status.pauses.clone(),
            free_time_seconds: status.free_time_seconds,
        })
        .collect::<Vec<_>>();

    write_task_file_status(&path, status.date, &tasks).unwrap();

    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.starts_with("- [x] a {"));
    assert!(raw.contains("\"state\":\"done\""));
    assert!(raw.contains("\"started_at\":\""));
    assert!(raw.contains("\"completed_at\":\""));

    fs::remove_file(path).unwrap();
}

#[test]
fn free_time_line_reads_cumulative_seconds_without_timestamps() {
    let parsed = parser::parse_task_file_content(
        "- [x] free time {\"date\":\"2026-05-18\",\"state\":\"done\",\"free_time_seconds\":42}\n",
        status_date(),
    )
    .unwrap();
    let status = parsed.status.unwrap();

    assert_eq!(parsed.tasks[0].name, "free time");
    assert_eq!(status.states[0].state, TaskState::Done);
    assert_eq!(status.states[0].free_time_seconds, Some(42));
    assert!(status.states[0].started_at.is_none());
    assert!(status.states[0].completed_at.is_none());
}

#[test]
fn checked_free_time_without_line_end_json_starts_at_zero_seconds() {
    let parsed = parser::parse_task_file_content("- [x] free time\n", status_date()).unwrap();
    let status = parsed.status.unwrap();

    assert_eq!(status.states[0].state, TaskState::Done);
    assert_eq!(status.states[0].free_time_seconds, Some(0));
    assert!(status.states[0].started_at.is_none());
    assert!(status.states[0].completed_at.is_none());
}
