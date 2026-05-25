use crossterm::event::{
    self as crossterm_event, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender, TryRecvError},
    thread,
    time::{Duration as StdDuration, SystemTime},
};

use crate::{clock, storage::KeyBindingsConfig};

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    TerminalResized,
    Tick,
    DayChanged,
    ConfigChanged,
    TasksChanged,
    StartupGitFinished(Result<String, String>),
}

#[derive(Debug, Clone)]
pub struct KeyBindings {
    bindings: Vec<(KeyBinding, KeyAction)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Next,
    Previous,
    Advance,
    Hold,
    Quit,
    Edit,
    NextTab,
    PreviousTab,
    ToggleView,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    label: String,
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyBindings {
    pub fn from_config(config: KeyBindingsConfig) -> Result<Self, String> {
        let mut bindings: Vec<(KeyBinding, KeyAction)> = Vec::new();

        for (raw_key, raw_action) in config {
            let binding = parse_binding(&raw_key)?;
            if let Some((existing, _)) = bindings
                .iter()
                .find(|(existing, _)| existing.same_key_as(&binding))
            {
                return Err(format!(
                    "keybinding が重複しています: '{}' と '{}'",
                    existing.label(),
                    binding.label()
                ));
            }

            bindings.push((binding, KeyAction::parse(&raw_action)?));
        }

        for action in KeyAction::ALL {
            if !bindings
                .iter()
                .any(|(_, bound_action)| *bound_action == action)
            {
                return Err(format!(
                    "keybindings に action '{}' がありません",
                    action.config_name()
                ));
            }
        }

        Ok(Self { bindings })
    }

    pub fn action_for(&self, key: &KeyEvent) -> Option<KeyAction> {
        self.bindings
            .iter()
            .find(|(binding, _)| binding.matches(key))
            .map(|(_, action)| *action)
    }

    pub fn label_for(&self, action: KeyAction) -> String {
        let labels = self
            .bindings
            .iter()
            .filter(|(_, bound_action)| *bound_action == action)
            .map(|(binding, _)| binding.label().to_string())
            .collect::<Vec<_>>();

        labels.join("/")
    }
}

impl KeyAction {
    pub const ALL: [Self; 10] = [
        Self::Next,
        Self::Previous,
        Self::Advance,
        Self::Hold,
        Self::Quit,
        Self::Edit,
        Self::NextTab,
        Self::PreviousTab,
        Self::ToggleView,
        Self::Help,
    ];

    fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim() {
            "next" => Ok(Self::Next),
            "previous" => Ok(Self::Previous),
            "advance" => Ok(Self::Advance),
            "hold" => Ok(Self::Hold),
            "quit" => Ok(Self::Quit),
            "edit" => Ok(Self::Edit),
            "next_tab" => Ok(Self::NextTab),
            "previous_tab" => Ok(Self::PreviousTab),
            "toggle_view" => Ok(Self::ToggleView),
            "help" => Ok(Self::Help),
            _ => Err(format!(
                "未対応の keybinding action です: '{raw}'。next、previous、advance、hold、quit、edit、next_tab、previous_tab、toggle_view、help を使ってください。"
            )),
        }
    }

    fn config_name(self) -> &'static str {
        match self {
            Self::Next => "next",
            Self::Previous => "previous",
            Self::Advance => "advance",
            Self::Hold => "hold",
            Self::Quit => "quit",
            Self::Edit => "edit",
            Self::NextTab => "next_tab",
            Self::PreviousTab => "previous_tab",
            Self::ToggleView => "toggle_view",
            Self::Help => "help",
        }
    }
}

impl KeyBinding {
    pub fn matches(&self, key: &KeyEvent) -> bool {
        Self::from_key_event(key).is_some_and(|pressed| self.same_key_as(&pressed))
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    fn from_key_event(key: &KeyEvent) -> Option<Self> {
        if key.kind != KeyEventKind::Press {
            return None;
        }

        let (code, modifiers) = normalize_key(key.code, key.modifiers);
        Some(Self {
            label: String::new(),
            code,
            modifiers,
        })
    }

    fn same_key_as(&self, other: &Self) -> bool {
        self.code == other.code && self.modifiers == other.modifiers
    }
}

pub fn spawn_event_threads(tx: Sender<AppEvent>, config_path: PathBuf, tasks_dir: PathBuf) {
    spawn_day_change_thread(tx.clone());
    spawn_file_change_thread(tx.clone(), config_path, AppEvent::ConfigChanged);
    spawn_tasks_change_thread(tx, tasks_dir);
}

pub fn read_next_event(rx: &Receiver<AppEvent>) -> Result<AppEvent, String> {
    loop {
        match rx.try_recv() {
            Ok(event) => return Ok(event),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                return Err("event channel が切断されました".to_string());
            }
        }

        if crossterm_event::poll(StdDuration::from_millis(50))
            .map_err(|err| format!("terminal event の poll に失敗しました: {err}"))?
        {
            let event = crossterm_event::read()
                .map_err(|err| format!("terminal event を読めませんでした: {err}"))?;
            if let Some(event) = app_event_from_terminal_event(event) {
                return Ok(event);
            }
        } else {
            return Ok(AppEvent::Tick);
        }
    }
}

