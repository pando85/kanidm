use crate::prelude::*;
use crate::server::access::{
    apply_delete_access, apply_modify_access, apply_search_access, AccessControlsTransaction,
    DeleteResult, ModifyResult, SearchResult,
};
use crate::server::QueryServerReadTransaction;
use kanidm_proto::internal::{
    AuthorizationAction, AuthorizationDecision, AuthorizationExplanation, AuthorizationRequest,
    AuthorizationResponse,
};
use std::collections::BTreeSet;

pub fn make_authorization_decision(
    qs: &mut QueryServerReadTransaction<'_>,
    ident: &Identity,
    req: &AuthorizationRequest,
) -> Result<AuthorizationResponse, OperationError> {
    let resource_uuid = req.resource;
    let action = req.action;
    let include_explanation = req.include_explanation;

    let filter = filter_all!(f_eq(Attribute::Uuid, PartialValue::Uuid(resource_uuid)));
    let search_event =
        crate::event::SearchEvent::from_internal_message(ident.clone(), &filter, None, qs)?;

    let entries = qs.search(&search_event)?;

    let target_entry = entries
        .into_iter()
        .next()
        .ok_or(OperationError::NoMatchingEntries)?;

    let access_controls = qs.get_accesscontrols();

    let mut matched_rules = Vec::new();
    let mut denied_by: Option<String> = None;

    let decision = match action {
        AuthorizationAction::Search => {
            let related_acp = access_controls.search_related_acp(ident, None);

            if related_acp.is_empty() {
                denied_by = Some("No applicable access control profiles".to_string());
                AuthorizationDecision::Deny
            } else {
                for acp in &related_acp {
                    matched_rules.push(acp.acp.acp.name.clone());
                }

                match apply_search_access(ident, &related_acp, &target_entry) {
                    SearchResult::Deny => {
                        denied_by = Some("Access denied by policy".to_string());
                        AuthorizationDecision::Deny
                    }
                    SearchResult::Grant => AuthorizationDecision::Allow,
                    SearchResult::Allow(attrs) => {
                        let attr_names: BTreeSet<String> = attrs
                            .into_iter()
                            .map(|a: Attribute| a.to_string())
                            .collect();

                        if attr_names.is_empty() {
                            denied_by = Some("No attributes accessible".to_string());
                            AuthorizationDecision::Deny
                        } else {
                            let mut response = AuthorizationResponse::allow_with_attributes(
                                resource_uuid,
                                action,
                                attr_names,
                            );
                            if include_explanation {
                                response = response.with_explanation(AuthorizationExplanation {
                                    matched_rules: matched_rules.clone(),
                                    denied_by: None,
                                    reason: "Access granted by matching access control profile"
                                        .to_string(),
                                });
                            }
                            return Ok(response);
                        }
                    }
                    SearchResult::ReauthRequired { reason } => {
                        denied_by = Some(reason.clone());
                        AuthorizationDecision::ReauthRequired
                    }
                }
            }
        }
        AuthorizationAction::Delete => {
            let related_acp = access_controls.delete_related_acp(ident);

            if related_acp.is_empty() {
                denied_by = Some("No applicable access control profiles".to_string());
                AuthorizationDecision::Deny
            } else {
                for acp in &related_acp {
                    matched_rules.push(acp.acp.acp.name.clone());
                }

                match apply_delete_access(ident, &related_acp, &target_entry) {
                    DeleteResult::Deny => {
                        denied_by = Some("Access denied by policy".to_string());
                        AuthorizationDecision::Deny
                    }
                    DeleteResult::Grant => AuthorizationDecision::Allow,
                    DeleteResult::ReauthRequired { reason } => {
                        denied_by = Some(reason.clone());
                        AuthorizationDecision::ReauthRequired
                    }
                }
            }
        }
        AuthorizationAction::Modify => {
            let related_acp = access_controls.modify_related_acp(ident);

            if related_acp.is_empty() {
                denied_by = Some("No applicable access control profiles".to_string());
                AuthorizationDecision::Deny
            } else {
                for acp in &related_acp {
                    matched_rules.push(acp.acp.acp.name.clone());
                }

                let sync_agmts = access_controls.get_sync_agreements();
                match apply_modify_access(ident, &related_acp, sync_agmts, &target_entry) {
                    ModifyResult::Deny => {
                        denied_by = Some("Access denied by policy".to_string());
                        AuthorizationDecision::Deny
                    }
                    ModifyResult::Grant => AuthorizationDecision::Allow,
                    ModifyResult::Allow { pres, rem, .. } => {
                        let mut all_attrs: BTreeSet<String> = BTreeSet::new();
                        for attr in pres.into_iter().chain(rem.into_iter()) {
                            all_attrs.insert(attr.to_string());
                        }

                        if all_attrs.is_empty() {
                            denied_by = Some("No attributes modifiable".to_string());
                            AuthorizationDecision::Deny
                        } else {
                            let mut response = AuthorizationResponse::allow_with_attributes(
                                resource_uuid,
                                action,
                                all_attrs,
                            );
                            if include_explanation {
                                response = response.with_explanation(AuthorizationExplanation {
                                    matched_rules: matched_rules.clone(),
                                    denied_by: None,
                                    reason: "Access granted by matching access control profile"
                                        .to_string(),
                                });
                            }
                            return Ok(response);
                        }
                    }
                    ModifyResult::ReauthRequired { reason } => {
                        denied_by = Some(reason.clone());
                        AuthorizationDecision::ReauthRequired
                    }
                }
            }
        }
        AuthorizationAction::Create => {
            denied_by =
                Some("Create action not supported for single-resource authorization".to_string());
            AuthorizationDecision::Deny
        }
    };

    let mut response = match decision {
        AuthorizationDecision::Allow => AuthorizationResponse::allow(resource_uuid, action),
        AuthorizationDecision::Deny => AuthorizationResponse::deny(resource_uuid, action),
        AuthorizationDecision::ReauthRequired => {
            AuthorizationResponse::reauth_required(resource_uuid, action)
        }
    };

    if include_explanation {
        response = response.with_explanation(AuthorizationExplanation {
            matched_rules,
            denied_by,
            reason: match decision {
                AuthorizationDecision::Allow => "Access granted".to_string(),
                AuthorizationDecision::Deny => "Access denied".to_string(),
                AuthorizationDecision::ReauthRequired => {
                    "Re-authentication required for this operation".to_string()
                }
            },
        });
    }

    Ok(response)
}
