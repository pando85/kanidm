use crate::{ApprovalOpt, ApprovalPolicyOpt, ApprovalRequestOpt, KanidmClientParser};
use kanidm_client::KanidmClient;
use kanidm_proto::v1::{
    ApprovalOperationType, ApprovalPattern, ApprovalPolicyCreateRequest, ApprovalRequestState,
};

impl ApprovalOpt {
    pub async fn exec(&self, opt: KanidmClientParser) {
        let client = opt.to_client(crate::common::OpType::Write).await;
        match self {
            ApprovalOpt::Policy { commands } => commands.exec(&opt, &client).await,
            ApprovalOpt::Request { commands } => commands.exec(&opt, &client).await,
        }
    }
}

impl ApprovalPolicyOpt {
    pub async fn exec(&self, opt: &KanidmClientParser, client: &KanidmClient) {
        match self {
            ApprovalPolicyOpt::List => {
                let policies = match client.approval_policy_list().await {
                    Ok(p) => p,
                    Err(e) => {
                        error!("Failed to list approval policies: {:?}", e);
                        return;
                    }
                };

                if policies.is_empty() {
                    println!("No approval policies configured");
                    return;
                }

                for policy in policies {
                    println!("Policy: {} ({})", policy.name, policy.uuid);
                    if let Some(desc) = &policy.description {
                        println!("  Description: {}", desc);
                    }
                    println!("  Enabled: {}", policy.enabled);
                    println!("  Pattern: {}", policy.pattern);
                    println!("  Timeout: {}s", policy.timeout_seconds);
                    println!("  Operations: {:?}", policy.operation_types);
                    println!("  Approvers: {:?}", policy.approvers);
                    if !policy.backup_approvers.is_empty() {
                        println!("  Backup Approvers: {:?}", policy.backup_approvers);
                    }
                    println!();
                }
            }
            ApprovalPolicyOpt::Get { name } => {
                let policy = match client.approval_policy_get(name).await {
                    Ok(p) => p,
                    Err(e) => {
                        error!("Failed to get approval policy '{}': {:?}", name, e);
                        return;
                    }
                };

                println!("Policy: {} ({})", policy.name, policy.uuid);
                if let Some(desc) = &policy.description {
                    println!("Description: {}", desc);
                }
                println!("Enabled: {}", policy.enabled);
                println!("Pattern: {}", policy.pattern);
                println!("Timeout: {}s", policy.timeout_seconds);
                if let Some(esc_timeout) = policy.escalation_timeout_seconds {
                    println!("Escalation Timeout: {}s", esc_timeout);
                }
                println!("Operations: {:?}", policy.operation_types);
                println!("Approvers: {:?}", policy.approvers);
                println!("Backup Approvers: {:?}", policy.backup_approvers);
            }
            ApprovalPolicyOpt::Create {
                name,
                description,
                operation_types,
                approvers,
                backup_approvers,
                pattern,
                timeout_seconds,
                escalation_timeout_seconds,
            } => {
                let op_types: Vec<ApprovalOperationType> = operation_types
                    .iter()
                    .filter_map(|s| {
                        match s.to_lowercase().replace('_', "-").replace('-', "_") {
                            s if s == "create_high_privilege_entry" => Some(ApprovalOperationType::CreateHighPrivilegeEntry),
                            s if s == "delete_high_privilege_entry" => Some(ApprovalOperationType::DeleteHighPrivilegeEntry),
                            s if s == "modify_high_privilege_entry" => Some(ApprovalOperationType::ModifyHighPrivilegeEntry),
                            s if s == "credential_reset_high_privilege" => Some(ApprovalOperationType::CredentialResetHighPrivilege),
                            s if s == "privilege_grant" => Some(ApprovalOperationType::PrivilegeGrant),
                            s if s == "privilege_revoke" => Some(ApprovalOperationType::PrivilegeRevoke),
                            s if s == "schema_modify" => Some(ApprovalOperationType::SchemaModify),
                            s if s == "access_control_modify" => Some(ApprovalOperationType::AccessControlModify),
                            s if s == "domain_config_modify" => Some(ApprovalOperationType::DomainConfigModify),
                            s if s == "key_provider_modify" => Some(ApprovalOperationType::KeyProviderModify),
                            s if s == "sync_account_modify" => Some(ApprovalOperationType::SyncAccountModify),
                            s if s == "oauth2_client_modify" => Some(ApprovalOperationType::OAuth2ClientModify),
                            s if s == "application_modify" => Some(ApprovalOperationType::ApplicationModify),
                            s if s == "group_membership_high_privilege" => Some(ApprovalOperationType::GroupMembershipHighPrivilege),
                            _ => {
                                warn!("Unknown operation type: {}", s);
                                None
                            }
                        }
                    })
                    .collect();

                if op_types.is_empty() {
                    error!("No valid operation types provided");
                    return;
                }

                let approver_uuids: Vec<uuid::Uuid> = approvers
                    .iter()
                    .filter_map(|s| uuid::Uuid::parse_str(s).ok())
                    .collect();

                if approver_uuids.is_empty() {
                    error!("No valid approver UUIDs provided");
                    return;
                }

                let backup_approver_uuids: Vec<uuid::Uuid> = backup_approvers
                    .iter()
                    .filter_map(|s| uuid::Uuid::parse_str(s).ok())
                    .collect();

                let approval_pattern = match pattern.to_lowercase() {
                    "any_one" | "anyone" => ApprovalPattern::AnyOne,
                    "majority" => ApprovalPattern::Majority,
                    "all" => ApprovalPattern::All,
                    _ => {
                        error!("Invalid pattern: {}. Must be 'any_one', 'majority', or 'all'", pattern);
                        return;
                    }
                };

                let request = ApprovalPolicyCreateRequest {
                    name: name.to_string(),
                    description: description.as_deref().map(String::from),
                    operation_types: op_types,
                    approvers: approver_uuids,
                    backup_approvers: backup_approver_uuids,
                    pattern: approval_pattern,
                    timeout_seconds: *timeout_seconds,
                    escalation_timeout_seconds: *escalation_timeout_seconds,
                };

                match client.approval_policy_create(&request).await {
                    Ok(_) => println!("Approval policy '{}' created successfully", name),
                    Err(e) => error!("Failed to create approval policy '{}': {:?}", name, e),
                }
            }
            ApprovalPolicyOpt::Delete { name } => {
                match client.approval_policy_delete(name).await {
                    Ok(_) => println!("Approval policy '{}' deleted successfully", name),
                    Err(e) => error!("Failed to delete approval policy '{}': {:?}", name, e),
                }
            }
            ApprovalPolicyOpt::Enable { name } => {
                match client.approval_policy_enable(name).await {
                    Ok(_) => println!("Approval policy '{}' enabled successfully", name),
                    Err(e) => error!("Failed to enable approval policy '{}': {:?}", name, e),
                }
            }
            ApprovalPolicyOpt::Disable { name } => {
                match client.approval_policy_disable(name).await {
                    Ok(_) => println!("Approval policy '{}' disabled successfully", name),
                    Err(e) => error!("Failed to disable approval policy '{}': {:?}", name, e),
                }
            }
        }
    }
}

