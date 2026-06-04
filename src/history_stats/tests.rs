use super::*;

fn report() -> HistoryStatsReport {
    HistoryStatsReport {
        scanned_revisions: 2,
        skipped_files: 0,
        timed_out: false,
        task_counts: vec![TaskNameCount {
            name: "朝食をいただく".to_string(),
            count: 2,
            typical_task_duration: None,
        }],
    }
}

fn cache_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cat-task-manager-{name}-{}-history-stats-cache.json",
        std::process::id()
    ))
}

#[test]
fn cached_history_stats_round_trips_for_matching_head() {
    let path = cache_path("matching-head");
    let report = report();

    write_cached_history_stats(&path, "head-a", &report);

    assert_eq!(read_cached_history_stats(&path, "head-a"), Some(report));
    let _ = fs::remove_file(path);
}

#[test]
fn cached_history_stats_ignores_other_head() {
    let path = cache_path("other-head");

    write_cached_history_stats(&path, "head-a", &report());

    assert_eq!(read_cached_history_stats(&path, "head-b"), None);
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
