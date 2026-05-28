use crate::history_stats::HistoryStatsReport;

use super::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    Tasks,
    HistoryStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryStatsState {
    Idle,
    Loading,
    Ready(HistoryStatsReport),
    Error(String),
}

impl HistoryStatsState {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }
}

impl App {
    pub fn screen(&self) -> AppScreen {
        self.screen
    }

    pub fn is_history_stats_screen(&self) -> bool {
        self.screen == AppScreen::HistoryStats
    }

    pub fn history_stats(&self) -> &HistoryStatsState {
        &self.history_stats
    }

    pub fn tick_history_stats(&mut self) {
        if self.history_stats.is_loading() {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
        }
    }

    pub fn start_history_stats_prefetch(&mut self) -> bool {
        self.start_history_stats_load(false)
    }

    pub fn toggle_history_stats_screen(&mut self) -> bool {
        if self.show_help {
            return false;
        }

        match self.screen {
            AppScreen::Tasks => {
                self.screen = AppScreen::HistoryStats;
                match self.history_stats {
                    HistoryStatsState::Ready(_) => {
                        self.message = "過去データ統計を表示しました".to_string();
                        false
                    }
                    _ => self.start_history_stats_load(true),
                }
            }
            AppScreen::HistoryStats => {
                self.screen = AppScreen::Tasks;
                self.message = "tasks画面へ戻りました".to_string();
                false
            }
        }
    }

    fn start_history_stats_load(&mut self, update_message: bool) -> bool {
        if self.history_stats.is_loading() {
            if update_message {
                self.message = "過去データ統計を取得中です".to_string();
            }
            return false;
        }

        if matches!(self.history_stats, HistoryStatsState::Ready(_)) {
            return false;
        }

        self.history_stats = HistoryStatsState::Loading;
        self.spinner_frame = 0;
        if update_message {
            self.message = "過去データ統計を取得中です".to_string();
        }
        true
    }

    pub fn finish_history_stats(&mut self, result: Result<HistoryStatsReport, String>) {
        let update_message = self.is_history_stats_screen();
        match result {
            Ok(report) => {
                let message = if report.timed_out {
                    "過去データ統計はtimeoutまでの途中結果です"
                } else {
                    "過去データ統計を取得しました"
                };
                self.history_stats = HistoryStatsState::Ready(report);
                if update_message {
                    self.message = message.to_string();
                }
            }
            Err(err) => {
                self.history_stats = HistoryStatsState::Error(err);
                if update_message {
                    self.message = "過去データ統計の取得に失敗しました".to_string();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::history_stats::{HistoryStatsReport, TaskNameCount};

    use super::*;

    fn app() -> App {
        App::new(Vec::new(), NaiveDate::from_ymd_opt(2026, 5, 18).unwrap())
    }

    fn report() -> HistoryStatsReport {
        HistoryStatsReport {
            scanned_revisions: 1,
            skipped_files: 0,
            timed_out: false,
            task_counts: vec![TaskNameCount {
                name: "a".to_string(),
                count: 1,
            }],
        }
    }

    #[test]
    fn prefetch_starts_without_opening_stats_screen() {
        let mut app = app();

        assert!(app.start_history_stats_prefetch());

        assert_eq!(app.screen(), AppScreen::Tasks);
        assert!(app.history_stats().is_loading());
        assert_eq!(app.message(), "待機中");
    }

    #[test]
    fn cached_report_opens_without_loading_again() {
        let mut app = app();
        app.finish_history_stats(Ok(report()));

        assert!(!app.toggle_history_stats_screen());

        assert_eq!(app.screen(), AppScreen::HistoryStats);
        assert!(matches!(app.history_stats(), HistoryStatsState::Ready(_)));
    }

    #[test]
    fn failed_stats_retries_when_opened() {
        let mut app = app();
        app.finish_history_stats(Err("failed".to_string()));

        assert!(app.toggle_history_stats_screen());

        assert!(app.history_stats().is_loading());
    }
}
