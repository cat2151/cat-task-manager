use std::{error::Error, io, path::Path, process::Command, sync::mpsc};

use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod clock;
mod event;
mod logging;
mod storage;
mod ui;

use app::{App, DailyTask};
use event::AppEvent;

fn main() -> Result<(), Box<dyn Error>> {
    let paths = storage::app_paths()?;
    storage::ensure_app_storage(&paths)?;
    let logger = logging::AppLogger::new(&paths.root_dir)?;
    let _app_run_log = logging::AppRunLog::start(&logger, &paths)?;

    let config_file = storage::load_config_file(&paths.config_path)?;
    let mut keybindings = event::KeyBindings::from_config(config_file.keybindings)?;
    let mut editors = config_file.editors;
    let task_file = storage::load_task_file(&paths.tasks_path)?;
    let mut app = App::new(task_file.task, clock::today_jst());
    let before_task_file_read = logging::task_snapshots(&app.tasks);
    let state_outcome = reflect_task_file_status(task_file.status, &mut app, true);
    logger.log_task_file_status_read(&paths.tasks_path, state_outcome)?;
    logger.log_task_changes(
        &before_task_file_read,
        &app.tasks,
        logging::TaskChangeCause::TaskFileRead,
    )?;
    storage::write_task_file_status(&paths.tasks_path, app.current_date, &app.tasks)?;

    let _terminal_guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let (tx, rx) = mpsc::channel();
    event::spawn_event_threads(tx, paths.config_path.clone(), paths.tasks_path.clone());

    terminal.draw(|frame| ui::draw(frame, &app, &keybindings))?;
    run_event_loop(
        &paths,
        &mut terminal,
        &mut app,
        &mut keybindings,
        &mut editors,
        rx,
        &logger,
    )?;

    Ok(())
}

fn run_event_loop(
    paths: &storage::AppPaths,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    keybindings: &mut event::KeyBindings,
    editors: &mut Vec<String>,
    rx: mpsc::Receiver<AppEvent>,
    logger: &logging::AppLogger,
) -> Result<(), Box<dyn Error>> {
    loop {
        match event::read_next_event(&rx)? {
            AppEvent::Key(key) if keybindings.quit.matches(&key) => {
                persist_tasks(paths, app);
                break;
            }
            AppEvent::Key(key) if keybindings.edit.matches(&key) => {
                let before = logging::task_snapshots(&app.tasks);
                edit_tasks(paths, terminal, app, editors)?;
                log_task_changes(logger, app, &before, logging::TaskChangeCause::TaskFileRead);
            }
            AppEvent::Key(key) => match reload_tasks(paths, app) {
                Ok(()) => {
                    let before = logging::task_snapshots(&app.tasks);
                    let cause = key_change_cause(&key, keybindings);
                    app.handle_key(key, keybindings);
                    log_task_changes(logger, app, &before, cause);
                }
                Err(err) => app.set_message(err),
            },
            AppEvent::DayChanged => match reload_tasks(paths, app) {
                Ok(()) => {
                    let before = logging::task_snapshots(&app.tasks);
                    app.complete_day(&paths.records_dir, clock::today_jst());
                    log_task_changes(logger, app, &before, logging::TaskChangeCause::DayChanged);
                }
                Err(err) => app.set_message(err),
            },
            AppEvent::ConfigChanged => match reload_config(paths, keybindings, editors) {
                Ok(()) => app.set_message("設定をhot reloadしました"),
                Err(err) => app.set_message(err),
            },
            AppEvent::TasksChanged => {
                let before_tasks = app.tasks.clone();
                let before = logging::task_snapshots(&app.tasks);
                match reload_tasks(paths, app) {
                    Ok(()) if tasks_differ(&before_tasks, &app.tasks) => {
                        app.set_message("tasks.txtをhot reloadしました");
                    }
                    Ok(()) => {}
                    Err(err) => app.set_message(err),
                }
                log_task_changes(logger, app, &before, logging::TaskChangeCause::TaskFileRead);
            }
        }
        persist_tasks(paths, app);
        terminal.draw(|frame| ui::draw(frame, app, keybindings))?;
    }

    Ok(())
}

