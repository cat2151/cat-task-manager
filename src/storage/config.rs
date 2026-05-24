use std::{collections::HashSet, fs, path::Path};

use serde::Deserialize;

const DEFAULT_EDITORS: [&str; 4] = ["fresh", "zed", "nvim", "code"];
const DEFAULT_KEYBINDINGS: [(&str, &str); 6] = [
    ("edit", "e"),
    ("hold", "p"),
    ("next_tab", "l"),
    ("previous_tab", "h"),
    ("toggle_view", "v"),
    ("help", "?"),
];
const OLD_DEFAULT_KEYBINDINGS: [(&str, &str, &str); 3] = [
    ("hold", "h", "p"),
    ("next_tab", "tab", "l"),
    ("previous_tab", "backtab", "h"),
];
const DEFAULT_CONFIG: &str = r#"editors = ["fresh", "zed", "nvim", "code"]

[keybindings]
next = "j"
previous = "k"
advance = "enter"
hold = "p"
quit = "q"
edit = "e"
next_tab = "l"
previous_tab = "h"
toggle_view = "v"
help = "?"
"#;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyBindingsConfig {
    pub next: Option<String>,
    pub previous: Option<String>,
    pub advance: Option<String>,
    pub hold: Option<String>,
    pub quit: Option<String>,
    pub edit: Option<String>,
    pub next_tab: Option<String>,
    pub previous_tab: Option<String>,
    pub toggle_view: Option<String>,
    pub help: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub keybindings: KeyBindingsConfig,
    pub editors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfigFile {
    #[serde(default)]
    keybindings: KeyBindingsConfig,
    #[serde(default = "default_editors")]
    editors: Vec<String>,
}

pub(super) fn ensure_config_file(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::write(path, DEFAULT_CONFIG).map_err(|err| {
            format!(
                "config fileを書き込めませんでした: {} ({err})",
                path.display()
            )
        })?;
    } else {
        ensure_config_defaults(path)?;
    }

    Ok(())
}

pub fn load_config_file(path: impl AsRef<Path>) -> Result<ConfigFile, String> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("config fileを読めませんでした: {} ({err})", path.display()))?;
    let file: RawConfigFile = toml::from_str(&raw).map_err(|err| {
        format!(
            "config fileをTOMLとして読めませんでした: {} ({err})",
            path.display()
        )
    })?;

    Ok(ConfigFile {
        keybindings: file.keybindings,
        editors: normalize_editors(file.editors),
    })
}

pub fn normalize_editors(editors: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = editors
        .into_iter()
        .filter_map(|editor| {
            let editor = editor.trim().to_string();
            if editor.is_empty() || !seen.insert(editor.to_lowercase()) {
                None
            } else {
                Some(editor)
            }
        })
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        normalized = default_editors();
    }

    normalized
}

fn default_editors() -> Vec<String> {
    DEFAULT_EDITORS
        .iter()
        .map(|editor| (*editor).to_string())
        .collect()
}

fn ensure_config_defaults(path: &Path) -> Result<(), String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("config fileを読めませんでした: {} ({err})", path.display()))?;
    let mut value: toml::Value = toml::from_str(&raw).map_err(|err| {
        format!(
            "config fileをTOMLとして読めませんでした: {} ({err})",
            path.display()
        )
    })?;

    let mut changed = false;
    let table = value
        .as_table_mut()
        .ok_or_else(|| format!("config fileのrootがtableではありません: {}", path.display()))?;

    if !matches!(table.get("editors"), Some(toml::Value::Array(_))) {
        table.insert(
            "editors".to_string(),
            toml::Value::Array(
                default_editors()
                    .into_iter()
                    .map(toml::Value::String)
                    .collect(),
            ),
        );
        changed = true;
    }

    if !table.contains_key("keybindings") {
        table.insert(
            "keybindings".to_string(),
            toml::Value::Table(toml::Table::new()),
        );
        changed = true;
    }

    let keybindings = table
        .get_mut("keybindings")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| {
            format!(
                "config fileのkeybindingsがtableではありません: {}",
                path.display()
            )
        })?;

    for (key, default) in DEFAULT_KEYBINDINGS {
        if !keybindings.contains_key(key) {
            keybindings.insert(key.to_string(), toml::Value::String(default.to_string()));
            changed = true;
        }
    }

    for (key, old_default, new_default) in OLD_DEFAULT_KEYBINDINGS {
        if keybindings.get(key).and_then(toml::Value::as_str) == Some(old_default) {
            keybindings.insert(
                key.to_string(),
                toml::Value::String(new_default.to_string()),
            );
            changed = true;
        }
    }

    if changed {
        let updated = toml::to_string_pretty(&value).map_err(|err| {
            format!(
                "config fileをTOMLとして書き出せませんでした: {} ({err})",
                path.display()
            )
        })?;
        fs::write(path, updated).map_err(|err| {
            format!(
                "config fileを書き込めませんでした: {} ({err})",
                path.display()
            )
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
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
        let raw = fs::read_to_string(path).unwrap();
        let value: toml::Value = toml::from_str(&raw).unwrap();
        value
            .get("keybindings")
            .and_then(|table| table.get(key))
            .and_then(toml::Value::as_str)
            .unwrap()
            .to_string()
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
    fn config_defaults_are_inserted_into_keybindings_table() {
        let path = temp_config_path("insert-keybindings");
        fs::write(
            &path,
            "editors = [\"nvim\"]\n\n[keybindings]\nnext = \"j\"\n",
        )
        .unwrap();

        ensure_config_defaults(&path).unwrap();

        assert_eq!(keybinding(&path, "next"), "j");
        assert_eq!(keybinding(&path, "hold"), "p");
        assert_eq!(keybinding(&path, "next_tab"), "l");
        assert_eq!(keybinding(&path, "previous_tab"), "h");

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn config_defaults_add_keybindings_table() {
        let path = temp_config_path("add-keybindings");
        fs::write(&path, "editors = [\"nvim\"]\n").unwrap();

        ensure_config_defaults(&path).unwrap();

        assert_eq!(keybinding(&path, "help"), "?");
        assert_eq!(keybinding(&path, "next_tab"), "l");

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn old_default_tab_keys_are_migrated_to_h_l() {
        let path = temp_config_path("migrate-tab-keys");
        fs::write(
            &path,
            "editors = [\"nvim\"]\n\n[keybindings]\nhold = \"h\"\nnext_tab = \"tab\"\nprevious_tab = \"backtab\"\n",
        )
        .unwrap();

        ensure_config_defaults(&path).unwrap();

        assert_eq!(keybinding(&path, "hold"), "p");
        assert_eq!(keybinding(&path, "next_tab"), "l");
        assert_eq!(keybinding(&path, "previous_tab"), "h");

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn default_config_is_config_only() {
        let file: RawConfigFile = toml::from_str(DEFAULT_CONFIG).unwrap();

        assert_eq!(file.editors, default_editors());
        assert_eq!(file.keybindings.hold.as_deref(), Some("p"));
        assert_eq!(file.keybindings.next_tab.as_deref(), Some("l"));
        assert_eq!(file.keybindings.previous_tab.as_deref(), Some("h"));
    }

    #[test]
    fn tasks_in_config_are_rejected() {
        let raw = "tasks = '''\na\n'''\n";

        assert!(toml::from_str::<RawConfigFile>(raw).is_err());
    }
}
