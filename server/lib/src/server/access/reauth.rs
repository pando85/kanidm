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

    let Some(session) = ident.get_session() else {
        return ReauthRequirement::NotRequired;
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

    fn create_test_session_with_auth_type(
        issued_at: OffsetDateTime,
        auth_type: AuthType,
    ) -> Session {
        Session {
            label: "test".to_string(),
            state: SessionState::NeverExpires,
            issued_at,
            issued_by: IdentityId::User(Uuid::nil()),
            cred_id: Uuid::nil(),
            scope: SessionScope::PrivilegeCapable,
            type_: auth_type,
            ext_metadata: crate::value::SessionExtMetadata::None,
        }
    }

    fn create_test_identity(scope: AccessScope) -> Identity {
        let session_id = Uuid::new_v4();
        let entry = crate::entry_init!(
            (
                crate::prelude::Attribute::Class,
                crate::prelude::EntryClass::Object.to_value()
            ),
            (
                crate::prelude::Attribute::Uuid,
                crate::prelude::Value::Uuid(Uuid::nil())
            )
        )
        .into_sealed_committed();

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

    fn create_test_identity_with_session(
        scope: AccessScope,
        issued_at: OffsetDateTime,
    ) -> Identity {
        let session_id = Uuid::new_v4();
        let session = create_test_session(issued_at);
        let entry = crate::entry_init!(
            (
                crate::prelude::Attribute::Class,
                crate::prelude::EntryClass::Object.to_value()
            ),
            (
                crate::prelude::Attribute::Uuid,
                crate::prelude::Value::Uuid(Uuid::nil())
            ),
            (
                crate::prelude::Attribute::UserAuthTokenSession,
                crate::prelude::Value::Session(session_id, session.clone())
            )
        )
        .into_sealed_committed();

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
        let ident = create_test_identity(AccessScope::ReadOnly);

        let result = evaluate_reauth_requirement(&ident, false, None, ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }

    #[test]
    fn test_no_reauth_required_when_readwrite() {
        let ct = Duration::from_secs(1000);
        let ident = create_test_identity(AccessScope::ReadWrite);

        let result = evaluate_reauth_requirement(&ident, true, None, ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }

    #[test]
    fn test_reauth_logic_without_session() {
        let ct = Duration::from_secs(1000);
        let ident = create_test_identity(AccessScope::ReadOnly);

        let result = evaluate_reauth_requirement(&ident, true, None, ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }

    #[test]
    fn test_session_creation() {
        let session = create_test_session(OffsetDateTime::UNIX_EPOCH);
        assert_eq!(session.label, "test");
        assert!(matches!(session.state, SessionState::NeverExpires));
    }

    #[test]
    fn test_trigger_policy_require_reauth_enabled() {
        let ct = Duration::from_secs(1000);
        let issued_at = OffsetDateTime::UNIX_EPOCH + ct;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let result = evaluate_reauth_requirement(&ident, true, None, ct);
        assert!(matches!(result, ReauthRequirement::Required { .. }));
        if let ReauthRequirement::Required { reason } = result {
            assert!(reason.contains("elevated privileges"));
        }
    }

    #[test]
    fn test_trigger_policy_max_age_expired() {
        let ct = Duration::from_secs(1000);
        let issued_at = OffsetDateTime::UNIX_EPOCH;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let max_age_secs = 100u32;
        let result = evaluate_reauth_requirement(&ident, false, Some(max_age_secs), ct);
        assert!(matches!(result, ReauthRequirement::Required { .. }));
        if let ReauthRequirement::Required { reason } = result {
            assert!(reason.contains("1000 seconds"));
            assert!(reason.contains("100 seconds"));
        }
    }

    #[test]
    fn test_trigger_policy_max_age_not_expired() {
        let ct = Duration::from_secs(50);
        let issued_at = OffsetDateTime::UNIX_EPOCH + ct;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let max_age_secs = 100u32;
        let result = evaluate_reauth_requirement(&ident, false, Some(max_age_secs), ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }

    #[test]
    fn test_trigger_policy_both_conditions() {
        let ct = Duration::from_secs(1000);
        let issued_at = OffsetDateTime::UNIX_EPOCH;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let max_age_secs = 100u32;
        let result = evaluate_reauth_requirement(&ident, true, Some(max_age_secs), ct);
        assert!(matches!(result, ReauthRequirement::Required { .. }));
    }

    #[test]
    fn test_trigger_policy_zero_max_age() {
        let ct = Duration::from_secs(1000);
        let issued_at = OffsetDateTime::UNIX_EPOCH;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let result = evaluate_reauth_requirement(&ident, false, Some(0), ct);
        assert!(matches!(result, ReauthRequirement::Required { .. }));
    }

    #[test]
    fn test_identity_scope_readonly_requires_reauth() {
        let ct = Duration::from_secs(1000);
        let issued_at = OffsetDateTime::UNIX_EPOCH + ct;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let result = evaluate_reauth_requirement(&ident, true, None, ct);
        assert!(matches!(result, ReauthRequirement::Required { .. }));
    }

    #[test]
    fn test_identity_scope_synchronise_blocked() {
        let ct = Duration::from_secs(1000);
        let issued_at = OffsetDateTime::UNIX_EPOCH + ct;
        let ident = create_test_identity_with_session(AccessScope::Synchronise, issued_at);

        let result = evaluate_reauth_requirement(&ident, true, None, ct);
        assert!(matches!(result, ReauthRequirement::Required { .. }));
    }

    #[test]
    fn test_session_just_expired_max_age() {
        let max_age_secs = 300u32;
        let ct = Duration::from_secs(max_age_secs as u64 + 1);
        let issued_at = OffsetDateTime::UNIX_EPOCH;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let result = evaluate_reauth_requirement(&ident, false, Some(max_age_secs), ct);
        assert!(matches!(result, ReauthRequirement::Required { .. }));
    }

    #[test]
    fn test_session_within_max_age_boundary() {
        let max_age_secs = 300u32;
        let ct = Duration::from_secs(max_age_secs as u64);
        let issued_at = OffsetDateTime::UNIX_EPOCH;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let result = evaluate_reauth_requirement(&ident, false, Some(max_age_secs), ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }

    #[test]
    fn test_different_auth_types_with_require_reauth() {
        for auth_type in [
            AuthType::Anonymous,
            AuthType::Password,
            AuthType::GeneratedPassword,
            AuthType::PasswordTotp,
            AuthType::PasswordBackupCode,
            AuthType::PasswordSecurityKey,
            AuthType::Passkey,
            AuthType::AttestedPasskey,
        ] {
            let ct = Duration::from_secs(1000);
            let issued_at = OffsetDateTime::UNIX_EPOCH + ct;
            let session_id = Uuid::new_v4();
            let session = create_test_session_with_auth_type(issued_at, auth_type);
            let entry = crate::entry_init!(
                (
                    crate::prelude::Attribute::Class,
                    crate::prelude::EntryClass::Object.to_value()
                ),
                (
                    crate::prelude::Attribute::Uuid,
                    crate::prelude::Value::Uuid(Uuid::nil())
                ),
                (
                    crate::prelude::Attribute::UserAuthTokenSession,
                    crate::prelude::Value::Session(session_id, session.clone())
                )
            )
            .into_sealed_committed();

            let ident = Identity::new(
                IdentType::User(IdentUser {
                    entry: Arc::new(entry),
                }),
                Source::Internal,
                session_id,
                AccessScope::ReadOnly,
                crate::be::Limits::unlimited(),
            );

            let result = evaluate_reauth_requirement(&ident, true, None, ct);
            assert!(matches!(result, ReauthRequirement::Required { .. }));
        }
    }

    #[test]
    fn test_large_max_age_value() {
        let ct = Duration::from_secs(1000);
        let issued_at = OffsetDateTime::UNIX_EPOCH + ct;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let max_age_secs = 86400u32;
        let result = evaluate_reauth_requirement(&ident, false, Some(max_age_secs), ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }

    #[test]
    fn test_session_age_at_exact_max_age() {
        let max_age_secs = 3600u32;
        let ct = Duration::from_secs(0);
        let issued_at = OffsetDateTime::UNIX_EPOCH + Duration::from_secs(max_age_secs as u64);
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let result = evaluate_reauth_requirement(&ident, false, Some(max_age_secs), ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }

    #[test]
    fn test_reauth_reason_message_content() {
        let ct = Duration::from_secs(1000);
        let issued_at = OffsetDateTime::UNIX_EPOCH;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let result = evaluate_reauth_requirement(&ident, true, None, ct);
        if let ReauthRequirement::Required { reason } = result {
            assert!(reason.contains("re-authenticate"));
            assert!(reason.contains("elevated privileges"));
        } else {
            panic!("Expected ReauthRequirement::Required");
        }
    }

    #[test]
    fn test_max_age_reason_message_content() {
        let max_age_secs = 100u32;
        let ct = Duration::from_secs(500);
        let issued_at = OffsetDateTime::UNIX_EPOCH;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let result = evaluate_reauth_requirement(&ident, false, Some(max_age_secs), ct);
        if let ReauthRequirement::Required { reason } = result {
            assert!(reason.contains("500 seconds"));
            assert!(reason.contains("100 seconds"));
            assert!(reason.contains("exceeds"));
        } else {
            panic!("Expected ReauthRequirement::Required");
        }
    }

    #[test]
    fn test_empty_trigger_policy() {
        let ct = Duration::from_secs(1000);
        let ident = create_test_identity(AccessScope::ReadOnly);

        let result = evaluate_reauth_requirement(&ident, false, None, ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }

    #[test]
    fn test_trigger_policy_disabled_both_false() {
        let ct = Duration::from_secs(1000);
        let issued_at = OffsetDateTime::UNIX_EPOCH + ct;
        let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

        let result = evaluate_reauth_requirement(&ident, false, None, ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }

    #[test]
    fn test_readwrite_bypasses_all_triggers() {
        let ct = Duration::from_secs(1000);
        let issued_at = OffsetDateTime::UNIX_EPOCH;
        let ident = create_test_identity_with_session(AccessScope::ReadWrite, issued_at);

        let result = evaluate_reauth_requirement(&ident, true, Some(1), ct);
        assert_eq!(result, ReauthRequirement::NotRequired);
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_session_with_null_uuid() {
            let ct = Duration::from_secs(1000);
            let issued_at = OffsetDateTime::UNIX_EPOCH + ct;
            let session_id = Uuid::nil();
            let session = create_test_session(issued_at);
            let entry = crate::entry_init!(
                (
                    crate::prelude::Attribute::Class,
                    crate::prelude::EntryClass::Object.to_value()
                ),
                (
                    crate::prelude::Attribute::Uuid,
                    crate::prelude::Value::Uuid(Uuid::nil())
                ),
                (
                    crate::prelude::Attribute::UserAuthTokenSession,
                    crate::prelude::Value::Session(session_id, session.clone())
                )
            )
            .into_sealed_committed();

            let ident = Identity::new(
                IdentType::User(IdentUser {
                    entry: Arc::new(entry),
                }),
                Source::Internal,
                session_id,
                AccessScope::ReadOnly,
                crate::be::Limits::unlimited(),
            );

            let result = evaluate_reauth_requirement(&ident, true, None, ct);
            assert!(matches!(result, ReauthRequirement::Required { .. }));
        }

        #[test]
        fn test_very_old_session() {
            let ct = Duration::from_secs(1_000_000_000);
            let issued_at = OffsetDateTime::UNIX_EPOCH;
            let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

            let result = evaluate_reauth_requirement(&ident, false, Some(3600), ct);
            assert!(matches!(result, ReauthRequirement::Required { .. }));
        }

        #[test]
        fn test_future_session_time() {
            let ct = Duration::from_secs(100);
            let issued_at = OffsetDateTime::UNIX_EPOCH + Duration::from_secs(200);
            let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

            let result = evaluate_reauth_requirement(&ident, false, Some(1000), ct);
            assert_eq!(result, ReauthRequirement::NotRequired);
        }
    }

    mod security_tests {
        use super::*;

        #[test]
        fn test_trigger_cannot_be_bypassed_by_scope() {
            let ct = Duration::from_secs(1000);
            let issued_at = OffsetDateTime::UNIX_EPOCH;
            let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

            let result = evaluate_reauth_requirement(&ident, true, None, ct);
            assert!(matches!(result, ReauthRequirement::Required { .. }));
        }

        #[test]
        fn test_expired_session_always_requires_reauth() {
            let max_age = 60u32;
            let ct = Duration::from_secs(120);
            let issued_at = OffsetDateTime::UNIX_EPOCH;
            let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

            let result = evaluate_reauth_requirement(&ident, false, Some(max_age), ct);
            assert!(matches!(result, ReauthRequirement::Required { .. }));
        }

        #[test]
        fn test_multiple_trigger_conditions_require_reauth() {
            let ct = Duration::from_secs(1000);
            let issued_at = OffsetDateTime::UNIX_EPOCH;
            let ident = create_test_identity_with_session(AccessScope::ReadOnly, issued_at);

            let result = evaluate_reauth_requirement(&ident, true, Some(100), ct);
            assert!(matches!(result, ReauthRequirement::Required { .. }));
            if let ReauthRequirement::Required { reason } = result {
                assert!(reason.contains("elevated privileges"));
            }
        }

        #[test]
        fn test_readwrite_scope_is_trusted() {
            let ct = Duration::from_secs(1000);
            let issued_at = OffsetDateTime::UNIX_EPOCH;
            let ident = create_test_identity_with_session(AccessScope::ReadWrite, issued_at);

            let result = evaluate_reauth_requirement(&ident, true, Some(1), ct);
            assert_eq!(result, ReauthRequirement::NotRequired);
        }
    }
}
