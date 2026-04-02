use crate::prelude::*;
use std::collections::BTreeSet;

use crate::filter::{Filter, FilterValid, FilterValidResolved};

use kanidm_proto::internal::Filter as ProtoFilter;

// =========================================================================
// PARSE ENTRY TO ACP, AND ACP MANAGEMENT
// =========================================================================

#[derive(Debug, Clone)]
pub struct AccessControlSearchResolved<'a> {
    pub acp: &'a AccessControlSearch,
    pub receiver_condition: AccessControlReceiverCondition,
    pub target_condition: AccessControlTargetCondition,
}

#[derive(Debug, Clone)]
pub struct AccessControlSearch {
    pub acp: AccessControlProfile,
    pub attrs: BTreeSet<Attribute>,
}

impl AccessControlSearch {
    pub fn try_from(
        qs: &mut QueryServerWriteTransaction,
        value: &Entry<EntrySealed, EntryCommitted>,
    ) -> Result<Self, OperationError> {
        if !value.attribute_equality(Attribute::Class, &EntryClass::AccessControlSearch.into()) {
            admin_error!("class {} not present.", EntryClass::AccessControlSearch);
            return Err(OperationError::InvalidAcpState(format!(
                "Missing {}",
                EntryClass::AccessControlSearch
            )));
        }

        let mut attrs: BTreeSet<_> = value
            .get_ava_iter_iutf8(Attribute::AcpSearchAttr)
            .ok_or_else(|| {
                admin_error!("Missing {}", Attribute::AcpSearchAttr);
                OperationError::InvalidAcpState(format!("Missing {}", Attribute::AcpSearchAttr))
            })?
            .map(Attribute::from)
            .collect();

        // Ability to search memberof, implies the ability to read directmemberof
        if attrs.contains(&Attribute::MemberOf) {
            attrs.insert(Attribute::DirectMemberOf);
        }

        let acp = AccessControlProfile::try_from(qs, value)?;

        Ok(AccessControlSearch { acp, attrs })
    }

    /// ⚠️  - Manually create a search access profile from values.
    /// This is a TEST ONLY method and will never be exposed in production.
    #[cfg(test)]
    pub(super) fn from_raw(
        name: &str,
        uuid: Uuid,
        receiver: Uuid,
        targetscope: Filter<FilterValid>,
        attrs: &str,
    ) -> Self {
        let mut attrs: BTreeSet<_> = attrs.split_whitespace().map(Attribute::from).collect();

        // Ability to search memberof, implies the ability to read directmemberof
        if attrs.contains(&Attribute::MemberOf) {
            attrs.insert(Attribute::DirectMemberOf);
        }

        AccessControlSearch {
            acp: AccessControlProfile {
                name: name.to_string(),
                uuid,
                receiver: AccessControlReceiver::Group(btreeset!(receiver)),
                target: AccessControlTarget::Scope(targetscope),
                require_reauth: false,
                reauth_max_age: None,
                time_restriction_start: None,
                time_restriction_end: None,
                scope_filter: None,
            },
            attrs,
        }
    }