impl ApprovalRequestOpt {
    pub async fn exec(&self, opt: &KanidmClientParser, client: &KanidmClient) {
        match self {
            ApprovalRequestOpt::List { state } => {
                let filter_state = match state {
                    Some(s) => match s.to_lowercase() {
                        "pending" => Some(ApprovalRequestState::Pending),
                        "approved" => Some(ApprovalRequestState::Approved),
                        "rejected" => Some(ApprovalRequestState::Rejected),
                        "expired" => Some(ApprovalRequestState::Expired),
                        "cancelled" => Some(ApprovalRequestState::Cancelled),
                        "escalated" => Some(ApprovalRequestState::Escalated),
                        _ => {
                            warn!("Unknown state filter: {}. Showing all requests.", s);
                            None
                        }
                    },
                    None => None,
                };

                let requests = match client.approval_request_list(filter_state).await {
                    Ok(r) => r,
                    Err(e) => {
                        error!("Failed to list approval requests: {:?}", e);
                        return;
                    }
                };

                if requests.is_empty() {
                    println!("No approval requests found");
                    return;
                }

                for req in requests {
                    println!("Request: {} ({})", req.uuid, req.state);
                    println!("  Policy: {} ({})", req.policy_name, req.policy_uuid);
                    println!("  Operation: {}", req.operation_type);
                    println!("  Target: {} ({})", req.target_spn, req.target_uuid);
                    println!("  Requestor: {} ({})", req.requestor_spn, req.requestor_uuid);
                    println!("  Created: {:?}", req.created_at);
                    if let Some(exp) = req.expires_at {
                        println!("  Expires: {:?}", exp);
                    }
                    println!("  Escalation Level: {}", req.escalation_level);
                    let approve_count = req.decisions.iter().filter(|d| d.action == kanidm_proto::v1::ApprovalDecisionAction::Approve).count();
                    let reject_count = req.decisions.iter().filter(|d| d.action == kanidm_proto::v1::ApprovalDecisionAction::Reject).count();
                    println!("  Decisions: {} approve, {} reject", approve_count, reject_count);
                    println!();
                }
            }
            ApprovalRequestOpt::Get { uuid } => {
                let req = match client.approval_request_get(uuid).await {
                    Ok(r) => r,
                    Err(e) => {
                        error!("Failed to get approval request '{}': {:?}", uuid, e);
                        return;
                    }
                };

                println!("Request: {} ({})", req.uuid, req.state);
                println!("Policy: {} ({})", req.policy_name, req.policy_uuid);
                println!("Operation: {}", req.operation_type);
                println!("Target: {} ({})", req.target_spn, req.target_uuid);
                println!("Requestor: {} ({})", req.requestor_spn, req.requestor_uuid);
                println!("Created: {:?}", req.created_at);
                if let Some(exp) = req.expires_at {
                    println!("Expires: {:?}", exp);
                }
                println!("Escalation Level: {}", req.escalation_level);
                println!("Required Decisions: {}", req.required_decisions);
                println!("Operation Details: {:?}", req.operation_details);
                println!("Decisions:");
                for dec in &req.decisions {
                    println!("  - {} ({}): {} at {:?}", dec.approver_spn, dec.approver_uuid, dec.action, dec.decision_time);
                    if let Some(c) = &dec.comment {
                        println!("    Comment: {}", c);
                    }
                }
            }
            ApprovalRequestOpt::Approve { uuid, comment } => {
                match client.approval_request_approve(uuid, comment.as_deref()).await {
                    Ok(_) => println!("Approval request '{}' approved successfully", uuid),
                    Err(e) => error!("Failed to approve request '{}': {:?}", uuid, e),
                }
            }
            ApprovalRequestOpt::Reject { uuid, comment } => {
                match client.approval_request_reject(uuid, comment.as_deref()).await {
                    Ok(_) => println!("Approval request '{}' rejected successfully", uuid),
                    Err(e) => error!("Failed to reject request '{}': {:?}", uuid, e),
                }
            }
            ApprovalRequestOpt::Cancel { uuid } => {
                match client.approval_request_cancel(uuid).await {
                    Ok(_) => println!("Approval request '{}' cancelled successfully", uuid),
                    Err(e) => error!("Failed to cancel request '{}': {:?}", uuid, e),
                }
            }
        }
    }
}