use std::{
    collections::{HashMap, HashSet},
    fs,
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

mod typical;
pub use typical::TypicalTaskDuration;

const HISTORY_STATS_CACHE_VERSION: u32 = 4;
const HISTORY_STATS_TIMEOUT: StdDuration = StdDuration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryStatsReport {
    pub scanned_revisions: usize,
    pub skipped_files: usize,
    pub timed_out: bool,
    pub typical_task_duration: Option<TypicalTaskDuration>,
    pub task_counts: Vec<TaskNameCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskNameCount {
    pub name: String,
    pub count: usize,
    pub typical_task_duration: Option<TypicalTaskDuration>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HistoryStatsCacheFile {
    version: u32,
    head: String,
    report: HistoryStatsReport,
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
    let head = current_head(&paths.root_dir)?;
    if let Some(report) = read_cached_history_stats(&paths.history_stats_cache_path, &head) {
        return Ok(report);
    }

    let report = collect_history_stats(&paths.root_dir)?;
    if !report.timed_out {
        write_cached_history_stats(&paths.history_stats_cache_path, &head, &report);
    }
    Ok(report)
}

fn current_head(root_dir: &Path) -> Result<String, String> {
    let deadline = Instant::now() + HISTORY_STATS_TIMEOUT;
    let Some(stdout) = git_stdout(root_dir, &["rev-parse", "HEAD"], deadline)? else {
        return Err("git HEAD の取得がtimeoutしました".to_string());
    };
    Ok(stdout.trim().to_string())
}

fn read_cached_history_stats(path: &Path, head: &str) -> Option<HistoryStatsReport> {
    let raw = fs::read_to_string(path).ok()?;
    let cache = serde_json::from_str::<HistoryStatsCacheFile>(&raw).ok()?;
    (cache.version == HISTORY_STATS_CACHE_VERSION && cache.head == head).then_some(cache.report)
}

fn write_cached_history_stats(path: &Path, head: &str, report: &HistoryStatsReport) {
    let Some(parent) = path.parent() else {
        return;
    };
    let cache = HistoryStatsCacheFile {
        version: HISTORY_STATS_CACHE_VERSION,
        head: head.to_string(),
        report: report.clone(),
    };
    let Ok(raw) = serde_json::to_string(&cache) else {
        return;
    };

    let _ = fs::create_dir_all(parent);
    let _ = fs::write(path, raw);
}

fn collect_history_stats(root_dir: &Path) -> Result<HistoryStatsReport, String> {
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
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    let mut task_counts = HashMap::new();
    let mut task_duration_candidates = HashSet::new();
    let mut scanned_revisions = 0;
    let mut skipped_files = 0;
    let mut timed_out = false;

    for revision in revisions {
        if Instant::now() >= deadline {
            timed_out = true;
            break;
        }
        let mut revision_tasks = HashSet::new();

        let Some(paths_stdout) = git_stdout(
            root_dir,
            &["ls-tree", "-r", "--name-only", &revision, "tasks"],
            deadline,
        )?
        else {
            timed_out = true;
            break;
        };

        for task_path in task_paths(&paths_stdout) {
            if Instant::now() >= deadline {
                timed_out = true;
                break;
            }

            let object = format!("{revision}:{task_path}");
            let Some(raw) = git_stdout(root_dir, &["show", &object], deadline)? else {
                timed_out = true;
                break;
            };

            match task_stats_from_raw(task_path, &raw) {
                Ok(stats) => {
                    for task_name in stats.task_names {
                        revision_tasks.insert(task_name);
                    }
                    task_duration_candidates.extend(stats.task_duration_candidates);
                }
                Err(_) => skipped_files += 1,
            }
        }

        if timed_out {
            break;
        }
        add_revision_tasks(&mut task_counts, revision_tasks);
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
        typical_task_duration: typical_task_durations.overall(),
        task_counts: sorted_task_counts(task_counts, &typical_task_durations),
    }
}

fn add_revision_tasks(task_counts: &mut HashMap<String, usize>, revision_tasks: HashSet<String>) {
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
