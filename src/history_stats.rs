use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::mpsc::Sender,
    thread,
    time::{Duration as StdDuration, Instant},
};

use serde::{Deserialize, Serialize};

use crate::{
    clock,
    event::AppEvent,
    storage::{self, AppPaths},
};

mod revision_cache;
mod typical;
use revision_cache::RevisionStats;
pub use typical::TypicalTaskDuration;

const HISTORY_STATS_TIMEOUT: StdDuration = StdDuration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryStatsReport {
    pub scanned_revisions: usize,
    pub skipped_files: usize,
    pub timed_out: bool,
    pub task_counts: Vec<TaskNameCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskNameCount {
    pub name: String,
    pub count: usize,
    pub typical_task_duration: Option<TypicalTaskDuration>,
}

enum CommandOutcome {
    Finished(Output),
    TimedOut,
}

struct RawTaskStats {
    task_names: Vec<String>,
    task_duration_candidates: Vec<typical::TaskDurationCandidate>,
}

pub fn spawn_history_stats(tx: Sender<AppEvent>, paths: AppPaths) {
    thread::spawn(move || {
        let result = load_or_collect_history_stats(&paths);
        let _ = tx.send(AppEvent::HistoryStatsFinished(result));
    });
}

fn load_or_collect_history_stats(paths: &AppPaths) -> Result<HistoryStatsReport, String> {
    let mut cached = revision_cache::read_revision_cache(&paths.history_stats_cache_path);
    let cached_count = cached.len();
    let result = collect_history_stats(&paths.root_dir, &mut cached);
    if cached.len() != cached_count {
        revision_cache::write_revision_cache(&paths.history_stats_cache_path, &cached);
    }
    result
}

fn collect_history_stats(
    root_dir: &Path,
    cached: &mut HashMap<String, RevisionStats>,
) -> Result<HistoryStatsReport, String> {
    let deadline = Instant::now() + HISTORY_STATS_TIMEOUT;

    let Some(revisions_stdout) =
        git_stdout(root_dir, &["log", "--format=%H", "--", "tasks"], deadline)?
    else {
        return Ok(report_from_counts(
            HashMap::new(),
            HashSet::new(),
            0,
            0,
            true,
        ));
    };
    let revisions = revisions_stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());

    let mut task_counts = HashMap::new();
    let mut task_duration_candidates = HashSet::new();
    let mut scanned_revisions = 0;
    let mut skipped_files = 0;
    let mut timed_out = false;

    for revision in revisions {
        if !cached.contains_key(revision) {
            if timed_out || Instant::now() >= deadline {
                timed_out = true;
                continue;
            }
            match read_revision_stats(root_dir, revision, deadline)? {
                Some(stats) => {
                    cached.insert(revision.to_string(), stats);
                }
                None => {
                    timed_out = true;
                    continue;
                }
            }
        }

        let stats = &cached[revision];
        add_revision_tasks(&mut task_counts, stats.task_names.iter().cloned());
        task_duration_candidates.extend(stats.task_duration_candidates.iter().cloned());
        skipped_files += stats.skipped_files;
        scanned_revisions += 1;
    }

    Ok(report_from_counts(
        task_counts,
        task_duration_candidates,
        scanned_revisions,
        skipped_files,
        timed_out,
    ))
}

/// timeout したら `None`。途中まで読んだ revision を cache に入れると、残りのファイルが欠けたまま固定されるため。
fn read_revision_stats(
    root_dir: &Path,
    revision: &str,
    deadline: Instant,
) -> Result<Option<RevisionStats>, String> {
    let Some(paths_stdout) = git_stdout(
        root_dir,
        &["ls-tree", "-r", "--name-only", revision, "tasks"],
        deadline,
    )?
    else {
        return Ok(None);
    };

    let mut stats = RevisionStats::default();
    for task_path in task_paths(&paths_stdout) {
        let object = format!("{revision}:{task_path}");
        let Some(raw) = git_stdout(root_dir, &["show", &object], deadline)? else {
            return Ok(None);
        };

        match task_stats_from_raw(task_path, &raw) {
            Ok(file_stats) => {
                stats.task_names.extend(file_stats.task_names);
                stats
                    .task_duration_candidates
                    .extend(file_stats.task_duration_candidates);
            }
            Err(_) => stats.skipped_files += 1,
        }
    }
    Ok(Some(stats))
}

