//! `server` contains the query server, which is the main high level construction
//! to coordinate queries and operations in the server.

// This is really only used for long lived, high level types that need clone
// that otherwise can't be cloned. Think Mutex.
// use actix::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::audit::AuditScope;
use crate::be::{Backend, BackendReadTransaction, BackendTransaction, BackendWriteTransaction};

use crate::access::{
    AccessControlCreate, AccessControlDelete, AccessControlModify, AccessControlSearch,
    AccessControls, AccessControlsReadTransaction, AccessControlsTransaction,
    AccessControlsWriteTransaction,
};
// We use so many, we just import them all ...
use crate::constants::*;
use crate::entry::{
    Entry, EntryCommitted, EntryInit, EntryInvalid, EntryNew, EntryReduced, EntrySealed, EntryValid,
};
use crate::event::{
    CreateEvent, DeleteEvent, Event, EventOrigin, ExistsEvent, ModifyEvent, ReviveRecycledEvent,
    SearchEvent,
};
use crate::filter::{f_eq, Filter, FilterInvalid, FilterValid};
use crate::modify::{Modify, ModifyInvalid, ModifyList, ModifyValid};
use crate::plugins::Plugins;
use crate::repl::cid::Cid;
use crate::schema::{
    Schema, SchemaAttribute, SchemaClass, SchemaReadTransaction, SchemaTransaction,
    SchemaWriteTransaction,
};
use crate::value::{PartialValue, SyntaxType, Value};
use kanidm_proto::v1::{ConsistencyError, OperationError, SchemaError};

lazy_static! {
    static ref PVCLASS_ATTRIBUTETYPE: PartialValue = PartialValue::new_class("attributetype");
    static ref PVCLASS_CLASSTYPE: PartialValue = PartialValue::new_class("classtype");
    static ref PVCLASS_TOMBSTONE: PartialValue = PartialValue::new_class("tombstone");
    static ref PVCLASS_RECYCLED: PartialValue = PartialValue::new_class("recycled");
    static ref PVCLASS_ACS: PartialValue = PartialValue::new_class("access_control_search");
    static ref PVCLASS_ACD: PartialValue = PartialValue::new_class("access_control_delete");
    static ref PVCLASS_ACM: PartialValue = PartialValue::new_class("access_control_modify");
    static ref PVCLASS_ACC: PartialValue = PartialValue::new_class("access_control_create");
    static ref PVCLASS_ACP: PartialValue = PartialValue::new_class("access_control_profile");
    static ref PVACP_ENABLE_FALSE: PartialValue = PartialValue::new_bool(false);
}

// This is the core of the server. It implements all
// the search and modify actions, applies access controls
// and get's everything ready to push back to the fe code
/// The `QueryServerTransaction` trait provides a set of common read only operations to be
/// shared between [`QueryServerReadTransaction`] and [`QueryServerWriteTransaction`]s.
///
/// These operations tend to be high level constructions, generally different types of searches
/// that are capable of taking different types of parameters and applying access controls or not,
/// impersonating accounts, or bypassing these via internal searches.
///
/// [`QueryServerReadTransaction`]: struct.QueryServerReadTransaction.html
/// [`QueryServerWriteTransaction`]: struct.QueryServerWriteTransaction.html
pub trait QueryServerTransaction {
    type BackendTransactionType: BackendTransaction;
    fn get_be_txn(&mut self) -> &mut Self::BackendTransactionType;

    type SchemaTransactionType: SchemaTransaction;
    fn get_schema(&self) -> &Self::SchemaTransactionType;

    type AccessControlsTransactionType: AccessControlsTransaction;
    fn get_accesscontrols(&self) -> &Self::AccessControlsTransactionType;

    /// Conduct a search and apply access controls to yield a set of entries that
    /// have been reduced to the set of user visible avas. Note that if you provide
    /// a `SearchEvent` for the internal user, this query will fail. It is invalid for
    /// the [`access`] module to attempt to reduce avas for internal searches, and you
    /// should use [`fn search`] instead.
    ///
    /// [`SearchEvent`]: ../event/struct.SearchEvent.html
    /// [`access`]: ../access/index.html
    /// [`fn search`]: trait.QueryServerTransaction.html#method.search
    fn search_ext(
        &mut self,
        au: &mut AuditScope,
        se: &SearchEvent,
    ) -> Result<Vec<Entry<EntryReduced, EntryCommitted>>, OperationError> {
        /*
         * This just wraps search, but it's for the external interface
         * so as a result it also reduces the entry set's attributes at
         * the end.
         */
        let entries = self.search(au, se)?;

        let mut audit_acp = AuditScope::new("access_control_profiles");
        let access = self.get_accesscontrols();
        let acp_res = access.search_filter_entry_attributes(&mut audit_acp, se, entries);
        au.append_scope(audit_acp);
        // Log and fail if something went wrong.
        let entries_filtered = try_audit!(au, acp_res);

        // This is the final entry set that was reduced.
        Ok(entries_filtered)
    }

    fn search(
        &mut self,
        au: &mut AuditScope,
        se: &SearchEvent,
    ) -> Result<Vec<Entry<EntrySealed, EntryCommitted>>, OperationError> {
        audit_log!(au, "search: filter -> {:?}", se.filter);

        // This is an important security step because it prevents us from
        // performing un-indexed searches on attr's that don't exist in the
        // server. This is why ExtensibleObject can only take schema that
        // exists in the server, not arbitrary attr names.
        //
        // This normalises and validates in a single step.
        //
        // NOTE: Filters are validated in event conversion.

        let schema = self.get_schema();
        let idxmeta = schema.get_idxmeta_set();
        // Now resolve all references and indexes.
        let vfr = try_audit!(au, se.filter.resolve(&se.event, Some(&idxmeta)));

        // NOTE: We currently can't build search plugins due to the inability to hand
        // the QS wr/ro to the plugin trait. However, there shouldn't be a need for search
        // plugis, because all data transforms should be in the write path.

        let mut audit_be = AuditScope::new("backend_search");
        let res = self
            .get_be_txn()
            .search(&mut audit_be, &vfr)
            .map(|r| r)
            .map_err(|_| OperationError::Backend);
        au.append_scope(audit_be);

        let res = try_audit!(au, res);

        // Apply ACP before we let the plugins "have at it".
        // WARNING; for external searches this is NOT the only
        // ACP application. There is a second application to reduce the
        // attribute set on the entries!
        //
        let mut audit_acp = AuditScope::new("access_control_profiles");
        let access = self.get_accesscontrols();
        let acp_res = access.search_filter_entries(&mut audit_acp, se, res);

        au.append_scope(audit_acp);
        let acp_res = try_audit!(au, acp_res);

        Ok(acp_res)
    }

    fn exists(&mut self, au: &mut AuditScope, ee: &ExistsEvent) -> Result<bool, OperationError> {
        let mut audit_be = AuditScope::new("backend_exists");

        let schema = self.get_schema();
        let idxmeta = schema.get_idxmeta_set();
        let vfr = try_audit!(au, ee.filter.resolve(&ee.event, Some(&idxmeta)));

        let res = self
            .get_be_txn()
            .exists(&mut audit_be, &vfr)
            .map(|r| r)
            .map_err(|_| OperationError::Backend);
        au.append_scope(audit_be);
        res
    }

    // Should this actually be names_to_uuids and we do batches?
    //  In the initial design "no", we can always write a batched
    //  interface later.
    //
    // The main question is if we need association between the name and
    // the request uuid - if we do, we need singular. If we don't, we can
    // just do the batching.
    //
    // Filter conversion likely needs 1:1, due to and/or conversions
    // but create/mod likely doesn't due to the nature of the attributes.
    //
    // In the end, singular is the simple and correct option, so lets do
    // that first, and we can add batched (and cache!) later.
    //
    // Remember, we don't care if the name is invalid, because search
    // will validate/normalise the filter we construct for us. COOL!
    fn name_to_uuid(&mut self, audit: &mut AuditScope, name: &str) -> Result<Uuid, OperationError> {
        // For now this just constructs a filter and searches, but later
        // we could actually improve this to contact the backend and do
        // index searches, completely bypassing id2entry.

        // construct the filter
        // Internal search - DO NOT SEARCH TOMBSTONES AND RECYCLE
        let filt = filter!(f_eq("name", PartialValue::new_iutf8s(name)));
        audit_log!(audit, "name_to_uuid: name -> {:?}", name);

        let res = match self.internal_search(audit, filt) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        audit_log!(audit, "name_to_uuid: results -- {:?}", res);

        if res.is_empty() {
            // If result len == 0, error no such result
            return Err(OperationError::NoMatchingEntries);
        } else if res.len() >= 2 {
            // if result len >= 2, error, invaid entry state.
            return Err(OperationError::InvalidDBState);
        }

        // error should never be triggered due to the len checks above.
        let e = res.first().ok_or(OperationError::NoMatchingEntries)?;
        // Get the uuid from the entry. Again, check it exists, and only one.
        let uuid_res: Uuid = *e.get_uuid();

        audit_log!(audit, "name_to_uuid: uuid <- {:?}", uuid_res);

        Ok(uuid_res)
    }

    fn uuid_to_name(
        &mut self,
        audit: &mut AuditScope,
        uuid: &Uuid,
    ) -> Result<Option<Value>, OperationError> {
        // construct the filter
        let filt = filter!(f_eq("uuid", PartialValue::new_uuidr(uuid)));
        audit_log!(audit, "uuid_to_name: uuid -> {:?}", uuid);

        // Internal search - DO NOT SEARCH TOMBSTONES AND RECYCLE
        let res = match self.internal_search(audit, filt) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        audit_log!(audit, "uuid_to_name: results -- {:?}", res);

        if res.is_empty() {
            // If result len == 0, error no such result
            audit_log!(audit, "uuid_to_name: name, no such entry <- Ok(None)");
            return Ok(None);
        } else if res.len() >= 2 {
            // if result len >= 2, error, invaid entry state.
            return Err(OperationError::InvalidDBState);
        }

        // fine for 0/1 case, but check len for >= 2 to eliminate that case.
        let e = res.first().ok_or(OperationError::NoMatchingEntries)?;
        // Get the uuid from the entry. Again, check it exists, and only one.
        let name_res = match e.get_ava(&String::from("name")) {
            Some(vas) => match vas.first() {
                Some(u) => (*u).clone(),
                // Name is in an invalid state in the db
                None => return Err(OperationError::InvalidEntryState),
            },
            None => {
                // No attr name, some types this is valid, IE schema.
                // return Err(OperationError::InvalidEntryState),
                return Ok(None);
            }
        };

        audit_log!(audit, "uuid_to_name: name <- {:?}", name_res);

        // Make sure it's the right type ... (debug only)
        debug_assert!(name_res.is_insensitive_utf8());

        Ok(Some(name_res))
    }

    fn posixid_to_uuid(
        &mut self,
        audit: &mut AuditScope,
        name: &str,
    ) -> Result<Uuid, OperationError> {
        let f_name = Some(f_eq("name", PartialValue::new_iutf8s(name)));

        let f_spn = PartialValue::new_spn_s(name).map(|v| f_eq("spn", v));

        let f_gidnumber = PartialValue::new_uint32_str(name).map(|v| f_eq("gidnumber", v));

        let x = vec![f_name, f_spn, f_gidnumber];

        let filt = filter!(f_or(x.into_iter().filter_map(|v| v).collect()));
        audit_log!(audit, "posixid_to_uuid: name -> {:?}", name);

        let res = match self.internal_search(audit, filt) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        audit_log!(audit, "posixid_to_uuid: results -- {:?}", res);

        if res.is_empty() {
            // If result len == 0, error no such result
            return Err(OperationError::NoMatchingEntries);
        } else if res.len() >= 2 {
            // if result len >= 2, error, invaid entry state.
            return Err(OperationError::InvalidDBState);
        }

        // error should never be triggered due to the len checks above.
        let e = res.first().ok_or(OperationError::NoMatchingEntries)?;
        // Get the uuid from the entry. Again, check it exists, and only one.
        let uuid_res: Uuid = *e.get_uuid();

        audit_log!(audit, "posixid_to_uuid: uuid <- {:?}", uuid_res);

        Ok(uuid_res)
    }

    // From internal, generate an exists event and dispatch
    fn internal_exists(
        &mut self,
        au: &mut AuditScope,
        filter: Filter<FilterInvalid>,
    ) -> Result<bool, OperationError> {
        // Check the filter
        let f_valid = filter
            .validate(self.get_schema())
            .map_err(OperationError::SchemaViolation)?;
        // Build an exists event
        let ee = ExistsEvent::new_internal(f_valid);
        // Submit it
        let mut audit_int = AuditScope::new("internal_exists");
        let res = self.exists(&mut audit_int, &ee);
        au.append_scope(audit_int);
        // return result
        res
    }

    fn internal_search(
        &mut self,
        audit: &mut AuditScope,
        filter: Filter<FilterInvalid>,
    ) -> Result<Vec<Entry<EntrySealed, EntryCommitted>>, OperationError> {
        let f_valid = filter
            .validate(self.get_schema())
            .map_err(OperationError::SchemaViolation)?;
        let se = SearchEvent::new_internal(f_valid);
        let mut audit_int = AuditScope::new("internal_search");
        let res = self.search(&mut audit_int, &se);
        audit.append_scope(audit_int);
        res
    }

    fn impersonate_search_valid(
        &mut self,
        audit: &mut AuditScope,
        f_valid: Filter<FilterValid>,
        f_intent_valid: Filter<FilterValid>,
        event: &Event,
    ) -> Result<Vec<Entry<EntrySealed, EntryCommitted>>, OperationError> {
        let se = SearchEvent::new_impersonate(event, f_valid, f_intent_valid);
        let mut audit_int = AuditScope::new("impersonate_search");
        let res = self.search(&mut audit_int, &se);
        audit.append_scope(audit_int);
        res
    }

    // this applys ACP to filter result entries.
    fn impersonate_search_ext_valid(
        &mut self,
        audit: &mut AuditScope,
        f_valid: Filter<FilterValid>,
        f_intent_valid: Filter<FilterValid>,
        event: &Event,
    ) -> Result<Vec<Entry<EntryReduced, EntryCommitted>>, OperationError> {
        let se = SearchEvent::new_impersonate(event, f_valid, f_intent_valid);
        let mut audit_int = AuditScope::new("impersonate_search_ext");
        let res = self.search_ext(&mut audit_int, &se);
        audit.append_scope(audit_int);
        res
    }

