use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApprovalPattern {
    AnyOne,
    Majority,
    All,
}

impl ApprovalPattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AnyOne => "any_one",
            Self::Majority => "majority",
            Self::All => "all",
        }
    }
}

impl std::fmt::Display for ApprovalPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApprovalOperationType {
    CreateHighPrivilegeEntry,
    DeleteHighPrivilegeEntry,
    ModifyHighPrivilegeEntry,
    CredentialResetHighPrivilege,
    PrivilegeGrant,
    PrivilegeRevoke,
    SchemaModify,
    AccessControlModify,
    DomainConfigModify,
    KeyProviderModify,
    SyncAccountModify,
    OAuth2ClientModify,
    ApplicationModify,
    GroupMembershipHighPrivilege,
}

impl ApprovalOperationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CreateHighPrivilegeEntry => "create_high_privilege_entry",
            Self::DeleteHighPrivilegeEntry => "delete_high_privilege_entry",
            Self::ModifyHighPrivilegeEntry => "modify_high_privilege_entry",
            Self::CredentialResetHighPrivilege => "credential_reset_high_privilege",
            Self::PrivilegeGrant => "privilege_grant",
            Self::PrivilegeRevoke => "privilege_revoke",
            Self::SchemaModify => "schema_modify",
            Self::AccessControlModify => "access_control_modify",
            Self::DomainConfigModify => "domain_config_modify",
            Self::KeyProviderModify => "key_provider_modify",
            Self::SyncAccountModify => "sync_account_modify",
            Self::OAuth2ClientModify => "oauth2_client_modify",
            Self::ApplicationModify => "application_modify",
            Self::GroupMembershipHighPrivilege => "group_membership_high_privilege",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalRequestState {
    Pending,
    Approved,
    Rejected,
    Expired,
    Cancelled,
    Escalated,
}

impl ApprovalRequestState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
            Self::Escalated => "escalated",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecisionAction {
    Approve,
    Reject,
}

