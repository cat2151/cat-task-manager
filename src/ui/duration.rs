pub(super) fn format_elapsed_seconds(total_seconds: i64) -> String {
    let total_seconds = total_seconds.max(0);
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours}時間"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}分"));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{seconds}秒"));
    }

    parts.join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_seconds_are_formatted_as_japanese_units() {
        assert_eq!(format_elapsed_seconds(-1), "0秒");
        assert_eq!(format_elapsed_seconds(0), "0秒");
        assert_eq!(format_elapsed_seconds(10), "10秒");
        assert_eq!(format_elapsed_seconds(30 * 60), "30分");
        assert_eq!(format_elapsed_seconds(60 * 60), "1時間");
        assert_eq!(
            format_elapsed_seconds(60 * 60 + 30 * 60 + 10),
            "1時間30分10秒"
        );
    }
}
