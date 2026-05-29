use chrono::{DateTime, Duration, Local};
use serde::{Deserialize, Serialize};

use crate::storage::{TaskFile, TaskStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentTaskDuration {
    pub elapsed_seconds: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct RecentTaskDurationCandidate {
    completed_at: DateTime<Local>,
    report: RecentTaskDuration,
}

impl RecentTaskDurationCandidate {
    pub(crate) fn completed_after(&self, other: &Self) -> bool {
        self.completed_at > other.completed_at
    }

    pub(crate) fn into_report(self) -> RecentTaskDuration {
        self.report
    }
}

pub(crate) fn candidate_from_task_file(file: &TaskFile) -> Option<RecentTaskDurationCandidate> {
    let status = file.status.as_ref()?;
    let mut latest = None;

    for task_status in &status.states {
        let Some(candidate) = candidate_from_task_status(task_status) else {
            continue;
        };
        let replace = match &latest {
            Some(current) => candidate.completed_after(current),
            None => true,
        };
        if replace {
            latest = Some(candidate);
        }
    }

    latest
}

fn candidate_from_task_status(status: &TaskStatus) -> Option<RecentTaskDurationCandidate> {
    let started_at = status.started_at?;
    let completed_at = status.completed_at?;
    let duration = completed_at - started_at;
    (duration >= Duration::zero()).then(|| RecentTaskDurationCandidate {
        completed_at,
        report: RecentTaskDuration {
            elapsed_seconds: duration.num_seconds(),
        },
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::NaiveDate;

    use crate::storage;

    use super::*;

    fn task_file(raw: &str) -> TaskFile {
        storage::load_task_file_content(
            "tasks",
            PathBuf::from("tasks/tasks.md"),
            raw,
            NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn candidate_uses_latest_completed_task_elapsed_time() {
        let file = task_file(
            "- [x] a {\"date\":\"2026-05-18\",\"state\":\"done\",\"started_at\":\"2026-05-18T09:00:00+09:00\",\"completed_at\":\"2026-05-18T09:05:00+09:00\"}\n\
             - [x] b {\"date\":\"2026-05-18\",\"state\":\"done\",\"started_at\":\"2026-05-18T10:00:00+09:00\",\"completed_at\":\"2026-05-18T10:30:00+09:00\"}\n",
        );

        let report = candidate_from_task_file(&file).unwrap().into_report();

        assert_eq!(report.elapsed_seconds, 30 * 60);
    }

    #[test]
    fn candidate_ignores_incomplete_and_negative_elapsed_times() {
        let file = task_file(
            "- [ ] active {\"date\":\"2026-05-18\",\"state\":\"in_progress\",\"started_at\":\"2026-05-18T11:00:00+09:00\"}\n\
             - [x] invalid {\"date\":\"2026-05-18\",\"state\":\"done\",\"started_at\":\"2026-05-18T12:00:00+09:00\",\"completed_at\":\"2026-05-18T11:30:00+09:00\"}\n",
        );

        assert!(candidate_from_task_file(&file).is_none());
    }
}
