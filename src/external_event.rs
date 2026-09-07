use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::Serialize;

use crate::{
    app::{TaskEvent, TaskEventKind},
    storage::ExternalEventConfig,
};

const INTERFACE_VERSION: u8 = 1;
static EVENT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize)]
struct InterfaceEvent<'a> {
    version: u8,
    event_id: String,
    kind: &'static str,
    occurred_at: String,
    task_name: &'a str,
    task_file: &'a Path,
    source_line: u32,
}

pub fn publish(config: &ExternalEventConfig, event: &TaskEvent) -> Result<(), String> {
    let Some(interface_file) = config.interface_file() else {
        return Ok(());
    };
    let parent = interface_file.parent().ok_or_else(|| {
        format!(
            "external event interface fileに親directoryがありません: {}",
            interface_file.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|err| {
        format!(
            "external event interface directoryを作成できませんでした: {} ({err})",
            parent.display()
        )
    })?;

    let sequence = EVENT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let record = InterfaceEvent {
        version: INTERFACE_VERSION,
        event_id: format!(
            "{}-{}-{sequence}",
            std::process::id(),
            event.occurred_at.timestamp_micros()
        ),
        kind: match event.kind {
            TaskEventKind::Started => "task_started",
            TaskEventKind::Completed => "task_completed",
        },
        occurred_at: crate::clock::format_rfc3339_jst(&event.occurred_at),
        task_name: &event.task_name,
        task_file: &event.task_file,
        source_line: event.source_line,
    };
    let source = toml::to_string(&record)
        .map_err(|err| format!("external eventをTOMLへ変換できませんでした: {err}"))?;
    atomic_write(interface_file, source.as_bytes())
}

fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), String> {
    let sequence = EVENT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = temporary_path(target, sequence);
    fs::write(&temporary, bytes).map_err(|err| {
        format!(
            "external eventの一時fileを書き込めませんでした: {} ({err})",
            temporary.display()
        )
    })?;
    if let Err(err) = replace_file(&temporary, target) {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "external event interface fileを置換できませんでした: {} ({err})",
            target.display()
        ));
    }
    Ok(())
}

fn temporary_path(target: &Path, sequence: u64) -> PathBuf {
    let file_name = target
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    target.with_file_name(format!(
        ".{file_name}.tmp-{}-{sequence}",
        std::process::id()
    ))
}

#[cfg(windows)]
fn replace_file(temporary: &Path, target: &Path) -> std::io::Result<()> {
    use std::{os::windows::ffi::OsStrExt, ptr};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn ReplaceFileW(
            replaced_file_name: *const u16,
            replacement_file_name: *const u16,
            backup_file_name: *const u16,
            replace_flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    const ERROR_FILE_NOT_FOUND: i32 = 2;
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    let temporary = wide(temporary);
    let target = wide(target);
    let replaced = unsafe {
        ReplaceFileW(
            target.as_ptr(),
            temporary.as_ptr(),
            ptr::null(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if replaced != 0 {
        return Ok(());
    }
    let replace_error = std::io::Error::last_os_error();
    if replace_error.raw_os_error() != Some(ERROR_FILE_NOT_FOUND) {
        return Err(replace_error);
    }
    let moved = unsafe {
        MoveFileExW(
            temporary.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(temporary, target)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use chrono::{DateTime, Local};
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct ReadEvent {
        version: u8,
        event_id: String,
        kind: String,
        occurred_at: String,
        task_name: String,
        task_file: PathBuf,
        source_line: u32,
    }

    #[test]
    fn writes_a_complete_versioned_event() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("external-event-{suffix}"));
        let path = directory.join("event.toml");
        let config = ExternalEventConfig::with_interface_file_for_test(path.clone());
        let occurred_at = DateTime::parse_from_rfc3339("2026-09-07T13:45:12+09:00")
            .unwrap()
            .with_timezone(&Local);
        let event = TaskEvent {
            kind: TaskEventKind::Started,
            occurred_at,
            task_name: "write tests".to_string(),
            task_file: PathBuf::from("tasks/test.md"),
            source_line: 42,
        };

        publish(&config, &event).unwrap();

        let parsed: ReadEvent = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(parsed.version, 1);
        assert!(!parsed.event_id.is_empty());
        assert_eq!(parsed.kind, "task_started");
        assert_eq!(parsed.occurred_at, "2026-09-07T13:45:12+09:00");
        assert_eq!(parsed.task_name, "write tests");
        assert_eq!(parsed.task_file, PathBuf::from("tasks/test.md"));
        assert_eq!(parsed.source_line, 42);

        fs::remove_dir_all(directory).unwrap();
    }
}
