use super::dbvalue::DbCidV1;
use crate::prelude::entries::Attribute;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum DbEntryChangeState {
    V1Live {
        at: DbCidV1,
        changes: BTreeMap<Attribute, DbCidV1>,
    },
    V1Tombstone {
        at: DbCidV1,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DbReplMeta {
    V1 { ruv: BTreeSet<DbCidV1> },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use uuid::Uuid;

    fn make_cid(server_id: Uuid, secs: u64) -> DbCidV1 {
        DbCidV1 {
            timestamp: Duration::from_secs(secs),
            server_id,
        }
    }

    #[test]
    fn test_dbentry_changestate_v1live_serde_roundtrip() {
        let sid = Uuid::new_v4();
        let cid = make_cid(sid, 100);
        let mut changes = BTreeMap::new();
        changes.insert(Attribute::UserId, make_cid(sid, 200));
        changes.insert(Attribute::Name, make_cid(sid, 300));

        let state = DbEntryChangeState::V1Live { at: cid, changes };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: DbEntryChangeState = serde_json::from_str(&json).unwrap();

        match deserialized {
            DbEntryChangeState::V1Live { at, changes } => {
                assert_eq!(at.timestamp, Duration::from_secs(100));
                assert_eq!(at.server_id, sid);
                assert_eq!(changes.len(), 2);
            }
            DbEntryChangeState::V1Tombstone { .. } => {
                panic!("Expected V1Live, got V1Tombstone");
            }
        }
    }

    #[test]
    fn test_dbentry_changestate_v1live_empty_changes() {
        let sid = Uuid::new_v4();
        let cid = make_cid(sid, 50);
        let changes = BTreeMap::new();

        let state = DbEntryChangeState::V1Live { at: cid, changes };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: DbEntryChangeState = serde_json::from_str(&json).unwrap();

        match deserialized {
            DbEntryChangeState::V1Live { at, changes } => {
                assert_eq!(at.timestamp, Duration::from_secs(50));
                assert!(changes.is_empty());
            }
            DbEntryChangeState::V1Tombstone { .. } => {
                panic!("Expected V1Live");
            }
        }
    }

    #[test]
    fn test_dbentry_changestate_v1tombstone_serde_roundtrip() {
        let sid = Uuid::new_v4();
        let cid = make_cid(sid, 999);

        let state = DbEntryChangeState::V1Tombstone { at: cid };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: DbEntryChangeState = serde_json::from_str(&json).unwrap();

        match deserialized {
            DbEntryChangeState::V1Tombstone { at } => {
                assert_eq!(at.timestamp, Duration::from_secs(999));
                assert_eq!(at.server_id, sid);
            }
            DbEntryChangeState::V1Live { .. } => {
                panic!("Expected V1Tombstone");
            }
        }
    }

    #[test]
    fn test_dbreplmeta_v1_serde_roundtrip() {
        let sid1 = Uuid::new_v4();
        let sid2 = Uuid::new_v4();
        let mut ruv = BTreeSet::new();
        ruv.insert(make_cid(sid1, 10));
        ruv.insert(make_cid(sid2, 20));
        ruv.insert(make_cid(sid1, 30));

        let meta = DbReplMeta::V1 { ruv };

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: DbReplMeta = serde_json::from_str(&json).unwrap();

        match deserialized {
            DbReplMeta::V1 { ruv } => {
                assert_eq!(ruv.len(), 3);
                assert!(ruv.contains(&make_cid(sid1, 10)));
                assert!(ruv.contains(&make_cid(sid2, 20)));
                assert!(ruv.contains(&make_cid(sid1, 30)));
            }
        }
    }

    #[test]
    fn test_dbreplmeta_v1_empty_ruv() {
        let ruv = BTreeSet::new();
        let meta = DbReplMeta::V1 { ruv };

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: DbReplMeta = serde_json::from_str(&json).unwrap();

        match deserialized {
            DbReplMeta::V1 { ruv } => {
                assert!(ruv.is_empty());
            }
        }
    }

    #[test]
    fn test_dbreplmeta_v1_ruv_ordering() {
        let sid = Uuid::new_v4();
        let mut ruv = BTreeSet::new();
        ruv.insert(make_cid(sid, 300));
        ruv.insert(make_cid(sid, 100));
        ruv.insert(make_cid(sid, 200));

        let meta = DbReplMeta::V1 { ruv };

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: DbReplMeta = serde_json::from_str(&json).unwrap();

        match deserialized {
            DbReplMeta::V1 { ruv } => {
                let timestamps: Vec<u64> = ruv.iter().map(|c| c.timestamp.as_secs()).collect();
                assert_eq!(timestamps, vec![100, 200, 300]);
            }
        }
    }

    #[test]
    fn test_dbentry_changestate_v1live_many_attributes() {
        let sid = Uuid::new_v4();
        let cid = make_cid(sid, 1);
        let mut changes = BTreeMap::new();
        changes.insert(Attribute::Uuid, make_cid(sid, 2));
        changes.insert(Attribute::UserId, make_cid(sid, 3));
        changes.insert(Attribute::Name, make_cid(sid, 4));
        changes.insert(Attribute::ClassName, make_cid(sid, 5));
        changes.insert(Attribute::Description, make_cid(sid, 6));

        let state = DbEntryChangeState::V1Live { at: cid, changes };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: DbEntryChangeState = serde_json::from_str(&json).unwrap();

        match deserialized {
            DbEntryChangeState::V1Live { at, changes } => {
                assert_eq!(at.timestamp, Duration::from_secs(1));
                assert_eq!(changes.len(), 5);
                assert_eq!(
                    changes.get(&Attribute::Uuid).unwrap().timestamp,
                    Duration::from_secs(2)
                );
                assert_eq!(
                    changes.get(&Attribute::Description).unwrap().timestamp,
                    Duration::from_secs(6)
                );
            }
            DbEntryChangeState::V1Tombstone { .. } => {
                panic!("Expected V1Live");
            }
        }
    }
}
