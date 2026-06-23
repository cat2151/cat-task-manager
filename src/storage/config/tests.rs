use super::*;
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_config_path(test_name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("cat-task-manager-{test_name}-{suffix}.toml"))
}

fn keybinding(path: &Path, key: &str) -> String {
    optional_keybinding(path, key).unwrap()
}

fn optional_keybinding(path: &Path, key: &str) -> Option<String> {
    let raw = fs::read_to_string(path).unwrap();
    let value: toml::Value = toml::from_str(&raw).unwrap();
    value
        .get("keybindings")
        .and_then(|table| table.get(key))
        .and_then(toml::Value::as_str)
        .map(str::to_string)
}

fn estimate_blink_value(path: &Path, key: &str) -> Option<toml::Value> {
    let raw = fs::read_to_string(path).unwrap();
    let value: toml::Value = toml::from_str(&raw).unwrap();
    value
        .get("ui")
        .and_then(|table| table.get("estimate_blink"))
        .and_then(|table| table.get(key))
        .cloned()
}

fn auto_free_time_value(path: &Path, key: &str) -> Option<toml::Value> {
    let raw = fs::read_to_string(path).unwrap();
    let value: toml::Value = toml::from_str(&raw).unwrap();
    value
        .get("auto_free_time")
        .and_then(|table| table.get(key))
        .cloned()
}

#[test]
fn normalize_editors_uses_default_when_empty() {
    assert_eq!(normalize_editors(vec![]), default_editors());
    assert_eq!(
        normalize_editors(vec![" ".to_string(), "\t".to_string()]),
        default_editors()
    );
}

#[test]
fn normalize_editors_trims_and_deduplicates() {
    assert_eq!(
        normalize_editors(vec![
            " nvim ".to_string(),
            "NVIM".to_string(),
            "code".to_string()
        ]),
        vec!["nvim".to_string(), "code".to_string()]
    );
}

#[test]
fn config_defaults_preserve_existing_keybindings_and_add_missing_actions() {
    let path = temp_config_path("preserve-keybindings");
    fs::write(
        &path,
        "editors = [\"nvim\"]\n\n[keybindings]\nj = \"next\"\nx = \"previous\"\n",
    )
    .unwrap();

    ensure_config_defaults(&path).unwrap();

    assert_eq!(keybinding(&path, "j"), "next");
    assert_eq!(keybinding(&path, "x"), "previous");
    assert_eq!(optional_keybinding(&path, "down"), None);
    assert_eq!(optional_keybinding(&path, "k"), None);
    assert_eq!(keybinding(&path, "enter"), "advance");
    assert_eq!(keybinding(&path, "d"), "defer");
    assert_eq!(keybinding(&path, "f"), "free_time");
    assert_eq!(keybinding(&path, "s"), "stats");

    fs::remove_file(path).unwrap();
}

#[test]
fn config_defaults_add_keybindings_table() {
    let path = temp_config_path("add-keybindings");
    fs::write(&path, "editors = [\"nvim\"]\n").unwrap();

    ensure_config_defaults(&path).unwrap();

    assert_eq!(keybinding(&path, "j"), "next");
    assert_eq!(keybinding(&path, "d"), "defer");
    assert_eq!(keybinding(&path, "f"), "free_time");
    assert_eq!(keybinding(&path, "right"), "next_tab");
    assert_eq!(keybinding(&path, "s"), "stats");
    assert_eq!(keybinding(&path, "?"), "help");

    fs::remove_file(path).unwrap();
}

#[test]
fn config_defaults_add_startup_git_table() {
    let path = temp_config_path("add-startup-git");
    fs::write(&path, "editors = [\"nvim\"]\n").unwrap();

    ensure_config_defaults(&path).unwrap();

    let raw = fs::read_to_string(&path).unwrap();
    let value: toml::Value = toml::from_str(&raw).unwrap();
    assert_eq!(
        value
            .get("startup_git")
            .and_then(|table| table.get("auto_commit_and_push"))
            .and_then(toml::Value::as_bool),
        Some(false)
    );

    fs::remove_file(path).unwrap();
}

