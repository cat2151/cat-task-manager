use std::{env, error::Error, io, path::Path, sync::mpsc};

use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod cli;
mod clock;
mod editor;
mod event;
mod git_snapshot;
mod history_stats;
mod logging;
mod self_update;
mod startup_git;
mod storage;
mod task_diff;
mod terminal;
mod ui;

use app::{App, TaskList};
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

    let _terminal_guard = terminal::TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let (tx, rx) = mpsc::channel();
    event::spawn_event_threads(
        tx.clone(),
        paths.config_path.clone(),
        paths.tasks_dir.clone(),
    );
    if startup_git_enabled {
        app.start_background_work("起動時git snapshotを実行中です");
        git_snapshot::spawn_startup_snapshot(tx.clone(), paths.clone(), logger.clone());
    } else if app.start_history_stats_prefetch() {
        history_stats::spawn_history_stats(tx.clone(), paths.clone());
    }

    terminal.draw(|frame| ui::draw(frame, &app, &keybindings))?;
    run_event_loop(
        &paths,
        &mut terminal,
        &mut app,
        &mut keybindings,
        &mut editors,
        &logger,
        EventLoopRuntime {
            tx,
            rx,
            startup_git_enabled,
        },
    )?;

    Ok(())
}

struct EventLoopRuntime {
    tx: mpsc::Sender<AppEvent>,
    rx: mpsc::Receiver<AppEvent>,
    startup_git_enabled: bool,
}

fn run_event_loop(
    paths: &storage::AppPaths,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    keybindings: &mut event::KeyBindings,
    editors: &mut Vec<String>,
    logger: &logging::AppLogger,
    runtime: EventLoopRuntime,
) -> Result<(), Box<dyn Error>> {
    let mut pending_day_change = false;

    loop {
        let app_event = event::read_next_event(&runtime.rx)?;
        let mut should_persist = event_should_persist(&app_event);
        let mut should_draw = true;

        match app_event {
            AppEvent::Tick => {
                if app.has_background_work() {
                    app.tick_background_work();
                } else if app.is_history_stats_screen() && app.history_stats().is_loading() {
                    app.tick_history_stats();
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
                            if app.is_history_stats_screen() {
                                app.set_message("統計画面では編集できません");
                                should_persist = false;
                            } else {
                                let before = logging::task_snapshots(app.tabs());
                                edit_tasks(paths, terminal, app, editors)?;
                                log_task_changes(
                                    logger,
                                    app,
                                    &before,
                                    logging::TaskChangeCause::TaskFileRead,
                                );
                            }
                        }
                        Some(event::KeyAction::Stats) => {
                            if app.toggle_history_stats_screen() {
                                history_stats::spawn_history_stats(
                                    runtime.tx.clone(),
                                    paths.clone(),
                                );
                            }
                            should_persist = false;
                        }
                        _ if app.is_history_stats_screen() => {
                            app.handle_key(key, keybindings);
                            should_persist = false;
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
            AppEvent::DayChanged => {
                if app.has_background_work() {
                    pending_day_change = true;
                    should_persist = false;
                    app.set_background_message("background処理後に日付変更処理を実行します");
                } else {
                    handle_day_changed(
                        paths,
                        app,
                        logger,
                        &runtime.tx,
                        runtime.startup_git_enabled,
                        &mut should_persist,
                    );
                }
            }
            AppEvent::ConfigChanged => match reload_config(paths, keybindings, editors) {
                Ok(()) => app.set_message("設定をhot reloadしました"),
                Err(err) => app.set_message(err),
            },
            AppEvent::TasksChanged => {
                let before_tabs = app.tabs().to_vec();
                let before = logging::task_snapshots(app.tabs());
                match reload_tasks(paths, app) {
                    Ok(()) if task_diff::tabs_differ(&before_tabs, app.tabs()) => {
                        app.set_message("tasks/をhot reloadしました");
                    }
                    Ok(()) => {}
                    Err(err) => app.set_message(err),
                }
                log_task_changes(logger, app, &before, logging::TaskChangeCause::TaskFileRead);
            }
            AppEvent::TerminalResized => {}
            AppEvent::BackgroundWorkMessage(message) => {
                app.set_background_message(message);
            }
            AppEvent::StartupGitFinished(result) => {
                app.finish_background_work();
                match result {
                    Ok(message) => app.set_message(format!("起動時git snapshot: {message}")),
                    Err(err) => app.set_message(format!("起動時git snapshot: {err}")),
                }
                if pending_day_change {
                    pending_day_change = false;
                    handle_day_changed(
                        paths,
                        app,
                        logger,
                        &runtime.tx,
                        runtime.startup_git_enabled,
                        &mut should_persist,
                    );
                }
                if !app.has_background_work() && app.start_history_stats_prefetch() {
                    history_stats::spawn_history_stats(runtime.tx.clone(), paths.clone());
                }
            }
            AppEvent::DayChangeGitFinished(result) => {
                pending_day_change = false;
                app.finish_background_work();
                match result {
                    Ok(message) => finish_day_change(app, logger, Some(&message)),
                    Err(err) => app.set_message(format!("日付変更前git snapshot: {err}")),
                }
            }
            AppEvent::HistoryStatsFinished(result) => {
                should_persist = false;
                app.finish_history_stats(result);
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
        AppEvent::TerminalResized
            | AppEvent::Tick
            | AppEvent::BackgroundWorkMessage(_)
            | AppEvent::StartupGitFinished(_)
            | AppEvent::HistoryStatsFinished(_)
    )
}

fn handle_day_changed(
    paths: &storage::AppPaths,
    app: &mut App,
    logger: &logging::AppLogger,
    tx: &mpsc::Sender<AppEvent>,
    startup_git_enabled: bool,
    should_persist: &mut bool,
) {
    match reload_tasks(paths, app) {
        Ok(()) if startup_git_enabled => {
            persist_tasks(app);
            *should_persist = false;
            app.start_background_work("日付変更前git snapshotを実行中です");
            git_snapshot::spawn_before_day_change_snapshot(
                tx.clone(),
                paths.clone(),
                logger.clone(),
                app.current_date,
            );
        }
        Ok(()) => finish_day_change(app, logger, None),
        Err(err) => app.set_message(err),
    }
}

fn finish_day_change(app: &mut App, logger: &logging::AppLogger, snapshot_message: Option<&str>) {
    let before = logging::task_snapshots(app.tabs());
    app.complete_day(clock::today_jst());
    log_task_changes(logger, app, &before, logging::TaskChangeCause::DayChanged);
    match snapshot_message {
        Some(message) => app.set_message(format!(
            "日付変更前git snapshot: {message}。tasksを翌日化しました"
        )),
        None => app.set_message("tasksを翌日化しました"),
    }
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

    terminal::TerminalGuard::suspend()?;
    let edit_result = editor::open_with_configured_editor(&path, editors);
    terminal::TerminalGuard::resume()?;
    terminal.clear()?;

    match edit_result {
        Ok(editor) => match reload_tasks(paths, app) {
            Ok(()) => app.set_message(format!("{label}.mdを編集しました: {editor}")),
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
