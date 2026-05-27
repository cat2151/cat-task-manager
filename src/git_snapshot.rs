use std::{sync::mpsc::Sender, thread, time::Duration as StdDuration};

use chrono::NaiveDate;

use crate::{
    clock,
    event::AppEvent,
    logging::AppLogger,
    startup_git::{self, SnapshotKind},
    storage::AppPaths,
};

const INITIAL_RETRY_WAIT: StdDuration = StdDuration::from_secs(1);
const MAX_RETRY_WAIT: StdDuration = StdDuration::from_secs(60 * 60);

pub fn spawn_startup_snapshot(tx: Sender<AppEvent>, paths: AppPaths, logger: AppLogger) {
    thread::spawn(move || {
        let result = run_snapshot(&paths, &logger, SnapshotKind::Startup, clock::today_jst());
        let _ = tx.send(AppEvent::StartupGitFinished(result));
    });
}

pub fn spawn_before_day_change_snapshot(
    tx: Sender<AppEvent>,
    paths: AppPaths,
    logger: AppLogger,
    date: NaiveDate,
) {
    thread::spawn(move || {
        let result = run_before_day_change_until_success(&tx, &paths, &logger, date);
        let _ = tx.send(AppEvent::DayChangeGitFinished(result));
    });
}

fn run_before_day_change_until_success(
    tx: &Sender<AppEvent>,
    paths: &AppPaths,
    logger: &AppLogger,
    date: NaiveDate,
) -> Result<String, String> {
    let mut retry_wait = INITIAL_RETRY_WAIT;

    loop {
        send_background_message(tx, "日付変更前git snapshotを実行中です")?;
        match run_snapshot(paths, logger, SnapshotKind::BeforeDayChange, date) {
            Ok(message) => return Ok(message),
            Err(err) => {
                let wait_message =
                    format!("{}後にretryします: {err}", format_retry_wait(retry_wait));
                let _ = logger
                    .log_git_snapshot(SnapshotKind::BeforeDayChange.log_label(), &wait_message);
                wait_before_retry(tx, retry_wait, &err)?;
                retry_wait = next_retry_wait(retry_wait);
            }
        }
    }
}

fn run_snapshot(
    paths: &AppPaths,
    logger: &AppLogger,
    kind: SnapshotKind,
    date: NaiveDate,
) -> Result<String, String> {
    match startup_git::commit_and_push(&paths.root_dir, date, kind) {
        Ok(outcome) => {
            let message = outcome.log_message();
            logger.log_git_snapshot(kind.log_label(), &message)?;
            Ok(message)
        }
        Err(err) => {
            let message = format!("失敗しました: {err}");
            logger.log_git_snapshot(kind.log_label(), &message)?;
            Err(message)
        }
    }
}

fn wait_before_retry(tx: &Sender<AppEvent>, wait: StdDuration, err: &str) -> Result<(), String> {
    let mut remaining = wait.as_secs();

    while remaining > 0 {
        send_background_message(
            tx,
            format!(
                "日付変更前snapshot retryまで{}: {err}",
                format_retry_wait(StdDuration::from_secs(remaining))
            ),
        )?;
        thread::sleep(StdDuration::from_secs(1));
        remaining -= 1;
    }

    Ok(())
}

fn send_background_message(
    tx: &Sender<AppEvent>,
    message: impl Into<String>,
) -> Result<(), String> {
    tx.send(AppEvent::BackgroundWorkMessage(message.into()))
        .map_err(|_| "event channel が切断されました".to_string())
}

fn next_retry_wait(current: StdDuration) -> StdDuration {
    StdDuration::from_secs(
        current
            .as_secs()
            .saturating_mul(2)
            .min(MAX_RETRY_WAIT.as_secs()),
    )
}

fn format_retry_wait(duration: StdDuration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    match (hours, minutes, seconds) {
        (0, 0, seconds) => format!("{seconds}秒"),
        (0, minutes, 0) => format!("{minutes}分"),
        (0, minutes, seconds) => format!("{minutes}分{seconds}秒"),
        (hours, 0, 0) => format!("{hours}時間"),
        (hours, minutes, 0) => format!("{hours}時間{minutes}分"),
        (hours, minutes, seconds) => format!("{hours}時間{minutes}分{seconds}秒"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_wait_doubles_until_one_hour() {
        assert_eq!(
            next_retry_wait(StdDuration::from_secs(1)),
            StdDuration::from_secs(2)
        );
        assert_eq!(
            next_retry_wait(StdDuration::from_secs(1800)),
            StdDuration::from_secs(3600)
        );
        assert_eq!(
            next_retry_wait(StdDuration::from_secs(3600)),
            StdDuration::from_secs(3600)
        );
    }

    #[test]
    fn retry_wait_formats_compactly() {
        assert_eq!(format_retry_wait(StdDuration::from_secs(1)), "1秒");
        assert_eq!(format_retry_wait(StdDuration::from_secs(60)), "1分");
        assert_eq!(format_retry_wait(StdDuration::from_secs(3600)), "1時間");
    }
}