#[test]
fn config_defaults_add_disabled_auto_free_time_table() {
    let path = temp_config_path("add-auto-free-time");
    fs::write(&path, "editors = [\"nvim\"]\n").unwrap();

    ensure_config_defaults(&path).unwrap();

    assert_eq!(
        auto_free_time_value(&path, "enabled").and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        auto_free_time_value(&path, "idle_seconds").and_then(|value| value.as_integer()),
        Some(60)
    );
    assert_eq!(
        auto_free_time_value(&path, "active_hours")
            .and_then(|value| value.as_str().map(str::to_string)),
        Some("09:00-17:00".to_string())
    );

    fs::remove_file(path).unwrap();
}

#[test]
fn config_defaults_add_ui_estimate_blink_table() {
    let path = temp_config_path("add-ui-estimate-blink");
    fs::write(&path, "editors = [\"nvim\"]\n").unwrap();

    ensure_config_defaults(&path).unwrap();

    assert_eq!(
        estimate_blink_value(&path, "enabled").and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        estimate_blink_value(&path, "foreground")
            .and_then(|value| value.as_str().map(str::to_string)),
        Some("green".to_string())
    );
    assert_eq!(
        estimate_blink_value(&path, "background")
            .and_then(|value| value.as_str().map(str::to_string)),
        Some("bg".to_string())
    );

    fs::remove_file(path).unwrap();
}

#[test]
fn default_config_is_config_only() {
    let file: RawConfigFile = toml::from_str(DEFAULT_CONFIG).unwrap();

    assert_eq!(file.editors, default_editors());
    assert!(!file.startup_git.auto_commit_and_push);
    assert!(!file.auto_free_time.enabled);
    assert_eq!(file.auto_free_time.idle_seconds, 60);
    assert!(file
        .auto_free_time
        .is_active_at(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap()));
    assert!(file.ui.estimate_blink.enabled);
    assert_eq!(file.ui.estimate_blink.foreground, MonokaiColorName::Green);
    assert_eq!(file.ui.estimate_blink.background, MonokaiColorName::Bg);
    assert_eq!(file.keybindings.get("j"), Some("next"));
    assert_eq!(file.keybindings.get("down"), Some("next"));
    assert_eq!(file.keybindings.get("space"), Some("advance"));
    assert_eq!(file.keybindings.get("d"), Some("defer"));
    assert_eq!(file.keybindings.get("f"), Some("free_time"));
    assert_eq!(file.keybindings.get("right"), Some("next_tab"));
    assert_eq!(file.keybindings.get("s"), Some("stats"));
    assert_eq!(file.keybindings.get("?"), Some("help"));
}

#[test]
fn startup_git_auto_commit_and_push_is_read_from_config() {
    let raw = r#"editors = ["nvim"]

[startup_git]
auto_commit_and_push = true
"#;

    let file: RawConfigFile = toml::from_str(raw).unwrap();

    assert!(file.startup_git.auto_commit_and_push);
}

#[test]
fn ui_estimate_blink_defaults_to_on_when_missing_from_config() {
    let raw = r#"editors = ["nvim"]"#;

    let file: RawConfigFile = toml::from_str(raw).unwrap();

    assert!(file.ui.estimate_blink.enabled);
    assert_eq!(file.ui.estimate_blink.foreground, MonokaiColorName::Green);
    assert_eq!(file.ui.estimate_blink.background, MonokaiColorName::Bg);
}

#[test]
fn ui_estimate_blink_reads_monokai_color_names() {
    let raw = r#"editors = ["nvim"]

[ui.estimate_blink]
enabled = false
foreground = "yellow"
background = "selection"
"#;

    let file: RawConfigFile = toml::from_str(raw).unwrap();

    assert!(!file.ui.estimate_blink.enabled);
    assert_eq!(file.ui.estimate_blink.foreground, MonokaiColorName::Yellow);
    assert_eq!(
        file.ui.estimate_blink.background,
        MonokaiColorName::Selection
    );
}

#[test]
fn ui_estimate_blink_rejects_unknown_monokai_color_name() {
    let raw = r#"editors = ["nvim"]

[ui.estimate_blink]
foreground = "cyan"
"#;

    assert!(toml::from_str::<RawConfigFile>(raw).is_err());
}

#[test]
fn tasks_in_config_are_rejected() {
    let raw = "tasks = '''\na\n'''\n";

    assert!(toml::from_str::<RawConfigFile>(raw).is_err());
}
