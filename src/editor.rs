use std::{path::Path, process::Command};

pub fn open_with_configured_editor(path: &Path, editors: &[String]) -> Result<String, String> {
    let mut failures = Vec::new();

    for editor in editors
        .iter()
        .map(|editor| editor.trim())
        .filter(|editor| !editor.is_empty())
    {
        match Command::new(editor).arg(path).status() {
            Ok(status) if status.success() => return Ok(editor.to_string()),
            Ok(status) => failures.push(format!("{editor}: 終了 status が失敗です ({status})")),
            Err(err) => failures.push(format!("{editor}: {err}")),
        }
    }

    if failures.is_empty() {
        return Err("エディタが設定されていません".to_string());
    }

    Err(format!(
        "ファイルを開けませんでした: {} ({})",
        path.display(),
        failures.join("; ")
    ))
}
