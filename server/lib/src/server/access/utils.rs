use time::OffsetDateTime;

#[allow(clippy::disallowed_methods)]
pub fn check_time_restriction(
    time_start: Option<OffsetDateTime>,
    time_end: Option<OffsetDateTime>,
) -> bool {
    let now = OffsetDateTime::now_utc();
    match (time_start, time_end) {
        (None, None) => true,
        (Some(start), None) => now >= start,
        (None, Some(end)) => now <= end,
        (Some(start), Some(end)) => now >= start && now <= end,
    }
}