fn app_event_from_terminal_event(event: Event) -> Option<AppEvent> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => Some(AppEvent::Key(key)),
        Event::Resize(_, _) => Some(AppEvent::TerminalResized),
        _ => None,
    }
}

fn spawn_day_change_thread(tx: Sender<AppEvent>) {
    thread::spawn(move || loop {
        thread::sleep(duration_until_next_midnight());
        if tx.send(AppEvent::DayChanged).is_err() {
            break;
        }
    });
}

fn duration_until_next_midnight() -> StdDuration {
    clock::duration_until_next_jst_midnight()
}

fn spawn_file_change_thread(tx: Sender<AppEvent>, path: PathBuf, event: AppEvent) {
    thread::spawn(move || {
        let mut last_signature = FileSignature::read(&path);

        loop {
            thread::sleep(StdDuration::from_millis(500));
            let current_signature = FileSignature::read(&path);
            if current_signature == last_signature {
                continue;
            }

            last_signature = current_signature;
            if tx.send(event.clone()).is_err() {
                break;
            }
        }
    });
}

fn spawn_tasks_change_thread(tx: Sender<AppEvent>, path: PathBuf) {
    thread::spawn(move || {
        let mut last_signature = TaskDirSignature::read(&path);

        loop {
            thread::sleep(StdDuration::from_millis(500));
            let current_signature = TaskDirSignature::read(&path);
            if current_signature == last_signature {
                continue;
            }

            last_signature = current_signature;
            if tx.send(AppEvent::TasksChanged).is_err() {
                break;
            }
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileSignature {
    modified: Option<SystemTime>,
    len: u64,
}

impl FileSignature {
    fn read(path: &Path) -> Option<Self> {
        fs::metadata(path).ok().map(|metadata| Self {
            modified: metadata.modified().ok(),
            len: metadata.len(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskDirSignature {
    files: Vec<(PathBuf, FileSignature)>,
}

impl TaskDirSignature {
    fn read(path: &Path) -> Option<Self> {
        let entries = fs::read_dir(path).ok()?;
        let mut files = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("txt") {
                continue;
            }
            let signature = FileSignature::read(&path)?;
            files.push((path, signature));
        }

        files.sort_by(|left, right| left.0.cmp(&right.0));
        Some(Self { files })
    }
}

fn parse_binding(raw: &str) -> Result<KeyBinding, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("keybinding を空にはできません".to_string());
    }

    let mut modifiers = KeyModifiers::empty();
    let mut key_part = trimmed;

    loop {
        if let Some(rest) = strip_modifier(key_part, "ctrl+") {
            modifiers |= KeyModifiers::CONTROL;
            key_part = rest;
        } else if let Some(rest) = strip_modifier(key_part, "alt+") {
            modifiers |= KeyModifiers::ALT;
            key_part = rest;
        } else if let Some(rest) = strip_modifier(key_part, "shift+") {
            modifiers |= KeyModifiers::SHIFT;
            key_part = rest;
        } else {
            break;
        }
    }

    let normalized_key_part = key_part.to_ascii_lowercase();
    let code = match normalized_key_part.as_str() {
        "enter" => KeyCode::Enter,
        "space" => KeyCode::Char(' '),
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        _ if key_part.chars().count() == 1 => KeyCode::Char(key_part.chars().next().expect("one char")),
        _ => {
            return Err(format!(
                "未対応の keybinding です: '{raw}'。1文字、enter、space、tab、矢印、ctrl+<key> を使ってください。"
            ))
        }
    };
    let (code, modifiers) = normalize_key(code, modifiers);

    Ok(KeyBinding {
        label: trimmed.to_string(),
        code,
        modifiers,
    })
}

fn strip_modifier<'a>(raw: &'a str, prefix: &str) -> Option<&'a str> {
    raw.get(..prefix.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
        .then(|| &raw[prefix.len()..])
}

fn normalize_key(code: KeyCode, mut modifiers: KeyModifiers) -> (KeyCode, KeyModifiers) {
    let code = match code {
        KeyCode::Char(character) if character.is_ascii_uppercase() => {
            modifiers |= KeyModifiers::SHIFT;
            KeyCode::Char(character.to_ascii_lowercase())
        }
        KeyCode::Char(character) if !character.is_ascii_alphabetic() => {
            modifiers.remove(KeyModifiers::SHIFT);
            KeyCode::Char(character)
        }
        KeyCode::BackTab => {
            modifiers.remove(KeyModifiers::SHIFT);
            KeyCode::BackTab
        }
        other => other,
    };

    (code, modifiers)
}

#[cfg(test)]
mod tests;
