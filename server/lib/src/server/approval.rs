use crate::prelude::*;
use kanidm_proto::v1::{
    ApprovalDecision, ApprovalDecisionAction, ApprovalDecisionRequest, ApprovalOperationType,
    ApprovalPattern, ApprovalPolicy, ApprovalPolicyCreateRequest, ApprovalRequest,
    ApprovalRequestState,
};
use std::collections::BTreeMap;
use time::OffsetDateTime;

impl QueryServerReadTransaction<'_> {
    pub fn approval_policy_list(
        &mut self,
        ident: &Identity,
    ) -> Result<Vec<ApprovalPolicy>, OperationError> {
        let filter = filter!(f_eq(Attribute::Class, EntryClass::ApprovalPolicy.into()));
        let entries = self.internal_search(filter)?;

        entries
            .into_iter()
            .map(|e| approval_policy_from_entry(&e))
            .collect()
    }

    pub fn approval_policy_get(
        &mut self,
        ident: &Identity,
        name: &str,
    ) -> Result<ApprovalPolicy, OperationError> {
        let filter = filter!(f_and!([
            f_eq(Attribute::Class, EntryClass::ApprovalPolicy.into()),
            f_eq(Attribute::ApprovalPolicyName, PartialValue::new_utf8s(name))
        ]));
        let entries = self.internal_search(filter)?;

        entries
            .first()
            .map(|e| approval_policy_from_entry(e))
            .ok_or(OperationError::NoMatchingEntries)
    }

    pub fn approval_request_list(
        &mut self,
        ident: &Identity,
        state: Option<ApprovalRequestState>,
    ) -> Result<Vec<ApprovalRequest>, OperationError> {
        let filter = match state {
            Some(s) => filter!(f_and!([
                f_eq(Attribute::Class, EntryClass::ApprovalRequest.into()),
                f_eq(
                    Attribute::ApprovalState,
                    PartialValue::new_utf8s(s.as_str())
                )
            ])),
            None => filter!(f_eq(Attribute::Class, EntryClass::ApprovalRequest.into())),
        };
        let entries = self.internal_search(filter)?;

        entries
            .into_iter()
            .map(|e| approval_request_from_entry(&e))
            .collect()
    }

    pub fn approval_request_get(
        &mut self,
        ident: &Identity,
        uuid: Uuid,
    ) -> Result<ApprovalRequest, OperationError> {
        let filter = filter!(f_and!([
            f_eq(Attribute::Class, EntryClass::ApprovalRequest.into()),
            f_eq(Attribute::Uuid, PartialValue::Uuid(uuid))
        ]));
        let entries = self.internal_search(filter)?;

        entries
            .first()
            .map(|e| approval_request_from_entry(e))
            .ok_or(OperationError::NoMatchingEntries)
    }
}