    // Who they are will go here
    fn impersonate_search(
        &mut self,
        audit: &mut AuditScope,
        filter: Filter<FilterInvalid>,
        filter_intent: Filter<FilterInvalid>,
        event: &Event,
    ) -> Result<Vec<Entry<EntrySealed, EntryCommitted>>, OperationError> {
        let f_valid = filter
            .validate(self.get_schema())
            .map_err(OperationError::SchemaViolation)?;
        let f_intent_valid = filter_intent
            .validate(self.get_schema())
            .map_err(OperationError::SchemaViolation)?;
        self.impersonate_search_valid(audit, f_valid, f_intent_valid, event)
    }

    fn impersonate_search_ext(
        &mut self,
        audit: &mut AuditScope,
        filter: Filter<FilterInvalid>,
        filter_intent: Filter<FilterInvalid>,
        event: &Event,
    ) -> Result<Vec<Entry<EntryReduced, EntryCommitted>>, OperationError> {
        let f_valid = filter
            .validate(self.get_schema())
            .map_err(OperationError::SchemaViolation)?;
        let f_intent_valid = filter_intent
            .validate(self.get_schema())
            .map_err(OperationError::SchemaViolation)?;
        self.impersonate_search_ext_valid(audit, f_valid, f_intent_valid, event)
    }

    // Get a single entry by it's UUID. This is heavily relied on for internal
    // server operations, especially in login and acp checks for acp.
    fn internal_search_uuid(
        &mut self,
        audit: &mut AuditScope,
        uuid: &Uuid,
    ) -> Result<Entry<EntrySealed, EntryCommitted>, OperationError> {
        let filter = filter!(f_eq("uuid", PartialValue::new_uuid(*uuid)));
        let f_valid = filter
            .validate(self.get_schema())
            .map_err(OperationError::SchemaViolation)?;
        let se = SearchEvent::new_internal(f_valid);
        let mut audit_int = AuditScope::new("internal_search_uuid");
        let res = self.search(&mut audit_int, &se);
        audit.append_scope(audit_int);
        match res {
            Ok(vs) => {
                if vs.len() > 1 {
                    return Err(OperationError::NoMatchingEntries);
                }
                vs.into_iter()
                    .next()
                    .ok_or(OperationError::NoMatchingEntries)
            }
            Err(e) => Err(e),
        }
    }

    fn impersonate_search_ext_uuid(
        &mut self,
        audit: &mut AuditScope,
        uuid: &Uuid,
        event: &Event,
    ) -> Result<Entry<EntryReduced, EntryCommitted>, OperationError> {
        let filter_intent = filter_all!(f_eq("uuid", PartialValue::new_uuid(*uuid)));
        let filter = filter!(f_eq("uuid", PartialValue::new_uuid(*uuid)));
        let res = self.impersonate_search_ext(audit, filter, filter_intent, event);
        match res {
            Ok(vs) => {
                if vs.len() > 1 {
                    return Err(OperationError::NoMatchingEntries);
                }
                vs.into_iter()
                    .next()
                    .ok_or(OperationError::NoMatchingEntries)
            }
            Err(e) => Err(e),
        }
    }

    /// Do a schema aware conversion from a String:String to String:Value for modification
    /// present.
    fn clone_value(
        &mut self,
        audit: &mut AuditScope,
        attr: &str,
        value: &str,
    ) -> Result<Value, OperationError> {
        let schema = self.get_schema();

        // Should this actually be a fn of Value - no - I think that introduces issues with the
        // monomorphisation of the trait for transactions, so we should have this here.

        // Normalise the attribute name for lookup.
        // TODO: Should we return this?
        let temp_a = schema.normalise_attr_name(attr);

        // Lookup the attr
        match schema.get_attributes().get(&temp_a) {
            Some(schema_a) => {
                match schema_a.syntax {
                    SyntaxType::UTF8STRING => Ok(Value::new_utf8(value.to_string())),
                    SyntaxType::UTF8STRING_INSENSITIVE => Ok(Value::new_iutf8s(value)),
                    SyntaxType::BOOLEAN => Value::new_bools(value)
                        .ok_or_else(|| OperationError::InvalidAttribute("Invalid boolean syntax".to_string())),
                    SyntaxType::SYNTAX_ID => Value::new_syntaxs(value)
                        .ok_or_else(|| OperationError::InvalidAttribute("Invalid Syntax syntax".to_string())),
                    SyntaxType::INDEX_ID => Value::new_indexs(value)
                        .ok_or_else(|| OperationError::InvalidAttribute("Invalid Index syntax".to_string())),
                    SyntaxType::UUID => {
                        // It's a uuid - we do NOT check for existance, because that
                        // could be revealing or disclosing - it is up to acp to assert
                        // if we can see the value or not, and it's not up to us to
                        // assert the filter value exists.
                        Value::new_uuids(value)
                            .or_else(|| {
                                // it's not a uuid, try to resolve it.
                                // if the value is NOT found, we map to "does not exist" to allow
                                // the value to continue being evaluated, which of course, will fail
                                // all subsequent filter tests because it ... well, doesn't exist.
                                let un = self
                                    .name_to_uuid(audit, value)
                                    .unwrap_or_else(|_| *UUID_DOES_NOT_EXIST);
                                Some(Value::new_uuid(un))
                            })
                            // I think this is unreachable due to how the .or_else works.
                            .ok_or_else(|| OperationError::InvalidAttribute("Invalid UUID syntax".to_string()))
                    }
                    SyntaxType::REFERENCE_UUID => {
                        // See comments above.
                        Value::new_refer_s(value)
                            .or_else(|| {
                                let un = self
                                    .name_to_uuid(audit, value)
                                    .unwrap_or_else(|_| *UUID_DOES_NOT_EXIST);
                                Some(Value::new_refer(un))
                            })
                            // I think this is unreachable due to how the .or_else works.
                            .ok_or_else(|| OperationError::InvalidAttribute("Invalid Reference syntax".to_string()))
                    }
                    SyntaxType::JSON_FILTER => Value::new_json_filter(value)
                        .ok_or_else(|| OperationError::InvalidAttribute("Invalid Filter syntax".to_string())),
                    SyntaxType::CREDENTIAL => Err(OperationError::InvalidAttribute("Credentials can not be supplied through modification - please use the IDM api".to_string())),
                    SyntaxType::RADIUS_UTF8STRING => Err(OperationError::InvalidAttribute("Radius secrets can not be supplied through modification - please use the IDM api".to_string())),
                    SyntaxType::SSHKEY => Err(OperationError::InvalidAttribute("SSH public keys can not be supplied through modification - please use the IDM api".to_string())),
                    SyntaxType::SERVICE_PRINCIPLE_NAME => Err(OperationError::InvalidAttribute("SPNs are generated and not able to be set.".to_string())),
                    SyntaxType::UINT32 => Value::new_uint32_str(value)
                        .ok_or_else(|| OperationError::InvalidAttribute("Invalid uint32 syntax".to_string())),
                    SyntaxType::CID => Err(OperationError::InvalidAttribute("CIDs are generated and not able to be set.".to_string())),
                }
            }
            None => {
                // No attribute of this name exists - fail fast, there is no point to
                // proceed, as nothing can be satisfied.
                Err(OperationError::InvalidAttributeName(temp_a))
            }
        }
    }

    fn clone_partialvalue(
        &mut self,
        audit: &mut AuditScope,
        attr: &str,
        value: &str,
    ) -> Result<PartialValue, OperationError> {
        let schema = self.get_schema();
        // TODO: Should we return this?
        let temp_a = schema.normalise_attr_name(attr);

        // Lookup the attr
        match schema.get_attributes().get(&temp_a) {
            Some(schema_a) => {
                match schema_a.syntax {
                    SyntaxType::UTF8STRING => Ok(PartialValue::new_utf8(value.to_string())),
                    SyntaxType::UTF8STRING_INSENSITIVE => Ok(PartialValue::new_iutf8s(value)),
                    SyntaxType::BOOLEAN => PartialValue::new_bools(value).ok_or_else(|| {
                        OperationError::InvalidAttribute("Invalid boolean syntax".to_string())
                    }),
                    SyntaxType::SYNTAX_ID => PartialValue::new_syntaxs(value).ok_or_else(|| {
                        OperationError::InvalidAttribute("Invalid Syntax syntax".to_string())
                    }),
                    SyntaxType::INDEX_ID => PartialValue::new_indexs(value).ok_or_else(|| {
                        OperationError::InvalidAttribute("Invalid Index syntax".to_string())
                    }),
                    SyntaxType::UUID => {
                        PartialValue::new_uuids(value)
                            .or_else(|| {
                                // it's not a uuid, try to resolve it.
                                // if the value is NOT found, we map to "does not exist" to allow
                                // the value to continue being evaluated, which of course, will fail
                                // all subsequent filter tests because it ... well, doesn't exist.
                                let un = self
                                    .name_to_uuid(audit, value)
                                    .unwrap_or_else(|_| *UUID_DOES_NOT_EXIST);
                                Some(PartialValue::new_uuid(un))
                            })
                            // I think this is unreachable due to how the .or_else works.
                            .ok_or_else(|| {
                                OperationError::InvalidAttribute("Invalid UUID syntax".to_string())
                            })
                    }
                    SyntaxType::REFERENCE_UUID => {
                        // See comments above.
                        PartialValue::new_refer_s(value)
                            .or_else(|| {
                                let un = self
                                    .name_to_uuid(audit, value)
                                    .unwrap_or_else(|_| *UUID_DOES_NOT_EXIST);
                                Some(PartialValue::new_refer(un))
                            })
                            // I think this is unreachable due to how the .or_else works.
                            .ok_or_else(|| {
                                OperationError::InvalidAttribute(
                                    "Invalid Reference syntax".to_string(),
                                )
                            })
                    }
                    SyntaxType::JSON_FILTER => {
                        PartialValue::new_json_filter(value).ok_or_else(|| {
                            OperationError::InvalidAttribute("Invalid Filter syntax".to_string())
                        })
                    }
                    SyntaxType::CREDENTIAL => Ok(PartialValue::new_credential_tag(value)),
                    SyntaxType::RADIUS_UTF8STRING => Ok(PartialValue::new_radius_string()),
                    SyntaxType::SSHKEY => Ok(PartialValue::new_sshkey_tag_s(value)),
                    SyntaxType::SERVICE_PRINCIPLE_NAME => PartialValue::new_spn_s(value)
                        .ok_or_else(|| {
                            OperationError::InvalidAttribute("Invalid SPN syntax".to_string())
                        }),
                    SyntaxType::UINT32 => PartialValue::new_uint32_str(value).ok_or_else(|| {
                        OperationError::InvalidAttribute("Invalid Uint32 syntax".to_string())
                    }),
                    SyntaxType::CID => PartialValue::new_cid_s(value).ok_or_else(|| {
                        OperationError::InvalidAttribute("Invalid Cid syntax".to_string())
                    }),
                }
            }
            None => {
                // No attribute of this name exists - fail fast, there is no point to
                // proceed, as nothing can be satisfied.
                Err(OperationError::InvalidAttributeName(temp_a))
            }
        }
    }

    // In the opposite direction, we can resolve values for presentation
    fn resolve_value(
        &mut self,
        audit: &mut AuditScope,
        value: &Value,
    ) -> Result<String, OperationError> {
        // Are we a reference type? Try and resolve.
        if let Some(ur) = value.to_ref_uuid() {
            let nv = self.uuid_to_name(audit, ur)?;
            return match nv {
                Some(v) => Ok(v.to_proto_string_clone()),
                None => Ok(value.to_proto_string_clone()),
            };
        }

        // Not? Okay, do the to string.
        Ok(value.to_proto_string_clone())
    }
}

pub struct QueryServerReadTransaction<'a> {
    be_txn: BackendReadTransaction<'a>,
    // Anything else? In the future, we'll need to have a schema transaction
    // type, maybe others?
    schema: SchemaReadTransaction,
    accesscontrols: AccessControlsReadTransaction,
}

// Actually conduct a search request
// This is the core of the server, as it processes the entire event
// applies all parts required in order and more.
impl<'a> QueryServerTransaction for QueryServerReadTransaction<'a> {
    type BackendTransactionType = BackendReadTransaction<'a>;

    fn get_be_txn(&mut self) -> &mut BackendReadTransaction<'a> {
        &mut self.be_txn
    }

    type SchemaTransactionType = SchemaReadTransaction;

    fn get_schema(&self) -> &SchemaReadTransaction {
        &self.schema
    }

    type AccessControlsTransactionType = AccessControlsReadTransaction;

    fn get_accesscontrols(&self) -> &AccessControlsReadTransaction {
        &self.accesscontrols
    }
}

impl<'a> QueryServerReadTransaction<'a> {
    // Verify the data content of the server is as expected. This will probably
    // call various functions for validation, including possibly plugin
    // verifications.
    fn verify(&mut self, au: &mut AuditScope) -> Vec<Result<(), ConsistencyError>> {
        let mut audit = AuditScope::new("verify");

        // If we fail after backend, we need to return NOW because we can't
        // assert any other faith in the DB states.
        //  * backend
        let be_errs = self.get_be_txn().verify();

        if !be_errs.is_empty() {
            au.append_scope(audit);
            return be_errs;
        }

        //  * in memory schema consistency.
        let sc_errs = self.get_schema().validate(&mut audit);

        if !sc_errs.is_empty() {
            au.append_scope(audit);
            return sc_errs;
        }

        //  * Indexing (req be + sch )
        /*
        idx_errs = self.get_be_txn()
            .verify_indexes();

        if !idx_errs.is_empty() {
            au.append_scope(audit);
            return idx_errs;
        }
         */

        // Ok BE passed, lets move on to the content.
        // Most of our checks are in the plugins, so we let them
        // do their job.

        // Now, call the plugins verification system.
        let pl_errs = Plugins::run_verify(&mut audit, self);

        // Finish up ...
        au.append_scope(audit);
        pl_errs
    }
}

pub struct QueryServerWriteTransaction<'a> {
    committed: bool,
    d_uuid: Uuid,
    cid: Cid,
    be_txn: BackendWriteTransaction<'a>,
    schema: SchemaWriteTransaction<'a>,
    accesscontrols: AccessControlsWriteTransaction<'a>,
    // We store a set of flags that indicate we need a reload of
    // schema or acp, which is tested by checking the classes of the
    // changing content.
    changed_schema: bool,
    changed_acp: bool,
}

