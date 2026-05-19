use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, FixedOffset, Local, NaiveDate};

const JST_OFFSET_SECONDS: i32 = 9 * 60 * 60;

fn jst_offset() -> FixedOffset {
    FixedOffset::east_opt(JST_OFFSET_SECONDS).expect("JST offset is valid")
}

pub fn now_jst() -> DateTime<FixedOffset> {
    Local::now().with_timezone(&jst_offset())
}

pub fn today_jst() -> NaiveDate {
    now_jst().date_naive()
}

pub fn format_rfc3339_jst(time: &DateTime<Local>) -> String {
    time.with_timezone(&jst_offset()).to_rfc3339()
}

pub fn duration_until_next_jst_midnight() -> StdDuration {
    let now = now_jst();
    let tomorrow = now.date_naive() + Duration::days(1);
    let next_midnight = tomorrow
        .and_hms_opt(0, 0, 0)
        .expect("00:00:00 is a valid time");
    let wait = next_midnight - now.naive_local();

    wait.to_std().unwrap_or_else(|_| StdDuration::from_secs(1))
}
