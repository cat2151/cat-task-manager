use std::{env, error::Error, io, path::Path, process::Command, sync::mpsc, thread};

use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod cli;
mod clock;
mod event;
mod logging;
mod self_update;
mod startup_git;
mod storage;
mod ui;

use app::{App, DailyTask, TaskList, TaskTab};
use event::AppEvent;

fn main() -> Result<(), Box<dyn Error>> {
    match cli::parse(env::args()) {
        Ok(cli::Command::RunApp) => run_app(),
        Ok(cli::Command::Update) => {
            if let Err(err) = self_update::run_update() {
                eprintln!("Update failed: {err}");
                std::process::exit(1);
            }
            Ok(())
        }
        Ok(cli::Command::Check) => {
            if let Err(err) = self_update::run_check() {
                eprintln!("Check failed: {err}");
                std::process::exit(1);
            }
            Ok(())
        }
        Ok(cli::Command::Help) => {
            print!("{}", cli::HELP);
            Ok(())
        }
        Err(err) => {
            eprintln!("{err}\n\n{}", cli::HELP.trim_end());
            std::process::exit(2);
        }
    }
}

fn run_app() -> Result<(), Box<dyn Error>> {
    let paths = storage::app_paths()?;
    storage::ensure_app_storage(&paths)?;
    let logger = logging::AppLogger::new(&paths.root_dir)?;
    let _app_run_log = logging::AppRunLog::start(&logger, &paths)?;

    let config_file = storage::load_config_file(&paths.config_path)?;
    let startup_git_enabled = config_file.startup_git.auto_commit_and_push;

    let mut keybindings = event::KeyBindings::from_config(config_file.keybindings)?;
    let mut editors = config_file.editors;
    let task_files = storage::load_task_files(&paths.tasks_dir)?;
    let mut app = App::new(task_lists_from_files(&task_files), clock::today_jst());
    let before_task_file_read = logging::task_snapshots(app.tabs());
    for (index, task_file) in task_files.iter().enumerate() {
        let state_outcome =
            reflect_task_file_status(task_file.status.clone(), &mut app, index, true);
        logger.log_task_file_status_read(&task_file.path, state_outcome)?;
    }
    logger.log_task_changes(
        &before_task_file_read,
        app.tabs(),
        logging::TaskChangeCause::TaskFileRead,
    )?;
    persist_tasks(&mut app);

    let _terminal_guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let (tx, rx) = mpsc::channel();
    event::spawn_event_threads(
        tx.clone(),
        paths.config_path.clone(),
        paths.tasks_dir.clone(),
    );
    if startup_git_enabled {
        app.start_background_work("起動時git snapshotを実行中です");
        spawn_startup_git_thread(tx, paths.clone(), logger.clone());
    }

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

fn spawn_startup_git_thread(
    tx: mpsc::Sender<AppEvent>,
    paths: storage::AppPaths,
    logger: logging::AppLogger,
) {
    thread::spawn(move || {
        let result = run_startup_git(&paths, &logger);
        let _ = tx.send(AppEvent::StartupGitFinished(result));
    });
}

fn run_startup_git(
    paths: &storage::AppPaths,
    logger: &logging::AppLogger,
) -> Result<String, String> {
    match startup_git::commit_and_push(&paths.root_dir, clock::today_jst()) {
        Ok(outcome) => {
            let message = outcome.log_message();
            logger.log_startup_git(&message)?;
            Ok(message)
        }
        Err(err) => {
            let message = format!("失敗しました: {err}");
            logger.log_startup_git(&message)?;
            Err(message)
        }
    }
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
        let app_event = event::read_next_event(&rx)?;
        let should_persist = event_should_persist(&app_event);
        let mut should_draw = true;

        match app_event {
            AppEvent::Tick => {
                if app.has_background_work() {
                    app.tick_background_work();
                } else {
                    should_draw = false;
                }
            }
            AppEvent::Key(key) => {
                let action = keybindings.action_for(&key);
                if app.has_background_work() {
                    app.set_message("background処理中です。完了まで待ってください");
                } else {
                    match action {
                        Some(event::KeyAction::Quit) => {
                            persist_tasks(app);
                            break;
                        }
                        Some(event::KeyAction::Edit) => {
                            let before = logging::task_snapshots(app.tabs());
                            edit_tasks(paths, terminal, app, editors)?;
                            log_task_changes(
                                logger,
                                app,
                                &before,
                                logging::TaskChangeCause::TaskFileRead,
                            );
                        }
                        _ => match reload_tasks(paths, app) {
                            Ok(()) => {
                                let before = logging::task_snapshots(app.tabs());
                                let cause = key_change_cause(action);
                                app.handle_key(key, keybindings);
                                log_task_changes(logger, app, &before, cause);
                            }
                            Err(err) => app.set_message(err),
                        },
                    }
                }
            }
            AppEvent::DayChanged => match reload_tasks(paths, app) {
                Ok(()) => {
                    let before = logging::task_snapshots(app.tabs());
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
                let before_tabs = app.tabs().to_vec();
                let before = logging::task_snapshots(app.tabs());
                match reload_tasks(paths, app) {
                    Ok(()) if tabs_differ(&before_tabs, app.tabs()) => {
                        app.set_message("tasks/をhot reloadしました");
                    }
                    Ok(()) => {}
                    Err(err) => app.set_message(err),
                }
                log_task_changes(logger, app, &before, logging::TaskChangeCause::TaskFileRead);
            }
            AppEvent::TerminalResized => {}
            AppEvent::StartupGitFinished(result) => {
                app.finish_background_work();
                match result {
                    Ok(message) => app.set_message(format!("起動時git snapshot: {message}")),
                    Err(err) => app.set_message(format!("起動時git snapshot: {err}")),
                }
            }
        }
        if should_persist {
            persist_tasks(app);
        }
        if should_draw {
            terminal.draw(|frame| ui::draw(frame, app, keybindings))?;
        }
    }

    Ok(())
}

fn event_should_persist(event: &AppEvent) -> bool {
    !matches!(
        event,
        AppEvent::TerminalResized | AppEvent::Tick | AppEvent::StartupGitFinished(_)
    )
}

fn reflect_task_file_status(
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

fn persist_tasks(app: &mut App) {
    let result = app.tabs().iter().try_for_each(|tab| {
        storage::write_task_file_status(&tab.path, app.current_date, &tab.tasks)
    });
    if let Err(err) = result {
        app.set_message(err);
    }
}

fn edit_tasks(
    paths: &storage::AppPaths,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    editors: &[String],
) -> Result<(), Box<dyn Error>> {
    let Some(path) = app.current_tab_path().map(Path::to_path_buf) else {
        app.set_message("allタブは編集できません");
        return Ok(());
    };
    let label = app.current_tab_label().to_string();

    TerminalGuard::suspend()?;
    let edit_result = open_with_configured_editor(&path, editors);
    TerminalGuard::resume()?;
    terminal.clear()?;

    match edit_result {
        Ok(editor) => match reload_tasks(paths, app) {
            Ok(()) => app.set_message(format!("{label}.txtを編集しました: {editor}")),
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
    let task_files = storage::load_task_files(&paths.tasks_dir)?;

    app.replace_tabs(task_lists_from_files(&task_files));
    for (index, task_file) in task_files.into_iter().enumerate() {
        reflect_task_file_status(task_file.status, app, index, false);
    }

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

fn key_change_cause(action: Option<event::KeyAction>) -> logging::TaskChangeCause {
    match action {
        Some(event::KeyAction::Advance) => logging::TaskChangeCause::KeyAdvance,
        Some(event::KeyAction::Hold) => logging::TaskChangeCause::KeyHold,
        Some(event::KeyAction::Defer) => logging::TaskChangeCause::KeyDefer,
        _ => logging::TaskChangeCause::KeyOther,
    }
}

fn log_task_changes(
    logger: &logging::AppLogger,
    app: &mut App,
    before: &[logging::TaskSnapshot],
    cause: logging::TaskChangeCause,
) {
    if let Err(err) = logger.log_task_changes(before, app.tabs(), cause) {
        app.set_message(err);
    }
}

fn task_lists_from_files(task_files: &[storage::TaskFile]) -> Vec<TaskList> {
    task_files
        .iter()
        .map(|task_file| TaskList {
            label: task_file.label.clone(),
            path: task_file.path.clone(),
            tasks: task_file.task.clone(),
        })
        .collect()
}

fn tabs_differ(before: &[TaskTab], after: &[TaskTab]) -> bool {
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
