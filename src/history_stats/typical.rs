use std::collections::HashMap;

use chrono::{DateTime, Duration, Local};
use serde::{Deserialize, Serialize};

use crate::storage::{TaskFile, TaskStatus};

const MIN_OUTLIER_SAMPLE_COUNT: usize = 4;
const HISTOGRAM_BIN_WIDTH_SECONDS: i64 = 5 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypicalTaskDuration {
    pub elapsed_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TaskDurationCandidate {
    task_name: String,
    started_at: DateTime<Local>,
    completed_at: DateTime<Local>,
}

#[derive(Debug, Default)]
pub(crate) struct TypicalTaskDurations {
    overall: Option<TypicalTaskDuration>,
    by_task: HashMap<String, TypicalTaskDuration>,
}

impl TypicalTaskDurations {
    pub(crate) fn overall(&self) -> Option<TypicalTaskDuration> {
        self.overall.clone()
    }

    pub(crate) fn for_task(&self, task_name: &str) -> Option<TypicalTaskDuration> {
        self.by_task.get(task_name).cloned()
    }
}

impl TaskDurationCandidate {
    fn task_name(&self) -> &str {
        &self.task_name
    }

    fn elapsed_seconds(&self) -> i64 {
        (self.completed_at - self.started_at).num_seconds()
    }
}

pub(crate) fn candidates_from_task_file(file: &TaskFile) -> Vec<TaskDurationCandidate> {
    let Some(status) = &file.status else {
        return Vec::new();
    };

    file.task
        .iter()
        .zip(&status.states)
        .filter_map(|(task, status)| candidate_from_task_status(&task.name, status))
        .collect()
}

pub(crate) fn summarize<'a>(
    candidates: impl IntoIterator<Item = &'a TaskDurationCandidate>,
) -> TypicalTaskDurations {
    let mut candidates_by_task = HashMap::<&str, Vec<&TaskDurationCandidate>>::new();
    for candidate in candidates {
        candidates_by_task
            .entry(candidate.task_name())
            .or_default()
            .push(candidate);
    }

    let mut accepted_candidates = Vec::new();
    let mut by_task = HashMap::new();
    for (task_name, task_candidates) in candidates_by_task {
        let accepted = remove_outliers(&task_candidates);
        if accepted.is_empty() {
            continue;
        }
        by_task.insert(task_name.to_string(), representative_duration(&accepted));
        accepted_candidates.extend(accepted);
    }

    TypicalTaskDurations {
        overall: (!accepted_candidates.is_empty())
            .then(|| representative_duration(&accepted_candidates)),
        by_task,
    }
}

fn candidate_from_task_status(
    task_name: &str,
    status: &TaskStatus,
) -> Option<TaskDurationCandidate> {
    let started_at = status.started_at?;
    let completed_at = status.completed_at?;
    let duration = completed_at - started_at;
    (duration >= Duration::zero()).then(|| TaskDurationCandidate {
        task_name: task_name.to_string(),
        started_at,
        completed_at,
    })
}

fn remove_outliers<'a>(candidates: &[&'a TaskDurationCandidate]) -> Vec<&'a TaskDurationCandidate> {
    let elapsed_seconds = candidates
        .iter()
        .map(|candidate| candidate.elapsed_seconds())
        .collect::<Vec<_>>();
    let accepted_range = accepted_duration_range(&elapsed_seconds);

    candidates
        .iter()
        .copied()
        .filter(|candidate| {
            accepted_range.is_none_or(|(minimum, maximum)| {
                let elapsed_seconds = candidate.elapsed_seconds() as f64;
                minimum <= elapsed_seconds && elapsed_seconds <= maximum
            })
        })
        .collect()
}

fn representative_duration(candidates: &[&TaskDurationCandidate]) -> TypicalTaskDuration {
    let mut elapsed_seconds = candidates
        .iter()
        .map(|candidate| candidate.elapsed_seconds())
        .collect::<Vec<_>>();
    elapsed_seconds.sort_unstable();
    let fallback = median(&elapsed_seconds);
    let mut bins = HashMap::<i64, Vec<i64>>::new();
    for elapsed_seconds in elapsed_seconds {
        bins.entry(elapsed_seconds / HISTOGRAM_BIN_WIDTH_SECONDS)
            .or_default()
            .push(elapsed_seconds);
    }
    let peak_size = bins.values().map(Vec::len).max().unwrap_or_default();
    let mut peaks = bins
        .values()
        .filter(|durations| durations.len() == peak_size);
    let representative = peaks.next().map(|peak| mean(peak)).unwrap_or(fallback);

    TypicalTaskDuration {
        elapsed_seconds: if peaks.next().is_none() {
            representative
        } else {
            fallback
        },
    }
}

