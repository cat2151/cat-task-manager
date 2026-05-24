use std::{fs, path::PathBuf};

use super::{config, tasks, APP_NAME};

const CONFIG_FILE_NAME: &str = "config.toml";
const TASKS_DIR_NAME: &str = "tasks";
const RECORDS_DIR_NAME: &str = "records";

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root_dir: PathBuf,
    pub config_path: PathBuf,
    pub tasks_dir: PathBuf,
    pub records_dir: PathBuf,
}

pub fn app_paths() -> Result<AppPaths, String> {
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "%LOCALAPPDATA% が未設定です".to_string())?;
    Ok(paths_from_local_app_data(local_app_data))
}

pub fn ensure_app_storage(paths: &AppPaths) -> Result<(), String> {
    fs::create_dir_all(&paths.root_dir).map_err(|err| {
        format!(
            "保存先 directory を作成できませんでした: {} ({err})",
            paths.root_dir.display()
        )
    })?;
    fs::create_dir_all(&paths.records_dir).map_err(|err| {
        format!(
            "records directory を作成できませんでした: {} ({err})",
            paths.records_dir.display()
        )
    })?;

    config::ensure_config_file(&paths.config_path)?;
    tasks::ensure_tasks_dir(&paths.tasks_dir)
}

fn paths_from_local_app_data(local_app_data: impl Into<PathBuf>) -> AppPaths {
    let root_dir = local_app_data.into().join(APP_NAME);
    AppPaths {
        config_path: root_dir.join(CONFIG_FILE_NAME),
        tasks_dir: root_dir.join(TASKS_DIR_NAME),
        records_dir: root_dir.join(RECORDS_DIR_NAME),
        root_dir,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_paths_use_local_app_data_app_directory() {
        let paths = paths_from_local_app_data(PathBuf::from(r"C:\Users\me\AppData\Local"));

        assert_eq!(
            paths.root_dir,
            PathBuf::from(r"C:\Users\me\AppData\Local\cat-task-manager")
        );
        assert_eq!(
            paths.config_path,
            PathBuf::from(r"C:\Users\me\AppData\Local\cat-task-manager\config.toml")
        );
        assert_eq!(
            paths.tasks_dir,
            PathBuf::from(r"C:\Users\me\AppData\Local\cat-task-manager\tasks")
        );
        assert_eq!(
            paths.records_dir,
            PathBuf::from(r"C:\Users\me\AppData\Local\cat-task-manager\records")
        );
    }
}
