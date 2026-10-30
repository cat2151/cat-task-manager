use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};

use super::typical::TaskDurationCandidate;

const REVISION_CACHE_VERSION: u32 = 7;

/// commit の中身は変わらないので、1 revision を読んだ結果は revision hash をキーに永続化して使い回す。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(super) struct RevisionStats {
    pub(super) task_names: HashSet<String>,
    pub(super) task_duration_candidates: Vec<TaskDurationCandidate>,
    pub(super) skipped_files: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct RevisionCacheFile {
    version: u32,
    revisions: HashMap<String, RevisionStats>,
}

pub(super) fn read_revision_cache(path: &Path) -> HashMap<String, RevisionStats> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<RevisionCacheFile>(&raw).ok())
        .filter(|cache| cache.version == REVISION_CACHE_VERSION)
        .map(|cache| cache.revisions)
        .unwrap_or_default()
}

pub(super) fn write_revision_cache(path: &Path, revisions: &HashMap<String, RevisionStats>) {
    let Some(parent) = path.parent() else {
        return;
    };
    let cache = RevisionCacheFile {
        version: REVISION_CACHE_VERSION,
        revisions: revisions.clone(),
    };
    let Ok(raw) = serde_json::to_string(&cache) else {
        return;
    };

    let _ = fs::create_dir_all(parent);
    let _ = fs::write(path, raw);
}
