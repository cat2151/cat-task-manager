use chrono::{DateTime, Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Modifier;
use ratatui::text::Line;
use std::path::PathBuf;

use super::tasks::{all_task_lines, one_line_task_lines, task_line};
use crate::{
    app::{App, DailyTask, TaskState},
    event::KeyBindings,
    history_stats::{HistoryStatsReport, TaskNameCount, TypicalTaskDuration},
    storage::{KeyBindingsConfig, Task},
};

fn task(name: &str, order: u32, source_line: u32) -> Task {
    Task {
        name: name.to_string(),
        order,
        source_line,
    }
}

fn task_list(label: &str, tasks: Vec<Task>) -> crate::app::TaskList {
    crate::app::TaskList {
        label: label.to_string(),
        path: PathBuf::from(format!("{label}.md")),
        tasks,
    }
}

fn timestamp(value: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Local)
}

fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

#[test]
fn all_task_lines_include_completed_tasks_with_work_duration() {
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

    let lines = all_task_lines(&app);

    assert_eq!(lines.len(), 2);
    assert!(line_text(&lines[0]).contains("完了"));
    assert!(line_text(&lines[0]).contains("作業時間 1時間5分"));
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
    };

    let line = task_line(&task, None, false);

    assert!(!line_text(&line).contains("作業時間"));
}

#[test]
fn task_line_shows_deferred_state() {
    let task = DailyTask {
        name: "later".to_string(),
        order: 1,
        source_line: 1,
        state: TaskState::Deferred,
        started_at: None,
        completed_at: None,
    };

    let line = task_line(&task, None, false);

    assert!(line_text(&line).contains("後回し"));
}

#[test]
fn task_line_shows_estimated_duration_instead_of_order() {
    let task = DailyTask {
        name: "breakfast".to_string(),
        order: 12,
        source_line: 1,
        state: TaskState::NotStarted,
        started_at: None,
        completed_at: None,
    };

    let line = task_line(&task, Some(30 * 60), false);
    let text = line_text(&line);
    let estimate_span = line
        .spans
        .iter()
        .find(|span| span.content.contains("30分"))
        .unwrap();

    assert!(text.starts_with("見込み"));
    assert!(text.contains("30分"));
    assert!(!text.contains("12."));
    assert!(estimate_span
        .style
        .add_modifier
        .contains(Modifier::SLOW_BLINK));
}

#[test]
fn task_lines_use_typical_duration_from_ready_stats() {
    let mut app = App::new(
        vec![task_list("0730", vec![task("breakfast", 1, 1)])],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );
    app.finish_history_stats(Ok(HistoryStatsReport {
        scanned_revisions: 1,
        skipped_files: 0,
        timed_out: false,
        task_counts: vec![TaskNameCount {
            name: "breakfast".to_string(),
            count: 3,
            typical_task_duration: Some(TypicalTaskDuration {
                elapsed_seconds: 60 * 60 + 30 * 60 + 10,
            }),
        }],
    }));

    let lines = all_task_lines(&app);

    assert!(line_text(&lines[0]).contains("1時間30分10秒"));
}

#[test]
fn individual_tab_one_line_on_hold_task_includes_note() {
    let mut app = App::new(
        vec![task_list("0730", vec![task("waiting", 1, 1)])],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );
    let keybindings = KeyBindings::from_config(KeyBindingsConfig::default()).unwrap();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty()),
        &keybindings,
    );
    app.tabs[0].tasks[0].state = TaskState::OnHold;

    let lines = one_line_task_lines(&app).unwrap();

    assert_eq!(lines.len(), 2);
    assert!(line_text(&lines[0]).contains("waiting"));
    assert!(line_text(&lines[0]).contains("保留"));
    assert!(line_text(&lines[1]).contains("他タブのタスク"));
}

#[test]
fn all_tab_one_line_skips_on_hold_task_without_note() {
    let mut app = App::new(
        vec![
            task_list(
                "0730",
                vec![task("waiting", 1, 1), task("same tab later", 2, 2)],
            ),
            task_list("0800", vec![task("next", 1, 1)]),
        ],
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
    );
    app.tabs[0].tasks[0].state = TaskState::OnHold;

    let lines = one_line_task_lines(&app).unwrap();

    assert_eq!(lines.len(), 1);
    assert!(line_text(&lines[0]).contains("next"));
}
