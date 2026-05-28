use chrono::{DateTime, Local, NaiveDate};
use serde::Deserialize;

use crate::app::TaskState;

use super::{Task, TaskFileStatus, TaskStatus};

const NOT_STARTED_PREFIX: &str = "- [ ] ";
const DONE_PREFIX: &str = "- [x] ";

#[derive(Debug)]
pub(super) struct ParsedTaskFile {
    pub tasks: Vec<Task>,
    pub status: Option<TaskFileStatus>,
}

#[derive(Debug)]
pub(super) struct TaskLineParts<'a> {
    pub task_text: &'a str,
    pub kind: TaskLineKind<'a>,
    pub status: Option<LineStatus>,
}

#[derive(Debug)]
pub(super) enum TaskLineKind<'a> {
    Task(ParsedTaskLine<'a>),
    Ignored,
    Invalid,
}

#[derive(Debug)]
pub(super) struct ParsedTaskLine<'a> {
    name: &'a str,
    checkbox_status: TaskStatus,
}

#[derive(Debug)]
pub(super) struct LineStatus {
    date: NaiveDate,
    task_status: TaskStatus,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLineStatus {
    date: String,
    #[serde(default)]
    state: String,
    started_at: Option<String>,
    completed_at: Option<String>,
}

pub(super) fn parse_task_file_content(
    raw: &str,
    checkbox_status_date: NaiveDate,
) -> Result<ParsedTaskFile, String> {
    let mut tasks = Vec::new();
    let mut line_statuses = Vec::new();
    let mut checkbox_statuses = Vec::new();
    let detected_at = Local::now();

    for (index, line) in raw.lines().enumerate() {
        let line_parts = split_task_line_with_detected_at(line, detected_at)?;
        match line_parts.kind {
            TaskLineKind::Task(task_line) => {
                let name = task_line.name.trim();
                if name.is_empty() {
                    return Err(
                        "task file のチェックボックスの後にタスク名を書いてください".to_string()
                    );
                }

                tasks.push(Task {
                    name: name.to_string(),
                    order: 0,
                    source_line: (index + 1) as u32,
                });
                line_statuses.push(line_parts.status);
                checkbox_statuses.push(task_line.checkbox_status);
            }
            TaskLineKind::Ignored => {
                if line_parts.status.is_some() {
                    return Err("task file の行末JSONの前にタスクを書いてください".to_string());
                }
            }
            TaskLineKind::Invalid => return Err(invalid_task_line_message(index + 1)),
        }
    }

    Ok(ParsedTaskFile {
        tasks,
        status: build_task_file_status(line_statuses, checkbox_statuses, checkbox_status_date)?,
    })
}

pub(super) fn split_task_line(line: &str) -> Result<TaskLineParts<'_>, String> {
    split_task_line_with_detected_at(line, Local::now())
}

fn split_task_line_with_detected_at(
    line: &str,
    detected_at: DateTime<Local>,
) -> Result<TaskLineParts<'_>, String> {
    let trimmed = line.trim_end();
    if is_ignored_task_text(trimmed) {
        return Ok(TaskLineParts {
            task_text: line,
            kind: TaskLineKind::Ignored,
            status: None,
        });
    }

    if trimmed.is_empty() || !trimmed.ends_with('}') {
        return Ok(TaskLineParts {
            task_text: line,
            kind: parse_task_line_kind(line, detected_at),
            status: None,
        });
    }

    let mut saw_json_candidate = false;
    for (index, _) in trimmed.match_indices('{').rev() {
        let has_separator = index == 0
            || trimmed[..index]
                .chars()
                .last()
                .map(char::is_whitespace)
                .unwrap_or(false);
        if !has_separator {
            continue;
        }

        saw_json_candidate = true;
        let json = &trimmed[index..];
        if let Ok(raw_status) = serde_json::from_str::<RawLineStatus>(json) {
            let task_text = &line[..index];
            return Ok(TaskLineParts {
                task_text,
                kind: parse_task_line_kind(task_text, detected_at),
                status: Some(parse_line_status(raw_status)?),
            });
        }
    }

    if saw_json_candidate {
        return Err("task file の行末JSONが不正です。修正するか削除してください".to_string());
    }

    Ok(TaskLineParts {
        task_text: line,
        kind: parse_task_line_kind(line, detected_at),
        status: None,
    })
}

pub(super) fn invalid_task_line_message(line_number: usize) -> String {
    format!("task file のタスク行は '- [ ] ' または '- [x] ' で始めてください: line {line_number}")
}

