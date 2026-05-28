use crate::app::{DailyTask, TaskTab};

pub fn tabs_differ(before: &[TaskTab], after: &[TaskTab]) -> bool {
    before.len() != after.len()
        || before.iter().zip(after).any(|(before, after)| {
            before.label != after.label
                || before.path != after.path
                || tasks_differ(&before.tasks, &after.tasks)
        })
}

fn tasks_differ(before: &[DailyTask], after: &[DailyTask]) -> bool {
    before.len() != after.len()
        || before.iter().zip(after).any(|(before, after)| {
            before.name != after.name
                || before.order != after.order
                || before.source_line != after.source_line
                || before.state != after.state
                || before.started_at != after.started_at
                || before.completed_at != after.completed_at
        })
}
