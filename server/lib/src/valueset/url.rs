use crate::prelude::*;
use crate::schema::SchemaAttribute;
use crate::valueset::{
    DbValueSetV2, ScimResolveStatus, ValueSet, ValueSetResolveStatus, ValueSetScimPut,
};
use kanidm_proto::scim_v1::JsonValue;
use smolset::SmolSet;

#[derive(Debug, Clone)]
pub struct ValueSetUrl {
    set: SmolSet<[Url; 1]>,
}

impl ValueSetUrl {
    pub fn new(b: Url) -> Box<Self> {
        let mut set = SmolSet::new();
        set.insert(b);
        Box::new(ValueSetUrl { set })
    }

    pub fn push(&mut self, b: Url) -> bool {
        self.set.insert(b)
    }

    pub fn from_dbvs2(data: Vec<Url>) -> Result<ValueSet, OperationError> {
        let set = data.into_iter().collect();
        Ok(Box::new(ValueSetUrl { set }))
    }

    // We need to allow this, because rust doesn't allow us to impl FromIterator on foreign
    // types, and Url is foreign.
    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<T>(iter: T) -> Option<Box<Self>>
    where
        T: IntoIterator<Item = Url>,
    {
        let set = iter.into_iter().collect();
        Some(Box::new(ValueSetUrl { set }))
    }
}

impl ValueSetScimPut for ValueSetUrl {
    fn from_scim_json_put(value: JsonValue) -> Result<ValueSetResolveStatus, OperationError> {
        let value: Url = serde_json::from_value(value).map_err(|err| {
            error!(?err, "SCIM URL syntax invalid");
            OperationError::SC0007UrlSyntaxInvalid
        })?;

        let mut set = SmolSet::new();
        set.insert(value);

        Ok(ValueSetResolveStatus::Resolved(Box::new(ValueSetUrl {
            set,
        })))
    }
}

impl ValueSetT for ValueSetUrl {
    fn insert_checked(&mut self, value: Value) -> Result<bool, OperationError> {
        match value {
            Value::Url(u) => Ok(self.set.insert(u)),
            _ => {
                debug_assert!(false);
                Err(OperationError::InvalidValueState)
            }
        }
    }

    fn clear(&mut self) {
        self.set.clear();
    }

    fn remove(&mut self, pv: &PartialValue, _cid: &Cid) -> bool {
        match pv {
            PartialValue::Url(u) => self.set.remove(u),
            _ => false,
        }
    }

    fn contains(&self, pv: &PartialValue) -> bool {
        match pv {
            PartialValue::Url(u) => self.set.contains(u),
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
        self.set.len()
    }

    fn generate_idx_eq_keys(&self) -> Vec<String> {
        self.set.iter().map(|u| u.to_string()).collect()
    }

    fn syntax(&self) -> SyntaxType {
        SyntaxType::Url
    }

    fn validate(&self, _schema_attr: &SchemaAttribute) -> bool {
        true
    }

    fn to_proto_string_clone_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(self.set.iter().map(|i| i.to_string()))
    }

    fn to_scim_value(&self) -> Option<ScimResolveStatus> {
        let mut iter = self.set.iter().map(|url| url.to_string());
        if self.len() == 1 {
            let v = iter.next().unwrap_or_default();
            Some(v.into())
        } else {
            let arr = iter.collect::<Vec<_>>();
            Some(arr.into())
        }
    }

    fn to_db_valueset_v2(&self) -> DbValueSetV2 {
        DbValueSetV2::Url(self.set.iter().cloned().collect())
    }

    fn to_partialvalue_iter(&self) -> Box<dyn Iterator<Item = PartialValue> + '_> {
        Box::new(self.set.iter().cloned().map(PartialValue::Url))
    }

    fn to_value_iter(&self) -> Box<dyn Iterator<Item = Value> + '_> {
        Box::new(self.set.iter().cloned().map(Value::Url))
    }

    fn equal(&self, other: &ValueSet) -> bool {
        if let Some(other) = other.as_url_set() {
            &self.set == other
        } else {
            debug_assert!(false);
            false
        }
    }

    fn merge(&mut self, other: &ValueSet) -> Result<(), OperationError> {
        if let Some(b) = other.as_url_set() {
            mergesets!(self.set, b)
        } else {
            debug_assert!(false);
            Err(OperationError::InvalidValueState)
        }
    }

    fn to_url_single(&self) -> Option<&Url> {
        if self.set.len() == 1 {
            self.set.iter().take(1).next()
        } else {
            None
        }
    }

    fn as_url_set(&self) -> Option<&SmolSet<[Url; 1]>> {
        Some(&self.set)
    }
}

