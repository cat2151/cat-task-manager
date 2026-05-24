use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use super::*;

fn temp_tasks_path(test_name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cat-task-manager-{test_name}-{}-{suffix}.txt",
        std::process::id()
    ))
}

fn temp_tasks_dir(test_name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cat-task-manager-{test_name}-{}-{suffix}",
        std::process::id()
    ))
}

fn daily_task(name: &str, order: u32, source_line: u32, state: TaskState) -> DailyTask {
    let completed_time = matches!(state, TaskState::Done).then(timestamp);
    DailyTask {
        name: name.to_string(),
        order,
        source_line,
        state,
        started_at: completed_time,
        completed_at: completed_time,
    }
}

fn status_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 5, 18).unwrap()
}

fn timestamp() -> DateTime<Local> {
    DateTime::parse_from_rfc3339("2026-05-18T09:12:00+09:00")
        .unwrap()
        .with_timezone(&Local)
}

fn parse_tasks_block(tasks_block: &str) -> Result<Vec<Task>, String> {
    Ok(parser::parse_task_file_content(tasks_block, status_date())?.tasks)
}

#[test]
fn markdown_task_lines_are_parsed_and_comments_are_ignored() {
    let tasks = parse_tasks_block(
        "\n# heading\n- [ ] first task\n- [x] second task\n\n# another heading\n- [ ] third task\n",
    )
    .unwrap();

    assert_eq!(
        tasks
            .iter()
            .map(|task| task.name.as_str())
            .collect::<Vec<_>>(),
        vec!["first task", "second task", "third task"]
    );
    assert_eq!(
        tasks
            .iter()
            .map(|task| task.source_line)
            .collect::<Vec<_>>(),
        vec![3, 4, 7]
    );
}

#[test]
fn current_tasks_txt_note_patterns_are_ignored() {
    let parsed = parser::parse_task_file_content(
        "\
## Project notes
### Section A
- [ ] First sample task
    - Indented note
        - Nested note
- [ ] Second sample task
- Related note
",
        status_date(),
    )
    .unwrap();

    assert_eq!(
        parsed
            .tasks
            .iter()
            .map(|task| task.name.as_str())
            .collect::<Vec<_>>(),
        vec!["First sample task", "Second sample task"]
    );
    assert_eq!(
        parsed
            .tasks
            .iter()
            .map(|task| task.source_line)
            .collect::<Vec<_>>(),
        vec![3, 6]
    );
    assert!(parsed.status.is_none());
}

