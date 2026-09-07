use chrono::{DateTime, Local};

use super::{App, TaskEvent, TaskEventKind, TaskLocation};

impl App {
    pub fn drain_task_events(&mut self) -> impl Iterator<Item = TaskEvent> + '_ {
        self.pending_task_events.drain(..)
    }

    pub(super) fn queue_task_event(
        &mut self,
        location: TaskLocation,
        kind: TaskEventKind,
        occurred_at: DateTime<Local>,
    ) {
        let tab = &self.tabs[location.tab_index];
        let task = &tab.tasks[location.task_index];
        self.pending_task_events.push_back(TaskEvent {
            kind,
            occurred_at,
            task_name: task.name.clone(),
            task_file: tab.path.clone(),
            source_line: task.source_line,
        });
    }
}
