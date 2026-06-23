use std::path::Path;

use chrono::NaiveTime;
use serde::{de::Error as _, Deserialize, Deserializer};

const DEFAULT_ACTIVE_HOURS: &str = "09:00-17:00";
const DEFAULT_IDLE_SECONDS: u64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFreeTimeConfig {
    pub enabled: bool,
    pub idle_seconds: u64,
    active_hours: ActiveHours,
}

impl AutoFreeTimeConfig {
    pub fn is_active_at(&self, time: NaiveTime) -> bool {
        self.active_hours.contains(time)
    }
}

impl Default for AutoFreeTimeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            idle_seconds: DEFAULT_IDLE_SECONDS,
            active_hours: ActiveHours::parse(DEFAULT_ACTIVE_HOURS)
                .expect("default active hours are valid"),
        }
    }
}

impl<'de> Deserialize<'de> for AutoFreeTimeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawAutoFreeTimeConfig::deserialize(deserializer)?;
        if raw.idle_seconds == 0 || raw.idle_seconds > i64::MAX as u64 {
            return Err(D::Error::custom(
                "auto_free_time.idle_seconds は1以上で指定してください",
            ));
        }

        Ok(Self {
            enabled: raw.enabled,
            idle_seconds: raw.idle_seconds,
            active_hours: ActiveHours::parse(&raw.active_hours).map_err(D::Error::custom)?,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAutoFreeTimeConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_idle_seconds")]
    idle_seconds: u64,
    #[serde(default = "default_active_hours")]
    active_hours: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActiveHours {
    start: NaiveTime,
    end: NaiveTime,
}

impl ActiveHours {
    fn parse(raw: &str) -> Result<Self, String> {
        let Some((start, end)) = raw.trim().split_once('-') else {
            return Err(active_hours_error(raw));
        };
        let start = parse_time(start).ok_or_else(|| active_hours_error(raw))?;
        let end = parse_time(end).ok_or_else(|| active_hours_error(raw))?;
        if start == end {
            return Err(
                "auto_free_time.active_hours の開始時刻と終了時刻は別にしてください".to_string(),
            );
        }

        Ok(Self { start, end })
    }

    fn contains(self, time: NaiveTime) -> bool {
        if self.start < self.end {
            self.start <= time && time < self.end
        } else {
            self.start <= time || time < self.end
        }
    }
}

pub(super) fn ensure_auto_free_time_defaults(
    root: &mut toml::Table,
    path: &Path,
) -> Result<bool, String> {
    let mut changed = false;
    if !root.contains_key("auto_free_time") {
        root.insert(
            "auto_free_time".to_string(),
            toml::Value::Table(toml::Table::new()),
        );
        changed = true;
    }

    let auto_free_time = root
        .get_mut("auto_free_time")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| {
            format!(
                "config fileのauto_free_timeがtableではありません: {}",
                path.display()
            )
        })?;

    changed |= insert_missing_bool(auto_free_time, "enabled", false);
    changed |= insert_missing_integer(auto_free_time, "idle_seconds", DEFAULT_IDLE_SECONDS as i64);
    changed |= insert_missing_string(auto_free_time, "active_hours", DEFAULT_ACTIVE_HOURS);

    Ok(changed)
}

fn parse_time(raw: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(raw.trim(), "%H:%M").ok()
}

fn active_hours_error(raw: &str) -> String {
    format!("auto_free_time.active_hours は HH:MM-HH:MM 形式で指定してください: '{raw}'")
}

fn insert_missing_bool(table: &mut toml::Table, key: &str, value: bool) -> bool {
    if table.contains_key(key) {
        return false;
    }
    table.insert(key.to_string(), toml::Value::Boolean(value));
    true
}

fn insert_missing_integer(table: &mut toml::Table, key: &str, value: i64) -> bool {
    if table.contains_key(key) {
        return false;
    }
    table.insert(key.to_string(), toml::Value::Integer(value));
    true
}

fn insert_missing_string(table: &mut toml::Table, key: &str, value: &str) -> bool {
    if table.contains_key(key) {
        return false;
    }
    table.insert(key.to_string(), toml::Value::String(value.to_string()));
    true
}

fn default_idle_seconds() -> u64 {
    DEFAULT_IDLE_SECONDS
}

fn default_active_hours() -> String {
    DEFAULT_ACTIVE_HOURS.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(raw: &str) -> AutoFreeTimeConfig {
        toml::from_str(raw).unwrap()
    }

    #[test]
    fn missing_config_is_disabled() {
        assert!(!AutoFreeTimeConfig::default().enabled);
    }

    #[test]
    fn active_hours_include_start_and_exclude_end() {
        let config = config(
            r#"
enabled = true
active_hours = "09:00-17:00"
"#,
        );

        assert!(config.is_active_at(parse_time("09:00").unwrap()));
        assert!(config.is_active_at(parse_time("16:59").unwrap()));
        assert!(!config.is_active_at(parse_time("17:00").unwrap()));
    }

    #[test]
    fn active_hours_can_cross_midnight() {
        let config = config(
            r#"
enabled = true
active_hours = "22:00-02:00"
"#,
        );

        assert!(config.is_active_at(parse_time("23:00").unwrap()));
        assert!(config.is_active_at(parse_time("01:00").unwrap()));
        assert!(!config.is_active_at(parse_time("12:00").unwrap()));
    }

    #[test]
    fn invalid_active_hours_are_rejected() {
        assert!(toml::from_str::<AutoFreeTimeConfig>(r#"active_hours = "morning""#).is_err());
        assert!(toml::from_str::<AutoFreeTimeConfig>(r#"active_hours = "09:00-09:00""#).is_err());
    }

    #[test]
    fn zero_idle_seconds_is_rejected() {
        assert!(toml::from_str::<AutoFreeTimeConfig>("idle_seconds = 0").is_err());
    }
}