fn reflect_task_file_status(
    status: Option<storage::TaskFileStatus>,
    app: &mut App,
    update_message: bool,
) -> logging::TaskFileStatusReadOutcome {
    if let Some(status) = status {
        if status.date == app.current_date {
            app.apply_statuses(&status.states);
            if update_message {
                app.set_message("tasks.txtの状態を読み込みました");
            }
            logging::TaskFileStatusReadOutcome::Loaded
        } else {
            if update_message {
                app.set_message("tasks.txtの状態日付が今日ではないため未着手で表示します");
            }
            logging::TaskFileStatusReadOutcome::DateMismatch {
                status_date: status.date,
            }
        }
    } else {
        logging::TaskFileStatusReadOutcome::Missing
    }
}

fn persist_tasks(paths: &storage::AppPaths, app: &mut App) {
    if let Err(err) =
        storage::write_task_file_status(&paths.tasks_path, app.current_date, &app.tasks)
    {
        app.set_message(err);
    }
}

fn edit_tasks(
    paths: &storage::AppPaths,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    editors: &[String],
) -> Result<(), Box<dyn Error>> {
    TerminalGuard::suspend()?;
    let edit_result = open_with_configured_editor(&paths.tasks_path, editors);
    TerminalGuard::resume()?;
    terminal.clear()?;

    match edit_result {
        Ok(editor) => match reload_tasks(paths, app) {
            Ok(()) => app.set_message(format!("tasks.txtを編集しました: {editor}")),
            Err(err) => app.set_message(err),
        },
        Err(err) => app.set_message(err),
    }

    Ok(())
}

fn reload_config(
    paths: &storage::AppPaths,
    keybindings: &mut event::KeyBindings,
    editors: &mut Vec<String>,
) -> Result<(), String> {
    storage::ensure_app_storage(paths)?;
    let config_file = storage::load_config_file(&paths.config_path)?;

    *keybindings = event::KeyBindings::from_config(config_file.keybindings)?;
    *editors = config_file.editors;

    Ok(())
}

fn reload_tasks(paths: &storage::AppPaths, app: &mut App) -> Result<(), String> {
    storage::ensure_app_storage(paths)?;
    let task_file = storage::load_task_file(&paths.tasks_path)?;

    app.replace_tasks(task_file.task);
    reflect_task_file_status(task_file.status, app, false);

    Ok(())
}

fn open_with_configured_editor(path: &Path, editors: &[String]) -> Result<String, String> {
    let mut failures = Vec::new();

    for editor in editors
        .iter()
        .map(|editor| editor.trim())
        .filter(|editor| !editor.is_empty())
    {
        match Command::new(editor).arg(path).status() {
            Ok(status) if status.success() => return Ok(editor.to_string()),
            Ok(status) => failures.push(format!("{editor}: 終了 status が失敗です ({status})")),
            Err(err) => failures.push(format!("{editor}: {err}")),
        }
    }

    if failures.is_empty() {
        return Err("エディタが設定されていません".to_string());
    }

    Err(format!(
        "ファイルを開けませんでした: {} ({})",
        path.display(),
        failures.join("; ")
    ))
}

fn key_change_cause(
    key: &crossterm::event::KeyEvent,
    keybindings: &event::KeyBindings,
) -> logging::TaskChangeCause {
    if keybindings.advance.matches(key) {
        logging::TaskChangeCause::KeyAdvance
    } else if keybindings.hold.matches(key) {
        logging::TaskChangeCause::KeyHold
    } else {
        logging::TaskChangeCause::KeyOther
    }
}

fn log_task_changes(
    logger: &logging::AppLogger,
    app: &mut App,
    before: &[logging::TaskSnapshot],
    cause: logging::TaskChangeCause,
) {
    if let Err(err) = logger.log_task_changes(before, &app.tasks, cause) {
        app.set_message(err);
    }
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

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(Self)
    }

    fn suspend() -> Result<(), Box<dyn Error>> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen, Show)?;
        Ok(())
    }

    fn resume() -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    }
}
