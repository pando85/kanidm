//! Kanidm Version 1
//!
//! Items defined in this module will remain stable, or change in ways that are forward
//! compatible with newer releases.

#![allow(non_upper_case_globals)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;

mod auth;
mod message;
mod unix;

pub use self::auth::*;
pub use self::message::*;
pub use self::unix::*;

/// The type of Account in use.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, ToSchema)]
pub enum AccountType {
    Person,
    ServiceAccount,
}

impl Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AccountType::Person => "person",
            AccountType::ServiceAccount => "service_account",
        })
    }
}

/* ===== higher level types ===== */
// These are all types that are conceptually layers on top of entry and
// friends. They allow us to process more complex requests and provide
// domain specific fields for the purposes of IDM, over the normal
// entry/ava/filter types. These related deeply to schema.

/// The current purpose of a User Auth Token. It may be read-only, read-write
/// or privilege capable (able to step up to read-write after re-authentication).
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum UatPurposeStatus {
    ReadOnly,
    ReadWrite,
    PrivilegeCapable,
}

/// The expiry of the User Auth Token.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum UatStatusState {
    #[serde(with = "time::serde::timestamp")]
    ExpiresAt(time::OffsetDateTime),
    NeverExpires,
    Revoked,
}

impl fmt::Display for UatStatusState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UatStatusState::ExpiresAt(odt) => write!(f, "expires at {odt}"),
            UatStatusState::NeverExpires => write!(f, "never expires"),
            UatStatusState::Revoked => write!(f, "revoked"),
        }
    }
}

/// The status of a User Auth Token
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct UatStatus {
    pub account_id: Uuid,
    pub session_id: Uuid,
    pub state: UatStatusState,
    #[serde(with = "time::serde::timestamp")]
    pub issued_at: time::OffsetDateTime,
    pub purpose: UatPurposeStatus,
}

impl fmt::Display for UatStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "account_id: {}", self.account_id)?;
        writeln!(f, "session_id: {}", self.session_id)?;
        writeln!(f, "state: {}", self.state)?;
        writeln!(f, "issued_at: {}", self.issued_at)?;
        match &self.purpose {
            UatPurposeStatus::ReadOnly => writeln!(f, "purpose: read only")?,
            UatPurposeStatus::ReadWrite => writeln!(f, "purpose: read write")?,
            UatPurposeStatus::PrivilegeCapable => writeln!(f, "purpose: privilege capable")?,
        }
        Ok(())
    }
}

/// A request to generate a new API token for a service account
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct ApiTokenGenerate {
    pub label: String,
    #[serde(with = "time::serde::timestamp::option")]
    pub expiry: Option<time::OffsetDateTime>,
    pub read_write: bool,
    #[serde(default)]
    pub compact: bool,
}

/* ===== low level proto types ===== */

/// A limited view of an entry in Kanidm.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct Entry {
    pub attrs: BTreeMap<String, Vec<String>>,
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "---")?;
        self.attrs
            .iter()
            .try_for_each(|(k, vs)| vs.iter().try_for_each(|v| writeln!(f, "{k}: {v}")))
    }
}

/// A response to a whoami request
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
pub struct WhoamiResponse {
    // Should we just embed the entry? Or destructure it?
    pub youare: Entry,
}

impl WhoamiResponse {
    pub fn new(youare: Entry) -> Self {
        WhoamiResponse { youare }
    }
}

// Simple string value provision.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SingleStringRequest {
    pub value: String,
}

impl SingleStringRequest {
    pub fn new(s: String) -> Self {
        SingleStringRequest { value: s }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalPattern {
    AnyOne,
    Majority,
    All,
}

impl fmt::Display for ApprovalPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApprovalPattern::AnyOne => f.write_str("any_one"),
            ApprovalPattern::Majority => f.write_str("majority"),
            ApprovalPattern::All => f.write_str("all"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "lowercase")]
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

impl fmt::Display for ApprovalOperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApprovalOperationType::CreateHighPrivilegeEntry => {
                f.write_str("create_high_privilege_entry")
            }
            ApprovalOperationType::DeleteHighPrivilegeEntry => {
                f.write_str("delete_high_privilege_entry")
            }
            ApprovalOperationType::ModifyHighPrivilegeEntry => {
                f.write_str("modify_high_privilege_entry")
            }
            ApprovalOperationType::CredentialResetHighPrivilege => {
                f.write_str("credential_reset_high_privilege")
            }
            ApprovalOperationType::PrivilegeGrant => f.write_str("privilege_grant"),
            ApprovalOperationType::PrivilegeRevoke => f.write_str("privilege_revoke"),
            ApprovalOperationType::SchemaModify => f.write_str("schema_modify"),
            ApprovalOperationType::AccessControlModify => f.write_str("access_control_modify"),
            ApprovalOperationType::DomainConfigModify => f.write_str("domain_config_modify"),
            ApprovalOperationType::KeyProviderModify => f.write_str("key_provider_modify"),
            ApprovalOperationType::SyncAccountModify => f.write_str("sync_account_modify"),
            ApprovalOperationType::OAuth2ClientModify => f.write_str("oauth2_client_modify"),
            ApprovalOperationType::ApplicationModify => f.write_str("application_modify"),
            ApprovalOperationType::GroupMembershipHighPrivilege => {
                f.write_str("group_membership_high_privilege")
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalRequestState {
    Pending,
    Approved,
    Rejected,
    Expired,
    Cancelled,
    Escalated,
}

impl fmt::Display for ApprovalRequestState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApprovalRequestState::Pending => f.write_str("pending"),
            ApprovalRequestState::Approved => f.write_str("approved"),
            ApprovalRequestState::Rejected => f.write_str("rejected"),
            ApprovalRequestState::Expired => f.write_str("expired"),
            ApprovalRequestState::Cancelled => f.write_str("cancelled"),
            ApprovalRequestState::Escalated => f.write_str("escalated"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalDecisionAction {
    Approve,
    Reject,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ApprovalDecision {
    pub approver_uuid: Uuid,
    pub approver_spn: String,
    #[serde(with = "time::serde::timestamp")]
    pub decision_time: time::OffsetDateTime,
    pub action: ApprovalDecisionAction,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ApprovalRequest {
    pub uuid: Uuid,
    pub policy_uuid: Uuid,
    pub policy_name: String,
    pub operation_type: ApprovalOperationType,
    pub target_uuid: Uuid,
    pub target_spn: String,
    pub requestor_uuid: Uuid,
    pub requestor_spn: String,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::timestamp::option")]
    pub expires_at: Option<time::OffsetDateTime>,
    pub state: ApprovalRequestState,
    pub decisions: Vec<ApprovalDecision>,
    pub escalation_level: u32,
    pub required_decisions: u32,
    pub operation_details: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApprovalPolicyCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub operation_types: Vec<ApprovalOperationType>,
    pub approvers: Vec<Uuid>,
    pub backup_approvers: Vec<Uuid>,
    pub pattern: ApprovalPattern,
    pub timeout_seconds: u32,
    pub escalation_timeout_seconds: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApprovalDecisionRequest {
    pub action: ApprovalDecisionAction,
    pub comment: Option<String>,
}
