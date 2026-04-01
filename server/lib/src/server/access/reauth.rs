use crate::prelude::*;
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReauthRequirement {
    NotRequired,
    Required { reason: String },
}

#[allow(dead_code)]
pub fn evaluate_reauth_requirement(
    ident: &Identity,
    require_reauth: bool,
    reauth_max_age: Option<u32>,
    ct: Duration,
) -> ReauthRequirement {
    if !require_reauth && reauth_max_age.is_none() {
        return ReauthRequirement::NotRequired;
    }

    let session = match ident.get_session() {
        Some(s) => s,
        None => return ReauthRequirement::NotRequired,
    };

    if matches!(ident.access_scope(), AccessScope::ReadWrite) {
        return ReauthRequirement::NotRequired;
    }

    if require_reauth {
        return ReauthRequirement::Required {
            reason:
                "This operation requires elevated privileges. Please re-authenticate to continue."
                    .to_string(),
        };
    }

    if let Some(max_age_secs) = reauth_max_age {
        let max_age = Duration::from_secs(max_age_secs as u64);
        let now = OffsetDateTime::UNIX_EPOCH + ct;
        let session_age = now - session.issued_at;

        if session_age > max_age {
            return ReauthRequirement::Required {
                reason: format!(
                    "Your session was authenticated {} seconds ago, which exceeds the maximum allowed age of {} seconds. Please re-authenticate.",
                    session_age.whole_seconds(),
                    max_age_secs
                ),
            };
        }
    }

    ReauthRequirement::NotRequired
}
