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
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < OffsetDateTime::now_utc()
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

#[derive(Serialize, Deserialize, Debug)]
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
