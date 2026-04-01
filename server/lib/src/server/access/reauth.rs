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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::identity::IdentityId;
    use crate::server::identity::{AccessScope, IdentType, IdentUser, Identity, Source};
    use crate::value::{AuthType, Session, SessionScope, SessionState};
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use uuid::Uuid;

    fn create_test_session(issued_at: OffsetDateTime) -> Session {
        Session {
            label: "test".to_string(),
            state: SessionState::NeverExpires,
            issued_at,
            issued_by: IdentityId::User(Uuid::nil()),
            cred_id: Uuid::nil(),
            scope: SessionScope::PrivilegeCapable,
            type_: AuthType::Passkey,
            ext_metadata: crate::value::SessionExtMetadata::None,
        }
    }

    fn create_test_identity(scope: AccessScope, session: Option<Session>) -> Identity {
        let session_id = Uuid::new_v4();
        let mut entry = crate::entry_init!(
            crate::prelude::Attribute::Class,
            crate::prelude::EntryClass::Object.to_value(),
            crate::prelude::Attribute::Uuid,
            crate::prelude::Value::Uuid(Uuid::nil())
        )
        .into_sealed_committed();

        if let Some(s) = session {
            use crate::prelude::Value;
            let mut sessions = std::collections::BTreeMap::new();
            sessions.insert(session_id, s);
            entry.set_ava(
                &crate::prelude::Attribute::UserAuthTokenSession,
                crate::valueset::ValueSetSession::new(
                    session_id,
                    create_test_session(OffsetDateTime::UNIX_EPOCH),
                ),
            );
        }

        Identity::new(
            IdentType::User(IdentUser {
                entry: Arc::new(entry),
            }),
            Source::Internal,
            session_id,
            scope,
            crate::be::Limits::unlimited(),
        )
    }

    #[test]
    fn test_no_reauth_required_when_disabled() {
        let ct = Duration::from_secs(1000);
        let ident = create_test_identity(AccessScope::ReadOnly, None);

        let result = evaluate_reauth_requirement(&ident, false, None, ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }

    #[test]
    fn test_reauth_required_when_policy_set_and_readonly() {
        let ct = Duration::from_secs(1000);
        let session = create_test_session(OffsetDateTime::UNIX_EPOCH);
        let ident = create_test_identity(AccessScope::ReadOnly, Some(session));

        let result = evaluate_reauth_requirement(&ident, true, None, ct);
        assert!(matches!(result, ReauthRequirement::Required { .. }));
    }

    #[test]
    fn test_no_reauth_required_when_readwrite() {
        let ct = Duration::from_secs(1000);
        let session = create_test_session(OffsetDateTime::UNIX_EPOCH);
        let ident = create_test_identity(AccessScope::ReadWrite, Some(session));

        let result = evaluate_reauth_requirement(&ident, true, None, ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }
}