fn accepted_duration_range(elapsed_seconds: &[i64]) -> Option<(f64, f64)> {
    if elapsed_seconds.len() < MIN_OUTLIER_SAMPLE_COUNT {
        return None;
    }

    let mut sorted = elapsed_seconds.to_vec();
    sorted.sort_unstable();
    let first_quartile = quantile(&sorted, 0.25);
    let third_quartile = quantile(&sorted, 0.75);
    let interquartile_range = third_quartile - first_quartile;

    Some((
        first_quartile - 1.5 * interquartile_range,
        third_quartile + 1.5 * interquartile_range,
    ))
}

fn quantile(sorted: &[i64], percentile: f64) -> f64 {
    let position = (sorted.len() - 1) as f64 * percentile;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    let fraction = position - lower_index as f64;

    sorted[lower_index] as f64 * (1.0 - fraction) + sorted[upper_index] as f64 * fraction
}

fn median(sorted: &[i64]) -> i64 {
    let middle = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]) / 2
    } else {
        sorted[middle]
    }
}

fn mean(values: &[i64]) -> i64 {
    (values.iter().map(|value| i128::from(*value)).sum::<i128>() / values.len() as i128) as i64
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

    fn candidate(task_name: &str, started_at: &str, completed_at: &str) -> TaskDurationCandidate {
        TaskDurationCandidate {
            task_name: task_name.to_string(),
            started_at: DateTime::parse_from_rfc3339(started_at)
                .unwrap()
                .with_timezone(&Local),
            completed_at: DateTime::parse_from_rfc3339(completed_at)
                .unwrap()
                .with_timezone(&Local),
        }
    }

    #[test]
    fn candidates_include_all_completed_task_elapsed_times() {
        let file = task_file(
            "- [x] a {\"date\":\"2026-05-18\",\"state\":\"done\",\"started_at\":\"2026-05-18T09:00:00+09:00\",\"completed_at\":\"2026-05-18T09:05:00+09:00\"}\n\
             - [x] b {\"date\":\"2026-05-18\",\"state\":\"done\",\"started_at\":\"2026-05-18T10:00:00+09:00\",\"completed_at\":\"2026-05-18T10:30:00+09:00\"}\n",
        );

        let candidates = candidates_from_task_file(&file);
        let summary = summarize(&candidates);

        assert_eq!(summary.overall().unwrap().elapsed_seconds, 17 * 60 + 30);
        assert_eq!(summary.for_task("a").unwrap().elapsed_seconds, 5 * 60);
        assert_eq!(summary.for_task("b").unwrap().elapsed_seconds, 30 * 60);
    }

    #[test]
    fn candidates_ignore_incomplete_and_negative_elapsed_times() {
        let file = task_file(
            "- [ ] active {\"date\":\"2026-05-18\",\"state\":\"in_progress\",\"started_at\":\"2026-05-18T11:00:00+09:00\"}\n\
             - [x] invalid {\"date\":\"2026-05-18\",\"state\":\"done\",\"started_at\":\"2026-05-18T12:00:00+09:00\",\"completed_at\":\"2026-05-18T11:30:00+09:00\"}\n",
        );

        assert!(candidates_from_task_file(&file).is_empty());
    }

    #[test]
    fn summary_uses_histogram_peak_after_removing_outliers_for_each_task() {
        let candidates = vec![
            candidate(
                "a",
                "2026-05-14T09:00:00+09:00",
                "2026-05-14T09:10:00+09:00",
            ),
            candidate(
                "a",
                "2026-05-15T09:00:00+09:00",
                "2026-05-15T09:11:00+09:00",
            ),
            candidate(
                "a",
                "2026-05-16T09:00:00+09:00",
                "2026-05-16T09:12:00+09:00",
            ),
            candidate(
                "a",
                "2026-05-17T09:00:00+09:00",
                "2026-05-17T09:40:00+09:00",
            ),
            candidate(
                "a",
                "2026-05-18T09:00:00+09:00",
                "2026-05-18T09:41:00+09:00",
            ),
            candidate(
                "a",
                "2026-05-19T09:00:00+09:00",
                "2026-05-19T19:00:00+09:00",
            ),
        ];

        let summary = summarize(&candidates);

        assert_eq!(summary.overall().unwrap().elapsed_seconds, 11 * 60);
        assert_eq!(summary.for_task("a").unwrap().elapsed_seconds, 11 * 60);
    }

    #[test]
    fn summary_does_not_compare_different_tasks_for_outlier_removal() {
        let candidates = vec![
            candidate(
                "short",
                "2026-05-17T09:00:00+09:00",
                "2026-05-17T09:01:00+09:00",
            ),
            candidate(
                "long",
                "2026-05-18T09:00:00+09:00",
                "2026-05-18T19:00:00+09:00",
            ),
        ];

        let summary = summarize(&candidates);

        assert_eq!(summary.overall().unwrap().elapsed_seconds, 5 * 60 * 60 + 30);
        assert_eq!(
            summary.for_task("long").unwrap().elapsed_seconds,
            10 * 60 * 60
        );
    }
}
