use super::*;
use crate::app::TaskPause;

fn time(raw: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(raw)
        .unwrap()
        .with_timezone(&Local)
}

#[test]
fn load_task_file_reads_closed_and_open_pauses() {
    let raw = concat!(
        "- [x] done {\"date\":\"2026-05-18\",\"state\":\"done\",",
        "\"started_at\":\"2026-05-18T09:00:00+09:00\",",
        "\"completed_at\":\"2026-05-18T10:00:00+09:00\",",
        "\"pauses\":[{\"paused_at\":\"2026-05-18T09:15:00+09:00\",",
        "\"resumed_at\":\"2026-05-18T09:25:00+09:00\"}]}\n",
        "- [ ] held {\"date\":\"2026-05-18\",\"state\":\"on_hold\",",
        "\"started_at\":\"2026-05-18T11:00:00+09:00\",",
        "\"pauses\":[{\"paused_at\":\"2026-05-18T11:30:00+09:00\"}]}\n",
    );

    let loaded = load_task_file_content("tasks", "tasks.md", raw, status_date()).unwrap();
    let states = loaded.status.unwrap().states;

    assert_eq!(states[0].pauses.len(), 1);
    assert_eq!(
        states[0].pauses[0].resumed_at,
        Some(time("2026-05-18T09:25:00+09:00"))
    );
    assert_eq!(states[1].state, TaskState::OnHold);
    assert_eq!(states[1].pauses.len(), 1);
    assert!(states[1].pauses[0].resumed_at.is_none());
}

#[test]
fn write_task_file_status_round_trips_pauses() {
    let path = temp_tasks_path("write-pauses");
    fs::write(&path, "- [ ] a\n").unwrap();
    let task = DailyTask {
        name: "a".to_string(),
        order: 1,
        source_line: 1,
        state: TaskState::Done,
        started_at: Some(time("2026-05-18T09:00:00+09:00")),
        completed_at: Some(time("2026-05-18T10:00:00+09:00")),
        pauses: vec![TaskPause {
            paused_at: time("2026-05-18T09:15:00+09:00"),
            resumed_at: Some(time("2026-05-18T09:25:00+09:00")),
        }],
        free_time_seconds: None,
    };

    write_task_file_status(&path, status_date(), &[task]).unwrap();
    let loaded = load_task_file(&path).unwrap();

    assert_eq!(loaded.status.unwrap().states[0].pauses.len(), 1);
    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("\"pauses\":[{\"paused_at\":\"2026-05-18T09:15:00+09:00\",\"resumed_at\":\"2026-05-18T09:25:00+09:00\"}]"));
    fs::remove_file(path).unwrap();
}

#[test]
fn on_hold_status_without_open_pause_is_rejected() {
    let raw = concat!(
        "- [ ] a {\"date\":\"2026-05-18\",\"state\":\"on_hold\",",
        "\"started_at\":\"2026-05-18T09:00:00+09:00\"}\n",
    );

    let err = load_task_file_content("tasks", "tasks.md", raw, status_date()).unwrap_err();

    assert!(err.contains("未終了のpauseが必要"));
}

#[test]
fn overlapping_pauses_are_rejected() {
    let raw = concat!(
        "- [x] a {\"date\":\"2026-05-18\",\"state\":\"done\",",
        "\"started_at\":\"2026-05-18T09:00:00+09:00\",",
        "\"completed_at\":\"2026-05-18T10:00:00+09:00\",",
        "\"pauses\":[",
        "{\"paused_at\":\"2026-05-18T09:15:00+09:00\",\"resumed_at\":\"2026-05-18T09:30:00+09:00\"},",
        "{\"paused_at\":\"2026-05-18T09:20:00+09:00\",\"resumed_at\":\"2026-05-18T09:25:00+09:00\"}]}\n",
    );

    let err = load_task_file_content("tasks", "tasks.md", raw, status_date()).unwrap_err();

    assert!(err.contains("時刻順で重ならない"));
}
