use super::*;
use crate::app::TaskPause;

#[test]
fn all_task_lines_subtract_pauses_from_completed_work_duration() {
    let mut app = App::new(
        vec![task_list(
            "0730",
            vec![task("done", 1, 1), task("next", 2, 2)],
        )],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );
    app.tabs[0].tasks[0].state = TaskState::Done;
    app.tabs[0].tasks[0].started_at = Some(timestamp("2026-05-18T09:00:00+09:00"));
    app.tabs[0].tasks[0].completed_at = Some(timestamp("2026-05-18T10:05:00+09:00"));
    app.tabs[0].tasks[0].pauses = vec![TaskPause {
        paused_at: timestamp("2026-05-18T09:15:00+09:00"),
        resumed_at: Some(timestamp("2026-05-18T09:25:00+09:00")),
    }];

    let lines = all_task_lines(&app);

    assert_eq!(lines.len(), 2);
    assert!(line_text(&lines[0]).contains("完了"));
    assert!(line_text(&lines[0]).contains("作業時間 55分"));
    assert!(line_text(&lines[1]).contains("未着手"));
}

#[test]
fn one_line_task_line_does_not_show_completed_duration() {
    let task = DailyTask {
        name: "done".to_string(),
        order: 1,
        source_line: 1,
        state: TaskState::Done,
        started_at: Some(timestamp("2026-05-18T09:00:00+09:00")),
        completed_at: Some(timestamp("2026-05-18T09:05:00+09:00")),
        pauses: Vec::new(),
        free_time_seconds: None,
    };

    let line = task_line(&task, None, false);

    assert!(!line_text(&line).contains("作業時間"));
    assert!(line
        .spans
        .iter()
        .all(|span| !span.style.add_modifier.contains(Modifier::SLOW_BLINK)));
}
