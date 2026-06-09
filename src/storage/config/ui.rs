use std::path::Path;

use serde::{de::Error as _, Deserialize, Deserializer};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiConfig {
    #[serde(default)]
    pub estimate_blink: EstimateBlinkConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstimateBlinkConfig {
    #[serde(default = "default_estimate_blink_enabled")]
    pub enabled: bool,
    #[serde(default = "default_estimate_blink_foreground")]
    pub foreground: MonokaiColorName,
    #[serde(default = "default_estimate_blink_background")]
    pub background: MonokaiColorName,
}

impl Default for EstimateBlinkConfig {
    fn default() -> Self {
        Self {
            enabled: default_estimate_blink_enabled(),
            foreground: default_estimate_blink_foreground(),
            background: default_estimate_blink_background(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonokaiColorName {
    Bg,
    Fg,
    Comment,
    Selection,
    Pink,
    Green,
    Yellow,
    Orange,
    Blue,
}

impl MonokaiColorName {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bg" | "background" | "monokai_bg" | "monokai_background" => Some(Self::Bg),
            "fg" | "foreground" | "monokai_fg" | "monokai_foreground" => Some(Self::Fg),
            "comment" | "monokai_comment" => Some(Self::Comment),
            "selection" | "monokai_selection" => Some(Self::Selection),
            "pink" | "monokai_pink" => Some(Self::Pink),
            "green" | "monokai_green" => Some(Self::Green),
            "yellow" | "monokai_yellow" => Some(Self::Yellow),
            "orange" | "monokai_orange" => Some(Self::Orange),
            "blue" | "monokai_blue" => Some(Self::Blue),
            _ => None,
        }
    }

    fn config_name(self) -> &'static str {
        match self {
            Self::Bg => "bg",
            Self::Fg => "fg",
            Self::Comment => "comment",
            Self::Selection => "selection",
            Self::Pink => "pink",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Orange => "orange",
            Self::Blue => "blue",
        }
    }
}

impl<'de> Deserialize<'de> for MonokaiColorName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).ok_or_else(|| {
            D::Error::custom(format!(
                "Monokai color name は {} のいずれかを指定してください: '{raw}'",
                MONOKAI_COLOR_NAMES.join(", ")
            ))
        })
    }
}

pub(super) fn ensure_ui_defaults(root: &mut toml::Table, path: &Path) -> Result<bool, String> {
    let mut changed = false;
    if !root.contains_key("ui") {
        root.insert("ui".to_string(), toml::Value::Table(toml::Table::new()));
        changed = true;
    }

    let ui = root
        .get_mut("ui")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| format!("config fileのuiがtableではありません: {}", path.display()))?;

    if !ui.contains_key("estimate_blink") {
        ui.insert(
            "estimate_blink".to_string(),
            toml::Value::Table(toml::Table::new()),
        );
        changed = true;
    }

    let estimate_blink = ui
        .get_mut("estimate_blink")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| {
            format!(
                "config fileのui.estimate_blinkがtableではありません: {}",
                path.display()
            )
        })?;

    changed |= insert_missing_bool(estimate_blink, "enabled", default_estimate_blink_enabled());
    changed |= insert_missing_string(
        estimate_blink,
        "foreground",
        default_estimate_blink_foreground().config_name(),
    );
    changed |= insert_missing_string(
        estimate_blink,
        "background",
        default_estimate_blink_background().config_name(),
    );

    Ok(changed)
}

fn insert_missing_bool(table: &mut toml::Table, key: &str, value: bool) -> bool {
    if table.contains_key(key) {
        return false;
    }

    table.insert(key.to_string(), toml::Value::Boolean(value));
    true
}

fn insert_missing_string(table: &mut toml::Table, key: &str, value: &str) -> bool {
    if table.contains_key(key) {
        return false;
    }

    table.insert(key.to_string(), toml::Value::String(value.to_string()));
    true
}

fn default_estimate_blink_enabled() -> bool {
    true
}

fn default_estimate_blink_foreground() -> MonokaiColorName {
    MonokaiColorName::Green
}

fn default_estimate_blink_background() -> MonokaiColorName {
    MonokaiColorName::Bg
}

const MONOKAI_COLOR_NAMES: [&str; 9] = [
    "bg",
    "fg",
    "comment",
    "selection",
    "pink",
    "green",
    "yellow",
    "orange",
    "blue",
];