    /// ⚠️  - Manually create a search access profile from values.
    /// This is a TEST ONLY method and will never be exposed in production.
    #[cfg(test)]
    pub(super) fn from_managed_by(
        name: &str,
        uuid: Uuid,
        target: AccessControlTarget,
        attrs: &str,
    ) -> Self {
        AccessControlSearch {
            acp: AccessControlProfile {
                name: name.to_string(),
                uuid,
                receiver: AccessControlReceiver::EntryManager,
                target,
                require_reauth: false,
                reauth_max_age: None,
                time_restriction_start: None,
                time_restriction_end: None,
                scope_filter: None,
            },
            attrs: attrs.split_whitespace().map(Attribute::from).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccessControlDeleteResolved<'a> {
    pub acp: &'a AccessControlDelete,
    pub receiver_condition: AccessControlReceiverCondition,
    pub target_condition: AccessControlTargetCondition,
}

#[derive(Debug, Clone)]
pub struct AccessControlDelete {
    pub acp: AccessControlProfile,
}

impl AccessControlDelete {
    pub fn try_from(
        qs: &mut QueryServerWriteTransaction,
        value: &Entry<EntrySealed, EntryCommitted>,
    ) -> Result<Self, OperationError> {
        if !value.attribute_equality(Attribute::Class, &EntryClass::AccessControlDelete.into()) {
            admin_error!("class access_control_delete not present.");
            return Err(OperationError::InvalidAcpState(
                "Missing access_control_delete".to_string(),
            ));
        }

        Ok(AccessControlDelete {
            acp: AccessControlProfile::try_from(qs, value)?,
        })
    }

    /// ⚠️  - Manually create a delete access profile from values.
    /// This is a TEST ONLY method and will never be exposed in production.
    #[cfg(test)]
    pub(super) fn from_raw(
        name: &str,
        uuid: Uuid,
        receiver: Uuid,
        targetscope: Filter<FilterValid>,
    ) -> Self {
        AccessControlDelete {
            acp: AccessControlProfile {
                name: name.to_string(),
                uuid,
                receiver: AccessControlReceiver::Group(btreeset!(receiver)),
                target: AccessControlTarget::Scope(targetscope),
                require_reauth: false,
                reauth_max_age: None,
                time_restriction_start: None,
                time_restriction_end: None,
                scope_filter: None,
            },
        }
    }

    /// ⚠️  - Manually create a delete access profile from values.
    /// This is a TEST ONLY method and will never be exposed in production.
    #[cfg(test)]
    pub(super) fn from_managed_by(name: &str, uuid: Uuid, target: AccessControlTarget) -> Self {
        AccessControlDelete {
            acp: AccessControlProfile {
                name: name.to_string(),
                uuid,
                receiver: AccessControlReceiver::EntryManager,
                target,
                require_reauth: false,
                reauth_max_age: None,
                time_restriction_start: None,
                time_restriction_end: None,
                scope_filter: None,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccessControlCreateResolved<'a> {
    pub acp: &'a AccessControlCreate,
    pub receiver_condition: AccessControlReceiverCondition,
    pub target_condition: AccessControlTargetCondition,
}

#[derive(Debug, Clone)]
pub struct AccessControlCreate {
    pub acp: AccessControlProfile,
    pub classes: Vec<AttrString>,
    pub attrs: Vec<Attribute>,
}

impl AccessControlCreate {
    pub fn try_from(
        qs: &mut QueryServerWriteTransaction,
        value: &Entry<EntrySealed, EntryCommitted>,
    ) -> Result<Self, OperationError> {
        if !value.attribute_equality(Attribute::Class, &EntryClass::AccessControlCreate.into()) {
            admin_error!("class {} not present.", EntryClass::AccessControlCreate);
            return Err(OperationError::InvalidAcpState(format!(
                "Missing {}",
                EntryClass::AccessControlCreate
            )));
        }

        let attrs = value
            .get_ava_iter_iutf8(Attribute::AcpCreateAttr)
            .map(|i| i.map(Attribute::from).collect())
            .unwrap_or_default();

        let classes = value
            .get_ava_iter_iutf8(Attribute::AcpCreateClass)
            .map(|i| i.map(AttrString::from).collect())
            .unwrap_or_default();

        Ok(AccessControlCreate {
            acp: AccessControlProfile::try_from(qs, value)?,
            classes,
            attrs,
        })
    }

    /// ⚠️  - Manually create a create access profile from values.
    /// This is a TEST ONLY method and will never be exposed in production.
    #[cfg(test)]
    pub(super) fn from_raw(
        name: &str,
        uuid: Uuid,
        receiver: Uuid,
        targetscope: Filter<FilterValid>,
        classes: &str,
        attrs: &str,
    ) -> Self {
        AccessControlCreate {
            acp: AccessControlProfile {
                name: name.to_string(),
                uuid,
                receiver: AccessControlReceiver::Group(btreeset!(receiver)),
                target: AccessControlTarget::Scope(targetscope),
                require_reauth: false,
                reauth_max_age: None,
                time_restriction_start: None,
                time_restriction_end: None,
                scope_filter: None,
            },
            classes: classes.split_whitespace().map(AttrString::from).collect(),
            attrs: attrs.split_whitespace().map(Attribute::from).collect(),
        }
    }

    /// ⚠️  - Manually create a create access profile from values.
    /// This is a TEST ONLY method and will never be exposed in production.
    #[cfg(test)]
    pub(super) fn from_managed_by(
        name: &str,
        uuid: Uuid,
        target: AccessControlTarget,
        classes: &str,
        attrs: &str,
    ) -> Self {
        AccessControlCreate {
            acp: AccessControlProfile {
                name: name.to_string(),
                uuid,
                receiver: AccessControlReceiver::EntryManager,
                target,
                require_reauth: false,
                reauth_max_age: None,
                time_restriction_start: None,
                time_restriction_end: None,
                scope_filter: None,
            },
            classes: classes.split_whitespace().map(AttrString::from).collect(),
            attrs: attrs.split_whitespace().map(Attribute::from).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccessControlModifyResolved<'a> {
    pub acp: &'a AccessControlModify,
    pub receiver_condition: AccessControlReceiverCondition,
    pub target_condition: AccessControlTargetCondition,
}

#[derive(Debug, Clone)]
pub struct AccessControlModify {
    pub acp: AccessControlProfile,
    pub presattrs: Vec<Attribute>,
    pub remattrs: Vec<Attribute>,
    pub pres_classes: Vec<AttrString>,
    pub rem_classes: Vec<AttrString>,
}

impl AccessControlModify {
    pub fn try_from(
        qs: &mut QueryServerWriteTransaction,
        value: &Entry<EntrySealed, EntryCommitted>,
    ) -> Result<Self, OperationError> {
        if !value.attribute_equality(Attribute::Class, &EntryClass::AccessControlModify.into()) {
            admin_error!("class access_control_modify not present.");
            return Err(OperationError::InvalidAcpState(
                "Missing access_control_modify".to_string(),
            ));
        }

        let presattrs = value
            .get_ava_iter_iutf8(Attribute::AcpModifyPresentAttr)
            .map(|i| i.map(Attribute::from).collect())
            .unwrap_or_default();

        let remattrs = value
            .get_ava_iter_iutf8(Attribute::AcpModifyRemovedAttr)
            .map(|i| i.map(Attribute::from).collect())
            .unwrap_or_default();

        let classes: Vec<AttrString> = value
            .get_ava_iter_iutf8(Attribute::AcpModifyClass)
            .map(|i| i.map(AttrString::from).collect())
            .unwrap_or_default();

        let pres_classes = value
            .get_ava_iter_iutf8(Attribute::AcpModifyPresentClass)
            .map(|i| i.map(AttrString::from).collect())
            .unwrap_or_else(|| classes.clone());

        let rem_classes = value
            .get_ava_iter_iutf8(Attribute::AcpModifyRemoveClass)
            .map(|i| i.map(AttrString::from).collect())
            .unwrap_or_else(|| classes);

        Ok(AccessControlModify {
            acp: AccessControlProfile::try_from(qs, value)?,
            pres_classes,
            rem_classes,
            presattrs,
            remattrs,
        })
    }

    /// ⚠️  - Manually create a modify access profile from values.
    /// This is a TEST ONLY method and will never be exposed in production.
    #[cfg(test)]
    pub(super) fn from_raw(
        name: &str,
        uuid: Uuid,
        receiver: Uuid,
        targetscope: Filter<FilterValid>,
        presattrs: &str,
        remattrs: &str,
        pres_classes: &str,
        rem_classes: &str,
    ) -> Self {
        AccessControlModify {
            acp: AccessControlProfile {
                name: name.to_string(),
                uuid,
                receiver: AccessControlReceiver::Group(btreeset!(receiver)),
                target: AccessControlTarget::Scope(targetscope),
                require_reauth: false,
                reauth_max_age: None,
                time_restriction_start: None,
                time_restriction_end: None,
                scope_filter: None,
            },
            pres_classes: pres_classes
                .split_whitespace()
                .map(AttrString::from)
                .collect(),
            rem_classes: rem_classes
                .split_whitespace()
                .map(AttrString::from)
                .collect(),
            presattrs: presattrs.split_whitespace().map(Attribute::from).collect(),
            remattrs: remattrs.split_whitespace().map(Attribute::from).collect(),
        }
    }

    /// ⚠️  - Manually create a modify access profile from values.
    /// This is a TEST ONLY method and will never be exposed in production.
    #[cfg(test)]
    pub(super) fn from_managed_by(
        name: &str,
        uuid: Uuid,
        target: AccessControlTarget,
        presattrs: &str,
        remattrs: &str,
        pres_classes: &str,
        rem_classes: &str,
    ) -> Self {
        AccessControlModify {
            acp: AccessControlProfile {
                name: name.to_string(),
                uuid,
                receiver: AccessControlReceiver::EntryManager,
                target,
                require_reauth: false,
                reauth_max_age: None,
                time_restriction_start: None,
                time_restriction_end: None,
                scope_filter: None,
            },
            pres_classes: pres_classes
                .split_whitespace()
                .map(AttrString::from)
                .collect(),
            rem_classes: rem_classes
                .split_whitespace()
                .map(AttrString::from)
                .collect(),
            presattrs: presattrs.split_whitespace().map(Attribute::from).collect(),
            remattrs: remattrs.split_whitespace().map(Attribute::from).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AccessControlReceiver {
    None,
    Group(BTreeSet<Uuid>),
    EntryManager,
    Delegated {
        scope_group: Option<Uuid>,
        scope_filter: Option<Filter<FilterValid>>,
    },
}

#[derive(Debug, Clone)]
pub enum AccessControlReceiverCondition {
    // None,
    GroupChecked,
    EntryManager,
    Delegated {
        scope_filter_resolved: Option<Filter<FilterValidResolved>>,
    },
}

/*
impl AccessControlReceiverCondition {
    pub(crate) fn is_none(&self) {
        matches!(self, AccessControlReceiverCondition::None)
    }
}
*/

#[derive(Debug, Clone)]
pub enum AccessControlTarget {
    None,
    Scope(Filter<FilterValid>),
    DelegatedScope {
        scope_group: Option<Uuid>,
        scope_filter: Option<Filter<FilterValid>>,
    },
}

#[derive(Debug, Clone)]
pub enum AccessControlTargetCondition {
    // None,
    Scope(Filter<FilterValidResolved>),
    DelegatedScope {
        scope_filter_resolved: Option<Filter<FilterValidResolved>>,
    },
}

/*
impl AccessControlTargetCondition {
    pub(crate) fn is_none(&self) {
        matches!(&self, AccessControlTargetCondition::None)
    }
}
*/

#[derive(Debug, Clone)]
pub struct AccessControlProfile {
    pub name: String,
    // Currently we retrieve this but don't use it. We could depending on how we change
    // the acp update routine.
    #[allow(dead_code)]
    uuid: Uuid,
    pub receiver: AccessControlReceiver,
    pub target: AccessControlTarget,
    pub require_reauth: bool,
    pub reauth_max_age: Option<u32>,
    pub time_restriction_start: Option<time::OffsetDateTime>,
    pub time_restriction_end: Option<time::OffsetDateTime>,
    pub scope_filter: Option<Filter<FilterValid>>,
}

impl AccessControlProfile {
    pub(super) fn try_from(
        qs: &mut QueryServerWriteTransaction,
        value: &Entry<EntrySealed, EntryCommitted>,
    ) -> Result<Self, OperationError> {
        // Assert we have class access_control_profile
        if !value.attribute_equality(Attribute::Class, &EntryClass::AccessControlProfile.into()) {
            error!("class access_control_profile not present.");
            return Err(OperationError::InvalidAcpState(
                "Missing access_control_profile".to_string(),
            ));
        }

        // copy name
        let name = value
            .get_ava_single_iname(Attribute::Name)
            .ok_or_else(|| {
                error!("Missing {}", Attribute::Name);
                OperationError::InvalidAcpState(format!("Missing {}", Attribute::Name))
            })?
            .to_string();
        // copy uuid
        let uuid = value.get_uuid();

        let receiver = if value.attribute_equality(
            Attribute::Class,
            &EntryClass::AccessControlReceiverGroup.into(),
        ) {
            value
                .get_ava_refer(Attribute::AcpReceiverGroup)
                .cloned()
                .map(AccessControlReceiver::Group)
                .ok_or_else(|| {
                    admin_error!("Missing {}", Attribute::AcpReceiverGroup);
                    OperationError::InvalidAcpState(format!(
                        "Missing {}",
                        Attribute::AcpReceiverGroup
                    ))
                })?
        } else if value.attribute_equality(
            Attribute::Class,
            &EntryClass::AccessControlReceiverEntryManager.into(),
        ) {
            AccessControlReceiver::EntryManager
        } else if value.attribute_equality(
            Attribute::Class,
            &EntryClass::AccessControlReceiverDelegated.into(),
        ) {
            let scope_group = value.get_ava_single_refer(Attribute::DelegatedScopeGroup);

            let scope_filter = if let Some(f) =
                value.get_ava_single_protofilter(Attribute::DelegatedScopeFilter)
            {
                let ident = Identity::from_internal();
                let filter_i = Filter::from_rw(&ident, f, qs).map_err(|e| {
                    admin_error!(
                        "{} validation failed {:?}",
                        Attribute::DelegatedScopeFilter,
                        e
                    );
                    e
                })?;
                Some(filter_i.validate(qs.get_schema()).map_err(|e| {
                    admin_error!(
                        "{} Schema Violation {:?}",
                        Attribute::DelegatedScopeFilter,
                        e
                    );
                    OperationError::SchemaViolation(e)
                })?)
            } else {
                None
            };

            AccessControlReceiver::Delegated {
                scope_group,
                scope_filter,
            }
        } else {
            warn!(
                ?name,
                "access control has no defined receivers - this will do nothing!"
            );
            AccessControlReceiver::None
        };

        let target = if value.attribute_equality(
            Attribute::Class,
            &EntryClass::AccessControlTargetScope.into(),
        ) {
            // targetscope, and turn to real filter
            let targetscope_f: ProtoFilter = value
                .get_ava_single_protofilter(Attribute::AcpTargetScope)
                .cloned()
                .ok_or_else(|| {
                    admin_error!("Missing {}", Attribute::AcpTargetScope);
                    OperationError::InvalidAcpState(format!(
                        "Missing {}",
                        Attribute::AcpTargetScope
                    ))
                })?;

            let ident = Identity::from_internal();

            let targetscope_i = Filter::from_rw(&ident, &targetscope_f, qs).map_err(|e| {
                admin_error!("{} validation failed {:?}", Attribute::AcpTargetScope, e);
                e
            })?;

            targetscope_i
                .validate(qs.get_schema())
                .map_err(|e| {
                    admin_error!("{} Schema Violation {:?}", Attribute::AcpTargetScope, e);
                    OperationError::SchemaViolation(e)
                })
                .map(AccessControlTarget::Scope)?
        } else if value.attribute_equality(
            Attribute::Class,
            &EntryClass::AccessControlTargetDelegatedScope.into(),
        ) {
            let scope_group = value.get_ava_single_refer(Attribute::DelegatedScopeGroup);

            let scope_filter = if let Some(f) =
                value.get_ava_single_protofilter(Attribute::DelegatedScopeFilter)
            {
                let ident = Identity::from_internal();
                let filter_i = Filter::from_rw(&ident, f, qs).map_err(|e| {
                    admin_error!(
                        "{} validation failed {:?}",
                        Attribute::DelegatedScopeFilter,
                        e
                    );
                    e
                })?;
                Some(filter_i.validate(qs.get_schema()).map_err(|e| {
                    admin_error!(
                        "{} Schema Violation {:?}",
                        Attribute::DelegatedScopeFilter,
                        e
                    );
                    OperationError::SchemaViolation(e)
                })?)
            } else {
                None
            };

            AccessControlTarget::DelegatedScope {
                scope_group,
                scope_filter,
            }
        } else {
            warn!(
                ?name,
                "access control has no defined targets - this will do nothing!"
            );
            AccessControlTarget::None
        };

        let require_reauth = value
            .get_ava_single_bool(Attribute::AcpRequireReauth)
            .unwrap_or(false);

        let reauth_max_age = value.get_ava_single_uint32(Attribute::AcpReauthMaxAge);

        let time_restriction_start = value
            .get_ava_single_datetime(Attribute::AcpTimeRestrictionStart)
            .map(|dt| dt.to_offset(time::UtcOffset::UTC));

        let time_restriction_end = value
            .get_ava_single_datetime(Attribute::AcpTimeRestrictionEnd)
            .map(|dt| dt.to_offset(time::UtcOffset::UTC));

        let scope_filter =
            if let Some(f) = value.get_ava_single_protofilter(Attribute::AcpScopeFilter) {
                let ident = Identity::from_internal();
                let filter_i = Filter::from_rw(&ident, f, qs).map_err(|e| {
                    admin_error!("{} validation failed {:?}", Attribute::AcpScopeFilter, e);
                    e
                })?;
                Some(filter_i.validate(qs.get_schema()).map_err(|e| {
                    admin_error!("{} Schema Violation {:?}", Attribute::AcpScopeFilter, e);
                    OperationError::SchemaViolation(e)
                })?)
            } else {
                None
            };

        Ok(AccessControlProfile {
            name,
            uuid,
            receiver,
            target,
            require_reauth,
            reauth_max_age,
            time_restriction_start,
            time_restriction_end,
            scope_filter,
        })
    }
}