fn report_from_counts(
    task_counts: HashMap<String, usize>,
    task_duration_candidates: HashSet<typical::TaskDurationCandidate>,
    scanned_revisions: usize,
    skipped_files: usize,
    timed_out: bool,
) -> HistoryStatsReport {
    let typical_task_durations = typical::summarize(&task_duration_candidates);
    HistoryStatsReport {
        scanned_revisions,
        skipped_files,
        timed_out,
        task_counts: sorted_task_counts(task_counts, &typical_task_durations),
    }
}

fn add_revision_tasks(
    task_counts: &mut HashMap<String, usize>,
    revision_tasks: impl IntoIterator<Item = String>,
) {
    for task in revision_tasks {
        *task_counts.entry(task).or_insert(0) += 1;
    }
}

fn sorted_task_counts(
    task_counts: HashMap<String, usize>,
    typical_task_durations: &typical::TypicalTaskDurations,
) -> Vec<TaskNameCount> {
    let mut counts = task_counts
        .into_iter()
        .map(|(name, count)| TaskNameCount {
            typical_task_duration: typical_task_durations.for_task(&name),
            name,
            count,
        })
        .collect::<Vec<_>>();
    counts.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    counts
}

fn task_paths(raw: &str) -> impl Iterator<Item = &str> {
    raw.lines()
        .map(str::trim)
        .filter(|path| path.starts_with("tasks/"))
        .filter(|path| path.ends_with(".md"))
}

fn task_stats_from_raw(task_path: &str, raw: &str) -> Result<RawTaskStats, String> {
    let file = storage::load_task_file_content(
        task_label(task_path),
        PathBuf::from(task_path),
        raw,
        clock::today_jst(),
    )?;
    let task_duration_candidates = typical::candidates_from_task_file(&file);
    let task_names = file.task.into_iter().map(|task| task.name).collect();
    Ok(RawTaskStats {
        task_names,
        task_duration_candidates,
    })
}

fn task_label(task_path: &str) -> String {
    task_path
        .rsplit('/')
        .next()
        .and_then(|file_name| file_name.strip_suffix(".md"))
        .unwrap_or(task_path)
        .to_string()
}

fn git_stdout(root_dir: &Path, args: &[&str], deadline: Instant) -> Result<Option<String>, String> {
    match run_git(root_dir, args, deadline)? {
        CommandOutcome::TimedOut => Ok(None),
        CommandOutcome::Finished(output) if output.status.success() => {
            Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
        }
        CommandOutcome::Finished(output) => Err(format!(
            "git {} に失敗しました: {}",
            args.join(" "),
            output_summary(&output)
        )),
    }
}

fn run_git(root_dir: &Path, args: &[&str], deadline: Instant) -> Result<CommandOutcome, String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root_dir)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            format!(
                "git {} を実行できませんでした: {} ({err})",
                args.join(" "),
                root_dir.display()
            )
        })?;

    loop {
        if child
            .try_wait()
            .map_err(|err| format!("git {} の終了確認に失敗しました: {err}", args.join(" ")))?
            .is_some()
        {
            return child
                .wait_with_output()
                .map(CommandOutcome::Finished)
                .map_err(|err| format!("git {} の出力取得に失敗しました: {err}", args.join(" ")));
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(CommandOutcome::TimedOut);
        }

        thread::sleep(StdDuration::from_millis(20));
    }
}

fn output_summary(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = stdout.trim();
    let stderr = stderr.trim();

    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => format!("status={}", output.status),
        (false, true) => format!("status={} stdout={stdout}", output.status),
        (true, false) => format!("status={} stderr={stderr}", output.status),
        (false, false) => format!("status={} stdout={stdout} stderr={stderr}", output.status),
    }
}

#[cfg(test)]
mod tests;