impl<'a> QueryServerTransaction for QueryServerWriteTransaction<'a> {
    type BackendTransactionType = BackendWriteTransaction<'a>;

    fn get_be_txn(&mut self) -> &mut BackendWriteTransaction<'a> {
        &mut self.be_txn
    }

    type SchemaTransactionType = SchemaWriteTransaction<'a>;

    fn get_schema(&self) -> &SchemaWriteTransaction<'a> {
        &self.schema
    }

    type AccessControlsTransactionType = AccessControlsWriteTransaction<'a>;

    fn get_accesscontrols(&self) -> &AccessControlsWriteTransaction<'a> {
        &self.accesscontrols
    }
}

#[derive(Clone)]
pub struct QueryServer {
    // log: actix::Addr<EventLog>,
    s_uuid: Uuid,
    d_uuid: Uuid,
    be: Backend,
    schema: Arc<Schema>,
    accesscontrols: Arc<AccessControls>,
}

impl QueryServer {
    pub fn new(be: Backend, schema: Schema) -> Self {
        let (s_uuid, d_uuid) = {
            let mut wr = be.write(BTreeSet::new());
            (wr.get_db_s_uuid(), wr.get_db_d_uuid())
        };
        info!("Server ID -> {:?}", s_uuid);
        info!("Domain ID -> {:?}", d_uuid);
        // log_event!(log, "Starting query worker ...");
        QueryServer {
            s_uuid,
            d_uuid,
            be,
            schema: Arc::new(schema),
            accesscontrols: Arc::new(AccessControls::new()),
        }
    }

    pub fn read(&self) -> QueryServerReadTransaction {
        QueryServerReadTransaction {
            be_txn: self.be.read(),
            schema: self.schema.read(),
            accesscontrols: self.accesscontrols.read(),
        }
    }

    pub fn write(&self, ts: Duration) -> QueryServerWriteTransaction {
        // Feed the current schema index metadata to the be write transaction.
        let schema_write = self.schema.write();
        let idxmeta = schema_write.get_idxmeta_set();

        let cid = Cid::new(self.s_uuid, self.d_uuid, ts);

        QueryServerWriteTransaction {
            // I think this is *not* needed, because commit is mut self which should
            // take ownership of the value, and cause the commit to "only be run
            // once".
            //
            // The commited flag is however used for abort-specific code in drop
            // which today I don't think we have ... yet.
            committed: false,
            d_uuid: self.d_uuid,
            cid,
            be_txn: self.be.write(idxmeta),
            schema: schema_write,
            accesscontrols: self.accesscontrols.write(),
            changed_schema: false,
            changed_acp: false,
        }
    }

    pub(crate) fn initialise_helper(
        &self,
        audit: &mut AuditScope,
        ts: Duration,
    ) -> Result<(), OperationError> {
        // First, check our database version - attempt to do an initial indexing
        // based on the in memory configuration
        //
        // If we ever change the core in memory schema, or the schema that we ship
        // in fixtures, we have to bump these values. This is how we manage the
        // first-run and upgrade reindexings.
        //
        // A major reason here to split to multiple transactions is to allow schema
        // reloading to occur, which causes the idxmeta to update, and allows validation
        // of the schema in the subsequent steps as we proceed.

        let mut reindex_write_1 = self.write(ts);
        reindex_write_1
            .upgrade_reindex(audit, SYSTEM_INDEX_VERSION)
            .and_then(|_| reindex_write_1.commit(audit))?;

        // Because we init the schema here, and commit, this reloads meaning
        // that the on-disk index meta has been loaded, so our subsequent
        // migrations will be correctly indexed.
        //
        // Remember, that this would normally mean that it's possible for schema
        // to be mis-indexed (IE we index the new schemas here before we read
        // the schema to tell us what's indexed), but because we have the in
        // mem schema that defines how schema is structuded, and this is all
        // marked "system", then we won't have an issue here.
        let mut ts_write_1 = self.write(ts);
        ts_write_1
            .initialise_schema_core(audit)
            .and_then(|_| ts_write_1.commit(audit))?;

        let mut ts_write_2 = self.write(ts);
        ts_write_2
            .initialise_schema_idm(audit)
            .and_then(|_| ts_write_2.commit(audit))?;

        // reindex and set to version + 1, this way when we bump the version
        // we are essetially pushing this version id back up to step write_1
        let mut reindex_write_2 = self.write(ts);
        reindex_write_2
            .upgrade_reindex(audit, SYSTEM_INDEX_VERSION + 1)
            .and_then(|_| reindex_write_2.commit(audit))?;

        let mut ts_write_3 = self.write(ts);
        ts_write_3
            .initialise_idm(audit)
            .and_then(|_| ts_write_3.commit(audit))
    }

    pub fn verify(&self, au: &mut AuditScope) -> Vec<Result<(), ConsistencyError>> {
        let mut r_txn = self.read();
        r_txn.verify(au)
    }
}

impl<'a> QueryServerWriteTransaction<'a> {
    pub fn create(&mut self, au: &mut AuditScope, ce: &CreateEvent) -> Result<(), OperationError> {
        // The create event is a raw, read only representation of the request
        // that was made to us, including information about the identity
        // performing the request.

        // Log the request

        // TODO #67: Do we need limits on number of creates, or do we constraint
        // based on request size in the frontend?

        // Copy the entries to a writeable form, this involves assigning a
        // change id so we can track what's happening.
        let candidates: Vec<Entry<EntryInit, EntryNew>> =
            ce.entries.iter().map(|e| e.clone()).collect();

        // Do we have rights to perform these creates?
        // create_allow_operation
        let mut audit_acp = AuditScope::new("access_control_profiles");
        let access = self.get_accesscontrols();
        let acp_res = access.create_allow_operation(&mut audit_acp, ce, &candidates);
        au.append_scope(audit_acp);
        if !try_audit!(au, acp_res) {
            return Err(OperationError::AccessDenied);
        }

        // Assign our replication metadata now, since we can proceed with this operation.
        let mut candidates: Vec<Entry<EntryInvalid, EntryNew>> = candidates
            .into_iter()
            .map(|e| e.clone().assign_cid(self.cid.clone()))
            .collect();

        // run any pre plugins, giving them the list of mutable candidates.
        // pre-plugins are defined here in their correct order of calling!
        // I have no intent to make these dynamic or configurable.

        let mut audit_plugin_pre_transform = AuditScope::new("plugin_pre_create_transform");
        let plug_pre_transform_res = Plugins::run_pre_create_transform(
            &mut audit_plugin_pre_transform,
            self,
            &mut candidates,
            ce,
        );
        au.append_scope(audit_plugin_pre_transform);

        try_audit!(
            au,
            plug_pre_transform_res,
            "Create operation failed (pre_transform plugin), {:?}"
        );

        // NOTE: This is how you map from Vec<Result<T>> to Result<Vec<T>>
        // remember, that you only get the first error and the iter terminates.

        // Now, normalise AND validate!

        let res: Result<Vec<Entry<EntrySealed, EntryNew>>, OperationError> = candidates
            .into_iter()
            .map(|e| {
                e.validate(&self.schema)
                    .map_err(OperationError::SchemaViolation)
                    .map(|e|
                    // Then seal the changes?
                    e.seal())
            })
            .collect();

        let norm_cand: Vec<Entry<_, _>> = try_audit!(au, res);

        // Run any pre-create plugins now with schema validated entries.
        // This is important for normalisation of certain types IE class
        // or attributes for these checks.
        let mut audit_plugin_pre = AuditScope::new("plugin_pre_create");
        let plug_pre_res = Plugins::run_pre_create(&mut audit_plugin_pre, self, &norm_cand, ce);
        au.append_scope(audit_plugin_pre);

        try_audit!(au, plug_pre_res, "Create operation failed (plugin), {:?}");

        let mut audit_be = AuditScope::new("backend_create");
        // We may change from ce.entries later to something else?
        let res = self.be_txn.create(&mut audit_be, norm_cand).map_err(|e| e);

        au.append_scope(audit_be);

        let commit_cand = try_audit!(au, res);
        // Run any post plugins

        let mut audit_plugin_post = AuditScope::new("plugin_post_create");
        let plug_post_res =
            Plugins::run_post_create(&mut audit_plugin_post, self, &commit_cand, ce);
        au.append_scope(audit_plugin_post);

        if plug_post_res.is_err() {
            audit_log!(
                au,
                "Create operation failed (post plugin), {:?}",
                plug_post_res
            );
            return plug_post_res;
        }

        // We have finished all plugs and now have a successful operation - flag if
        // schema or acp requires reload.
        self.changed_schema = commit_cand.iter().fold(false, |acc, e| {
            if acc {
                acc
            } else {
                e.attribute_value_pres("class", &PVCLASS_CLASSTYPE)
                    || e.attribute_value_pres("class", &PVCLASS_ATTRIBUTETYPE)
            }
        });
        self.changed_acp = commit_cand.iter().fold(false, |acc, e| {
            if acc {
                acc
            } else {
                e.attribute_value_pres("class", &PVCLASS_ACP)
            }
        });
        audit_log!(
            au,
            "Schema reload: {:?}, ACP reload: {:?}",
            self.changed_schema,
            self.changed_acp
        );

        // We are complete, finalise logging and return

        audit_log!(au, "Create operation success");
        Ok(())
    }

    pub fn delete(&mut self, au: &mut AuditScope, de: &DeleteEvent) -> Result<(), OperationError> {
        // Do you have access to view all the set members? Reduce based on your
        // read permissions and attrs
        // THIS IS PRETTY COMPLEX SEE THE DESIGN DOC
        // In this case we need a search, but not INTERNAL to keep the same
        // associated credentials.
        // We only need to retrieve uuid though ...

        // Now, delete only what you can see
        let pre_candidates = match self.impersonate_search_valid(
            au,
            de.filter.clone(),
            de.filter_orig.clone(),
            &de.event,
        ) {
            Ok(results) => results,
            Err(e) => {
                audit_log!(au, "delete: error in pre-candidate selection {:?}", e);
                return Err(e);
            }
        };

        // Apply access controls to reduce the set if required.
        // delete_allow_operation
        let mut audit_acp = AuditScope::new("access_control_profiles");
        let access = self.get_accesscontrols();
        let acp_res = access.delete_allow_operation(&mut audit_acp, de, &pre_candidates);
        au.append_scope(audit_acp);
        if !try_audit!(au, acp_res) {
            return Err(OperationError::AccessDenied);
        }

        // Is the candidate set empty?
        if pre_candidates.is_empty() {
            audit_log!(au, "delete: no candidates match filter {:?}", de.filter);
            return Err(OperationError::NoMatchingEntries);
        };

        let mut candidates: Vec<Entry<EntryInvalid, EntryCommitted>> = pre_candidates
            .iter()
            // Invalidate and assign change id's
            .map(|er| er.clone().invalidate(self.cid.clone()))
            .collect();

        audit_log!(au, "delete: candidates -> {:?}", candidates);

        // Pre delete plugs
        let mut audit_plugin_pre = AuditScope::new("plugin_pre_delete");
        let plug_pre_res =
            Plugins::run_pre_delete(&mut audit_plugin_pre, self, &mut candidates, de);
        au.append_scope(audit_plugin_pre);

        if plug_pre_res.is_err() {
            audit_log!(au, "Delete operation failed (plugin), {:?}", plug_pre_res);
            return plug_pre_res;
        }

        audit_log!(
            au,
            "delete: now marking candidates as recycled -> {:?}",
            candidates
        );

        let res: Result<Vec<Entry<EntrySealed, EntryCommitted>>, SchemaError> = candidates
            .into_iter()
            .map(|e| {
                e.to_recycled()
                    .validate(&self.schema)
                    // seal if it worked.
                    .map(|r| r.seal())
            })
            .collect();

        let del_cand: Vec<Entry<_, _>> = match res {
            Ok(v) => v,
            Err(e) => return Err(OperationError::SchemaViolation(e)),
        };

        let mut audit_be = AuditScope::new("backend_modify");

        let res = self
            .be_txn
            .modify(&mut audit_be, &pre_candidates, &del_cand);
        au.append_scope(audit_be);

        if res.is_err() {
            // be_txn is dropped, ie aborted here.
            audit_log!(au, "Delete operation failed (backend), {:?}", res);
            return res;
        }

        // Post delete plugs
        let mut audit_plugin_post = AuditScope::new("plugin_post_delete");
        let plug_post_res = Plugins::run_post_delete(&mut audit_plugin_post, self, &del_cand, de);
        au.append_scope(audit_plugin_post);

        if plug_post_res.is_err() {
            audit_log!(au, "Delete operation failed (plugin), {:?}", plug_post_res);
            return plug_post_res;
        }

        // We have finished all plugs and now have a successful operation - flag if
        // schema or acp requires reload.
        self.changed_schema = del_cand.iter().fold(false, |acc, e| {
            if acc {
                acc
            } else {
                e.attribute_value_pres("class", &PVCLASS_CLASSTYPE)
                    || e.attribute_value_pres("class", &PVCLASS_ATTRIBUTETYPE)
            }
        });
        self.changed_acp = del_cand.iter().fold(false, |acc, e| {
            if acc {
                acc
            } else {
                e.attribute_value_pres("class", &PVCLASS_ACP)
            }
        });
        audit_log!(
            au,
            "Schema reload: {:?}, ACP reload: {:?}",
            self.changed_schema,
            self.changed_acp
        );

        // Send result
        audit_log!(au, "Delete operation success");
        res
    }

    pub fn purge_tombstones(&mut self, au: &mut AuditScope) -> Result<(), OperationError> {
        // delete everything that is a tombstone.

        // TODO #68: Has an appropriate amount of time/condition past (ie replication events?)
        // Search for tombstones
        let cid = try_audit!(au, self.cid.sub_secs(CHANGELOG_MAX_AGE));
        let ts = match self.internal_search(
            au,
            filter_all!(f_and!([
                f_eq("class", PVCLASS_TOMBSTONE.clone()),
                f_lt("last_modified_cid", PartialValue::new_cid(cid)),
            ])),
        ) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        if ts.is_empty() {
            audit_log!(au, "No Tombstones present - purge operation success");
            return Ok(());
        }

        // Delete them
        let mut audit_be = AuditScope::new("backend_delete");

        let res = self
            .be_txn
            // Change this to an update, not delete.
            .delete(&mut audit_be, &ts);
        au.append_scope(audit_be);

        if res.is_err() {
            // be_txn is dropped, ie aborted here.
            audit_log!(au, "Tombstone purge operation failed (backend), {:?}", res);
            return res;
        }

        // Send result
        audit_log!(au, "Tombstone purge operation success");
        res
    }