fn parse_task_line_kind(line: &str, detected_at: DateTime<Local>) -> TaskLineKind<'_> {
    let task_text = line.trim_end();
    if task_text.trim().is_empty()
        || is_ignored_task_text(task_text)
        || task_text.trim_start().starts_with('#')
    {
        return TaskLineKind::Ignored;
    }

    if let Some(name) = task_text.strip_prefix(NOT_STARTED_PREFIX) {
        return TaskLineKind::Task(ParsedTaskLine {
            name,
            checkbox_status: TaskStatus {
                state: TaskState::NotStarted,
                started_at: None,
                completed_at: None,
            },
        });
    }

    if let Some(name) = task_text.strip_prefix(DONE_PREFIX) {
        return TaskLineKind::Task(ParsedTaskLine {
            name,
            checkbox_status: TaskStatus {
                state: TaskState::Done,
                started_at: Some(detected_at),
                completed_at: Some(detected_at),
            },
        });
    }

    TaskLineKind::Invalid
}

fn is_ignored_task_text(task_text: &str) -> bool {
    task_text.starts_with(' ') || is_supplemental_list_item(task_text)
}

fn is_supplemental_list_item(task_text: &str) -> bool {
    task_text.starts_with("- ")
        && !task_text.starts_with("- [ ]")
        && !task_text.starts_with("- [x]")
}

fn parse_line_status(raw_status: RawLineStatus) -> Result<LineStatus, String> {
    let date = NaiveDate::parse_from_str(&raw_status.date, "%Y-%m-%d").map_err(|err| {
        format!(
            "task file 行末JSONの日付を読めませんでした: '{}' ({err})",
            raw_status.date
        )
    })?;

    let task_state =
        TaskState::from_status_value(&raw_status.state).unwrap_or(TaskState::NotStarted);
    let completed_at = if task_state == TaskState::Done {
        Some(
            parse_optional_time("completed_at", raw_status.completed_at)?
                .unwrap_or_else(Local::now),
        )
    } else {
        None
    };
    let started_at = parse_optional_time("started_at", raw_status.started_at)?.or(completed_at);

    Ok(LineStatus {
        date,
        task_status: TaskStatus {
            state: task_state,
            started_at,
            completed_at,
        },
    })
}

fn parse_optional_time(
    field_name: &str,
    value: Option<String>,
) -> Result<Option<DateTime<Local>>, String> {
    value
        .as_deref()
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|value| value.with_timezone(&Local))
                .map_err(|err| {
                    format!("task file 行末JSONの{field_name}を読めませんでした: '{value}' ({err})")
                })
        })
        .transpose()
}

fn build_task_file_status(
    line_statuses: Vec<Option<LineStatus>>,
    checkbox_statuses: Vec<TaskStatus>,
    checkbox_status_date: NaiveDate,
) -> Result<Option<TaskFileStatus>, String> {
    let date = line_statuses
        .iter()
        .filter_map(|status| status.as_ref().map(|status| status.date))
        .next();

    if let Some(date) = date {
        let mut states = Vec::with_capacity(line_statuses.len());
        for (line_status, checkbox_status) in line_statuses.into_iter().zip(checkbox_statuses) {
            match line_status {
                Some(line_status) => {
                    if line_status.date != date {
                        return Err("task file の行末JSONの日付が行ごとに異なります".to_string());
                    }
                    states.push(resolve_task_status(
                        Some(line_status.task_status),
                        checkbox_status,
                    ));
                }
                None => states.push(resolve_task_status(None, checkbox_status)),
            }
        }

        return Ok(Some(TaskFileStatus { date, states }));
    }

    if checkbox_statuses.iter().any(|status| {
        status.state != TaskState::NotStarted
            || status.started_at.is_some()
            || status.completed_at.is_some()
    }) {
        return Ok(Some(TaskFileStatus {
            date: checkbox_status_date,
            states: checkbox_statuses,
        }));
    }

    Ok(None)
}

fn resolve_task_status(line_status: Option<TaskStatus>, checkbox_status: TaskStatus) -> TaskStatus {
    let Some(line_status) = line_status else {
        return checkbox_status;
    };

    match (&checkbox_status.state, &line_status.state) {
        (TaskState::Done, TaskState::Done) => line_status,
        (TaskState::Done, _) => TaskStatus {
            state: TaskState::Done,
            started_at: line_status.started_at.or(checkbox_status.started_at),
            completed_at: line_status.completed_at.or(checkbox_status.completed_at),
        },
        (_, TaskState::Done) => TaskStatus {
            state: TaskState::NotStarted,
            started_at: None,
            completed_at: None,
        },
        _ => line_status,
    }
}
