use crate::{app::App, auto_free_time::AutoFreeTimeTracker, event, storage, task_runtime};

pub fn reload_config(
    paths: &storage::AppPaths,
    keybindings: &mut event::KeyBindings,
    editors: &mut Vec<String>,
    auto_free_time: &mut AutoFreeTimeTracker,
    external_event_config: &mut storage::ExternalEventConfig,
    ui_config: &mut storage::UiConfig,
) -> Result<(), String> {
    storage::ensure_app_storage(paths)?;
    let config_file = storage::load_config_file(&paths.config_path)?;

    *keybindings = event::KeyBindings::from_config(config_file.keybindings)?;
    *editors = config_file.editors;
    auto_free_time.update_config(config_file.auto_free_time);
    *external_event_config = config_file.external_event;
    *ui_config = config_file.ui;

    Ok(())
}

pub fn reload_tasks(paths: &storage::AppPaths, app: &mut App) -> Result<(), String> {
    storage::ensure_app_storage(paths)?;
    let task_files = storage::load_task_files(&paths.tasks_dir)?;

    app.replace_tabs(task_runtime::task_lists_from_files(&task_files));
    for (index, task_file) in task_files.into_iter().enumerate() {
        task_runtime::reflect_task_file_status(task_file.status, app, index, false);
    }

    Ok(())
}