impl ApprovalDecisionAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Reject => "reject",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApprovalDecision {
    pub approver_uuid: Uuid,
    pub approver_spn: String,
    #[serde(with = "time::serde::timestamp")]
    pub decision_time: OffsetDateTime,
    pub action: ApprovalDecisionAction,
    pub comment: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApprovalRequest {
    pub uuid: Uuid,
    pub policy_uuid: Uuid,
    pub operation_type: ApprovalOperationType,
    pub target_uuid: Uuid,
    pub target_spn: String,
    pub requestor_uuid: Uuid,
    pub requestor_spn: String,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp::option")]
    pub expires_at: Option<OffsetDateTime>,
    pub state: ApprovalRequestState,
    pub decisions: Vec<ApprovalDecision>,
    pub escalation_level: u32,
    pub required_decisions: u32,
    pub operation_details: BTreeMap<String, String>,
}

impl ApprovalRequest {
    pub fn is_expired(&self, current_time: OffsetDateTime) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < current_time
        } else {
            false
        }
    }

    pub fn approve_count(&self) -> u32 {
        self.decisions
            .iter()
            .filter(|d| d.action == ApprovalDecisionAction::Approve)
            .count() as u32
    }

    pub fn reject_count(&self) -> u32 {
        self.decisions
            .iter()
            .filter(|d| d.action == ApprovalDecisionAction::Reject)
            .count() as u32
    }

    pub fn has_decision_from(&self, approver_uuid: &Uuid) -> bool {
        self.decisions
            .iter()
            .any(|d| d.approver_uuid == *approver_uuid)
    }

    pub fn check_satisfied(&self, pattern: ApprovalPattern, total_approvers: u32) -> bool {
        let approve_count = self.approve_count();
        let reject_count = self.reject_count();

        match pattern {
            ApprovalPattern::AnyOne => approve_count >= 1,
            ApprovalPattern::Majority => approve_count > (total_approvers / 2),
            ApprovalPattern::All => approve_count >= total_approvers && reject_count == 0,
        }
    }

    pub fn check_rejected(&self, pattern: ApprovalPattern, total_approvers: u32) -> bool {
        let reject_count = self.reject_count();

        match pattern {
            ApprovalPattern::AnyOne => reject_count >= 1,
            ApprovalPattern::Majority => reject_count > (total_approvers / 2),
            ApprovalPattern::All => reject_count >= 1,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApprovalPolicy {
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub operation_types: Vec<ApprovalOperationType>,
    pub approvers: Vec<Uuid>,
    pub backup_approvers: Vec<Uuid>,
    pub pattern: ApprovalPattern,
    pub timeout_seconds: u32,
    pub escalation_timeout_seconds: Option<u32>,
    pub enabled: bool,
}

impl ApprovalPolicy {
    pub fn total_approvers(&self) -> u32 {
        self.approvers.len() as u32
    }

    pub fn get_current_approvers(&self, escalation_level: u32) -> Vec<Uuid> {
        if escalation_level == 0 || self.backup_approvers.is_empty() {
            self.approvers.clone()
        } else {
            self.backup_approvers.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ApprovalAuditAction {
    RequestCreated,
    DecisionSubmitted,
    RequestApproved,
    RequestRejected,
    RequestExpired,
    RequestCancelled,
    RequestEscalated,
    PolicyCreated,
    PolicyModified,
    PolicyDeleted,
    PolicyEnabled,
    PolicyDisabled,
}

impl ApprovalAuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RequestCreated => "request_created",
            Self::DecisionSubmitted => "decision_submitted",
            Self::RequestApproved => "request_approved",
            Self::RequestRejected => "request_rejected",
            Self::RequestExpired => "request_expired",
            Self::RequestCancelled => "request_cancelled",
            Self::RequestEscalated => "request_escalated",
            Self::PolicyCreated => "policy_created",
            Self::PolicyModified => "policy_modified",
            Self::PolicyDeleted => "policy_deleted",
            Self::PolicyEnabled => "policy_enabled",
            Self::PolicyDisabled => "policy_disabled",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ApprovalAuditEvent {
    pub action: ApprovalAuditAction,
    pub request_uuid: Option<Uuid>,
    pub policy_uuid: Option<Uuid>,
    pub actor_uuid: Uuid,
    pub actor_spn: String,
    #[serde(with = "time::serde::timestamp")]
    pub timestamp: OffsetDateTime,
    pub details: BTreeMap<String, String>,
}

pub struct ApprovalRequestCreateEvent {
    pub ident: Identity,
    pub policy_uuid: Uuid,
    pub operation_type: ApprovalOperationType,
    pub target_uuid: Uuid,
    pub operation_details: BTreeMap<String, String>,
}

pub struct ApprovalDecisionEvent {
    pub ident: Identity,
    pub request_uuid: Uuid,
    pub action: ApprovalDecisionAction,
    pub comment: Option<String>,
}

pub struct ApprovalRequestCancelEvent {
    pub ident: Identity,
    pub request_uuid: Uuid,
}

pub struct ApprovalPolicyCreateEvent {
    pub ident: Identity,
    pub name: String,
    pub description: Option<String>,
    pub operation_types: Vec<ApprovalOperationType>,
    pub approvers: Vec<Uuid>,
    pub backup_approvers: Vec<Uuid>,
    pub pattern: ApprovalPattern,
    pub timeout_seconds: u32,
    pub escalation_timeout_seconds: Option<u32>,
}

pub struct ApprovalPolicyModifyEvent {
    pub ident: Identity,
    pub policy_uuid: Uuid,
    pub description: Option<String>,
    pub operation_types: Vec<ApprovalOperationType>,
    pub approvers: Vec<Uuid>,
    pub backup_approvers: Vec<Uuid>,
    pub pattern: ApprovalPattern,
    pub timeout_seconds: u32,
    pub escalation_timeout_seconds: Option<u32>,
}

pub struct ApprovalPolicyDeleteEvent {
    pub ident: Identity,
    pub policy_uuid: Uuid,
}

pub struct ApprovalPolicyEnableEvent {
    pub ident: Identity,
    pub policy_uuid: Uuid,
    pub enable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration as TimeDuration;

    fn make_approval_request() -> ApprovalRequest {
        ApprovalRequest {
            uuid: Uuid::new_v4(),
            policy_uuid: Uuid::new_v4(),
            operation_type: ApprovalOperationType::CreateHighPrivilegeEntry,
            target_uuid: Uuid::new_v4(),
            target_spn: "admin@example.com".to_string(),
            requestor_uuid: Uuid::new_v4(),
            requestor_spn: "requestor@example.com".to_string(),
            created_at: OffsetDateTime::UNIX_EPOCH,
            expires_at: Some(OffsetDateTime::UNIX_EPOCH + TimeDuration::hours(24)),
            state: ApprovalRequestState::Pending,
            decisions: vec![],
            escalation_level: 0,
            required_decisions: 1,
            operation_details: BTreeMap::new(),
        }
    }

    fn make_approval_decision(action: ApprovalDecisionAction) -> ApprovalDecision {
        ApprovalDecision {
            approver_uuid: Uuid::new_v4(),
            approver_spn: "approver@example.com".to_string(),
            decision_time: OffsetDateTime::UNIX_EPOCH,
            action,
            comment: Some("Looks good".to_string()),
        }
    }

    #[test]
    fn test_approval_pattern_as_str() {
        assert_eq!(ApprovalPattern::AnyOne.as_str(), "any_one");
        assert_eq!(ApprovalPattern::Majority.as_str(), "majority");
        assert_eq!(ApprovalPattern::All.as_str(), "all");
    }

    #[test]
    fn test_approval_pattern_display() {
        assert_eq!(ApprovalPattern::AnyOne.to_string(), "any_one");
        assert_eq!(ApprovalPattern::Majority.to_string(), "majority");
        assert_eq!(ApprovalPattern::All.to_string(), "all");
    }

    #[test]
    fn test_approval_operation_type_as_str() {
        assert_eq!(
            ApprovalOperationType::CreateHighPrivilegeEntry.as_str(),
            "create_high_privilege_entry"
        );
        assert_eq!(
            ApprovalOperationType::SchemaModify.as_str(),
            "schema_modify"
        );
        assert_eq!(
            ApprovalOperationType::OAuth2ClientModify.as_str(),
            "oauth2_client_modify"
        );
    }

    #[test]
    fn test_approval_request_state_as_str() {
        assert_eq!(ApprovalRequestState::Pending.as_str(), "pending");
        assert_eq!(ApprovalRequestState::Approved.as_str(), "approved");
        assert_eq!(ApprovalRequestState::Rejected.as_str(), "rejected");
        assert_eq!(ApprovalRequestState::Expired.as_str(), "expired");
        assert_eq!(ApprovalRequestState::Cancelled.as_str(), "cancelled");
        assert_eq!(ApprovalRequestState::Escalated.as_str(), "escalated");
    }

    #[test]
    fn test_approval_decision_action_as_str() {
        assert_eq!(ApprovalDecisionAction::Approve.as_str(), "approve");
        assert_eq!(ApprovalDecisionAction::Reject.as_str(), "reject");
    }

    #[test]
    fn test_approval_request_is_expired() {
        let mut request = make_approval_request();

        // Not expired yet
        let current = OffsetDateTime::UNIX_EPOCH + TimeDuration::hours(1);
        assert!(!request.is_expired(current));

        // Past expiry
        let current = OffsetDateTime::UNIX_EPOCH + TimeDuration::hours(48);
        assert!(request.is_expired(current));

        // No expiry - never expired
        request.expires_at = None;
        assert!(!request.is_expired(current));
    }

    #[test]
    fn test_approval_request_approve_count() {
        let mut request = make_approval_request();
        assert_eq!(request.approve_count(), 0);

        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Approve));
        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Approve));
        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Reject));
        assert_eq!(request.approve_count(), 2);
    }

    #[test]
    fn test_approval_request_reject_count() {
        let mut request = make_approval_request();
        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Approve));
        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Reject));
        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Reject));
        assert_eq!(request.reject_count(), 2);
    }

    #[test]
    fn test_approval_request_has_decision_from() {
        let mut request = make_approval_request();
        let approver = Uuid::new_v4();
        request.decisions.push(ApprovalDecision {
            approver_uuid: approver,
            approver_spn: "approver@example.com".to_string(),
            decision_time: OffsetDateTime::UNIX_EPOCH,
            action: ApprovalDecisionAction::Approve,
            comment: None,
        });

        assert!(request.has_decision_from(&approver));
        assert!(!request.has_decision_from(&Uuid::new_v4()));
    }

    #[test]
    fn test_approval_check_satisfied_anyone() {
        let mut request = make_approval_request();
        assert!(!request.check_satisfied(ApprovalPattern::AnyOne, 3));

        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Approve));
        assert!(request.check_satisfied(ApprovalPattern::AnyOne, 3));
    }

    #[test]
    fn test_approval_check_satisfied_majority() {
        let mut request = make_approval_request();
        // Need > 3/2 = > 1, so need 2 approvals
        assert!(!request.check_satisfied(ApprovalPattern::Majority, 3));

        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Approve));
        assert!(!request.check_satisfied(ApprovalPattern::Majority, 3));

        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Approve));
        assert!(request.check_satisfied(ApprovalPattern::Majority, 3));
    }

    #[test]
    fn test_approval_check_satisfied_all() {
        let mut request = make_approval_request();
        assert!(!request.check_satisfied(ApprovalPattern::All, 2));

        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Approve));
        assert!(!request.check_satisfied(ApprovalPattern::All, 2));

        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Approve));
        assert!(request.check_satisfied(ApprovalPattern::All, 2));

        // Any reject means not satisfied
        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Reject));
        assert!(!request.check_satisfied(ApprovalPattern::All, 2));
    }

    #[test]
    fn test_approval_check_rejected_anyone() {
        let mut request = make_approval_request();
        assert!(!request.check_rejected(ApprovalPattern::AnyOne, 3));

        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Reject));
        assert!(request.check_rejected(ApprovalPattern::AnyOne, 3));
    }

    #[test]
    fn test_approval_check_rejected_majority() {
        let mut request = make_approval_request();
        // Need > 3/2 = > 1, so need 2 rejections
        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Reject));
        assert!(!request.check_rejected(ApprovalPattern::Majority, 3));

        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Reject));
        assert!(request.check_rejected(ApprovalPattern::Majority, 3));
    }

    #[test]
    fn test_approval_check_rejected_all() {
        let mut request = make_approval_request();
        // Any single reject means rejected for All pattern
        request
            .decisions
            .push(make_approval_decision(ApprovalDecisionAction::Reject));
        assert!(request.check_rejected(ApprovalPattern::All, 3));
    }

    #[test]
    fn test_approval_policy_total_approvers() {
        let policy = ApprovalPolicy {
            uuid: Uuid::new_v4(),
            name: "test_policy".to_string(),
            description: None,
            operation_types: vec![],
            approvers: vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()],
            backup_approvers: vec![Uuid::new_v4()],
            pattern: ApprovalPattern::Majority,
            timeout_seconds: 3600,
            escalation_timeout_seconds: None,
            enabled: true,
        };
        assert_eq!(policy.total_approvers(), 3);
    }

    #[test]
    fn test_approval_policy_get_current_approvers() {
        let primary = vec![Uuid::new_v4(), Uuid::new_v4()];
        let backup = vec![Uuid::new_v4()];
        let policy = ApprovalPolicy {
            uuid: Uuid::new_v4(),
            name: "test".to_string(),
            description: None,
            operation_types: vec![],
            approvers: primary.clone(),
            backup_approvers: backup.clone(),
            pattern: ApprovalPattern::AnyOne,
            timeout_seconds: 3600,
            escalation_timeout_seconds: None,
            enabled: true,
        };

        // Level 0 - primary approvers
        assert_eq!(policy.get_current_approvers(0), primary);
        // Level 1 - backup approvers
        assert_eq!(policy.get_current_approvers(1), backup);
        // No backup_approvers - always primary
        let policy_no_backup = ApprovalPolicy {
            uuid: Uuid::new_v4(),
            name: "test".to_string(),
            description: None,
            operation_types: vec![],
            approvers: primary.clone(),
            backup_approvers: vec![],
            pattern: ApprovalPattern::AnyOne,
            timeout_seconds: 3600,
            escalation_timeout_seconds: None,
            enabled: true,
        };
        assert_eq!(policy_no_backup.get_current_approvers(5), primary);
    }

    #[test]
    fn test_approval_audit_action_as_str() {
        assert_eq!(
            ApprovalAuditAction::RequestCreated.as_str(),
            "request_created"
        );
        assert_eq!(
            ApprovalAuditAction::RequestEscalated.as_str(),
            "request_escalated"
        );
        assert_eq!(
            ApprovalAuditAction::PolicyDisabled.as_str(),
            "policy_disabled"
        );
    }

    #[test]
    fn test_approval_request_serde_roundtrip() {
        let request = make_approval_request();
        let json = serde_json::to_string(&request).expect("Failed to serialize");
        let deserialized: ApprovalRequest =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(request.uuid, deserialized.uuid);
        assert_eq!(request.state, deserialized.state);
    }

    #[test]
    fn test_approval_decision_serde_roundtrip() {
        let decision = make_approval_decision(ApprovalDecisionAction::Approve);
        let json = serde_json::to_string(&decision).expect("Failed to serialize");
        let deserialized: ApprovalDecision =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(decision.action, deserialized.action);
        assert_eq!(decision.approver_spn, deserialized.approver_spn);
    }

    #[test]
    fn test_approval_policy_serde_roundtrip() {
        let policy = ApprovalPolicy {
            uuid: Uuid::new_v4(),
            name: "security_policy".to_string(),
            description: Some("Requires approval for privileged operations".to_string()),
            operation_types: vec![
                ApprovalOperationType::CreateHighPrivilegeEntry,
                ApprovalOperationType::SchemaModify,
            ],
            approvers: vec![Uuid::new_v4()],
            backup_approvers: vec![Uuid::new_v4()],
            pattern: ApprovalPattern::Majority,
            timeout_seconds: 7200,
            escalation_timeout_seconds: Some(3600),
            enabled: true,
        };
        let json = serde_json::to_string(&policy).expect("Failed to serialize");
        let deserialized: ApprovalPolicy =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(policy.name, deserialized.name);
        assert_eq!(policy.pattern, deserialized.pattern);
        assert_eq!(policy.timeout_seconds, deserialized.timeout_seconds);
    }

    #[test]
    fn test_approval_audit_event_serde() {
        let event = ApprovalAuditEvent {
            action: ApprovalAuditAction::RequestApproved,
            request_uuid: Some(Uuid::new_v4()),
            policy_uuid: Some(Uuid::new_v4()),
            actor_uuid: Uuid::new_v4(),
            actor_spn: "approver@example.com".to_string(),
            timestamp: OffsetDateTime::UNIX_EPOCH,
            details: BTreeMap::new(),
        };
        let json = serde_json::to_string(&event).expect("Failed to serialize");
        let deserialized: ApprovalAuditEvent =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(event.action, deserialized.action);
        assert_eq!(event.actor_spn, deserialized.actor_spn);
    }
}
