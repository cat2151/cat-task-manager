use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, NaiveDate};
use serde::Serialize;

use crate::{
    app::{DailyTask, TaskState},
    clock,
};

mod parser;

use parser::{invalid_task_line_message, parse_task_file_content, split_task_line, TaskLineKind};

const DEFAULT_TASKS: &str = "- [ ] Morning routine\n- [ ] Check mail\n- [ ] Code review\n";
const TASK_FILE_EXTENSION: &str = "md";
const DEFAULT_TASKS_FILE_NAME: &str = "tasks.md";

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub order: u32,
    pub source_line: u32,
}

#[derive(Debug, Clone)]
pub struct TaskFile {
    pub label: String,
    pub path: PathBuf,
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

pub(super) fn ensure_tasks_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|err| {
        format!(
            "tasks directory を作成できませんでした: {} ({err})",
            path.display()
        )
    })?;

    if tasks_dir_is_empty(path)? {
        let default_tasks_path = path.join(DEFAULT_TASKS_FILE_NAME);
        fs::write(&default_tasks_path, DEFAULT_TASKS).map_err(|err| {
            format!(
                "tasks fileを書き込めませんでした: {} ({err})",
                default_tasks_path.display()
            )
        })?;
    }

    Ok(())
}

fn tasks_dir_is_empty(path: &Path) -> Result<bool, String> {
    let mut entries = fs::read_dir(path).map_err(|err| {
        format!(
            "tasks directoryを読めませんでした: {} ({err})",
            path.display()
        )
    })?;
    entries
        .next()
        .transpose()
        .map(|entry| entry.is_none())
        .map_err(|err| {
            format!(
                "tasks directory entryを読めませんでした: {} ({err})",
                path.display()
            )
        })
}

pub fn load_task_files(dir: impl AsRef<Path>) -> Result<Vec<TaskFile>, String> {
    task_file_paths(dir.as_ref())?
        .into_iter()
        .map(load_task_file)
        .collect()
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
        label: task_file_label(path)?,
        path: path.to_path_buf(),
        task: tasks,
        status: parsed.status,
    })
}

fn task_file_paths(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(dir).map_err(|err| {
        format!(
            "tasks directoryを読めませんでした: {} ({err})",
            dir.display()
        )
    })?;
    let mut paths = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|err| {
            format!(
                "tasks directory entryを読めませんでした: {} ({err})",
                dir.display()
            )
        })?;
        let path = entry.path();
        if !is_task_file_path(&path) {
            continue;
        }
        paths.push(path);
    }

    paths.sort_by_key(|path| task_file_label(path).unwrap_or_default());
    Ok(paths)
}

pub fn is_task_file_path(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case(TASK_FILE_EXTENSION))
}

fn task_file_label(path: &Path) -> Result<String, String> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_string())
        .ok_or_else(|| format!("tasks file名をtab名にできません: {}", path.display()))
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
    if task_count != tasks.len() {
        return Err(format!(
            "task file のタスク数とアプリ内状態が一致しません: file={task_count}, app={}",
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
                    return Err("task file の行末JSONの前にタスクを書いてください".to_string());
                }
                output.push_str(line);
                output.push('\n');
                continue;
            }
            TaskLineKind::Invalid => return Err(invalid_task_line_message(line_number + 1)),
        }

        let task = tasks
            .get(task_index)
            .ok_or_else(|| "task file のタスク数がアプリ内状態より多いです".to_string())?;
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
                    return Err("task file の行末JSONの前にタスクを書いてください".to_string());
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
