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

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(clippy::disallowed_methods)]
    use time::{Duration, OffsetDateTime};

    #[test]
    fn test_check_time_restriction_none_none() {
        assert!(check_time_restriction(None, None));
    }

    #[test]
    fn test_check_time_restriction_start_past_no_end() {
        let now = OffsetDateTime::now_utc();
        assert!(check_time_restriction(Some(now - Duration::hours(1)), None));
    }

    #[test]
    fn test_check_time_restriction_start_future_no_end() {
        let now = OffsetDateTime::now_utc();
        assert!(!check_time_restriction(
            Some(now + Duration::hours(1)),
            None
        ));
    }

    #[test]
    fn test_check_time_restriction_no_start_end_future() {
        let now = OffsetDateTime::now_utc();
        assert!(check_time_restriction(None, Some(now + Duration::hours(1))));
    }

    #[test]
    fn test_check_time_restriction_no_start_end_past() {
        let now = OffsetDateTime::now_utc();
        assert!(!check_time_restriction(
            None,
            Some(now - Duration::hours(1))
        ));
    }

    #[test]
    fn test_check_time_restriction_both_in_range() {
        let now = OffsetDateTime::now_utc();
        assert!(check_time_restriction(
            Some(now - Duration::hours(1)),
            Some(now + Duration::hours(1))
        ));
    }

    #[test]
    fn test_check_time_restriction_both_expired() {
        let now = OffsetDateTime::now_utc();
        assert!(!check_time_restriction(
            Some(now - Duration::hours(2)),
            Some(now - Duration::hours(1))
        ));
    }

    #[test]
    fn test_check_time_restriction_both_future() {
        let now = OffsetDateTime::now_utc();
        assert!(!check_time_restriction(
            Some(now + Duration::hours(1)),
            Some(now + Duration::hours(2))
        ));
    }

    #[test]
    fn test_check_time_restriction_start_at_now() {
        let now = OffsetDateTime::now_utc();
        assert!(check_time_restriction(
            Some(now - Duration::seconds(1)),
            None
        ));
    }

    #[test]
    fn test_check_time_restriction_end_at_now() {
        let now = OffsetDateTime::now_utc();
        assert!(check_time_restriction(
            None,
            Some(now + Duration::seconds(1))
        ));
    }

    #[test]
    fn test_check_time_restriction_start_equals_end() {
        let now = OffsetDateTime::now_utc();
        assert!(check_time_restriction(
            Some(now - Duration::seconds(1)),
            Some(now + Duration::seconds(1))
        ));
    }
}
