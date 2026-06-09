use crate::{
    app::{App, TaskList},
    logging, storage,
};

pub fn reflect_task_file_status(
    status: Option<storage::TaskFileStatus>,
    app: &mut App,
    tab_index: usize,
    update_message: bool,
) -> logging::TaskFileStatusReadOutcome {
    if let Some(status) = status {
        if status.date == app.current_date {
            app.apply_statuses(tab_index, &status.states);
            if update_message {
                app.set_message("task fileの状態を読み込みました");
            }
            logging::TaskFileStatusReadOutcome::Loaded
        } else {
            if update_message {
                app.set_message("task fileの状態日付が今日ではないため未着手で表示します");
            }
            logging::TaskFileStatusReadOutcome::DateMismatch {
                status_date: status.date,
            }
        }
    } else {
        logging::TaskFileStatusReadOutcome::Missing
    }
}

pub fn persist_tasks(app: &mut App) {
    app.sync_free_time_elapsed();
    let result = app.tabs().iter().try_for_each(|tab| {
        storage::write_task_file_status(&tab.path, app.current_date, &tab.tasks)
    });
    if let Err(err) = result {
        app.set_message(err);
    }
}

pub fn task_lists_from_files(task_files: &[storage::TaskFile]) -> Vec<TaskList> {
    task_files
        .iter()
        .map(|task_file| TaskList {
            label: task_file.label.clone(),
            path: task_file.path.clone(),
            tasks: task_file.task.clone(),
        })
        .collect()
}