    pub fn purge_recycled(&mut self, au: &mut AuditScope) -> Result<(), OperationError> {
        // Send everything that is recycled to tombstone
        // Search all recycled

        let cid = try_audit!(au, self.cid.sub_secs(RECYCLEBIN_MAX_AGE));
        let rc = match self.internal_search(
            au,
            filter_all!(f_and!([
                f_eq("class", PVCLASS_RECYCLED.clone()),
                f_lt("last_modified_cid", PartialValue::new_cid(cid)),
            ])),
        ) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        if rc.is_empty() {
            audit_log!(au, "No recycled present - purge operation success");
            return Ok(());
        }

        // Modify them to strip all avas except uuid
        let tombstone_cand: Result<Vec<_>, _> = rc
            .iter()
            .map(|e| {
                e.to_tombstone(self.cid.clone())
                    .validate(&self.schema)
                    .map_err(OperationError::SchemaViolation)
                    // seal if it worked.
                    .map(|r| r.seal())
            })
            .collect();

        let tombstone_cand = try_audit!(au, tombstone_cand);

        // Backend Modify
        let mut audit_be = AuditScope::new("backend_modify");

        let res = self.be_txn.modify(&mut audit_be, &rc, &tombstone_cand);
        au.append_scope(audit_be);

        if res.is_err() {
            // be_txn is dropped, ie aborted here.
            audit_log!(au, "Purge recycled operation failed (backend), {:?}", res);
            return res;
        }

        // return
        audit_log!(au, "Purge recycled operation success");
        res
    }

    // Should this take a revive event?
    pub fn revive_recycled(
        &mut self,
        au: &mut AuditScope,
        re: &ReviveRecycledEvent,
    ) -> Result<(), OperationError> {
        // Revive an entry to live. This is a specialised (limited)
        // modify proxy.
        //
        // impersonate modify will require ability to search the class=recycled
        // and the ability to remove that from the object.

        // create the modify
        // tl;dr, remove the class=recycled
        let modlist = ModifyList::new_list(vec![Modify::Removed(
            "class".to_string(),
            PVCLASS_RECYCLED.clone(),
        )]);

        let m_valid = try_audit!(
            au,
            modlist
                .validate(self.get_schema())
                .map_err(OperationError::SchemaViolation)
        );

        // Get the entries we are about to revive.
        //    we make a set of per-entry mod lists. A list of lists even ...
        let revive_cands =
            self.impersonate_search_valid(au, re.filter.clone(), re.filter.clone(), &re.event)?;

        let mut dm_mods: BTreeMap<Uuid, ModifyList<ModifyInvalid>> = BTreeMap::new();

        revive_cands.into_iter().for_each(|e| {
            // Get this entries uuid.
            let u: Uuid = e.get_uuid().clone();

            e.get_ava_reference_uuid("directmemberof").and_then(|list| {
                list.into_iter().for_each(|g_uuid| {
                    dm_mods
                        .entry(g_uuid.clone())
                        .and_modify(|mlist| {
                            let m = Modify::Present("member".to_string(), Value::new_refer_r(&u));
                            mlist.push_mod(m);
                        })
                        .or_insert({
                            let m = Modify::Present("member".to_string(), Value::new_refer_r(&u));
                            ModifyList::new_list(vec![m])
                        });
                });
                Some(())
            });
        });

        // Now impersonate the modify
        self.impersonate_modify_valid(
            au,
            re.filter.clone(),
            re.filter.clone(),
            m_valid,
            &re.event,
        )?;
        // If and only if that succeeds, apply the direct membership modifications
        // if possible.
        let r: Result<_, _> = dm_mods
            .into_iter()
            .map(|(g, mods)| {
                // I think the filter/filter_all shouldn't matter here because the only
                // valid direct memberships should be still valid/live references.
                let f = filter_all!(f_eq("uuid", PartialValue::new_uuid(g)));
                self.internal_modify(au, f, mods)
            })
            .collect();
        r
    }

    pub fn modify(&mut self, au: &mut AuditScope, me: &ModifyEvent) -> Result<(), OperationError> {
        // Get the candidates.
        // Modify applies a modlist to a filter, so we need to internal search
        // then apply.

        // Validate input.

        // Is the modlist non zero?
        if me.modlist.len() == 0 {
            audit_log!(au, "modify: empty modify request");
            return Err(OperationError::EmptyRequest);
        }

        // Is the modlist valid?
        // This is now done in the event transform

        // Is the filter invalid to schema?
        // This is now done in the event transform

        // This also checks access controls due to use of the impersonation.
        let pre_candidates = match self.impersonate_search_valid(
            au,
            me.filter.clone(),
            me.filter_orig.clone(),
            &me.event,
        ) {
            Ok(results) => results,
            Err(e) => {
                audit_log!(au, "modify: error in pre-candidate selection {:?}", e);
                return Err(e);
            }
        };

        if pre_candidates.is_empty() {
            match me.event.origin {
                EventOrigin::Internal => {
                    audit_log!(
                        au,
                        "modify: no candidates match filter ... continuing {:?}",
                        me.filter
                    );
                    return Ok(());
                }
                _ => {
                    audit_log!(
                        au,
                        "modify: no candidates match filter, failure {:?}",
                        me.filter
                    );
                    return Err(OperationError::NoMatchingEntries);
                }
            }
        };

        // Are we allowed to make the changes we want to?
        // modify_allow_operation
        let mut audit_acp = AuditScope::new("access_control_profiles");
        let access = self.get_accesscontrols();
        let acp_res = access.modify_allow_operation(&mut audit_acp, me, &pre_candidates);
        au.append_scope(audit_acp);
        if !try_audit!(au, acp_res) {
            return Err(OperationError::AccessDenied);
        }

        // Clone a set of writeables.
        // Apply the modlist -> Remember, we have a set of origs
        // and the new modified ents.
        let mut candidates: Vec<Entry<EntryInvalid, EntryCommitted>> = pre_candidates
            .iter()
            .map(|er| er.clone().invalidate(self.cid.clone()))
            .collect();

        candidates
            .iter_mut()
            .for_each(|er| er.apply_modlist(&me.modlist));

        audit_log!(au, "modify: candidates -> {:?}", candidates);

        // Pre mod plugins
        let mut audit_plugin_pre = AuditScope::new("plugin_pre_modify");
        // We should probably supply the pre-post cands here.
        let plug_pre_res =
            Plugins::run_pre_modify(&mut audit_plugin_pre, self, &mut candidates, me);
        au.append_scope(audit_plugin_pre);

        if plug_pre_res.is_err() {
            audit_log!(au, "Modify operation failed (plugin), {:?}", plug_pre_res);
            return plug_pre_res;
        }

        // NOTE: There is a potential optimisation here, where if
        // candidates == pre-candidates, then we don't need to store anything
        // because we effectively just did an assert. However, like all
        // optimisations, this could be premature - so we for now, just
        // do the CORRECT thing and recommit as we may find later we always
        // want to add CSN's or other.

        let res: Result<Vec<Entry<EntrySealed, EntryCommitted>>, SchemaError> = candidates
            .into_iter()
            .map(|e| e.validate(&self.schema).map(|e| e.seal()))
            .collect();

        let norm_cand: Vec<Entry<_, _>> = match res {
            Ok(v) => v,
            Err(e) => return Err(OperationError::SchemaViolation(e)),
        };

        // Backend Modify
        let mut audit_be = AuditScope::new("backend_modify");

        let res = self
            .be_txn
            .modify(&mut audit_be, &pre_candidates, &norm_cand);
        au.append_scope(audit_be);

        if res.is_err() {
            // be_txn is dropped, ie aborted here.
            audit_log!(au, "Modify operation failed (backend), {:?}", res);
            return res;
        }

        // Post Plugins
        //
        // memberOf actually wants the pre cand list and the norm_cand list to see what
        // changed. Could be optimised, but this is correct still ...
        let mut audit_plugin_post = AuditScope::new("plugin_post_modify");
        let plug_post_res = Plugins::run_post_modify(
            &mut audit_plugin_post,
            self,
            &pre_candidates,
            &norm_cand,
            me,
        );
        au.append_scope(audit_plugin_post);

        if plug_post_res.is_err() {
            audit_log!(au, "Modify operation failed (plugin), {:?}", plug_post_res);
            return plug_post_res;
        }

        // We have finished all plugs and now have a successful operation - flag if
        // schema or acp requires reload. Remember, this is a modify, so we need to check
        // pre and post cands.
        self.changed_schema =
            norm_cand
                .iter()
                .chain(pre_candidates.iter())
                .fold(false, |acc, e| {
                    if acc {
                        acc
                    } else {
                        e.attribute_value_pres("class", &PVCLASS_CLASSTYPE)
                            || e.attribute_value_pres("class", &PVCLASS_ATTRIBUTETYPE)
                    }
                });
        self.changed_acp = norm_cand
            .iter()
            .chain(pre_candidates.iter())
            .fold(false, |acc, e| {
                if acc {
                    acc
                } else {
                    e.attribute_value_pres("class", &PVCLASS_ACP)
                }
            });
        audit_log!(
            au,
            "Schema reload: {:?}, ACP reload: {:?}",
            self.changed_schema,
            self.changed_acp
        );

        // return
        audit_log!(au, "Modify operation success");
        res
    }

    // These are where searches and other actions are actually implemented. This
    // is the "internal" version, where we define the event as being internal
    // only, allowing certain plugin by passes etc.

    pub fn internal_create(
        &mut self,
        audit: &mut AuditScope,
        entries: Vec<Entry<EntryInit, EntryNew>>,
    ) -> Result<(), OperationError> {
        // Start the audit scope
        let mut audit_int = AuditScope::new("internal_create");
        // Create the CreateEvent
        let ce = CreateEvent::new_internal(entries);
        let res = self.create(&mut audit_int, &ce);
        audit.append_scope(audit_int);
        res
    }

    pub fn internal_delete(
        &mut self,
        audit: &mut AuditScope,
        filter: Filter<FilterInvalid>,
    ) -> Result<(), OperationError> {
        let f_valid = filter
            .validate(self.get_schema())
            .map_err(OperationError::SchemaViolation)?;
        let mut audit_int = AuditScope::new("internal_delete");
        let de = DeleteEvent::new_internal(f_valid);
        let res = self.delete(&mut audit_int, &de);
        audit.append_scope(audit_int);
        res
    }

    pub fn internal_modify(
        &mut self,
        audit: &mut AuditScope,
        filter: Filter<FilterInvalid>,
        modlist: ModifyList<ModifyInvalid>,
    ) -> Result<(), OperationError> {
        let f_valid = filter
            .validate(self.get_schema())
            .map_err(OperationError::SchemaViolation)?;
        let m_valid = modlist
            .validate(self.get_schema())
            .map_err(OperationError::SchemaViolation)?;
        let mut audit_int = AuditScope::new("internal_modify");
        let me = ModifyEvent::new_internal(f_valid, m_valid);
        let res = self.modify(&mut audit_int, &me);
        audit.append_scope(audit_int);
        res
    }

    pub fn impersonate_modify_valid(
        &mut self,
        audit: &mut AuditScope,
        f_valid: Filter<FilterValid>,
        f_intent_valid: Filter<FilterValid>,
        m_valid: ModifyList<ModifyValid>,
        event: &Event,
    ) -> Result<(), OperationError> {
        let mut audit_int = AuditScope::new("impersonate_modify");
        let me = ModifyEvent::new_impersonate(event, f_valid, f_intent_valid, m_valid);
        let res = self.modify(&mut audit_int, &me);
        audit.append_scope(audit_int);
        res
    }

    pub fn impersonate_modify(
        &mut self,
        audit: &mut AuditScope,
        filter: Filter<FilterInvalid>,
        filter_intent: Filter<FilterInvalid>,
        modlist: ModifyList<ModifyInvalid>,
        event: &Event,
    ) -> Result<(), OperationError> {
        let f_valid = try_audit!(
            audit,
            filter
                .validate(self.get_schema())
                .map_err(OperationError::SchemaViolation)
        );
        let f_intent_valid = try_audit!(
            audit,
            filter_intent
                .validate(self.get_schema())
                .map_err(OperationError::SchemaViolation)
        );
        let m_valid = try_audit!(
            audit,
            modlist
                .validate(self.get_schema())
                .map_err(OperationError::SchemaViolation)
        );
        self.impersonate_modify_valid(audit, f_valid, f_intent_valid, m_valid, event)
    }

    // internal server operation types.
    // These just wrap the fn create/search etc, but they allow
    // creating the needed create event with the correct internal flags
    // and markers. They act as though they have the highest level privilege
    // IE there are no access control checks.

    pub fn internal_exists_or_create(
        &self,
        _e: Entry<EntryValid, EntryNew>,
    ) -> Result<(), OperationError> {
        // If the thing exists, stop.
        // if not, create from Entry.
        unimplemented!()
    }

    pub fn internal_migrate_or_create_str(
        &mut self,
        audit: &mut AuditScope,
        e_str: &str,
    ) -> Result<(), OperationError> {
        let res = audit_segment!(audit, || Entry::from_proto_entry_str(audit, e_str, self)
            /*
            .and_then(|e: Entry<EntryInvalid, EntryNew>| {
                let schema = self.get_schema();
                e.validate(schema).map_err(OperationError::SchemaViolation)
            })
            */
            .and_then(
                |e: Entry<EntryInit, EntryNew>| self.internal_migrate_or_create(audit, e)
            ));
        audit_log!(audit, "internal_migrate_or_create_str -> result {:?}", res);
        debug_assert!(res.is_ok());
        res
    }

