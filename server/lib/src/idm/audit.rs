use crate::idm::approval::ApprovalAuditEvent;
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum AuditSource {
    Internal,
    Https(IpAddr),
    Ldaps(IpAddr),
}

impl From<Source> for AuditSource {
    fn from(value: Source) -> Self {
        match value {
            Source::Internal => AuditSource::Internal,
            Source::Https(ip) => AuditSource::Https(ip),
            Source::Ldaps(ip) => AuditSource::Ldaps(ip),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum AuditEvent {
    AuthenticationDenied {
        source: AuditSource,
        uuid: Uuid,
        spn: String,
        #[serde(with = "time::serde::timestamp")]
        time: OffsetDateTime,
    },
    ApprovalEvent {
        source: AuditSource,
        event: ApprovalAuditEvent,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_audit_source_from_internal() {
        let source = Source::Internal;
        let audit_source = AuditSource::from(source);
        assert_eq!(audit_source, AuditSource::Internal);
    }

    #[test]
    fn test_audit_source_from_https() {
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let source = Source::Https(ip);
        let audit_source = AuditSource::from(source);
        assert_eq!(audit_source, AuditSource::Https(ip));
    }

    #[test]
    fn test_audit_source_from_ldaps() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let source = Source::Ldaps(ip);
        let audit_source = AuditSource::from(source);
        assert_eq!(audit_source, AuditSource::Ldaps(ip));
    }

    #[test]
    fn test_audit_event_authentication_denied_serde() {
        let event = AuditEvent::AuthenticationDenied {
            source: AuditSource::Https(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))),
            uuid: Uuid::nil(),
            spn: "testuser@example.com".to_string(),
            time: OffsetDateTime::UNIX_EPOCH,
        };
        let json = serde_json::to_string(&event).expect("Failed to serialize");
        assert!(json.contains("AuthenticationDenied"));
        assert!(json.contains("testuser@example.com"));

        let deserialized: AuditEvent = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_audit_event_equality() {
        let event1 = AuditEvent::AuthenticationDenied {
            source: AuditSource::Internal,
            uuid: Uuid::nil(),
            spn: "user@test".to_string(),
            time: OffsetDateTime::UNIX_EPOCH,
        };
        let event2 = AuditEvent::AuthenticationDenied {
            source: AuditSource::Internal,
            uuid: Uuid::nil(),
            spn: "user@test".to_string(),
            time: OffsetDateTime::UNIX_EPOCH,
        };
        assert_eq!(event1, event2);
    }
}
