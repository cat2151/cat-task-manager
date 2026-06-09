use super::*;

#[test]
fn write_task_file_status_replaces_line_end_json() {
    let path = temp_tasks_path("write-status");
    fs::write(
        &path,
        "- [ ] a {\"date\":\"2026-05-17\",\"state\":\"not_started\"}\n\n# comment\n- [x] b {\"date\":\"2026-05-17\",\"state\":\"done\"}\n",
    )
    .unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 5, 18).unwrap();
    let tasks = vec![
        daily_task("a", 1, 1, TaskState::Done),
        daily_task("b", 2, 2, TaskState::InProgress),
    ];

    write_task_file_status(&path, date, &tasks).unwrap();

    let raw = fs::read_to_string(&path).unwrap();
    assert_eq!(
        raw,
        "- [x] a {\"date\":\"2026-05-18\",\"state\":\"done\",\"started_at\":\"2026-05-18T09:12:00+09:00\",\"completed_at\":\"2026-05-18T09:12:00+09:00\"}\n\n# comment\n- [ ] b {\"date\":\"2026-05-18\",\"state\":\"in_progress\"}\n"
    );
    assert!(!raw.contains("\"states\""));
    assert!(!raw.contains("\"name\""));
    assert!(!raw.contains("\"line\""));
    assert!(!raw.contains("\"order\""));

    fs::remove_file(path).unwrap();
}

#[test]
fn write_task_file_status_saves_gmt_timestamps_as_jst() {
    let path = temp_tasks_path("write-status-jst");
    fs::write(&path, "- [ ] a\n").unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 5, 18).unwrap();
    let time = DateTime::parse_from_rfc3339("2026-05-18T00:12:00+00:00")
        .unwrap()
        .with_timezone(&Local);
    let tasks = vec![DailyTask {
        name: "a".to_string(),
        order: 1,
        source_line: 1,
        state: TaskState::Done,
        started_at: Some(time),
        completed_at: Some(time),
        free_time_seconds: None,
    }];

    write_task_file_status(&path, date, &tasks).unwrap();

    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("\"started_at\":\"2026-05-18T09:12:00+09:00\""));
    assert!(raw.contains("\"completed_at\":\"2026-05-18T09:12:00+09:00\""));
    assert!(!raw.contains("+00:00"));

    fs::remove_file(path).unwrap();
}

#[test]
fn write_task_file_status_saves_free_time_seconds_without_timestamps() {
    let path = temp_tasks_path("write-free-time-status");
    fs::write(&path, "- [x] free time\n").unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 5, 18).unwrap();
    let tasks = vec![DailyTask {
        name: "free time".to_string(),
        order: 1,
        source_line: 1,
        state: TaskState::Done,
        started_at: None,
        completed_at: None,
        free_time_seconds: Some(65),
    }];

    write_task_file_status(&path, date, &tasks).unwrap();

    let raw = fs::read_to_string(&path).unwrap();
    assert_eq!(
        raw,
        "- [x] free time {\"date\":\"2026-05-18\",\"state\":\"done\",\"free_time_seconds\":65}\n"
    );
    assert!(!raw.contains("started_at"));
    assert!(!raw.contains("completed_at"));

    fs::remove_file(path).unwrap();
}
