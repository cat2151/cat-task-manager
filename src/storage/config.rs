use std::{collections::HashSet, fs, path::Path};

use serde::Deserialize;

const DEFAULT_EDITORS: [&str; 4] = ["fresh", "zed", "nvim", "code"];
const DEFAULT_CONFIG: &str = r#"editors = ["fresh", "zed", "nvim", "code"]

[keybindings]
next = "j"
previous = "k"
advance = "enter"
hold = "h"
quit = "q"
edit = "e"
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
    let value: toml::Value = toml::from_str(&raw).map_err(|err| {
        format!(
            "config fileをTOMLとして読めませんでした: {} ({err})",
            path.display()
        )
    })?;

    let needs_editors = !matches!(value.get("editors"), Some(toml::Value::Array(_)));
    let keybindings = value.get("keybindings").and_then(toml::Value::as_table);
    let missing_keybindings = [("edit", "e"), ("toggle_view", "v"), ("help", "?")]
        .into_iter()
        .filter(|(key, _)| keybindings.is_none_or(|table| !table.contains_key(*key)))
        .collect::<Vec<_>>();

    if needs_editors || !missing_keybindings.is_empty() {
        let updated = apply_config_defaults(&raw, needs_editors, &missing_keybindings);
        fs::write(path, updated).map_err(|err| {
            format!(
                "config fileを書き込めませんでした: {} ({err})",
                path.display()
            )
        })?;
    }

    Ok(())
}

fn apply_config_defaults(
    raw: &str,
    needs_editors: bool,
    missing_keybindings: &[(&str, &str)],
) -> String {
    let mut updated = raw.to_string();

    if !missing_keybindings.is_empty() {
        updated = insert_keybindings(&updated, missing_keybindings);
    }
    if needs_editors {
        updated = format!(
            "editors = [\"fresh\", \"zed\", \"nvim\", \"code\"]\n\n{}",
            updated
        );
    }
    if !updated.ends_with('\n') {
        updated.push('\n');
    }

    updated
}

fn insert_keybindings(raw: &str, keybindings: &[(&str, &str)]) -> String {
    let lines = raw.lines().collect::<Vec<_>>();
    if let Some(index) = lines.iter().position(|line| line.trim() == "[keybindings]") {
        let mut output = String::new();
        for (line_index, line) in lines.iter().enumerate() {
            output.push_str(line);
            output.push('\n');
            if line_index == index {
                for (key, value) in keybindings {
                    output.push_str(&format!("{key} = \"{value}\"\n"));
                }
            }
        }
        output
    } else {
        let mut output = raw.to_string();
        if !output.ends_with('\n') {
            output.push('\n');
        }
        if !output.ends_with("\n\n") {
            output.push('\n');
        }
        output.push_str("[keybindings]\n");
        for (key, value) in keybindings {
            output.push_str(&format!("{key} = \"{value}\"\n"));
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let raw = "editors = [\"nvim\"]\n\n[keybindings]\nnext = \"j\"\n";
        let updated = apply_config_defaults(
            raw,
            false,
            &[("edit", "e"), ("toggle_view", "v"), ("help", "?")],
        );

        assert!(updated.contains(
            "[keybindings]\nedit = \"e\"\ntoggle_view = \"v\"\nhelp = \"?\"\nnext = \"j\""
        ));
    }

    #[test]
    fn config_defaults_add_keybindings_table() {
        let raw = "editors = [\"nvim\"]\n";
        let updated = apply_config_defaults(raw, false, &[("help", "?")]);
        let value: toml::Value = toml::from_str(&updated).unwrap();

        assert_eq!(
            value
                .get("keybindings")
                .and_then(|table| table.get("help"))
                .and_then(toml::Value::as_str),
            Some("?")
        );
    }

    #[test]
    fn default_config_is_config_only() {
        let file: RawConfigFile = toml::from_str(DEFAULT_CONFIG).unwrap();

        assert_eq!(file.editors, default_editors());
    }

    #[test]
    fn tasks_in_config_are_rejected() {
        let raw = "tasks = '''\na\n'''\n";

        assert!(toml::from_str::<RawConfigFile>(raw).is_err());
    }
}
