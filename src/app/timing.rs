use chrono::{DateTime, Duration, Local};

use super::{DailyTask, TaskPause, TaskState};

impl DailyTask {
    pub(super) fn pause_at(&mut self, state: TaskState, paused_at: DateTime<Local>) {
        debug_assert_eq!(self.state, TaskState::InProgress);
        debug_assert!(matches!(state, TaskState::OnHold | TaskState::Deferred));
        debug_assert!(self
            .pauses
            .last()
            .is_none_or(|pause| pause.resumed_at.is_some()));

        self.pauses.push(TaskPause {
            paused_at,
            resumed_at: None,
        });
        self.state = state;
    }

    pub(super) fn resume_at(&mut self, resumed_at: DateTime<Local>) {
        debug_assert!(matches!(
            self.state,
            TaskState::OnHold | TaskState::Deferred
        ));
        let pause = self
            .pauses
            .last_mut()
            .expect("中断中のtaskには未終了のpauseが必要です");
        debug_assert!(pause.resumed_at.is_none());

        pause.resumed_at = Some(resumed_at);
        self.state = TaskState::InProgress;
    }
}

pub(crate) fn completed_work_duration(
    started_at: Option<DateTime<Local>>,
    completed_at: Option<DateTime<Local>>,
    pauses: &[TaskPause],
) -> Option<Duration> {
    let started_at = started_at?;
    let completed_at = completed_at?;
    validate_pause_timeline(started_at, Some(completed_at), pauses, false).ok()?;

    let paused_duration = pauses.iter().try_fold(Duration::zero(), |total, pause| {
        Some(total + (pause.resumed_at? - pause.paused_at))
    })?;
    let duration = completed_at - started_at - paused_duration;
    (duration >= Duration::zero()).then_some(duration)
}

pub(crate) fn validate_task_timing(
    state: &TaskState,
    started_at: Option<DateTime<Local>>,
    completed_at: Option<DateTime<Local>>,
    pauses: &[TaskPause],
) -> Result<(), &'static str> {
    match state {
        TaskState::NotStarted => {
            if started_at.is_some() || completed_at.is_some() || !pauses.is_empty() {
                return Err("未着手taskには時刻やpausesを書けません");
            }
            return Ok(());
        }
        TaskState::InProgress => {
            if completed_at.is_some() {
                return Err("実施中taskにはcompleted_atを書けません");
            }
        }
        TaskState::Done => {
            if completed_at.is_none() {
                return Err("完了taskにはcompleted_atが必要です");
            }
        }
        TaskState::OnHold => {
            if completed_at.is_some() {
                return Err("保留taskにはcompleted_atを書けません");
            }
        }
        TaskState::Deferred => {
            if completed_at.is_some() {
                return Err("後回しtaskにはcompleted_atを書けません");
            }
            if started_at.is_none() {
                if pauses.is_empty() {
                    return Ok(());
                }
                return Err("未着手の後回しtaskにはpausesを書けません");
            }
        }
    }

    let started_at = started_at.ok_or("着手済みtaskにはstarted_atが必要です")?;
    let expects_open_pause = matches!(state, TaskState::OnHold | TaskState::Deferred);
    validate_pause_timeline(started_at, completed_at, pauses, expects_open_pause)
}

fn validate_pause_timeline(
    started_at: DateTime<Local>,
    completed_at: Option<DateTime<Local>>,
    pauses: &[TaskPause],
    expects_open_pause: bool,
) -> Result<(), &'static str> {
    let mut previous_end = started_at;
    let mut has_open_pause = false;
    for (index, pause) in pauses.iter().enumerate() {
        if pause.paused_at < previous_end {
            return Err("pausesは時刻順で重ならないようにしてください");
        }
        if completed_at.is_some_and(|completed_at| pause.paused_at > completed_at) {
            return Err("pauseをcompleted_atより後にはできません");
        }

        match pause.resumed_at {
            Some(resumed_at) => {
                if resumed_at < pause.paused_at {
                    return Err("resumed_atをpaused_atより前にはできません");
                }
                if completed_at.is_some_and(|completed_at| resumed_at > completed_at) {
                    return Err("resumed_atをcompleted_atより後にはできません");
                }
                previous_end = resumed_at;
            }
            None if index + 1 == pauses.len() => has_open_pause = true,
            None => return Err("未終了のpauseは最後の1件だけにしてください"),
        }
    }

    if has_open_pause != expects_open_pause {
        return if expects_open_pause {
            Err("保留中または着手後の後回しtaskには未終了のpauseが必要です")
        } else {
            Err("実施中または完了taskに未終了のpauseは残せません")
        };
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(raw: &str) -> DateTime<Local> {
        DateTime::parse_from_rfc3339(raw)
            .unwrap()
            .with_timezone(&Local)
    }

    #[test]
    fn completed_work_duration_subtracts_all_pauses() {
        let pauses = vec![
            TaskPause {
                paused_at: time("2026-09-06T09:15:00+09:00"),
                resumed_at: Some(time("2026-09-06T09:25:00+09:00")),
            },
            TaskPause {
                paused_at: time("2026-09-06T09:40:00+09:00"),
                resumed_at: Some(time("2026-09-06T09:45:00+09:00")),
            },
        ];

        let duration = completed_work_duration(
            Some(time("2026-09-06T09:00:00+09:00")),
            Some(time("2026-09-06T10:00:00+09:00")),
            &pauses,
        )
        .unwrap();

        assert_eq!(duration.num_seconds(), 45 * 60);
    }

    #[test]
    fn completed_work_duration_rejects_open_pause() {
        let pauses = vec![TaskPause {
            paused_at: time("2026-09-06T09:15:00+09:00"),
            resumed_at: None,
        }];

        assert!(completed_work_duration(
            Some(time("2026-09-06T09:00:00+09:00")),
            Some(time("2026-09-06T10:00:00+09:00")),
            &pauses,
        )
        .is_none());
    }
}