    pub fn internal_migrate_or_create(
        &mut self,
        audit: &mut AuditScope,
        e: Entry<EntryInit, EntryNew>,
    ) -> Result<(), OperationError> {
        // if the thing exists, ensure the set of attributes on
        // Entry A match and are present (but don't delete multivalue, or extended
        // attributes in the situation.
        // If not exist, create from Entry B
        //
        // This will extra classes an attributes alone!
        //
        // NOTE: gen modlist IS schema aware and will handle multivalue
        // correctly!
        audit_log!(
            audit,
            "internal_migrate_or_create operating on {:?}",
            e.get_uuid()
        );

        let filt = match e.filter_from_attrs(&[String::from("uuid")]) {
            Some(f) => f,
            None => return Err(OperationError::FilterGeneration),
        };

        match self.internal_search(audit, filt.clone()) {
            Ok(results) => {
                if results.is_empty() {
                    // It does not exist. Create it.
                    self.internal_create(audit, vec![e])
                } else if results.len() == 1 {
                    // If the thing is subset, pass
                    match e.gen_modlist_assert(&self.schema) {
                        Ok(modlist) => {
                            // Apply to &results[0]
                            audit_log!(audit, "Generated modlist -> {:?}", modlist);
                            self.internal_modify(audit, filt, modlist)
                        }
                        Err(e) => Err(OperationError::SchemaViolation(e)),
                    }
                } else {
                    Err(OperationError::InvalidDBState)
                }
            }
            Err(e) => {
                // An error occured. pass it back up.
                Err(e)
            }
        }
    }

    /*
    pub fn internal_assert_or_create_str(
        &mut self,
        audit: &mut AuditScope,
        e_str: &str,
    ) -> Result<(), OperationError> {
        let res = audit_segment!(audit, || Entry::from_proto_entry_str(audit, e_str, self)
            .and_then(
                |e: Entry<EntryInit, EntryNew>| self.internal_assert_or_create(audit, e)
            ));
        audit_log!(audit, "internal_assert_or_create_str -> result {:?}", res);
        debug_assert!(res.is_ok());
        res
    }

    // Should this take a be_txn?
    pub fn internal_assert_or_create(
        &mut self,
        audit: &mut AuditScope,
        e: Entry<EntryInit, EntryNew>,
    ) -> Result<(), OperationError> {
        // If exists, ensure the object is exactly as provided
        // else, if not exists, create it. IE no extra or excess
        // attributes and classes.

        audit_log!(
            audit,
            "internal_assert_or_create operating on {:?}",
            e.get_uuid()
        );

        // Create a filter from the entry for assertion.
        let filt = match e.filter_from_attrs(&[String::from("uuid")]) {
            Some(f) => f,
            None => return Err(OperationError::FilterGeneration),
        };

        // Does it exist? we use search here, not exists, so that if the entry does exist
        // we can compare it is identical, which avoids a delete/create cycle that would
        // trigger csn/repl each time we start up.
        match self.internal_search(audit, filt.clone()) {
            Ok(results) => {
                if results.is_empty() {
                    // It does not exist. Create it.
                    self.internal_create(audit, vec![e])
                } else if results.len() == 1 {
                    // it exists. To guarantee content exactly as is, we compare if it's identical.
                    if !e.compare(&results[0]) {
                        self.internal_delete(audit, filt)
                            .and_then(|_| self.internal_create(audit, vec![e]))
                    } else {
                        // No action required
                        Ok(())
                    }
                } else {
                    Err(OperationError::InvalidDBState)
                }
            }
            Err(er) => {
                // An error occured. pass it back up.
                Err(er)
            }
        }
    }
    */

    pub fn initialise_schema_core(&mut self, audit: &mut AuditScope) -> Result<(), OperationError> {
        // Load in all the "core" schema, that we already have in "memory".
        let entries = self.schema.to_entries();

        // internal_migrate_or_create.
        let r: Result<_, _> = entries
            .into_iter()
            .map(|e| {
                audit_log!(audit, "init schema -> {}", e);
                self.internal_migrate_or_create(audit, e)
            })
            .collect();
        audit_log!(audit, "initialise_schema_core -> result {:?}", r);
        debug_assert!(r.is_ok());
        r
    }

    pub fn initialise_schema_idm(&mut self, audit: &mut AuditScope) -> Result<(), OperationError> {
        // List of IDM schemas to init.
        let idm_schema: Vec<&str> = vec![
            JSON_SCHEMA_ATTR_DISPLAYNAME,
            JSON_SCHEMA_ATTR_LEGALNAME,
            JSON_SCHEMA_ATTR_MAIL,
            JSON_SCHEMA_ATTR_SSH_PUBLICKEY,
            JSON_SCHEMA_ATTR_PRIMARY_CREDENTIAL,
            JSON_SCHEMA_ATTR_RADIUS_SECRET,
            JSON_SCHEMA_ATTR_DOMAIN_NAME,
            JSON_SCHEMA_ATTR_DOMAIN_UUID,
            JSON_SCHEMA_ATTR_DOMAIN_SSID,
            JSON_SCHEMA_ATTR_GIDNUMBER,
            JSON_SCHEMA_ATTR_BADLIST_PASSWORD,
            JSON_SCHEMA_ATTR_LOGINSHELL,
            JSON_SCHEMA_ATTR_UNIX_PASSWORD,
            JSON_SCHEMA_CLASS_PERSON,
            JSON_SCHEMA_CLASS_GROUP,
            JSON_SCHEMA_CLASS_ACCOUNT,
            JSON_SCHEMA_CLASS_DOMAIN_INFO,
            JSON_SCHEMA_CLASS_POSIXACCOUNT,
            JSON_SCHEMA_CLASS_POSIXGROUP,
            JSON_SCHEMA_CLASS_SYSTEM_CONFIG,
        ];

        let mut audit_si = AuditScope::new("start_initialise_schema_idm");
        let r: Result<Vec<()>, _> = idm_schema
            .iter()
            // Each item individually logs it's result
            .map(|e_str| self.internal_migrate_or_create_str(&mut audit_si, e_str))
            .collect();
        audit.append_scope(audit_si);
        audit_log!(audit, "initialise_schema_idm -> result {:?}", r);
        debug_assert!(r.is_ok());

        r.map(|_| ())
    }

    // This function is idempotent
    pub fn initialise_idm(&mut self, audit: &mut AuditScope) -> Result<(), OperationError> {
        // First, check the system_info object. This stores some server information
        // and details. It's a pretty const thing. Also check anonymous, important to many
        // concepts.
        let mut audit_an = AuditScope::new("start_system_core_items");
        let res = self
            .internal_migrate_or_create_str(&mut audit_an, JSON_SYSTEM_INFO_V1)
            .and_then(|_| self.internal_migrate_or_create_str(&mut audit_an, JSON_DOMAIN_INFO_V1))
            .and_then(|_| {
                self.internal_migrate_or_create_str(&mut audit_an, JSON_SYSTEM_CONFIG_V1)
            });
        audit.append_scope(audit_an);
        audit_log!(audit, "initialise_idm p1 -> result {:?}", res);
        debug_assert!(res.is_ok());
        if res.is_err() {
            return res;
        }

        // The domain info now exists, we should be able to do these migrations as they will
        // cause SPN regenerations to occur

        // Check the admin object exists (migrations).
        // Create the default idm_admin group.
        let admin_entries = [
            JSON_ANONYMOUS_V1,
            JSON_ADMIN_V1,
            JSON_IDM_ADMIN_V1,
            JSON_IDM_ADMINS_V1,
            JSON_SYSTEM_ADMINS_V1,
        ];
        let mut audit_an = AuditScope::new("start_idm_admin_migrations");
        let res: Result<(), _> = admin_entries
            .iter()
            // Each item individually logs it's result
            .map(|e_str| self.internal_migrate_or_create_str(&mut audit_an, e_str))
            .collect();
        audit.append_scope(audit_an);
        audit_log!(audit, "initialise_idm p2 -> result {:?}", res);
        debug_assert!(res.is_ok());
        if res.is_err() {
            return res;
        }

        // Create any system default schema entries.

        // Create any system default access profile entries.
        let mut audit_an = AuditScope::new("start_idm_migrations_internal");
        let idm_entries = [
            // Builtin groups
            JSON_IDM_PEOPLE_MANAGE_PRIV_V1,
            JSON_IDM_PEOPLE_ACCOUNT_PASSWORD_IMPORT_PRIV_V1,
            JSON_IDM_PEOPLE_EXTEND_PRIV_V1,
            JSON_IDM_PEOPLE_WRITE_PRIV_V1,
            JSON_IDM_PEOPLE_READ_PRIV_V1,
            JSON_IDM_GROUP_MANAGE_PRIV_V1,
            JSON_IDM_GROUP_WRITE_PRIV_V1,
            JSON_IDM_GROUP_UNIX_EXTEND_PRIV_V1,
            JSON_IDM_ACCOUNT_MANAGE_PRIV_V1,
            JSON_IDM_ACCOUNT_WRITE_PRIV_V1,
            JSON_IDM_ACCOUNT_UNIX_EXTEND_PRIV_V1,
            JSON_IDM_ACCOUNT_READ_PRIV_V1,
            JSON_IDM_RADIUS_SERVERS_V1,
            // Write deps on read, so write must be added first.
            JSON_IDM_HP_ACCOUNT_MANAGE_PRIV_V1,
            JSON_IDM_HP_ACCOUNT_WRITE_PRIV_V1,
            JSON_IDM_HP_ACCOUNT_READ_PRIV_V1,
            JSON_IDM_SCHEMA_MANAGE_PRIV_V1,
            JSON_IDM_HP_GROUP_MANAGE_PRIV_V1,
            JSON_IDM_HP_GROUP_WRITE_PRIV_V1,
            JSON_IDM_ACP_MANAGE_PRIV_V1,
            JSON_DOMAIN_ADMINS,
            JSON_IDM_HIGH_PRIVILEGE_V1,
            // Built in access controls.
            JSON_IDM_ADMINS_ACP_RECYCLE_SEARCH_V1,
            JSON_IDM_ADMINS_ACP_REVIVE_V1,
            // JSON_IDM_ADMINS_ACP_MANAGE_V1,
            JSON_IDM_ALL_ACP_READ_V1,
            JSON_IDM_SELF_ACP_READ_V1,
            JSON_IDM_SELF_ACP_WRITE_V1,
            JSON_IDM_ACP_PEOPLE_READ_PRIV_V1,
            JSON_IDM_ACP_PEOPLE_WRITE_PRIV_V1,
            JSON_IDM_ACP_PEOPLE_MANAGE_PRIV_V1,
            JSON_IDM_ACP_GROUP_WRITE_PRIV_V1,
            JSON_IDM_ACP_GROUP_MANAGE_PRIV_V1,
            JSON_IDM_ACP_ACCOUNT_READ_PRIV_V1,
            JSON_IDM_ACP_ACCOUNT_WRITE_PRIV_V1,
            JSON_IDM_ACP_ACCOUNT_MANAGE_PRIV_V1,
            JSON_IDM_ACP_RADIUS_SERVERS_V1,
            JSON_IDM_ACP_HP_ACCOUNT_READ_PRIV_V1,
            JSON_IDM_ACP_HP_ACCOUNT_WRITE_PRIV_V1,
            JSON_IDM_ACP_HP_ACCOUNT_MANAGE_PRIV_V1,
            JSON_IDM_ACP_HP_GROUP_WRITE_PRIV_V1,
            JSON_IDM_ACP_HP_GROUP_MANAGE_PRIV_V1,
            JSON_IDM_ACP_SCHEMA_WRITE_ATTRS_PRIV_V1,
            JSON_IDM_ACP_SCHEMA_WRITE_CLASSES_PRIV_V1,
            JSON_IDM_ACP_ACP_MANAGE_PRIV_V1,
            JSON_IDM_ACP_DOMAIN_ADMIN_PRIV_V1,
            JSON_IDM_ACP_SYSTEM_CONFIG_PRIV_V1,
            JSON_IDM_ACP_ACCOUNT_UNIX_EXTEND_PRIV_V1,
            JSON_IDM_ACP_GROUP_UNIX_EXTEND_PRIV_V1,
            JSON_IDM_ACP_PEOPLE_ACCOUNT_PASSWORD_IMPORT_PRIV_V1,
            JSON_IDM_ACP_PEOPLE_EXTEND_PRIV_V1,
        ];

        let res: Result<(), _> = idm_entries
            .iter()
            // Each item individually logs it's result
            .map(|e_str| self.internal_migrate_or_create_str(&mut audit_an, e_str))
            .collect();
        audit.append_scope(audit_an);
        audit_log!(audit, "initialise_idm p3 -> result {:?}", res);
        debug_assert!(res.is_ok());
        if res.is_err() {
            return res;
        }

        Ok(())
    }

    fn reload_schema(&mut self, audit: &mut AuditScope) -> Result<(), OperationError> {
        // supply entries to the writable schema to reload from.
        // find all attributes.
        let filt = filter!(f_eq("class", PVCLASS_ATTRIBUTETYPE.clone()));
        let res = try_audit!(audit, self.internal_search(audit, filt));
        // load them.
        let attributetypes: Result<Vec<_>, _> = res
            .iter()
            .map(|e| SchemaAttribute::try_from(audit, e))
            .collect();
        let attributetypes = try_audit!(audit, attributetypes);

        try_audit!(audit, self.schema.update_attributes(attributetypes));

        // find all classes
        let filt = filter!(f_eq("class", PVCLASS_CLASSTYPE.clone()));
        let res = try_audit!(audit, self.internal_search(audit, filt));
        // load them.
        let classtypes: Result<Vec<_>, _> = res
            .iter()
            .map(|e| SchemaClass::try_from(audit, e))
            .collect();
        let classtypes = try_audit!(audit, classtypes);

        try_audit!(audit, self.schema.update_classes(classtypes));

        // validate.
        let valid_r = self.schema.validate(audit);

        // Translate the result.
        if valid_r.is_empty() {
            Ok(())
        } else {
            // Log the failures?
            audit_log!(audit, "Schema reload failed -> {:?}", valid_r);
            Err(OperationError::ConsistencyError(valid_r))
        }
    }

