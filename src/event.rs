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
    DayChanged,
    ConfigChanged,
    TasksChanged,
}

#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub next: KeyBinding,
    pub previous: KeyBinding,
    pub advance: KeyBinding,
    pub hold: KeyBinding,
    pub quit: KeyBinding,
    pub edit: KeyBinding,
    pub toggle_view: KeyBinding,
    pub help: KeyBinding,
}

#[derive(Debug, Clone)]
pub struct KeyBinding {
    label: String,
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyBindings {
    pub fn from_config(config: KeyBindingsConfig) -> Result<Self, String> {
        Ok(Self {
            next: parse_or_default(config.next, "j")?,
            previous: parse_or_default(config.previous, "k")?,
            advance: parse_or_default(config.advance, "enter")?,
            hold: parse_or_default(config.hold, "h")?,
            quit: parse_or_default(config.quit, "q")?,
            edit: parse_or_default(config.edit, "e")?,
            toggle_view: parse_or_default(config.toggle_view, "v")?,
            help: parse_or_default(config.help, "?")?,
        })
    }
}

impl KeyBinding {
    pub fn matches(&self, key: &KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }

        if key.code != self.code {
            return false;
        }

        key.modifiers == self.modifiers
            || (matches!(self.code, KeyCode::Char(_))
                && self.modifiers.is_empty()
                && key.modifiers == KeyModifiers::SHIFT)
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

pub fn spawn_event_threads(tx: Sender<AppEvent>, config_path: PathBuf, tasks_path: PathBuf) {
    spawn_day_change_thread(tx.clone());
    spawn_file_change_thread(tx.clone(), config_path, AppEvent::ConfigChanged);
    spawn_file_change_thread(tx, tasks_path, AppEvent::TasksChanged);
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
        }
    }
}

fn app_event_from_terminal_event(event: Event) -> Option<AppEvent> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => Some(AppEvent::Key(key)),
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

fn parse_or_default(configured: Option<String>, default: &str) -> Result<KeyBinding, String> {
    parse_binding(configured.as_deref().unwrap_or(default))
}

fn parse_binding(raw: &str) -> Result<KeyBinding, String> {
    let normalized = raw.trim().to_lowercase();
    if normalized.is_empty() {
        return Err("keybinding を空にはできません".to_string());
    }

    let mut modifiers = KeyModifiers::empty();
    let mut key_part = normalized.as_str();

    if let Some(rest) = key_part.strip_prefix("ctrl+") {
        modifiers |= KeyModifiers::CONTROL;
        key_part = rest;
    } else if let Some(rest) = key_part.strip_prefix("alt+") {
        modifiers |= KeyModifiers::ALT;
        key_part = rest;
    } else if let Some(rest) = key_part.strip_prefix("shift+") {
        modifiers |= KeyModifiers::SHIFT;
        key_part = rest;
    }

    let code = match key_part {
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
        char_key if char_key.chars().count() == 1 => {
            KeyCode::Char(char_key.chars().next().expect("one char"))
        }
        _ => {
            return Err(format!(
                "未対応の keybinding です: '{raw}'。1文字、enter、space、tab、矢印、ctrl+<key> を使ってください。"
            ))
        }
    };

    Ok(KeyBinding {
        label: raw.trim().to_string(),
        code,
        modifiers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_character_binding() {
        let binding = parse_binding("j").unwrap();
        assert!(binding.matches(&KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty())));
    }

    #[test]
    fn parses_control_binding() {
        let binding = parse_binding("ctrl+c").unwrap();
        assert!(binding.matches(&KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
    }

    #[test]
    fn parses_shifted_printable_character_binding() {
        let binding = parse_binding("?").unwrap();
        assert!(binding.matches(&KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT)));
    }

    #[test]
    fn binding_does_not_match_key_release() {
        let binding = parse_binding("enter").unwrap();

        assert!(!binding.matches(&KeyEvent::new_with_kind(
            KeyCode::Enter,
            KeyModifiers::empty(),
            KeyEventKind::Release,
        )));
    }

    #[test]
    fn terminal_key_press_becomes_app_key_event() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());

        match app_event_from_terminal_event(Event::Key(key)) {
            Some(AppEvent::Key(received)) => assert_eq!(received, key),
            Some(AppEvent::DayChanged | AppEvent::ConfigChanged | AppEvent::TasksChanged)
            | None => {
                panic!("expected key press event")
            }
        }
    }

    #[test]
    fn terminal_non_press_keys_are_ignored() {
        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            let key = KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::empty(), kind);

            assert!(app_event_from_terminal_event(Event::Key(key)).is_none());
        }
    }
}
