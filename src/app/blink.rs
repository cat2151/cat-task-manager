use super::{App, AppScreen, TaskState, ViewMode};

const ESTIMATE_BLINK_TICKS_PER_PHASE: u8 = 10;

impl App {
    pub fn set_window_focused(&mut self, focused: bool) {
        if self.window_focused == focused {
            return;
        }

        self.window_focused = focused;
        self.estimate_blink_phase = false;
        self.estimate_blink_tick = 0;
    }

    pub fn estimate_blink_phase(&self) -> bool {
        self.estimate_blink_phase
    }

    pub fn estimate_blink_context(&self) -> bool {
        self.screen == AppScreen::Tasks
            && self.window_focused
            && self.view_mode == ViewMode::OneLine
            && self.selected_task_is_not_started()
    }

    pub fn tick_estimate_blink(&mut self) -> bool {
        self.estimate_blink_tick = self.estimate_blink_tick.saturating_add(1);
        if self.estimate_blink_tick < ESTIMATE_BLINK_TICKS_PER_PHASE {
            return false;
        }

        self.estimate_blink_tick = 0;
        self.estimate_blink_phase = !self.estimate_blink_phase;
        true
    }

    fn selected_task_is_not_started(&self) -> bool {
        self.selected_task_location()
            .map(|(_, location)| self.task_at(location).state == TaskState::NotStarted)
            .unwrap_or(false)
    }
}
