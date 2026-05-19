use std::{fs, path::Path};

use chrono::{DateTime, Local, NaiveDate};
use serde::Serialize;

use crate::{
    app::{DailyTask, TaskState},
    clock,
};

mod parser;

use parser::{invalid_task_line_message, parse_task_file_content, split_task_line, TaskLineKind};

const DEFAULT_TASKS: &str = "- [ ] Morning routine\n- [ ] Check mail\n- [ ] Code review\n";

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub order: u32,
    pub source_line: u32,
}

#[derive(Debug, Clone)]
pub struct TaskFile {
    pub task: Vec<Task>,
    pub status: Option<TaskFileStatus>,
}

#[derive(Debug, Clone)]
pub struct TaskFileStatus {
    pub date: NaiveDate,
    pub states: Vec<TaskStatus>,
}

#[derive(Debug, Clone)]
pub struct TaskStatus {
    pub state: TaskState,
    pub started_at: Option<DateTime<Local>>,
    pub completed_at: Option<DateTime<Local>>,
}

#[derive(Debug, Serialize)]
struct LineStatusRecord<'a> {
    date: &'a str,
    state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<&'a str>,
}

pub(super) fn ensure_tasks_file(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::write(path, DEFAULT_TASKS).map_err(|err| {
            format!(
                "tasks fileを書き込めませんでした: {} ({err})",
                path.display()
            )
        })?;
    }

    Ok(())
}

pub fn load_task_file(path: impl AsRef<Path>) -> Result<TaskFile, String> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("tasks fileを読めませんでした: {} ({err})", path.display()))?;
    let parsed = parse_task_file_content(&raw, clock::today_jst())?;
    let mut tasks = parsed.tasks;
    validate_tasks(&tasks)?;
    assign_task_orders(&mut tasks);

    Ok(TaskFile {
        task: tasks,
        status: parsed.status,
    })
}

pub fn write_task_file_status(
    path: impl AsRef<Path>,
    date: NaiveDate,
    tasks: &[DailyTask],
) -> Result<(), String> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("tasks fileを読めませんでした: {} ({err})", path.display()))?;
    let task_count = count_task_lines(&raw)?;
    if task_count == 0 {
        return Err("tasks.txt にタスクを書いてください".to_string());
    }
    if task_count != tasks.len() {
        return Err(format!(
            "tasks.txt のタスク数とアプリ内状態が一致しません: file={task_count}, app={}",
            tasks.len()
        ));
    }

    let output = render_task_file_with_line_status(&raw, &date.to_string(), tasks, path)?;
    if raw == output {
        return Ok(());
    }

    fs::write(path, output).map_err(|err| {
        format!(
            "tasks fileを書き込めませんでした: {} ({err})",
            path.display()
        )
    })
}

fn render_task_file_with_line_status(
    raw: &str,
    date: &str,
    tasks: &[DailyTask],
    path: &Path,
) -> Result<String, String> {
    let mut output = String::new();
    let mut task_index = 0;

    for (line_number, line) in raw.lines().enumerate() {
        let line_parts = split_task_line(line)?;
        match line_parts.kind {
            TaskLineKind::Task(_) => {}
            TaskLineKind::Ignored => {
                if line_parts.status.is_some() {
                    return Err("tasks.txt の行末JSONの前にタスクを書いてください".to_string());
                }
                output.push_str(line);
                output.push('\n');
                continue;
            }
            TaskLineKind::Invalid => return Err(invalid_task_line_message(line_number + 1)),
        }

        let task = tasks
            .get(task_index)
            .ok_or_else(|| "tasks.txt のタスク数がアプリ内状態より多いです".to_string())?;
        validate_completion_times(task)?;
        let started_at = task.started_at.as_ref().map(clock::format_rfc3339_jst);
        let completed_at = task.completed_at.as_ref().map(clock::format_rfc3339_jst);
        let status = LineStatusRecord {
            date,
            state: task.state.record_value(),
            started_at: started_at.as_deref(),
            completed_at: completed_at.as_deref(),
        };
        let json = serde_json::to_string(&status).map_err(|err| {
            format!(
                "tasks fileの行末JSONを作れませんでした: {} ({err})",
                path.display()
            )
        })?;

        output.push_str(&render_task_text_for_state(
            line_parts.task_text,
            &task.state,
        ));
        output.push(' ');
        output.push_str(&json);
        output.push('\n');
        task_index += 1;
    }

    Ok(output)
}

fn count_task_lines(raw: &str) -> Result<usize, String> {
    let mut count = 0;

    for (line_number, line) in raw.lines().enumerate() {
        let line_parts = split_task_line(line)?;
        match line_parts.kind {
            TaskLineKind::Task(_) => count += 1,
            TaskLineKind::Ignored => {
                if line_parts.status.is_some() {
                    return Err("tasks.txt の行末JSONの前にタスクを書いてください".to_string());
                }
            }
            TaskLineKind::Invalid => return Err(invalid_task_line_message(line_number + 1)),
        }
    }

    Ok(count)
}

fn render_task_text_for_state(task_text: &str, state: &TaskState) -> String {
    let task_text = task_text.trim_end();
    let name = task_text
        .strip_prefix("- [ ] ")
        .or_else(|| task_text.strip_prefix("- [x] "))
        .unwrap_or(task_text);
    let checkbox = if matches!(state, TaskState::Done) {
        "- [x] "
    } else {
        "- [ ] "
    };

    format!("{checkbox}{name}")
}

fn validate_completion_times(task: &DailyTask) -> Result<(), String> {
    if matches!(task.state, TaskState::Done)
        && (task.started_at.is_none() || task.completed_at.is_none())
    {
        return Err(format!(
            "完了タスクに着手時刻または完了時刻がありません: {}",
            task.name
        ));
    }

    Ok(())
}

fn validate_tasks(tasks: &[Task]) -> Result<(), String> {
    if tasks.is_empty() {
        return Err("tasks.txt にタスクを書いてください".to_string());
    }

    for task in tasks {
        if task.name.trim().is_empty() {
            return Err("タスク名を空にはできません".to_string());
        }
    }

    Ok(())
}

fn assign_task_orders(tasks: &mut [Task]) {
    for (index, task) in tasks.iter_mut().enumerate() {
        task.order = (index + 1) as u32;
    }
}

#[cfg(test)]
mod tests;
