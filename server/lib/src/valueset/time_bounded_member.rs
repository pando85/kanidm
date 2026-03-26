use crate::be::dbvalue::DbValueTimeBoundedMemberV1;
use crate::prelude::*;
use crate::schema::SchemaAttribute;
use crate::value::TimeBoundedMember;
use crate::valueset::{DbValueSetV2, ValueSet};
use std::collections::BTreeMap;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct ValueSetTimeBoundedMember {
    map: BTreeMap<Uuid, TimeBoundedMember>,
}

impl ValueSetTimeBoundedMember {
    pub fn new(m: TimeBoundedMember) -> Box<Self> {
        let mut map = BTreeMap::new();
        map.insert(m.uuid, m);
        Box::new(ValueSetTimeBoundedMember { map })
    }

    pub fn push(&mut self, m: TimeBoundedMember) -> bool {
        self.map.insert(m.uuid, m).is_none()
    }

    pub fn from_dbvs2(data: Vec<DbValueTimeBoundedMemberV1>) -> Result<ValueSet, OperationError> {
        let map = data
            .into_iter()
            .filter_map(|dbv| {
                let valid_from = dbv.valid_from.and_then(|vf| {
                    OffsetDateTime::parse(&vf, &Rfc3339)
                        .map(|odt| odt.to_offset(time::UtcOffset::UTC))
                        .ok()
                });

                let valid_until = OffsetDateTime::parse(&dbv.valid_until, &Rfc3339)
                    .map(|odt| odt.to_offset(time::UtcOffset::UTC))
                    .ok()?;

                let member = TimeBoundedMember {
                    uuid: dbv.uuid,
                    valid_from,
                    valid_until,
                };

                Some((dbv.uuid, member))
            })
            .collect();
        Ok(Box::new(ValueSetTimeBoundedMember { map }))
    }

    pub fn from_iter<T>(iter: T) -> Option<Box<Self>>
    where
        T: IntoIterator<Item = TimeBoundedMember>,
    {
        let map: BTreeMap<Uuid, TimeBoundedMember> =
            iter.into_iter().map(|m| (m.uuid, m)).collect();
        if map.is_empty() {
            None
        } else {
            Some(Box::new(ValueSetTimeBoundedMember { map }))
        }
    }

    pub fn get_valid_members_at(&self, when: OffsetDateTime) -> BTreeSet<Uuid> {
        self.map
            .values()
            .filter(|m| m.is_valid_at(when))
            .map(|m| m.uuid)
            .collect()
    }
}

impl ValueSetT for ValueSetTimeBoundedMember {
    fn insert_checked(&mut self, value: Value) -> Result<bool, OperationError> {
        match value {
            Value::TimeBoundedMember(m) => {
                let r = self.map.insert(m.uuid, m.clone()).is_none();
                Ok(r)
            }
            _ => {
                debug_assert!(false);
                Err(OperationError::InvalidValueState)
            }
        }
    }

    fn clear(&mut self) {
        self.map.clear();
    }

    fn remove(&mut self, pv: &PartialValue, _cid: &Cid) -> bool {
        match pv {
            PartialValue::TimeBoundedMember(uuid) => self.map.remove(uuid).is_some(),
            _ => {
                debug_assert!(false);
                false
            }
        }
    }

    fn contains(&self, pv: &PartialValue) -> bool {
        match pv {
            PartialValue::TimeBoundedMember(uuid) => self.map.contains_key(uuid),
            _ => false,
        }
    }

    fn substring(&self, _pv: &PartialValue) -> bool {
        false
    }

    fn startswith(&self, _pv: &PartialValue) -> bool {
        false
    }

    fn endswith(&self, _pv: &PartialValue) -> bool {
        false
    }

    fn lessthan(&self, _pv: &PartialValue) -> bool {
        false
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn generate_idx_eq_keys(&self) -> Vec<String> {
        self.map
            .keys()
            .map(|u| u.as_hyphenated().to_string())
            .collect()
    }

    fn syntax(&self) -> SyntaxType {
        SyntaxType::TimeBoundedMember
    }

    fn validate(&self, _schema_attr: &SchemaAttribute) -> bool {
        true
    }

    fn to_proto_string_clone_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(self.map.values().map(|m| {
            let from_str = m
                .valid_from
                .map(|vf| {
                    vf.format(&Rfc3339)
                        .unwrap_or_else(|_| "invalid".to_string())
                })
                .unwrap_or_else(|| "now".to_string());
            let until_str = m
                .valid_until
                .format(&Rfc3339)
                .unwrap_or_else(|_| "invalid".to_string());
            format!("{}: {} - {}", m.uuid, from_str, until_str)
        }))
    }

    fn to_db_valueset_v2(&self) -> DbValueSetV2 {
        DbValueSetV2::TimeBoundedMember(
            self.map
                .values()
                .map(|m| DbValueTimeBoundedMemberV1 {
                    uuid: m.uuid,
                    valid_from: m.valid_from.and_then(|vf| vf.format(&Rfc3339).ok()),
                    valid_until: m
                        .valid_until
                        .format(&Rfc3339)
                        .unwrap_or_else(|_| "invalid".to_string()),
                })
                .collect(),
        )
    }

    fn to_partialvalue_iter(&self) -> Box<dyn Iterator<Item = PartialValue> + '_> {
        Box::new(
            self.map
                .keys()
                .copied()
                .map(PartialValue::TimeBoundedMember),
        )
    }

    fn to_value_iter(&self) -> Box<dyn Iterator<Item = Value> + '_> {
        Box::new(self.map.values().cloned().map(Value::TimeBoundedMember))
    }

    fn equal(&self, other: &ValueSet) -> bool {
        if let Some(other) = other.as_time_bounded_member_set() {
            &self.map == other
        } else {
            debug_assert!(false);
            false
        }
    }

    fn merge(&mut self, other: &ValueSet) -> Result<(), OperationError> {
        if let Some(other) = other.as_time_bounded_member_set() {
            for (k, v) in other.iter() {
                self.map.insert(*k, v.clone());
            }
            Ok(())
        } else {
            debug_assert!(false);
            Err(OperationError::InvalidValueState)
        }
    }

    fn as_time_bounded_member_set(&self) -> Option<&BTreeMap<Uuid, TimeBoundedMember>> {
        Some(&self.map)
    }

    fn as_ref_uuid_iter(&self) -> Option<Box<dyn Iterator<Item = Uuid> + '_>> {
        Some(Box::new(self.map.keys().copied()))
    }
}
