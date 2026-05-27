use std::{
    path::Path,
    process::{Command, Output},
};

use chrono::NaiveDate;

const STARTUP_SNAPSHOT_COMMIT_PREFIX: &str = "起動時snapshot";
const BEFORE_DAY_CHANGE_SNAPSHOT_COMMIT_PREFIX: &str = "日付変更前snapshot";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotKind {
    Startup,
    BeforeDayChange,
}

impl SnapshotKind {
    pub fn log_label(self) -> &'static str {
        match self {
            SnapshotKind::Startup => "起動時git snapshot",
            SnapshotKind::BeforeDayChange => "日付変更前git snapshot",
        }
    }

    fn commit_prefix(self) -> &'static str {
        match self {
            SnapshotKind::Startup => STARTUP_SNAPSHOT_COMMIT_PREFIX,
            SnapshotKind::BeforeDayChange => BEFORE_DAY_CHANGE_SNAPSHOT_COMMIT_PREFIX,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupGitOutcome {
    Committed { commit_message: String },
    AlreadyCommitted { commit_message: String },
    NoChanges { commit_message: String },
}

impl StartupGitOutcome {
    pub fn log_message(&self) -> String {
        match self {
            StartupGitOutcome::Committed { commit_message } => {
                format!("commitしてpushしました: {commit_message}")
            }
            StartupGitOutcome::AlreadyCommitted { commit_message } => {
                format!("対象日のcommitは作成済みです。pushだけ実行しました: {commit_message}")
            }
            StartupGitOutcome::NoChanges { commit_message } => {
                format!("commit対象の変更はありません。pushだけ実行しました: {commit_message}")
            }
        }
    }
}

pub fn commit_and_push(
    root_dir: impl AsRef<Path>,
    date: NaiveDate,
    kind: SnapshotKind,
) -> Result<StartupGitOutcome, String> {
    let root_dir = root_dir.as_ref();
    ensure_git_work_tree(root_dir)?;

    let commit_message = snapshot_commit_message(kind, date);
    if has_snapshot_commit(root_dir, &commit_message)? {
        run_git(root_dir, &["push"])?;
        return Ok(StartupGitOutcome::AlreadyCommitted { commit_message });
    }

    run_git(root_dir, &["add", "-A"])?;
    if !has_staged_changes(root_dir)? {
        run_git(root_dir, &["push"])?;
        return Ok(StartupGitOutcome::NoChanges { commit_message });
    }

    run_git_with_message(root_dir, &["commit", "-m"], &commit_message)?;
    run_git(root_dir, &["push"])?;

    Ok(StartupGitOutcome::Committed { commit_message })
}

fn ensure_git_work_tree(root_dir: &Path) -> Result<(), String> {
    let output = run_git(root_dir, &["rev-parse", "--is-inside-work-tree"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim() == "true" {
        Ok(())
    } else {
        Err(format!(
            "git repositoryではありません: {}",
            root_dir.display()
        ))
    }
}

fn has_snapshot_commit(root_dir: &Path, commit_message: &str) -> Result<bool, String> {
    let output = run_git_with_message(
        root_dir,
        &["log", "--format=%s", "--fixed-strings", "--grep"],
        commit_message,
    );

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.lines().any(|line| line == commit_message))
        }
        Err(err) if err.contains("does not have any commits yet") => Ok(false),
        Err(err) => Err(err),
    }
}

fn has_staged_changes(root_dir: &Path) -> Result<bool, String> {
    let output = git_command(root_dir, &["diff", "--cached", "--quiet"]).output();
    let output = output.map_err(|err| {
        format!(
            "git diff --cached --quiet を実行できませんでした: {} ({err})",
            root_dir.display()
        )
    })?;

    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(format!(
            "git diff --cached --quiet に失敗しました: {}",
            output_summary(&output)
        )),
    }
}

fn run_git(root_dir: &Path, args: &[&str]) -> Result<Output, String> {
    let output = git_command(root_dir, args).output().map_err(|err| {
        format!(
            "git {} を実行できませんでした: {} ({err})",
            args.join(" "),
            root_dir.display()
        )
    })?;

    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "git {} に失敗しました: {}",
            args.join(" "),
            output_summary(&output)
        ))
    }
}

fn run_git_with_message(
    root_dir: &Path,
    prefix_args: &[&str],
    message: &str,
) -> Result<Output, String> {
    let mut command = git_command(root_dir, prefix_args);
    command.arg(message);
    let output = command.output().map_err(|err| {
        format!(
            "git {} を実行できませんでした: {} ({err})",
            prefix_args.join(" "),
            root_dir.display()
        )
    })?;

    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "git {} に失敗しました: {}",
            prefix_args.join(" "),
            output_summary(&output)
        ))
    }
}

fn git_command(root_dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(root_dir).args(args);
    command
}

fn snapshot_commit_message(kind: SnapshotKind, date: NaiveDate) -> String {
    format!("{} {date}", kind.commit_prefix())
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
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn snapshot_commit_message_uses_snapshot_kind_and_date() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 25).unwrap();

        assert_eq!(
            snapshot_commit_message(SnapshotKind::Startup, date),
            "起動時snapshot 2026-05-25"
        );
        assert_eq!(
            snapshot_commit_message(SnapshotKind::BeforeDayChange, date),
            "日付変更前snapshot 2026-05-25"
        );
    }

    #[test]
    fn outcome_messages_describe_action() {
        let outcome = StartupGitOutcome::Committed {
            commit_message: "起動時snapshot 2026-05-25".to_string(),
        };

        assert_eq!(
            outcome.log_message(),
            "commitしてpushしました: 起動時snapshot 2026-05-25"
        );
    }
}
