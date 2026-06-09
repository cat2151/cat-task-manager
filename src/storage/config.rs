use std::{collections::HashSet, fs, path::Path};

use serde::{de::Error as _, Deserialize, Deserializer};

mod ui;
pub use ui::{MonokaiColorName, UiConfig};

const DEFAULT_EDITORS: [&str; 4] = ["fresh", "zed", "nvim", "code"];
const DEFAULT_KEYBINDINGS: [(&str, &str); 18] = [
    ("j", "next"),
    ("down", "next"),
    ("k", "previous"),
    ("up", "previous"),
    ("enter", "advance"),
    ("space", "advance"),
    ("p", "hold"),
    ("d", "defer"),
    ("f", "free_time"),
    ("q", "quit"),
    ("e", "edit"),
    ("l", "next_tab"),
    ("right", "next_tab"),
    ("h", "previous_tab"),
    ("left", "previous_tab"),
    ("v", "toggle_view"),
    ("s", "stats"),
    ("?", "help"),
];
const DEFAULT_CONFIG: &str = r#"editors = ["fresh", "zed", "nvim", "code"]

[startup_git]
auto_commit_and_push = false

[ui.estimate_blink]
enabled = true
foreground = "green"
background = "bg"

[keybindings]
j = "next"
down = "next"
k = "previous"
up = "previous"
enter = "advance"
space = "advance"
p = "hold"
d = "defer"
f = "free_time"
q = "quit"
e = "edit"
l = "next_tab"
right = "next_tab"
h = "previous_tab"
left = "previous_tab"
v = "toggle_view"
s = "stats"
"?" = "help"
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBindingsConfig {
    bindings: Vec<(String, String)>,
}

impl KeyBindingsConfig {
    pub fn from_pairs(
        pairs: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self {
            bindings: pairs
                .into_iter()
                .map(|(key, action)| (key.into(), action.into()))
                .collect(),
        }
    }

    #[cfg(test)]
    fn get(&self, key: &str) -> Option<&str> {
        self.bindings
            .iter()
            .find(|(binding_key, _)| binding_key == key)
            .map(|(_, action)| action.as_str())
    }
}

impl Default for KeyBindingsConfig {
    fn default() -> Self {
        Self::from_pairs(DEFAULT_KEYBINDINGS)
    }
}

impl IntoIterator for KeyBindingsConfig {
    type Item = (String, String);
    type IntoIter = std::vec::IntoIter<(String, String)>;

    fn into_iter(self) -> Self::IntoIter {
        self.bindings.into_iter()
    }
}

impl<'de> Deserialize<'de> for KeyBindingsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let table = toml::Table::deserialize(deserializer)?;
        let mut bindings = Vec::new();

        for (key, value) in table {
            let toml::Value::String(action) = value else {
                return Err(D::Error::custom(format!(
                    "keybindings.{key} は文字列である必要があります"
                )));
            };
            bindings.push((key, action));
        }

        Ok(Self { bindings })
    }
}

#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub keybindings: KeyBindingsConfig,
    pub editors: Vec<String>,
    pub startup_git: StartupGitConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfigFile {
    #[serde(default)]
    keybindings: KeyBindingsConfig,
    #[serde(default = "default_editors")]
    editors: Vec<String>,
    #[serde(default)]
    startup_git: StartupGitConfig,
    #[serde(default)]
    ui: UiConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupGitConfig {
    #[serde(default)]
    pub auto_commit_and_push: bool,
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
        startup_git: file.startup_git,
        ui: file.ui,
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
            toml::Value::Table(default_keybindings_table()),
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

    for (key, action) in DEFAULT_KEYBINDINGS {
        let action_exists = keybindings
            .values()
            .any(|value| value.as_str() == Some(action));
        if !action_exists && !keybindings.contains_key(key) {
            keybindings.insert(key.to_string(), toml::Value::String(action.to_string()));
            changed = true;
        }
    }

    if !table.contains_key("startup_git") {
        table.insert(
            "startup_git".to_string(),
            toml::Value::Table(toml::Table::new()),
        );
        changed = true;
    }

    let startup_git = table
        .get_mut("startup_git")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| {
            format!(
                "config fileのstartup_gitがtableではありません: {}",
                path.display()
            )
        })?;

    if !startup_git.contains_key("auto_commit_and_push") {
        startup_git.insert(
            "auto_commit_and_push".to_string(),
            toml::Value::Boolean(false),
        );
        changed = true;
    }

    changed |= ui::ensure_ui_defaults(table, path)?;

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

fn default_keybindings_table() -> toml::Table {
    DEFAULT_KEYBINDINGS
        .into_iter()
        .map(|(key, action)| (key.to_string(), toml::Value::String(action.to_string())))
        .collect()
}

#[cfg(test)]
mod tests;
