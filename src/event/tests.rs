use super::*;

#[test]
fn parses_character_binding() {
    let binding = parse_binding("j").unwrap();
    assert!(binding.matches(&KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty())));
    assert!(!binding.matches(&KeyEvent::new(KeyCode::Char('J'), KeyModifiers::empty())));
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
fn parses_uppercase_character_as_shifted_key() {
    let binding = parse_binding("J").unwrap();

    assert!(binding.matches(&KeyEvent::new(KeyCode::Char('J'), KeyModifiers::empty())));
    assert!(binding.matches(&KeyEvent::new(KeyCode::Char('j'), KeyModifiers::SHIFT)));
    assert!(!binding.matches(&KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty())));
}

#[test]
fn keybindings_map_multiple_keys_to_one_action() {
    let keybindings = KeyBindings::from_config(KeyBindingsConfig::from_pairs([
        ("j", "next"),
        ("down", "next"),
        ("k", "previous"),
        ("enter", "advance"),
        ("space", "advance"),
        ("p", "hold"),
        ("d", "defer"),
        ("q", "quit"),
        ("e", "edit"),
        ("l", "next_tab"),
        ("h", "previous_tab"),
        ("v", "toggle_view"),
        ("?", "help"),
    ]))
    .unwrap();

    assert_eq!(
        keybindings.action_for(&KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty())),
        Some(KeyAction::Next)
    );
    assert_eq!(
        keybindings.action_for(&KeyEvent::new(KeyCode::Down, KeyModifiers::empty())),
        Some(KeyAction::Next)
    );
    assert_eq!(
        keybindings.action_for(&KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty())),
        Some(KeyAction::Advance)
    );
    assert_eq!(
        keybindings.action_for(&KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty())),
        Some(KeyAction::Defer)
    );
}

#[test]
fn duplicate_normalized_keybindings_are_rejected() {
    let err = KeyBindings::from_config(KeyBindingsConfig::from_pairs([
        ("j", "next"),
        ("J", "previous"),
        ("shift+j", "advance"),
        ("p", "hold"),
        ("q", "quit"),
        ("e", "edit"),
        ("l", "next_tab"),
        ("h", "previous_tab"),
        ("v", "toggle_view"),
        ("?", "help"),
    ]))
    .unwrap_err();

    assert!(err.contains("重複"));
}

#[test]
fn unknown_keybinding_action_is_rejected() {
    let err = KeyBindings::from_config(KeyBindingsConfig::from_pairs([
        ("j", "nnext"),
        ("k", "previous"),
        ("enter", "advance"),
        ("p", "hold"),
        ("q", "quit"),
        ("e", "edit"),
        ("l", "next_tab"),
        ("h", "previous_tab"),
        ("v", "toggle_view"),
        ("?", "help"),
    ]))
    .unwrap_err();

    assert!(err.contains("未対応の keybinding action"));
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
        Some(
            AppEvent::TerminalResized
            | AppEvent::Tick
            | AppEvent::DayChanged
            | AppEvent::ConfigChanged
            | AppEvent::TasksChanged
            | AppEvent::BackgroundWorkMessage(_)
            | AppEvent::StartupGitFinished(_)
            | AppEvent::DayChangeGitFinished(_),
        )
        | None => panic!("expected key press event"),
    }
}

#[test]
fn terminal_resize_becomes_app_resize_event() {
    assert!(matches!(
        app_event_from_terminal_event(Event::Resize(80, 24)),
        Some(AppEvent::TerminalResized)
    ));
}

#[test]
fn terminal_non_press_keys_are_ignored() {
    for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
        let key = KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::empty(), kind);

        assert!(app_event_from_terminal_event(Event::Key(key)).is_none());
    }
}