    fn reload_accesscontrols(&mut self, audit: &mut AuditScope) -> Result<(), OperationError> {
        // supply entries to the writable access controls to reload from.
        // This has to be done in FOUR passes - one for each type!
        //
        // Note, we have to do the search, parse, then submit here, because of the
        // requirement to have the write query server reference in the parse stage - this
        // would cause a rust double-borrow if we had AccessControls to try to handle
        // the entry lists themself.

        // Update search
        let filt = filter!(f_and!([
            f_eq("class", PVCLASS_ACP.clone()),
            f_eq("class", PVCLASS_ACS.clone()),
            f_andnot(f_eq("acp_enable", PVACP_ENABLE_FALSE.clone())),
        ]));

        let res = try_audit!(audit, self.internal_search(audit, filt));
        let search_acps: Result<Vec<_>, _> = res
            .iter()
            .map(|e| AccessControlSearch::try_from(audit, self, e))
            .collect();

        let search_acps = try_audit!(audit, search_acps);

        try_audit!(audit, self.accesscontrols.update_search(search_acps));
        // Update create
        let filt = filter!(f_and!([
            f_eq("class", PVCLASS_ACP.clone()),
            f_eq("class", PVCLASS_ACC.clone()),
            f_andnot(f_eq("acp_enable", PVACP_ENABLE_FALSE.clone())),
        ]));

        let res = try_audit!(audit, self.internal_search(audit, filt));
        let create_acps: Result<Vec<_>, _> = res
            .iter()
            .map(|e| AccessControlCreate::try_from(audit, self, e))
            .collect();

        let create_acps = try_audit!(audit, create_acps);

        try_audit!(audit, self.accesscontrols.update_create(create_acps));
        // Update modify
        let filt = filter!(f_and!([
            f_eq("class", PVCLASS_ACP.clone()),
            f_eq("class", PVCLASS_ACM.clone()),
            f_andnot(f_eq("acp_enable", PVACP_ENABLE_FALSE.clone())),
        ]));

        let res = try_audit!(audit, self.internal_search(audit, filt));
        let modify_acps: Result<Vec<_>, _> = res
            .iter()
            .map(|e| AccessControlModify::try_from(audit, self, e))
            .collect();

        let modify_acps = try_audit!(audit, modify_acps);

        try_audit!(audit, self.accesscontrols.update_modify(modify_acps));
        // Update delete
        let filt = filter!(f_and!([
            f_eq("class", PVCLASS_ACP.clone()),
            f_eq("class", PVCLASS_ACD.clone()),
            f_andnot(f_eq("acp_enable", PVACP_ENABLE_FALSE.clone())),
        ]));

        let res = try_audit!(audit, self.internal_search(audit, filt));
        let delete_acps: Result<Vec<_>, _> = res
            .iter()
            .map(|e| AccessControlDelete::try_from(audit, self, e))
            .collect();

        let delete_acps = try_audit!(audit, delete_acps);

        try_audit!(audit, self.accesscontrols.update_delete(delete_acps));
        // Alternately, we just get ACP class, and just let acctrl work it out ...
        Ok(())
    }

    pub(crate) fn get_domain_uuid(&self) -> Uuid {
        self.d_uuid
    }

    /// Initiate a domain rename process. This is generally an internal function but it's
    /// exposed to the cli for admins to be able to initiate the process.
    pub fn domain_rename(
        &mut self,
        audit: &mut AuditScope,
        new_domain_name: &str,
    ) -> Result<(), OperationError> {
        let modl = ModifyList::new_purge_and_set("domain_name", Value::new_iutf8s(new_domain_name));
        let udi = PartialValue::new_uuids(UUID_DOMAIN_INFO).ok_or(OperationError::InvalidUuid)?;
        let filt = filter_all!(f_eq("uuid", udi));
        self.internal_modify(audit, filt, modl)
    }

    pub fn reindex(&mut self, audit: &mut AuditScope) -> Result<(), OperationError> {
        // initiate a be reindex here. This could have been from first run checking
        // the versions, or it could just be from the cli where an admin needs to do an
        // indexing.
        self.be_txn.reindex(audit)
    }

    pub(crate) fn upgrade_reindex(
        &mut self,
        audit: &mut AuditScope,
        v: i64,
    ) -> Result<(), OperationError> {
        self.be_txn.upgrade_reindex(audit, v)
    }

    pub fn commit(mut self, audit: &mut AuditScope) -> Result<(), OperationError> {
        // This could be faster if we cache the set of classes changed
        // in an operation so we can check if we need to do the reload or not
        //
        // Reload the schema from qs.
        if self.changed_schema {
            self.reload_schema(audit)?;
        }
        // Determine if we need to update access control profiles
        // based on any modifications that have occured.
        // IF SCHEMA CHANGED WE MUST ALSO RELOAD!!! IE if schema had an attr removed
        // that we rely on we MUST fail this here!!
        if self.changed_schema || self.changed_acp {
            self.reload_accesscontrols(audit)?;
        }

        // Now destructure the transaction ready to reset it.
        let QueryServerWriteTransaction {
            committed,
            be_txn,
            schema,
            accesscontrols,
            ..
        } = self;
        debug_assert!(!committed);
        // Begin an audit.
        // Validate the schema as we just loaded it.
        let r = schema.validate(audit);

        if r.is_empty() {
            // Schema has been validated, so we can go ahead and commit it with the be
            // because both are consistent.
            schema
                .commit()
                .and_then(|_| accesscontrols.commit().and_then(|_| be_txn.commit(audit)))
        } else {
            Err(OperationError::ConsistencyError(r))
        }
        // Audit done
    }
}

// Auth requests? How do we structure these ...

#[cfg(test)]
mod tests {
    use crate::audit::AuditScope;
    use crate::constants::{CHANGELOG_MAX_AGE, JSON_ADMIN_V1, RECYCLEBIN_MAX_AGE, UUID_ADMIN};
    use crate::credential::Credential;
    use crate::entry::{Entry, EntryInit, EntryNew};
    use crate::event::{CreateEvent, DeleteEvent, ModifyEvent, ReviveRecycledEvent, SearchEvent};
    use crate::modify::{Modify, ModifyList};
    use crate::server::{QueryServerTransaction, QueryServerWriteTransaction};
    use crate::value::{PartialValue, Value};
    use kanidm_proto::v1::{OperationError, SchemaError};
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn test_qs_create_user() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            let mut server_txn = server.write(duration_from_epoch_now());
            let filt = filter!(f_eq("name", PartialValue::new_iutf8s("testperson")));
            let admin = server_txn
                .internal_search_uuid(audit, &UUID_ADMIN)
                .expect("failed");

            let se1 = unsafe { SearchEvent::new_impersonate_entry(admin.clone(), filt.clone()) };
            let se2 = unsafe { SearchEvent::new_impersonate_entry(admin, filt) };