#[test]
fn leading_space_lines_are_ignored_even_with_line_end_status_json() {
    let parsed = parser::parse_task_file_content(
        "- [x] Active sample task {\"date\":\"2026-05-18\",\"state\":\"done\"}\n    - [ ] Nested sample note {\"date\":\"2026-05-18\",\"state\":\"done\"}\n- [ ] Next sample task\n",
        status_date(),
    )
    .unwrap();
    let status = parsed.status.unwrap();

    assert_eq!(
        parsed
            .tasks
            .iter()
            .map(|task| task.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Active sample task", "Next sample task"]
    );
    assert_eq!(status.states.len(), 2);
    assert_eq!(status.states[0].state, TaskState::Done);
    assert_eq!(status.states[1].state, TaskState::NotStarted);
}

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
fn non_markdown_task_lines_are_rejected() {
    let err = parser::parse_task_file_content("plain task\n", status_date())
        .unwrap_err()
        .to_string();

    assert!(err.contains("'- [ ] '"));
}

#[test]
fn duplicate_task_names_are_allowed() {
    let mut tasks = parse_tasks_block("- [ ] repeat\n- [ ] repeat\n").unwrap();

    validate_tasks(&tasks).unwrap();
    assign_task_orders(&mut tasks);

    assert_eq!(
        tasks
            .iter()
            .map(|task| (task.name.as_str(), task.source_line, task.order))
            .collect::<Vec<_>>(),
        vec![("repeat", 1, 1), ("repeat", 2, 2)]
    );
}

#[test]
fn load_task_file_reads_markdown_tasks_without_status_json() {
    let path = temp_tasks_path("markdown-tasks");
    fs::write(&path, "- [ ] a\n- [ ] b\n").unwrap();

    let loaded = load_task_file(&path).unwrap();

    assert_eq!(
        loaded
            .task
            .iter()
            .map(|task| task.name.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert!(loaded.status.is_none());

    fs::remove_file(path).unwrap();
}

#[test]
fn load_task_files_reads_txt_files_as_sorted_tabs() {
    let dir = temp_tasks_dir("task-tabs");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("tasks.txt"), "- [ ] later\n").unwrap();
    fs::write(dir.join("0730.txt"), "- [ ] morning\n").unwrap();
    fs::write(dir.join("notes.md"), "- [ ] ignored\n").unwrap();

    let loaded = load_task_files(&dir).unwrap();

    assert_eq!(
        loaded
            .iter()
            .map(|file| file.label.as_str())
            .collect::<Vec<_>>(),
        vec!["0730", "tasks"]
    );
    assert_eq!(loaded[0].task[0].name, "morning");
    assert_eq!(loaded[1].task[0].name, "later");

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn ensure_tasks_dir_creates_default_tasks_file() {
    let dir = temp_tasks_dir("ensure-task-tabs");

    ensure_tasks_dir(&dir).unwrap();

    let raw = fs::read_to_string(dir.join("tasks.txt")).unwrap();
    assert!(raw.contains("- [ ] Morning routine"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn load_task_file_reads_line_end_status_json_without_task_identity() {
    let path = temp_tasks_path("line-end-status-json");
    fs::write(
        &path,
        "- [x] a typo fixed {\"date\":\"2026-05-18\",\"state\":\"done\",\"started_at\":\"2026-05-18T09:00:00+09:00\",\"completed_at\":\"2026-05-18T09:05:00+09:00\"}\n- [ ] b {\"date\":\"2026-05-18\",\"state\":\"in_progress\",\"started_at\":\"2026-05-18T09:12:00+09:00\"}\n",
    )
    .unwrap();

    let loaded = load_task_file(&path).unwrap();
    let status = loaded.status.unwrap();

    assert_eq!(loaded.task[0].name, "a typo fixed");
    assert_eq!(status.date, NaiveDate::from_ymd_opt(2026, 5, 18).unwrap());
    assert_eq!(status.states[0].state, TaskState::Done);
    assert!(status.states[0].completed_at.is_some());
    assert_eq!(status.states[1].state, TaskState::InProgress);
    assert!(status.states[1].started_at.is_some());
    assert!(status.states[1].completed_at.is_none());

    fs::remove_file(path).unwrap();
}

#[test]
fn missing_line_end_status_defaults_to_not_started() {
    let path = temp_tasks_path("partial-line-end-status-json");
    fs::write(
        &path,
        "- [ ] a {\"date\":\"2026-05-18\",\"state\":\"in_progress\",\"started_at\":\"2026-05-18T09:12:00+09:00\"}\n- [ ] b\n",
    )
    .unwrap();

    let loaded = load_task_file(&path).unwrap();
    let status = loaded.status.unwrap();

    assert_eq!(status.states[0].state, TaskState::InProgress);
    assert_eq!(status.states[1].state, TaskState::NotStarted);

    fs::remove_file(path).unwrap();
}

#[test]
fn checked_checkbox_overrides_not_started_line_end_status_and_persists_completion_times() {
    let path = temp_tasks_path("checked-overrides-not-started-json");
    let date = clock::today_jst();
    fs::write(
        &path,
        format!("- [x] a {{\"date\":\"{date}\",\"state\":\"not_started\"}}\n"),
    )
    .unwrap();

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
fn checked_checkbox_completes_in_progress_line_end_status_with_existing_start_time() {
    let parsed = parser::parse_task_file_content(
        "- [x] a {\"date\":\"2026-05-18\",\"state\":\"in_progress\",\"started_at\":\"2026-05-18T09:12:00+09:00\"}\n",
        status_date(),
    )
    .unwrap();
    let status = parsed.status.unwrap();

    assert_eq!(status.states[0].state, TaskState::Done);
    assert_eq!(status.states[0].started_at, Some(timestamp()));
    assert!(status.states[0].completed_at.is_some());
}

#[test]
fn unchecked_checkbox_overrides_done_line_end_status_to_not_started() {
    let parsed = parser::parse_task_file_content(
        "- [ ] a {\"date\":\"2026-05-18\",\"state\":\"done\",\"started_at\":\"2026-05-18T09:00:00+09:00\",\"completed_at\":\"2026-05-18T09:05:00+09:00\"}\n",
        status_date(),
    )
    .unwrap();
    let status = parsed.status.unwrap();

    assert_eq!(status.states[0].state, TaskState::NotStarted);
    assert!(status.states[0].started_at.is_none());
    assert!(status.states[0].completed_at.is_none());
}

#[test]
fn invalid_line_end_json_is_rejected() {
    let err =
        parser::parse_task_file_content("- [ ] a {\"date\":\"2026-05-18\",}\n", status_date())
            .unwrap_err()
            .to_string();

    assert!(err.contains("行末JSONが不正"));
}

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
    }];

    write_task_file_status(&path, date, &tasks).unwrap();

    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("\"started_at\":\"2026-05-18T09:12:00+09:00\""));
    assert!(raw.contains("\"completed_at\":\"2026-05-18T09:12:00+09:00\""));
    assert!(!raw.contains("+00:00"));

    fs::remove_file(path).unwrap();
}
