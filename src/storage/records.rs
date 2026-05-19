use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::NaiveDate;
use serde::Serialize;

use crate::{app::DailyTask, clock};

#[derive(Debug, Serialize)]
struct DayRecord<'a> {
    date: String,
    tasks: Vec<RecordTask<'a>>,
}

#[derive(Debug, Serialize)]
struct RecordTask<'a> {
    line: u32,
    name: &'a str,
    order: u32,
    final_state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<String>,
}

pub fn write_day_record(
    records_dir: impl AsRef<Path>,
    date: NaiveDate,
    tasks: &[DailyTask],
) -> Result<PathBuf, String> {
    let records_dir = records_dir.as_ref();
    fs::create_dir_all(records_dir).map_err(|err| {
        format!(
            "records directory を作成できませんでした: {} ({err})",
            records_dir.display()
        )
    })?;

    let path = records_dir.join(format!("{date}.toml"));
    let record = DayRecord {
        date: date.to_string(),
        tasks: tasks
            .iter()
            .map(|task| RecordTask {
                line: task.source_line,
                name: &task.name,
                order: task.order,
                final_state: task.state.on_day_changed().record_value(),
                started_at: task.started_at.as_ref().map(clock::format_rfc3339_jst),
                completed_at: task.completed_at.as_ref().map(clock::format_rfc3339_jst),
            })
            .collect(),
    };

    let toml = toml::to_string_pretty(&record).map_err(|err| {
        format!(
            "record fileをTOMLに変換できませんでした: {} ({err})",
            path.display()
        )
    })?;
    fs::write(&path, toml).map_err(|err| {
        format!(
            "record fileを書き込めませんでした: {} ({err})",
            path.display()
        )
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use chrono::{DateTime, Local};

    use super::*;
    use crate::app::TaskState;

    #[test]
    fn write_day_record_saves_gmt_timestamps_as_jst() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let records_dir = std::env::temp_dir().join(format!(
            "cat-task-manager-records-{}-{suffix}",
            std::process::id()
        ));
        let date = NaiveDate::from_ymd_opt(2026, 5, 18).unwrap();
        let time = DateTime::parse_from_rfc3339("2026-05-18T00:12:00+00:00")
            .unwrap()
            .with_timezone(&Local);
        let task = DailyTask {
            name: "a".to_string(),
            order: 1,
            source_line: 1,
            state: TaskState::Done,
            started_at: Some(time),
            completed_at: Some(time),
        };

        let path = write_day_record(&records_dir, date, &[task]).unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("started_at = \"2026-05-18T09:12:00+09:00\""));
        assert!(raw.contains("completed_at = \"2026-05-18T09:12:00+09:00\""));
        assert!(!raw.contains("+00:00"));

        fs::remove_file(path).unwrap();
        fs::remove_dir(records_dir).unwrap();
    }
}