#[cfg(test)]
mod tests {
    use super::ValueSetUrl;
    use crate::prelude::{Url, ValueSet};

    #[test]
    fn test_scim_url() {
        let u = Url::parse("https://idm.example.com").unwrap();
        let vs: ValueSet = ValueSetUrl::new(u);
        crate::valueset::scim_json_reflexive(&vs, r#""https://idm.example.com/""#);

        // Test that we can parse json values into a valueset.
        crate::valueset::scim_json_put_reflexive::<ValueSetUrl>(&vs, &[])
    }

    #[test]
    fn test_valueset_url_new() {
        let u = Url::parse("https://example.com").unwrap();
        let vs: ValueSet = ValueSetUrl::new(u);
        assert_eq!(vs.len(), 1);
    }

    #[test]
    fn test_valueset_url_insert_checked() {
        let u1 = Url::parse("https://a.com").unwrap();
        let u2 = Url::parse("https://b.com").unwrap();
        let mut vs: ValueSet = ValueSetUrl::new(u1);
        assert!(vs.insert_checked(crate::prelude::Value::Url(u2)).unwrap());
        assert!(!vs
            .insert_checked(crate::prelude::Value::Url(
                Url::parse("https://a.com").unwrap()
            ))
            .unwrap());
        assert_eq!(vs.len(), 2);
    }

    #[test]
    fn test_valueset_url_equal() {
        let u = Url::parse("https://example.com").unwrap();
        let vs1: ValueSet = ValueSetUrl::new(u.clone());
        let vs2: ValueSet = ValueSetUrl::new(u);
        assert!(vs1.equal(&vs2));
    }

    #[test]
    fn test_valueset_url_not_equal() {
        let u1 = Url::parse("https://a.com").unwrap();
        let u2 = Url::parse("https://b.com").unwrap();
        let vs1: ValueSet = ValueSetUrl::new(u1);
        let vs2: ValueSet = ValueSetUrl::new(u2);
        assert!(!vs1.equal(&vs2));
    }

    #[test]
    fn test_valueset_url_merge() {
        let u1 = Url::parse("https://a.com").unwrap();
        let u2 = Url::parse("https://b.com").unwrap();
        let mut vs_a: ValueSet = ValueSetUrl::new(u1);
        let vs_b: ValueSet = ValueSetUrl::new(u2);
        vs_a.merge(&vs_b).expect("Failed to merge");
        assert_eq!(vs_a.len(), 2);
    }

    #[test]
    fn test_valueset_url_clear() {
        let u = Url::parse("https://example.com").unwrap();
        let mut vs: ValueSet = ValueSetUrl::new(u);
        vs.clear();
        assert_eq!(vs.len(), 0);
    }

    #[test]
    fn test_valueset_url_len() {
        let u1 = Url::parse("https://a.com").unwrap();
        let u2 = Url::parse("https://b.com").unwrap();
        let mut vs: ValueSet = ValueSetUrl::new(u1);
        assert_eq!(vs.len(), 1);
        vs.insert_checked(crate::prelude::Value::Url(u2)).unwrap();
        assert_eq!(vs.len(), 2);
    }

    #[test]
    fn test_valueset_url_dbv2_roundtrip() {
        let u = Url::parse("https://idm.example.com").unwrap();
        let vs: ValueSet = ValueSetUrl::new(u);
        let dbvs = vs.to_db_valueset_v2();
        let vs2 = crate::valueset::from_db_valueset_v2(dbvs).expect("Failed to restore");
        assert!(vs.equal(&vs2));
    }

    #[test]
    fn test_valueset_url_to_url_single() {
        let u = Url::parse("https://single.example.com").unwrap();
        let vs: ValueSet = ValueSetUrl::new(u.clone());
        assert_eq!(vs.to_url_single(), Some(&u));
    }

    #[test]
    fn test_valueset_url_to_url_single_none_when_multiple() {
        let u1 = Url::parse("https://a.com").unwrap();
        let u2 = Url::parse("https://b.com").unwrap();
        let mut vs: ValueSet = ValueSetUrl::new(u1);
        vs.insert_checked(crate::prelude::Value::Url(u2)).unwrap();
        assert_eq!(vs.to_url_single(), None);
    }

    #[test]
    fn test_valueset_url_contains() {
        let u = Url::parse("https://example.com").unwrap();
        let vs: ValueSet = ValueSetUrl::new(u.clone());
        assert!(vs.contains(&crate::prelude::PartialValue::Url(u)));
        assert!(!vs.contains(&crate::prelude::PartialValue::Url(
            Url::parse("https://other.com").unwrap()
        )));
    }
}