impl QueryServerWriteTransaction<'_> {
    pub fn approval_policy_create(
        &mut self,
        ident: &Identity,
        request: ApprovalPolicyCreateRequest,
    ) -> Result<(), OperationError> {
        let uuid = Uuid::new_v4();

        let approvers: Vec<Value> = request.approvers.iter().map(|u| Value::Refer(*u)).collect();

        let backup_approvers: Vec<Value> = request
            .backup_approvers
            .iter()
            .map(|u| Value::Refer(*u))
            .collect();

        let op_types: Vec<Value> = request
            .operation_types
            .iter()
            .map(|o| Value::new_utf8(o.to_string()))
            .collect();

        let mut entry = EntryInit::new();
        entry.set_ava(
            Attribute::Class,
            vec![EntryClass::ApprovalPolicy.into(), EntryClass::Object.into()],
        );
        entry.set_ava(Attribute::Uuid, vec![Value::Uuid(uuid)]);
        entry.set_ava(
            Attribute::ApprovalPolicyName,
            vec![Value::new_utf8(request.name.clone())],
        );
        entry.set_ava(
            Attribute::ApprovalPatternAttr,
            vec![Value::new_utf8(request.pattern.to_string())],
        );
        entry.set_ava(
            Attribute::ApprovalTimeout,
            vec![Value::Uint32(request.timeout_seconds)],
        );
        entry.set_ava(Attribute::Approver, approvers);

        if let Some(desc) = &request.description {
            entry.set_ava(Attribute::Description, vec![Value::new_utf8(desc.clone())]);
        }

        if !backup_approvers.is_empty() {
            entry.set_ava(Attribute::BackupApprover, backup_approvers);
        }

        if let Some(esc_timeout) = request.escalation_timeout_seconds {
            entry.set_ava(
                Attribute::EscalationTimeout,
                vec![Value::Uint32(esc_timeout)],
            );
        }

        entry.set_ava(Attribute::ApprovalOperationType, op_types);
        entry.set_ava(Attribute::ApprovalPolicyEnabled, vec![Value::Bool(true)]);

        let create_event = CreateEvent::new_internal(vec![entry]);
        self.create(&create_event)
    }

    pub fn approval_policy_delete(
        &mut self,
        ident: &Identity,
        name: &str,
    ) -> Result<(), OperationError> {
        let filter = filter!(f_and!([
            f_eq(Attribute::Class, EntryClass::ApprovalPolicy.into()),
            f_eq(Attribute::ApprovalPolicyName, PartialValue::new_utf8s(name))
        ]));
        let delete_event = DeleteEvent::new_internal(filter);
        self.delete(&delete_event)
    }

    pub fn approval_policy_enable(
        &mut self,
        ident: &Identity,
        name: &str,
        enable: bool,
    ) -> Result<(), OperationError> {
        let filter = filter!(f_and!([
            f_eq(Attribute::Class, EntryClass::ApprovalPolicy.into()),
            f_eq(Attribute::ApprovalPolicyName, PartialValue::new_utf8s(name))
        ]));
        let modlist =
            ModifyList::new_append(Attribute::ApprovalPolicyEnabled, vec![Value::Bool(enable)]);
        let modify_event = ModifyEvent::new_internal(filter, modlist);
        self.modify(&modify_event)
    }

    pub fn approval_request_decision(
        &mut self,
        ident: &Identity,
        request_uuid: Uuid,
        request: ApprovalDecisionRequest,
        current_time: OffsetDateTime,
    ) -> Result<(), OperationError> {
        let entries = self.internal_search(filter!(f_and!([
            f_eq(Attribute::Class, EntryClass::ApprovalRequest.into()),
            f_eq(Attribute::Uuid, PartialValue::Uuid(request_uuid))
        ])))?;

        let entry = entries.first().ok_or(OperationError::NoMatchingEntries)?;

        let current_state: ApprovalRequestState = entry
            .get_ava_single_iutf8(Attribute::ApprovalState)
            .and_then(|s| match s {
                "pending" => Some(ApprovalRequestState::Pending),
                "approved" => Some(ApprovalRequestState::Approved),
                "rejected" => Some(ApprovalRequestState::Rejected),
                "expired" => Some(ApprovalRequestState::Expired),
                "cancelled" => Some(ApprovalRequestState::Cancelled),
                "escalated" => Some(ApprovalRequestState::Escalated),
                _ => None,
            })
            .ok_or(OperationError::InvalidRequestState)?;

        if current_state != ApprovalRequestState::Pending {
            return Err(OperationError::InvalidRequestState);
        }

        let decision = ApprovalDecision {
            approver_uuid: ident.get_uuid().ok_or(OperationError::InvalidState)?,
            approver_spn: ident.get_uuid().to_string(),
            decision_time: current_time,
            action: match request.action {
                ApprovalDecisionAction::Approve => ApprovalDecisionAction::Approve,
                ApprovalDecisionAction::Reject => ApprovalDecisionAction::Reject,
            },
            comment: request.comment,
        };

        let decision_json =
            serde_json::to_string(&decision).map_err(|_| OperationError::InvalidState)?;

        let new_state = if decision.action == ApprovalDecisionAction::Reject {
            ApprovalRequestState::Rejected
        } else {
            ApprovalRequestState::Approved
        };

        let filter = filter!(f_eq(Attribute::Uuid, PartialValue::Uuid(request_uuid)));
        let modlist = ModifyList::new_list(vec![
            Modify::Present(Attribute::ApprovalDecision, Value::Json(decision_json)),
            Modify::Purged(Attribute::ApprovalState),
            Modify::Present(
                Attribute::ApprovalState,
                Value::new_iutf8(new_state.as_str()),
            ),
        ]);

        let modify_event = ModifyEvent::new_internal(filter, modlist);
        self.modify(&modify_event)
    }

    pub fn approval_request_cancel(
        &mut self,
        ident: &Identity,
        request_uuid: Uuid,
    ) -> Result<(), OperationError> {
        let filter = filter!(f_and!([
            f_eq(Attribute::Class, EntryClass::ApprovalRequest.into()),
            f_eq(Attribute::Uuid, PartialValue::Uuid(request_uuid))
        ]));
        let modlist = ModifyList::new_list(vec![
            Modify::Purged(Attribute::ApprovalState),
            Modify::Present(Attribute::ApprovalState, Value::new_iutf8("cancelled")),
        ]);
        let modify_event = ModifyEvent::new_internal(filter, modlist);
        self.modify(&modify_event)
    }
}

