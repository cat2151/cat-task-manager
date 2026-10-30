use super::*;
use std::fs;

fn cache_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cat-task-manager-{name}-{}-history-stats-cache.json",
        std::process::id()
    ))
}

#[test]
fn revision_cache_round_trips() {
    let path = cache_path("round-trip");
    let revisions = HashMap::from([(
        "rev-a".to_string(),
        RevisionStats {
            task_names: HashSet::from(["朝食をいただく".to_string()]),
            task_duration_candidates: Vec::new(),
            skipped_files: 1,
        },
    )]);

    revision_cache::write_revision_cache(&path, &revisions);

    assert_eq!(revision_cache::read_revision_cache(&path), revisions);
    let _ = fs::remove_file(path);
}

#[test]
fn revision_cache_is_empty_for_unreadable_file() {
    let path = cache_path("unreadable");
    fs::write(&path, "{\"version\":5}").unwrap();

    assert!(revision_cache::read_revision_cache(&path).is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn sorted_task_counts_orders_by_count_then_name_without_limit() {
    let mut counts = HashMap::new();
    for index in 0..12 {
        counts.insert(format!("task {index:02}"), index);
    }
    counts.insert("aaa".to_string(), 11);

    let sorted = sorted_task_counts(counts, &typical::TypicalTaskDurations::default());

    assert_eq!(sorted.len(), 13);
    assert_eq!(sorted[0].name, "aaa");
    assert_eq!(sorted[0].count, 11);
    assert_eq!(sorted[1].name, "task 11");
    assert_eq!(sorted[12].name, "task 00");
}

#[test]
fn add_revision_tasks_counts_each_task_once_per_revision() {
    let mut counts = HashMap::new();

    add_revision_tasks(
        &mut counts,
        HashSet::from(["朝食をいただく".to_string(), "散歩".to_string()]),
    );
    add_revision_tasks(
        &mut counts,
        HashSet::from(["朝食をいただく".to_string(), "朝食をいただく".to_string()]),
    );

    assert_eq!(counts.get("朝食をいただく"), Some(&2));
    assert_eq!(counts.get("散歩"), Some(&1));
}