            let e: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "description": ["testperson"],
                    "displayname": ["testperson"]
                }
            }"#,
            );

            let ce = CreateEvent::new_internal(vec![e.clone()]);

            let r1 = server_txn.search(audit, &se1).expect("search failure");
            assert!(r1.is_empty());

            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            let r2 = server_txn.search(audit, &se2).expect("search failure");
            println!("--> {:?}", r2);
            assert!(r2.len() == 1);

            let expected = unsafe { vec![e.into_sealed_committed()] };

            assert_eq!(r2, expected);

            assert!(server_txn.commit(audit).is_ok());
        });
    }

    #[test]
    fn test_qs_init_idempotent_schema_core() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            {
                // Setup and abort.
                let mut server_txn = server.write(duration_from_epoch_now());
                assert!(server_txn.initialise_schema_core(audit).is_ok());
            }
            {
                let mut server_txn = server.write(duration_from_epoch_now());
                assert!(server_txn.initialise_schema_core(audit).is_ok());
                assert!(server_txn.initialise_schema_core(audit).is_ok());
                assert!(server_txn.commit(audit).is_ok());
            }
            {
                // Now do it again in a new txn, but abort
                let mut server_txn = server.write(duration_from_epoch_now());
                assert!(server_txn.initialise_schema_core(audit).is_ok());
            }
            {
                // Now do it again in a new txn.
                let mut server_txn = server.write(duration_from_epoch_now());
                assert!(server_txn.initialise_schema_core(audit).is_ok());
                assert!(server_txn.commit(audit).is_ok());
            }
        });
    }

    #[test]
    fn test_qs_modify() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            // Create an object
            let mut server_txn = server.write(duration_from_epoch_now());

            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "description": ["testperson1"],
                    "displayname": ["testperson1"]
                }
            }"#,
            );

            let e2: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson2"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63932"],
                    "description": ["testperson2"],
                    "displayname": ["testperson2"]
                }
            }"#,
            );

            let ce = CreateEvent::new_internal(vec![e1.clone(), e2.clone()]);

            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            // Empty Modlist (filter is valid)
            let me_emp = unsafe {
                ModifyEvent::new_internal_invalid(
                    filter!(f_pres("class")),
                    ModifyList::new_list(vec![]),
                )
            };
            assert!(server_txn.modify(audit, &me_emp) == Err(OperationError::EmptyRequest));

            // Mod changes no objects
            let me_nochg = unsafe {
                ModifyEvent::new_impersonate_entry_ser(
                    JSON_ADMIN_V1,
                    filter!(f_eq("name", PartialValue::new_iutf8s("flarbalgarble"))),
                    ModifyList::new_list(vec![Modify::Present(
                        "description".to_string(),
                        Value::from("anusaosu"),
                    )]),
                )
            };
            assert!(server_txn.modify(audit, &me_nochg) == Err(OperationError::NoMatchingEntries));

            // Filter is invalid to schema - to check this due to changes in the way events are
            // handled, we put this via the internal modify function to get the modlist
            // checked for us. Normal server operation doesn't allow weird bypasses like
            // this.
            let r_inv_1 = server_txn.internal_modify(
                audit,
                filter!(f_eq("tnanuanou", PartialValue::new_iutf8s("Flarbalgarble"))),
                ModifyList::new_list(vec![Modify::Present(
                    "description".to_string(),
                    Value::from("anusaosu"),
                )]),
            );
            assert!(
                r_inv_1
                    == Err(OperationError::SchemaViolation(
                        SchemaError::InvalidAttribute
                    ))
            );

            // Mod is invalid to schema
            let me_inv_m = unsafe {
                ModifyEvent::new_internal_invalid(
                    filter!(f_pres("class")),
                    ModifyList::new_list(vec![Modify::Present(
                        "htnaonu".to_string(),
                        Value::from("anusaosu"),
                    )]),
                )
            };
            assert!(
                server_txn.modify(audit, &me_inv_m)
                    == Err(OperationError::SchemaViolation(
                        SchemaError::InvalidAttribute
                    ))
            );

            // Mod single object
            let me_sin = unsafe {
                ModifyEvent::new_internal_invalid(
                    filter!(f_eq("name", PartialValue::new_iutf8s("testperson2"))),
                    ModifyList::new_list(vec![Modify::Present(
                        "description".to_string(),
                        Value::from("anusaosu"),
                    )]),
                )
            };
            assert!(server_txn.modify(audit, &me_sin).is_ok());

            // Mod multiple object
            let me_mult = unsafe {
                ModifyEvent::new_internal_invalid(
                    filter!(f_or!([
                        f_eq("name", PartialValue::new_iutf8s("testperson1")),
                        f_eq("name", PartialValue::new_iutf8s("testperson2")),
                    ])),
                    ModifyList::new_list(vec![Modify::Present(
                        "description".to_string(),
                        Value::from("anusaosu"),
                    )]),
                )
            };
            assert!(server_txn.modify(audit, &me_mult).is_ok());

            assert!(server_txn.commit(audit).is_ok());
        })
    }

    #[test]
    fn test_modify_invalid_class() {
        // Test modifying an entry and adding an extra class, that would cause the entry
        // to no longer conform to schema.
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            let mut server_txn = server.write(duration_from_epoch_now());

            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "description": ["testperson1"],
                    "displayname": ["testperson1"]
                }
            }"#,
            );

            let ce = CreateEvent::new_internal(vec![e1.clone()]);

            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            // Add class but no values
            let me_sin = unsafe {
                ModifyEvent::new_internal_invalid(
                    filter!(f_eq("name", PartialValue::new_iutf8s("testperson1"))),
                    ModifyList::new_list(vec![Modify::Present(
                        "class".to_string(),
                        Value::new_class("system_info"),
                    )]),
                )
            };
            assert!(server_txn.modify(audit, &me_sin).is_err());

            // Add multivalue where not valid
            let me_sin = unsafe {
                ModifyEvent::new_internal_invalid(
                    filter!(f_eq("name", PartialValue::new_iutf8s("testperson1"))),
                    ModifyList::new_list(vec![Modify::Present(
                        "name".to_string(),
                        Value::new_iutf8s("testpersonx"),
                    )]),
                )
            };
            assert!(server_txn.modify(audit, &me_sin).is_err());

            // add class and valid values?
            let me_sin = unsafe {
                ModifyEvent::new_internal_invalid(
                    filter!(f_eq("name", PartialValue::new_iutf8s("testperson1"))),
                    ModifyList::new_list(vec![
                        Modify::Present("class".to_string(), Value::new_class("system_info")),
                        // Modify::Present("domain".to_string(), Value::new_iutf8s("domain.name")),
                        Modify::Present("version".to_string(), Value::new_iutf8s("1")),
                    ]),
                )
            };
            assert!(server_txn.modify(audit, &me_sin).is_ok());

            // Replace a value
            let me_sin = unsafe {
                ModifyEvent::new_internal_invalid(
                    filter!(f_eq("name", PartialValue::new_iutf8s("testperson1"))),
                    ModifyList::new_list(vec![
                        Modify::Purged("name".to_string()),
                        Modify::Present("name".to_string(), Value::new_iutf8s("testpersonx")),
                    ]),
                )
            };
            assert!(server_txn.modify(audit, &me_sin).is_ok());
        })
    }

    #[test]
    fn test_qs_delete() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            // Create
            let mut server_txn = server.write(duration_from_epoch_now());

            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "description": ["testperson"],
                    "displayname": ["testperson1"]
                }
            }"#,
            );

            let e2: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson2"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63932"],
                    "description": ["testperson"],
                    "displayname": ["testperson2"]
                }
            }"#,
            );

            let e3: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson3"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63933"],
                    "description": ["testperson"],
                    "displayname": ["testperson3"]
                }
            }"#,
            );

            let ce = CreateEvent::new_internal(vec![e1.clone(), e2.clone(), e3.clone()]);

            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            // Delete filter is syntax invalid
            let de_inv =
                unsafe { DeleteEvent::new_internal_invalid(filter!(f_pres("nhtoaunaoehtnu"))) };
            assert!(server_txn.delete(audit, &de_inv).is_err());

            // Delete deletes nothing
            let de_empty = unsafe {
                DeleteEvent::new_internal_invalid(filter!(f_eq(
                    "uuid",
                    PartialValue::new_uuids("cc8e95b4-c24f-4d68-ba54-000000000000").unwrap()
                )))
            };
            assert!(server_txn.delete(audit, &de_empty).is_err());

            // Delete matches one
            let de_sin = unsafe {
                DeleteEvent::new_internal_invalid(filter!(f_eq(
                    "name",
                    PartialValue::new_iutf8s("testperson3")
                )))
            };
            assert!(server_txn.delete(audit, &de_sin).is_ok());

            // Delete matches many
            let de_mult = unsafe {
                DeleteEvent::new_internal_invalid(filter!(f_eq(
                    "description",
                    PartialValue::new_utf8s("testperson")
                )))
            };
            assert!(server_txn.delete(audit, &de_mult).is_ok());

            assert!(server_txn.commit(audit).is_ok());
        })
    }

    #[test]
    fn test_qs_tombstone() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            // First we setup some timestamps
            let time_p1 = duration_from_epoch_now();
            let time_p2 = time_p1 + Duration::from_secs(CHANGELOG_MAX_AGE * 2);

            let mut server_txn = server.write(time_p1);
            let admin = server_txn
                .internal_search_uuid(audit, &UUID_ADMIN)
                .expect("failed");

            let filt_i_ts = filter_all!(f_eq("class", PartialValue::new_class("tombstone")));

            // Create fake external requests. Probably from admin later
            // Should we do this with impersonate instead of using the external
            let me_ts = unsafe {
                ModifyEvent::new_impersonate_entry(
                    admin.clone(),
                    filt_i_ts.clone(),
                    ModifyList::new_list(vec![Modify::Present(
                        "class".to_string(),
                        Value::new_class("tombstone"),
                    )]),
                )
            };

            let de_ts =
                unsafe { DeleteEvent::new_impersonate_entry(admin.clone(), filt_i_ts.clone()) };
            let se_ts = unsafe { SearchEvent::new_ext_impersonate_entry(admin, filt_i_ts.clone()) };

            // First, create a tombstone
            let e_ts: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["tombstone", "object"],
                    "uuid": ["9557f49c-97a5-4277-a9a5-097d17eb8317"]
                }
            }"#,
            );

            let ce = CreateEvent::new_internal(vec![e_ts]);
            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            // Can it be seen (external search)
            let r1 = server_txn.search(audit, &se_ts).expect("search failed");
            assert!(r1.is_empty());

            // Can it be deleted (external delete)
            // Should be err-no candidates.
            assert!(server_txn.delete(audit, &de_ts).is_err());

            // Can it be modified? (external modify)
            // Should be err-no candidates
            assert!(server_txn.modify(audit, &me_ts).is_err());

            // Can it be seen (internal search)
            // Internal search should see it.
            let r2 = server_txn
                .internal_search(audit, filt_i_ts.clone())
                .expect("internal search failed");
            assert!(r2.len() == 1);

            // If we purge now, nothing happens, we aren't past the time window.
            assert!(server_txn.purge_tombstones(audit).is_ok());

            let r3 = server_txn
                .internal_search(audit, filt_i_ts.clone())
                .expect("internal search failed");
            assert!(r3.len() == 1);

            // Commit
            assert!(server_txn.commit(audit).is_ok());

            // New txn, push the cid forward.
            let mut server_txn = server.write(time_p2);

            // Now purge
            assert!(server_txn.purge_tombstones(audit).is_ok());

            // Assert it's gone
            // Internal search should not see it.
            let r4 = server_txn
                .internal_search(audit, filt_i_ts)
                .expect("internal search failed");
            assert!(r4.is_empty());

            assert!(server_txn.commit(audit).is_ok());
        })
    }

    #[test]
    fn test_qs_recycle_simple() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            // First we setup some timestamps
            let time_p1 = duration_from_epoch_now();
            let time_p2 = time_p1 + Duration::from_secs(RECYCLEBIN_MAX_AGE * 2);

            let mut server_txn = server.write(time_p1);
            let admin = server_txn
                .internal_search_uuid(audit, &UUID_ADMIN)
                .expect("failed");

            let filt_i_rc = filter_all!(f_eq("class", PartialValue::new_class("recycled")));

            let filt_i_ts = filter_all!(f_eq("class", PartialValue::new_class("tombstone")));

            let filt_i_per = filter_all!(f_eq("class", PartialValue::new_class("person")));

            // Create fake external requests. Probably from admin later
            let me_rc = unsafe {
                ModifyEvent::new_impersonate_entry(
                    admin.clone(),
                    filt_i_rc.clone(),
                    ModifyList::new_list(vec![Modify::Present(
                        "class".to_string(),
                        Value::new_class("recycled"),
                    )]),
                )
            };

            let de_rc =
                unsafe { DeleteEvent::new_impersonate_entry(admin.clone(), filt_i_rc.clone()) };

            let se_rc =
                unsafe { SearchEvent::new_ext_impersonate_entry(admin.clone(), filt_i_rc.clone()) };

            let sre_rc =
                unsafe { SearchEvent::new_rec_impersonate_entry(admin.clone(), filt_i_rc.clone()) };

            let rre_rc = unsafe {
                ReviveRecycledEvent::new_impersonate_entry(
                    admin,
                    filter_all!(f_eq("name", PartialValue::new_iutf8s("testperson1"))),
                )
            };

            // Create some recycled objects
            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "attrs": {
                    "class": ["object", "person", "recycled"],
                    "name": ["testperson1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "description": ["testperson"],
                    "displayname": ["testperson1"]
                }
            }"#,
            );

            let e2: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "attrs": {
                    "class": ["object", "person", "recycled"],
                    "name": ["testperson2"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63932"],
                    "description": ["testperson"],
                    "displayname": ["testperson2"]
                }
            }"#,
            );

            let ce = CreateEvent::new_internal(vec![e1, e2]);
            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            // Can it be seen (external search)
            let r1 = server_txn.search(audit, &se_rc).expect("search failed");
            assert!(r1.is_empty());

            // Can it be deleted (external delete)
            // Should be err-no candidates.
            assert!(server_txn.delete(audit, &de_rc).is_err());

            // Can it be modified? (external modify)
            // Should be err-no candidates
            assert!(server_txn.modify(audit, &me_rc).is_err());

            // Can in be seen by special search? (external recycle search)
            let r2 = server_txn.search(audit, &sre_rc).expect("search failed");
            assert!(r2.len() == 2);

            // Can it be seen (internal search)
            // Internal search should see it.
            let r2 = server_txn
                .internal_search(audit, filt_i_rc.clone())
                .expect("internal search failed");
            assert!(r2.len() == 2);

            // There are now two paths forward
            //  revival or purge!
            assert!(server_txn.revive_recycled(audit, &rre_rc).is_ok());

            // Not enough time has passed, won't have an effect for purge to TS
            assert!(server_txn.purge_recycled(audit).is_ok());
            let r3 = server_txn
                .internal_search(audit, filt_i_rc.clone())
                .expect("internal search failed");
            assert!(r3.len() == 1);

            // Commit
            assert!(server_txn.commit(audit).is_ok());

            // Now, establish enough time for the recycled items to be purged.
            let mut server_txn = server.write(time_p2);

            //  purge to tombstone, now that time has passed.
            assert!(server_txn.purge_recycled(audit).is_ok());

            // Should be no recycled objects.
            let r4 = server_txn
                .internal_search(audit, filt_i_rc.clone())
                .expect("internal search failed");
            assert!(r4.is_empty());

            // There should be one tombstone
            let r5 = server_txn
                .internal_search(audit, filt_i_ts.clone())
                .expect("internal search failed");
            assert!(r5.len() == 1);

            // There should be one entry
            let r6 = server_txn
                .internal_search(audit, filt_i_per.clone())
                .expect("internal search failed");
            assert!(r6.len() == 1);

            assert!(server_txn.commit(audit).is_ok());
        })
    }

    // The delete test above should be unaffected by recycle anyway
    #[test]
    fn test_qs_recycle_advanced() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            // Create items
            let mut server_txn = server.write(duration_from_epoch_now());
            let admin = server_txn
                .internal_search_uuid(audit, &UUID_ADMIN)
                .expect("failed");

            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "description": ["testperson"],
                    "displayname": ["testperson1"]
                }
            }"#,
            );
            let ce = CreateEvent::new_internal(vec![e1]);

            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());
            // Delete and ensure they became recycled.
            let de_sin = unsafe {
                DeleteEvent::new_internal_invalid(filter!(f_eq(
                    "name",
                    PartialValue::new_iutf8s("testperson1")
                )))
            };
            assert!(server_txn.delete(audit, &de_sin).is_ok());
            // Can in be seen by special search? (external recycle search)
            let filt_rc = filter_all!(f_eq("class", PartialValue::new_class("recycled")));
            let sre_rc = unsafe { SearchEvent::new_rec_impersonate_entry(admin, filt_rc.clone()) };
            let r2 = server_txn.search(audit, &sre_rc).expect("search failed");
            assert!(r2.len() == 1);

            // Create dup uuid (rej)
            // After a delete -> recycle, create duplicate name etc.
            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_err());

            assert!(server_txn.commit(audit).is_ok());
        })
    }

    #[test]
    fn test_qs_name_to_uuid() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            let mut server_txn = server.write(duration_from_epoch_now());

            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "description": ["testperson"],
                    "displayname": ["testperson1"]
                }
                }"#,
            );
            let ce = CreateEvent::new_internal(vec![e1]);
            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            // Name doesn't exist
            let r1 = server_txn.name_to_uuid(audit, "testpers");
            assert!(r1.is_err());
            // Name doesn't exist (not syntax normalised)
            let r2 = server_txn.name_to_uuid(audit, "tEsTpErS");
            assert!(r2.is_err());
            // Name does exist
            let r3 = server_txn.name_to_uuid(audit, "testperson1");
            assert!(r3.is_ok());
            // Name is not syntax normalised (but exists)
            let r4 = server_txn.name_to_uuid(audit, "tEsTpErSoN1");
            assert!(r4.is_ok());
        })
    }

    #[test]
    fn test_qs_uuid_to_name() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            let mut server_txn = server.write(duration_from_epoch_now());

            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "description": ["testperson"],
                    "displayname": ["testperson1"]
                }
            }"#,
            );
            let ce = CreateEvent::new_internal(vec![e1]);
            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            // Name doesn't exist
            let r1 = server_txn.uuid_to_name(
                audit,
                &Uuid::parse_str("bae3f507-e6c3-44ba-ad01-f8ff1083534a").unwrap(),
            );
            // There is nothing.
            assert!(r1 == Ok(None));
            // Name does exist
            let r3 = server_txn.uuid_to_name(
                audit,
                &Uuid::parse_str("cc8e95b4-c24f-4d68-ba54-8bed76f63930").unwrap(),
            );
            assert!(r3.is_ok());
            // Name is not syntax normalised (but exists)
            let r4 = server_txn.uuid_to_name(
                audit,
                &Uuid::parse_str("CC8E95B4-C24F-4D68-BA54-8BED76F63930").unwrap(),
            );
            assert!(r4.is_ok());
        })
    }

    #[test]
    fn test_qs_clone_value() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            let mut server_txn = server.write(duration_from_epoch_now());
            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "description": ["testperson"],
                    "displayname": ["testperson1"]
                }
            }"#,
            );
            let ce = CreateEvent::new_internal(vec![e1]);
            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            // test attr not exist
            let r1 =
                server_txn.clone_value(audit, &"tausau".to_string(), &"naoeutnhaou".to_string());

            assert!(r1.is_err());

            // test attr not-normalised
            // test attr not-reference
            let r2 = server_txn.clone_value(audit, &"NaMe".to_string(), &"NaMe".to_string());

            assert!(r2 == Ok(Value::new_iutf8s("NaMe")));

            // test attr reference
            let r3 =
                server_txn.clone_value(audit, &"member".to_string(), &"testperson1".to_string());

            assert!(r3 == Ok(Value::new_refer_s("cc8e95b4-c24f-4d68-ba54-8bed76f63930").unwrap()));

            // test attr reference already resolved.
            let r4 = server_txn.clone_value(
                audit,
                &"member".to_string(),
                &"cc8e95b4-c24f-4d68-ba54-8bed76f63930".to_string(),
            );

            println!("{:?}", r4);
            assert!(r4 == Ok(Value::new_refer_s("cc8e95b4-c24f-4d68-ba54-8bed76f63930").unwrap()));
        })
    }

    #[test]
    fn test_qs_resolve_value() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            let mut server_txn = server.write(duration_from_epoch_now());
            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["object", "person"],
                    "name": ["testperson1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "description": ["testperson"],
                    "displayname": ["testperson1"]
                }
            }"#,
            );
            let e_ts: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["tombstone", "object"],
                    "uuid": ["9557f49c-97a5-4277-a9a5-097d17eb8317"]
                }
            }"#,
            );
            let ce = CreateEvent::new_internal(vec![e1, e_ts]);
            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            // Resolving most times should yield expected results
            let t1 = Value::new_utf8s("teststring");
            let r1 = server_txn.resolve_value(audit, &t1);
            assert!(r1 == Ok("teststring".to_string()));

            // Resolve UUID with matching name
            let t_uuid = Value::new_refer_s("cc8e95b4-c24f-4d68-ba54-8bed76f63930").unwrap();
            let r_uuid = server_txn.resolve_value(audit, &t_uuid);
            debug!("{:?}", r_uuid);
            assert!(r_uuid == Ok("testperson1".to_string()));

            // Resolve UUID non-exist
            let t_uuid_non = Value::new_refer_s("b83e98f0-3d2e-41d2-9796-d8d993289c86").unwrap();
            let r_uuid_non = server_txn.resolve_value(audit, &t_uuid_non);
            debug!("{:?}", r_uuid_non);
            assert!(r_uuid_non == Ok("b83e98f0-3d2e-41d2-9796-d8d993289c86".to_string()));

            // Resolve UUID to tombstone/recycled (same an non-exst)
            let t_uuid_ts = Value::new_refer_s("9557f49c-97a5-4277-a9a5-097d17eb8317").unwrap();
            let r_uuid_ts = server_txn.resolve_value(audit, &t_uuid_ts);
            debug!("{:?}", r_uuid_ts);
            assert!(r_uuid_ts == Ok("9557f49c-97a5-4277-a9a5-097d17eb8317".to_string()));
        })
    }

    #[test]
    fn test_qs_dynamic_schema_class() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["object", "testclass"],
                    "name": ["testobj1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"]
                }
            }"#,
            );

            // Class definition
            let e_cd: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "valid": null,
                "state": null,
                "attrs": {
                    "class": ["object", "classtype"],
                    "classname": ["testclass"],
                    "uuid": ["cfcae205-31c3-484b-8ced-667d1709c5e3"],
                    "description": ["Test Class"],
                    "may": ["name"]
                }
            }"#,
            );

            let mut server_txn = server.write(duration_from_epoch_now());
            // Add a new class.
            let ce_class = CreateEvent::new_internal(vec![e_cd.clone()]);
            assert!(server_txn.create(audit, &ce_class).is_ok());
            // Trying to add it now should fail.
            let ce_fail = CreateEvent::new_internal(vec![e1.clone()]);
            assert!(server_txn.create(audit, &ce_fail).is_err());

            // Commit
            server_txn.commit(audit).expect("should not fail");

            // Start a new write
            let mut server_txn = server.write(duration_from_epoch_now());
            // Add the class to an object
            // should work
            let ce_work = CreateEvent::new_internal(vec![e1.clone()]);
            assert!(server_txn.create(audit, &ce_work).is_ok());

            // Commit
            server_txn.commit(audit).expect("should not fail");

            // Start a new write
            let mut server_txn = server.write(duration_from_epoch_now());
            // delete the class
            let de_class = unsafe {
                DeleteEvent::new_internal_invalid(filter!(f_eq(
                    "classname",
                    PartialValue::new_iutf8s("testclass")
                )))
            };
            assert!(server_txn.delete(audit, &de_class).is_ok());
            // Commit
            server_txn.commit(audit).expect("should not fail");

            // Start a new write
            let mut server_txn = server.write(duration_from_epoch_now());
            // Trying to add now should fail
            let ce_fail = CreateEvent::new_internal(vec![e1.clone()]);
            assert!(server_txn.create(audit, &ce_fail).is_err());
            // Search our entry
            let testobj1 = server_txn
                .internal_search_uuid(
                    audit,
                    &Uuid::parse_str("cc8e95b4-c24f-4d68-ba54-8bed76f63930").unwrap(),
                )
                .expect("failed");
            assert!(testobj1.attribute_value_pres("class", &PartialValue::new_iutf8s("testclass")));

            // Should still be good
            server_txn.commit(audit).expect("should not fail");
            // Commit.
        })
    }

    #[test]
    fn test_qs_dynamic_schema_attr() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "attrs": {
                    "class": ["object", "extensibleobject"],
                    "name": ["testobj1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "testattr": ["test"]
                }
            }"#,
            );

            // Attribute definition
            let e_ad: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "attrs": {
                    "class": ["object", "attributetype"],
                    "attributename": ["testattr"],
                    "uuid": ["cfcae205-31c3-484b-8ced-667d1709c5e3"],
                    "description": ["Test Attribute"],
                    "multivalue": ["false"],
                    "unique": ["false"],
                    "syntax": ["UTF8STRING"]
                }
            }"#,
            );

            let mut server_txn = server.write(duration_from_epoch_now());
            // Add a new attribute.
            let ce_attr = CreateEvent::new_internal(vec![e_ad.clone()]);
            assert!(server_txn.create(audit, &ce_attr).is_ok());
            // Trying to add it now should fail. (use extensible object)
            let ce_fail = CreateEvent::new_internal(vec![e1.clone()]);
            assert!(server_txn.create(audit, &ce_fail).is_err());

            // Commit
            server_txn.commit(audit).expect("should not fail");

            // Start a new write
            let mut server_txn = server.write(duration_from_epoch_now());
            // Add the attr to an object
            // should work
            let ce_work = CreateEvent::new_internal(vec![e1.clone()]);
            assert!(server_txn.create(audit, &ce_work).is_ok());

            // Commit
            server_txn.commit(audit).expect("should not fail");

            // Start a new write
            let mut server_txn = server.write(duration_from_epoch_now());
            // delete the attr
            let de_attr = unsafe {
                DeleteEvent::new_internal_invalid(filter!(f_eq(
                    "attributename",
                    PartialValue::new_iutf8s("testattr")
                )))
            };
            assert!(server_txn.delete(audit, &de_attr).is_ok());
            // Commit
            server_txn.commit(audit).expect("should not fail");

            // Start a new write
            let mut server_txn = server.write(duration_from_epoch_now());
            // Trying to add now should fail
            let ce_fail = CreateEvent::new_internal(vec![e1.clone()]);
            assert!(server_txn.create(audit, &ce_fail).is_err());
            // Search our attribute - should FAIL
            let filt = filter!(f_eq("testattr", PartialValue::new_utf8s("test")));
            assert!(server_txn.internal_search(audit, filt).is_err());
            // Search the entry - the attribute will still be present
            // even if we can't search on it.
            let testobj1 = server_txn
                .internal_search_uuid(
                    audit,
                    &Uuid::parse_str("cc8e95b4-c24f-4d68-ba54-8bed76f63930").unwrap(),
                )
                .expect("failed");
            assert!(testobj1.attribute_value_pres("testattr", &PartialValue::new_utf8s("test")));

            server_txn.commit(audit).expect("should not fail");
            // Commit.
        })
    }

    #[test]
    fn test_qs_modify_password_only() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            let e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
                r#"{
                "attrs": {
                    "class": ["object", "person", "account"],
                    "name": ["testperson1"],
                    "uuid": ["cc8e95b4-c24f-4d68-ba54-8bed76f63930"],
                    "description": ["testperson"],
                    "displayname": ["testperson1"]
                }
            }"#,
            );
            let mut server_txn = server.write(duration_from_epoch_now());
            // Add the entry. Today we have no syntax to take simple str to a credential
            // but honestly, that's probably okay :)
            let ce = CreateEvent::new_internal(vec![e1]);
            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            // Build the credential.
            let cred = Credential::new_password_only("test_password");
            let v_cred = Value::new_credential("primary", cred);
            assert!(v_cred.validate());

            // now modify and provide a primary credential.
            let me_inv_m = unsafe {
                ModifyEvent::new_internal_invalid(
                    filter!(f_eq("name", PartialValue::new_iutf8s("testperson1"))),
                    ModifyList::new_list(vec![Modify::Present(
                        "primary_credential".to_string(),
                        v_cred,
                    )]),
                )
            };
            // go!
            assert!(server_txn.modify(audit, &me_inv_m).is_ok());

            // assert it exists and the password checks out
            let test_ent = server_txn
                .internal_search_uuid(
                    audit,
                    &Uuid::parse_str("cc8e95b4-c24f-4d68-ba54-8bed76f63930").unwrap(),
                )
                .expect("failed");
            // get the primary ava
            let cred_ref = test_ent
                .get_ava_single_credential("primary_credential")
                .expect("Failed");
            // do a pw check.
            assert!(cred_ref.verify_password("test_password"));
        })
    }

    fn create_user(name: &str, uuid: &str) -> Entry<EntryInit, EntryNew> {
        let mut e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
            r#"{
            "attrs": {
                "class": ["object", "person"],
                "description": ["testperson-entry"]
            }
            }"#,
        );
        e1.add_ava("uuid", &Value::new_uuids(uuid).unwrap());
        e1.add_ava("name", &Value::new_iutf8s(name));
        e1.add_ava("displayname", &Value::new_utf8s(name));
        e1
    }

    fn create_group(name: &str, uuid: &str, members: &[&str]) -> Entry<EntryInit, EntryNew> {
        let mut e1: Entry<EntryInit, EntryNew> = Entry::unsafe_from_entry_str(
            r#"{
            "attrs": {
                "class": ["object", "group"],
                "description": ["testgroup-entry"]
            }
            }"#,
        );
        e1.add_ava("name", &Value::new_iutf8s(name));
        e1.add_ava("uuid", &Value::new_uuids(uuid).unwrap());
        members
            .iter()
            .for_each(|m| e1.add_ava("member", &Value::new_refer_s(m).unwrap()));
        e1
    }

    fn check_entry_has_mo(
        qs: &mut QueryServerWriteTransaction,
        audit: &mut AuditScope,
        name: &str,
        mo: &str,
    ) -> bool {
        let e = qs
            .internal_search(audit, filter!(f_eq("name", PartialValue::new_iutf8s(name))))
            .unwrap()
            .pop()
            .unwrap();

        e.attribute_value_pres("memberof", &PartialValue::new_refer_s(mo).unwrap())
    }

    #[test]
    fn test_qs_revive_advanced_directmemberships() {
        run_test!(|server: &QueryServer, audit: &mut AuditScope| {
            // Create items
            let mut server_txn = server.write(duration_from_epoch_now());
            let admin = server_txn
                .internal_search_uuid(audit, &UUID_ADMIN)
                .expect("failed");

            // Right need a user in a direct group.
            let u1 = create_user("u1", "22b47373-d123-421f-859e-9ddd8ab14a2a");
            let g1 = create_group(
                "g1",
                "cca2bbfc-5b43-43f3-be9e-f5b03b3defec",
                &["22b47373-d123-421f-859e-9ddd8ab14a2a"],
            );

            // Need a user in A -> B -> User, such that A/B are re-adde as MO
            let u2 = create_user("u2", "5c19a4a2-b9f0-4429-b130-5782de5fddda");
            let g2a = create_group(
                "g2a",
                "e44cf9cd-9941-44cb-a02f-307b6e15ac54",
                &["5c19a4a2-b9f0-4429-b130-5782de5fddda"],
            );
            let g2b = create_group(
                "g2b",
                "d3132e6e-18ce-4b87-bee1-1d25e4bfe96d",
                &["e44cf9cd-9941-44cb-a02f-307b6e15ac54"],
            );

            // Need a user in a group that is recycled after, then revived at the same time.
            let u3 = create_user("u3", "68467a41-6e8e-44d0-9214-a5164e75ca03");
            let g3 = create_group(
                "g3",
                "36048117-e479-45ed-aeb5-611e8d83d5b1",
                &["68467a41-6e8e-44d0-9214-a5164e75ca03"],
            );

            // A user in a group that is recycled, user is revived, THEN the group is. Group
            // should be present in MO after the second revive.
            let u4 = create_user("u4", "d696b10f-1729-4f1a-83d0-ca06525c2f59");
            let g4 = create_group(
                "g4",
                "d5c59ac6-c533-4b00-989f-d0e183f07bab",
                &["d696b10f-1729-4f1a-83d0-ca06525c2f59"],
            );

            let ce = CreateEvent::new_internal(vec![u1, g1, u2, g2a, g2b, u3, g3, u4, g4]);
            let cr = server_txn.create(audit, &ce);
            assert!(cr.is_ok());

            // Now recycle the needed entries.
            let de = unsafe {
                DeleteEvent::new_internal_invalid(filter!(f_or(vec![
                    f_eq("name", PartialValue::new_iutf8s("u1")),
                    f_eq("name", PartialValue::new_iutf8s("u2")),
                    f_eq("name", PartialValue::new_iutf8s("u3")),
                    f_eq("name", PartialValue::new_iutf8s("g3")),
                    f_eq("name", PartialValue::new_iutf8s("u4")),
                    f_eq("name", PartialValue::new_iutf8s("g4"))
                ])))
            };
            assert!(server_txn.delete(audit, &de).is_ok());

            // Now revive and check each one, one at a time.
            let rev1 = unsafe {
                ReviveRecycledEvent::new_impersonate_entry(
                    admin.clone(),
                    filter_all!(f_eq("name", PartialValue::new_iutf8s("u1"))),
                )
            };
            assert!(server_txn.revive_recycled(audit, &rev1).is_ok());
            // check u1 contains MO ->
            assert!(check_entry_has_mo(
                &mut server_txn,
                audit,
                "u1",
                "cca2bbfc-5b43-43f3-be9e-f5b03b3defec"
            ));

            // Revive u2 and check it has two mo.
            let rev2 = unsafe {
                ReviveRecycledEvent::new_impersonate_entry(
                    admin.clone(),
                    filter_all!(f_eq("name", PartialValue::new_iutf8s("u2"))),
                )
            };
            assert!(server_txn.revive_recycled(audit, &rev2).is_ok());
            assert!(check_entry_has_mo(
                &mut server_txn,
                audit,
                "u2",
                "e44cf9cd-9941-44cb-a02f-307b6e15ac54"
            ));
            assert!(check_entry_has_mo(
                &mut server_txn,
                audit,
                "u2",
                "d3132e6e-18ce-4b87-bee1-1d25e4bfe96d"
            ));

            // Revive u3 and g3 at the same time.
            let rev3 = unsafe {
                ReviveRecycledEvent::new_impersonate_entry(
                    admin.clone(),
                    filter_all!(f_or(vec![
                        f_eq("name", PartialValue::new_iutf8s("u3")),
                        f_eq("name", PartialValue::new_iutf8s("g3"))
                    ])),
                )
            };
            assert!(server_txn.revive_recycled(audit, &rev3).is_ok());
            assert!(
                check_entry_has_mo(
                    &mut server_txn,
                    audit,
                    "u3",
                    "36048117-e479-45ed-aeb5-611e8d83d5b1"
                ) == false
            );

            // Revive u4, should NOT have the MO.
            let rev4a = unsafe {
                ReviveRecycledEvent::new_impersonate_entry(
                    admin.clone(),
                    filter_all!(f_eq("name", PartialValue::new_iutf8s("u4"))),
                )
            };
            assert!(server_txn.revive_recycled(audit, &rev4a).is_ok());
            assert!(
                check_entry_has_mo(
                    &mut server_txn,
                    audit,
                    "u4",
                    "d5c59ac6-c533-4b00-989f-d0e183f07bab"
                ) == false
            );

            // Now revive g4, should allow MO onto u4.
            let rev4b = unsafe {
                ReviveRecycledEvent::new_impersonate_entry(
                    admin,
                    filter_all!(f_eq("name", PartialValue::new_iutf8s("g4"))),
                )
            };
            assert!(server_txn.revive_recycled(audit, &rev4b).is_ok());
            assert!(
                check_entry_has_mo(
                    &mut server_txn,
                    audit,
                    "u4",
                    "d5c59ac6-c533-4b00-989f-d0e183f07bab"
                ) == false
            );

            assert!(server_txn.commit(audit).is_ok());
        })
    }

    /*
    #[test]
    fn test_qs_schema_dump_attrs() {
        run_test!(|server: &QueryServer, _audit: &mut AuditScope| {
            use crate::schema::SchemaTransaction;
            let server_txn = server.write();
            let schema = server_txn.get_schema();

            for k in schema.get_attributes().keys() {
                println!("{}", k);
            }
            println!("====");
            for k in schema.get_classes().keys() {
                println!("{}", k);
            }

        })
    }
    */
}