fn approval_policy_from_entry(
    entry: &Entry<EntrySealed, EntryCommitted>,
) -> Result<ApprovalPolicy, OperationError> {
    let uuid = entry.get_uuid();

    let name = entry
        .get_ava_single_utf8(Attribute::ApprovalPolicyName)
        .ok_or(OperationError::MissingAttribute(
            Attribute::ApprovalPolicyName,
        ))?;

    let description = entry.get_ava_single_utf8(Attribute::Description);

    let pattern = entry
        .get_ava_single_iutf8(Attribute::ApprovalPatternAttr)
        .and_then(|s| match s {
            "any_one" => Some(ApprovalPattern::AnyOne),
            "majority" => Some(ApprovalPattern::Majority),
            "all" => Some(ApprovalPattern::All),
            _ => None,
        })
        .ok_or(OperationError::InvalidValueState)?;

    let timeout_seconds = entry
        .get_ava_single_uint32(Attribute::ApprovalTimeout)
        .ok_or(OperationError::MissingAttribute(Attribute::ApprovalTimeout))?;

    let escalation_timeout_seconds = entry.get_ava_single_uint32(Attribute::EscalationTimeout);

    let enabled = entry
        .get_ava_single_bool(Attribute::ApprovalPolicyEnabled)
        .unwrap_or(true);

    let operation_types: Vec<ApprovalOperationType> = entry
        .get_ava_iter_iutf8(Attribute::ApprovalOperationType)
        .ok_or(OperationError::MissingAttribute(
            Attribute::ApprovalOperationType,
        ))?
        .filter_map(|s| match s {
            "create_high_privilege_entry" => Some(ApprovalOperationType::CreateHighPrivilegeEntry),
            "delete_high_privilege_entry" => Some(ApprovalOperationType::DeleteHighPrivilegeEntry),
            "modify_high_privilege_entry" => Some(ApprovalOperationType::ModifyHighPrivilegeEntry),
            "credential_reset_high_privilege" => {
                Some(ApprovalOperationType::CredentialResetHighPrivilege)
            }
            "privilege_grant" => Some(ApprovalOperationType::PrivilegeGrant),
            "privilege_revoke" => Some(ApprovalOperationType::PrivilegeRevoke),
            "schema_modify" => Some(ApprovalOperationType::SchemaModify),
            "access_control_modify" => Some(ApprovalOperationType::AccessControlModify),
            "domain_config_modify" => Some(ApprovalOperationType::DomainConfigModify),
            "key_provider_modify" => Some(ApprovalOperationType::KeyProviderModify),
            "sync_account_modify" => Some(ApprovalOperationType::SyncAccountModify),
            "oauth2_client_modify" => Some(ApprovalOperationType::OAuth2ClientModify),
            "application_modify" => Some(ApprovalOperationType::ApplicationModify),
            "group_membership_high_privilege" => {
                Some(ApprovalOperationType::GroupMembershipHighPrivilege)
            }
            _ => None,
        })
        .collect();

    let approvers: Vec<Uuid> = entry
        .get_ava_refer(Attribute::Approver)
        .unwrap_or_default()
        .into_iter()
        .collect();

    let backup_approvers: Vec<Uuid> = entry
        .get_ava_refer(Attribute::BackupApprover)
        .unwrap_or_default()
        .into_iter()
        .collect();

    Ok(ApprovalPolicy {
        uuid,
        name: name.to_string(),
        description: description.map(String::from),
        operation_types,
        approvers,
        backup_approvers,
        pattern,
        timeout_seconds,
        escalation_timeout_seconds,
        enabled,
    })
}

fn approval_request_from_entry(
    entry: &Entry<EntrySealed, EntryCommitted>,
) -> Result<ApprovalRequest, OperationError> {
    let uuid = entry.get_uuid();

    let policy_uuid = entry
        .get_ava_single_refer(Attribute::ApprovalRequestPolicy)
        .ok_or(OperationError::MissingAttribute(
            Attribute::ApprovalRequestPolicy,
        ))?;

    let operation_type = entry
        .get_ava_single_iutf8(Attribute::ApprovalOperationType)
        .and_then(|s| match s {
            "create_high_privilege_entry" => Some(ApprovalOperationType::CreateHighPrivilegeEntry),
            "delete_high_privilege_entry" => Some(ApprovalOperationType::DeleteHighPrivilegeEntry),
            "modify_high_privilege_entry" => Some(ApprovalOperationType::ModifyHighPrivilegeEntry),
            "credential_reset_high_privilege" => {
                Some(ApprovalOperationType::CredentialResetHighPrivilege)
            }
            "privilege_grant" => Some(ApprovalOperationType::PrivilegeGrant),
            "privilege_revoke" => Some(ApprovalOperationType::PrivilegeRevoke),
            "schema_modify" => Some(ApprovalOperationType::SchemaModify),
            "access_control_modify" => Some(ApprovalOperationType::AccessControlModify),
            "domain_config_modify" => Some(ApprovalOperationType::DomainConfigModify),
            "key_provider_modify" => Some(ApprovalOperationType::KeyProviderModify),
            "sync_account_modify" => Some(ApprovalOperationType::SyncAccountModify),
            "oauth2_client_modify" => Some(ApprovalOperationType::OAuth2ClientModify),
            "application_modify" => Some(ApprovalOperationType::ApplicationModify),
            "group_membership_high_privilege" => {
                Some(ApprovalOperationType::GroupMembershipHighPrivilege)
            }
            _ => None,
        })
        .ok_or(OperationError::InvalidValueState)?;

    let target_uuid = entry
        .get_ava_single_refer(Attribute::ApprovalTarget)
        .ok_or(OperationError::MissingAttribute(Attribute::ApprovalTarget))?;

    let requestor_uuid = entry
        .get_ava_single_refer(Attribute::ApprovalRequestor)
        .ok_or(OperationError::MissingAttribute(
            Attribute::ApprovalRequestor,
        ))?;

    let created_at = entry
        .get_ava_single_datetime(Attribute::CreatedAt)
        .ok_or(OperationError::MissingAttribute(Attribute::CreatedAt))?;

    let expires_at = entry.get_ava_single_datetime(Attribute::ApprovalExpires);

    let state = entry
        .get_ava_single_iutf8(Attribute::ApprovalState)
        .and_then(|s| match s {
            "pending" => Some(ApprovalRequestState::Pending),
            "approved" => Some(ApprovalRequestState::Approved),
            "rejected" => Some(ApprovalRequestState::Rejected),
            "expired" => Some(ApprovalRequestState::Expired),
            "cancelled" => Some(ApprovalRequestState::Cancelled),
            "escalated" => Some(ApprovalRequestState::Escalated),
            _ => None,
        })
        .ok_or(OperationError::InvalidValueState)?;

    let decisions: Vec<ApprovalDecision> = entry
        .get_ava_iter_json(Attribute::ApprovalDecision)
        .unwrap_or_default()
        .filter_map(|json| serde_json::from_str(&json).ok())
        .collect();

    let escalation_level = entry
        .get_ava_single_uint32(Attribute::ApprovalEscalationLevel)
        .unwrap_or(0);

    let required_decisions = 1;

    let operation_details: BTreeMap<String, String> = entry
        .get_ava_iter_json(Attribute::ApprovalOperationDetails)
        .unwrap_or_default()
        .filter_map(|json| serde_json::from_str(&json).ok())
        .collect();

    Ok(ApprovalRequest {
        uuid,
        policy_uuid,
        policy_name: String::new(),
        operation_type,
        target_uuid,
        target_spn: target_uuid.to_string(),
        requestor_uuid,
        requestor_spn: requestor_uuid.to_string(),
        created_at,
        expires_at,
        state,
        decisions,
        escalation_level,
        required_decisions,
        operation_details,
    })
}
