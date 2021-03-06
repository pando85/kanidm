use crate::audit::AuditScope;
use crate::be::idl_sqlite::{
    IdlSqlite, IdlSqliteReadTransaction, IdlSqliteTransaction, IdlSqliteWriteTransaction,
};
use crate::be::{IdRawEntry, IDL};
use crate::entry::{Entry, EntryCommitted, EntrySealed};
use crate::value::IndexType;
use concread::cache::arc::{Arc, ArcReadTxn, ArcWriteTxn};
use idlset::IDLBitRange;
use kanidm_proto::v1::{ConsistencyError, OperationError};
use uuid::Uuid;

// use std::borrow::Borrow;

const DEFAULT_CACHE_TARGET: usize = 1024;
const DEFAULT_IDL_CACHE_RATIO: usize = 16;
const DEFAULT_CACHE_RMISS: usize = 8;
const DEFAULT_CACHE_WMISS: usize = 8;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
struct IdlCacheKey {
    a: String,
    i: IndexType,
    k: String,
}

/*
impl Borrow<(&str, &IndexType, &str)> for IdlCacheKey {
    #[inline]
    fn borrow(&self) -> &(&str, &IndexType, &str) {
        &(self.a.as_str(), &self.i, self.k.as_str())
    }
}

impl From<(&str, &IndexType, &str)> for IdlCacheKey {
    fn from((a, i, k): (&str, &IndexType, &str)) -> IdlCacheKey {
        IdlCacheKey {
            a: a.to_string(), i: (*i).clone(), k: k.to_string()
        }
    }
}
*/

pub struct IdlArcSqlite {
    db: IdlSqlite,
    entry_cache: Arc<u64, Box<Entry<EntrySealed, EntryCommitted>>>,
    idl_cache: Arc<IdlCacheKey, Box<IDLBitRange>>,
}

pub struct IdlArcSqliteReadTransaction<'a> {
    db: IdlSqliteReadTransaction,
    entry_cache: ArcReadTxn<'a, u64, Box<Entry<EntrySealed, EntryCommitted>>>,
    idl_cache: ArcReadTxn<'a, IdlCacheKey, Box<IDLBitRange>>,
}

pub struct IdlArcSqliteWriteTransaction<'a> {
    db: IdlSqliteWriteTransaction,
    entry_cache: ArcWriteTxn<'a, u64, Box<Entry<EntrySealed, EntryCommitted>>>,
    idl_cache: ArcWriteTxn<'a, IdlCacheKey, Box<IDLBitRange>>,
}

macro_rules! get_identry {
    (
        $self:expr,
        $au:expr,
        $idl:expr
    ) => {{
        match $idl {
            IDL::Partial(idli) | IDL::Indexed(idli) => {
                let mut result: Vec<Entry<_, _>> = Vec::new();
                let mut nidl = IDLBitRange::new();

                idli.into_iter().for_each(|i| {
                    // For all the id's in idl.
                    // is it in the cache?
                    match $self.entry_cache.get(&i) {
                        Some(eref) => result.push(eref.as_ref().clone()),
                        None => unsafe { nidl.push_id(i) },
                    }
                });

                // Now, get anything from nidl that is needed.
                let mut db_result = $self.db.get_identry($au, &IDL::Partial(nidl))?;

                // Clone everything from db_result into the cache.
                db_result.iter().for_each(|e| {
                    $self.entry_cache.insert(e.get_id(), Box::new(e.clone()));
                });

                // Merge the two vecs
                result.append(&mut db_result);

                // Return
                Ok(result)
            }
            IDL::ALLIDS => $self.db.get_identry($au, $idl),
        }
    }};
}

macro_rules! get_identry_raw {
    (
        $self:expr,
        $au:expr,
        $idl:expr
    ) => {{
        // As a cache we have no concept of this, so we just bypass to the db.
        $self.db.get_identry_raw($au, $idl)
    }};
}

macro_rules! exists_idx {
    (
        $self:expr,
        $audit:expr,
        $attr:expr,
        $itype:expr
    ) => {{
        // As a cache we have no concept of this, so we just bypass to the db.
        $self.db.exists_idx($audit, $attr, $itype)
    }};
}

macro_rules! get_idl {
    (
        $self:expr,
        $audit:expr,
        $attr:expr,
        $itype:expr,
        $idx_key:expr
    ) => {{
        // TODO: Find a way to implement borrow for this properly
        // First attempt to get from this cache.
        let cache_key = IdlCacheKey {
            a: $attr.to_string(),
            i: $itype.clone(),
            k: $idx_key.to_string(),
        };
        let cache_r = $self.idl_cache.get(&cache_key);
        // If hit, continue.
        if let Some(ref data) = cache_r {
            return Ok(Some(data.as_ref().clone()));
        }
        // If miss, get from db *and* insert to the cache.
        let db_r = $self.db.get_idl($audit, $attr, $itype, $idx_key)?;
        if let Some(ref idl) = db_r {
            $self.idl_cache.insert(cache_key, Box::new(idl.clone()))
        }
        Ok(db_r)
    }};
}

pub trait IdlArcSqliteTransaction {
    fn get_identry(
        &mut self,
        au: &mut AuditScope,
        idl: &IDL,
    ) -> Result<Vec<Entry<EntrySealed, EntryCommitted>>, OperationError>;

    fn get_identry_raw(
        &self,
        au: &mut AuditScope,
        idl: &IDL,
    ) -> Result<Vec<IdRawEntry>, OperationError>;

    fn exists_idx(
        &mut self,
        audit: &mut AuditScope,
        attr: &str,
        itype: &IndexType,
    ) -> Result<bool, OperationError>;

    fn get_idl(
        &mut self,
        audit: &mut AuditScope,
        attr: &str,
        itype: &IndexType,
        idx_key: &str,
    ) -> Result<Option<IDLBitRange>, OperationError>;

    fn get_db_s_uuid(&self) -> Result<Option<Uuid>, OperationError>;

    fn get_db_d_uuid(&self) -> Result<Option<Uuid>, OperationError>;

    fn verify(&self) -> Vec<Result<(), ConsistencyError>>;
}

impl<'a> IdlArcSqliteTransaction for IdlArcSqliteReadTransaction<'a> {
    fn get_identry(
        &mut self,
        au: &mut AuditScope,
        idl: &IDL,
    ) -> Result<Vec<Entry<EntrySealed, EntryCommitted>>, OperationError> {
        get_identry!(self, au, idl)
    }

    fn get_identry_raw(
        &self,
        au: &mut AuditScope,
        idl: &IDL,
    ) -> Result<Vec<IdRawEntry>, OperationError> {
        get_identry_raw!(self, au, idl)
    }

    fn exists_idx(
        &mut self,
        audit: &mut AuditScope,
        attr: &str,
        itype: &IndexType,
    ) -> Result<bool, OperationError> {
        exists_idx!(self, audit, attr, itype)
    }

    fn get_idl(
        &mut self,
        audit: &mut AuditScope,
        attr: &str,
        itype: &IndexType,
        idx_key: &str,
    ) -> Result<Option<IDLBitRange>, OperationError> {
        get_idl!(self, audit, attr, itype, idx_key)
    }

    fn get_db_s_uuid(&self) -> Result<Option<Uuid>, OperationError> {
        self.db.get_db_s_uuid()
    }

    fn get_db_d_uuid(&self) -> Result<Option<Uuid>, OperationError> {
        self.db.get_db_d_uuid()
    }

    fn verify(&self) -> Vec<Result<(), ConsistencyError>> {
        self.db.verify()
    }
}

impl<'a> IdlArcSqliteTransaction for IdlArcSqliteWriteTransaction<'a> {
    fn get_identry(
        &mut self,
        au: &mut AuditScope,
        idl: &IDL,
    ) -> Result<Vec<Entry<EntrySealed, EntryCommitted>>, OperationError> {
        get_identry!(self, au, idl)
    }

    fn get_identry_raw(
        &self,
        au: &mut AuditScope,
        idl: &IDL,
    ) -> Result<Vec<IdRawEntry>, OperationError> {
        get_identry_raw!(self, au, idl)
    }

    fn exists_idx(
        &mut self,
        audit: &mut AuditScope,
        attr: &str,
        itype: &IndexType,
    ) -> Result<bool, OperationError> {
        exists_idx!(self, audit, attr, itype)
    }

    fn get_idl(
        &mut self,
        audit: &mut AuditScope,
        attr: &str,
        itype: &IndexType,
        idx_key: &str,
    ) -> Result<Option<IDLBitRange>, OperationError> {
        get_idl!(self, audit, attr, itype, idx_key)
    }

    fn get_db_s_uuid(&self) -> Result<Option<Uuid>, OperationError> {
        self.db.get_db_s_uuid()
    }

    fn get_db_d_uuid(&self) -> Result<Option<Uuid>, OperationError> {
        self.db.get_db_d_uuid()
    }

    fn verify(&self) -> Vec<Result<(), ConsistencyError>> {
        self.db.verify()
    }
}

impl<'a> IdlArcSqliteWriteTransaction<'a> {
    pub fn commit(self, audit: &mut AuditScope) -> Result<(), OperationError> {
        let IdlArcSqliteWriteTransaction {
            db,
            entry_cache,
            idl_cache,
        } = self;
        // Undo the caches in the reverse order.
        db.commit(audit).and_then(|r| {
            idl_cache.commit();
            entry_cache.commit();
            Ok(r)
        })
    }

    pub fn get_id2entry_max_id(&self) -> Result<u64, OperationError> {
        // TODO: We could cache this too, and have this via the setup call
        // to get the init value, using the ArcCell.
        self.db.get_id2entry_max_id()
    }

    pub fn write_identries<'b, I>(
        &'b mut self,
        au: &mut AuditScope,
        entries: I,
    ) -> Result<(), OperationError>
    where
        I: Iterator<Item = &'b Entry<EntrySealed, EntryCommitted>>,
    {
        // Danger! We know that the entry cache is valid to manipulate here
        // but rust doesn't know that so it prevents the mut/immut borrow.
        let e_cache = unsafe { &mut *(&mut self.entry_cache as *mut ArcWriteTxn<_, _>) };
        let m_entries = entries.map(|e| {
            e_cache.insert(e.get_id(), Box::new(e.clone()));
            e
        });
        self.db.write_identries(au, m_entries)
    }

    pub fn write_identries_raw<I>(
        &mut self,
        au: &mut AuditScope,
        entries: I,
    ) -> Result<(), OperationError>
    where
        I: Iterator<Item = IdRawEntry>,
    {
        // Drop the entry cache.
        self.entry_cache.clear();
        // Write the raw ents
        self.db.write_identries_raw(au, entries)
    }

    pub fn delete_identry<I>(&mut self, au: &mut AuditScope, idl: I) -> Result<(), OperationError>
    where
        I: Iterator<Item = u64>,
    {
        // Danger! We know that the entry cache is valid to manipulate here
        // but rust doesn't know that so it prevents the mut/immut borrow.
        let e_cache = unsafe { &mut *(&mut self.entry_cache as *mut ArcWriteTxn<_, _>) };
        let m_idl = idl.map(|i| {
            e_cache.remove(i);
            i
        });
        self.db.delete_identry(au, m_idl)
    }

    pub fn write_idl(
        &mut self,
        audit: &mut AuditScope,
        attr: &str,
        itype: &IndexType,
        idx_key: &str,
        idl: &IDLBitRange,
    ) -> Result<(), OperationError> {
        let cache_key = IdlCacheKey {
            a: attr.to_string(),
            i: itype.clone(),
            k: idx_key.to_string(),
        };
        // On idl == 0 the db will remove this, and synthesise an empty IDL on a miss
        // but we can cache this as a new empty IDL instead, so that we can avoid the
        // db lookup on this idl.
        if idl.len() == 0 {
            self.idl_cache
                .insert(cache_key, Box::new(IDLBitRange::new()));
        } else {
            self.idl_cache.insert(cache_key, Box::new(idl.clone()));
        }
        self.db.write_idl(audit, attr, itype, idx_key, idl)
    }

    pub fn create_name2uuid(&self, audit: &mut AuditScope) -> Result<(), OperationError> {
        self.db.create_name2uuid(audit)
    }

    pub fn create_uuid2name(&self, audit: &mut AuditScope) -> Result<(), OperationError> {
        self.db.create_uuid2name(audit)
    }

    pub fn create_idx(
        &self,
        audit: &mut AuditScope,
        attr: &str,
        itype: &IndexType,
    ) -> Result<(), OperationError> {
        // We don't need to affect this, so pass it down.
        self.db.create_idx(audit, attr, itype)
    }

    pub fn list_idxs(&self, audit: &mut AuditScope) -> Result<Vec<String>, OperationError> {
        // This is only used in tests, so bypass the cache.
        self.db.list_idxs(audit)
    }

    pub unsafe fn purge_idxs(&mut self, audit: &mut AuditScope) -> Result<(), OperationError> {
        self.db.purge_idxs(audit).and_then(|r| {
            self.idl_cache.clear();
            Ok(r)
        })
    }

    pub unsafe fn purge_id2entry(&mut self, audit: &mut AuditScope) -> Result<(), OperationError> {
        self.db.purge_id2entry(audit).and_then(|r| {
            self.entry_cache.clear();
            Ok(r)
        })
    }

    pub fn write_db_s_uuid(&self, nsid: Uuid) -> Result<(), OperationError> {
        self.db.write_db_s_uuid(nsid)
    }

    pub fn write_db_d_uuid(&self, nsid: Uuid) -> Result<(), OperationError> {
        self.db.write_db_d_uuid(nsid)
    }

    pub(crate) fn get_db_index_version(&self) -> i64 {
        self.db.get_db_index_version()
    }

    pub(crate) fn set_db_index_version(&self, v: i64) -> Result<(), OperationError> {
        self.db.set_db_index_version(v)
    }

    pub fn setup(&self, audit: &mut AuditScope) -> Result<(), OperationError> {
        self.db.setup(audit)
    }
}

impl IdlArcSqlite {
    pub fn new(audit: &mut AuditScope, path: &str, pool_size: u32) -> Result<Self, OperationError> {
        let db = IdlSqlite::new(audit, path, pool_size)?;
        let entry_cache = Arc::new(
            DEFAULT_CACHE_TARGET,
            pool_size as usize,
            DEFAULT_CACHE_RMISS,
            DEFAULT_CACHE_WMISS,
        );
        // The idl cache should have smaller items, and is critical for fast searches
        // so we allow it to have a higher ratio of items relative to the entries.
        let idl_cache = Arc::new(
            DEFAULT_CACHE_TARGET * DEFAULT_IDL_CACHE_RATIO,
            pool_size as usize,
            DEFAULT_CACHE_RMISS,
            DEFAULT_CACHE_WMISS,
        );

        Ok(IdlArcSqlite {
            db,
            entry_cache,
            idl_cache,
        })
    }

    pub fn read(&self) -> IdlArcSqliteReadTransaction {
        // IMPORTANT! Always take entrycache FIRST
        let entry_cache_read = self.entry_cache.read();
        let idl_cache_read = self.idl_cache.read();
        let db_read = self.db.read();
        IdlArcSqliteReadTransaction {
            db: db_read,
            entry_cache: entry_cache_read,
            idl_cache: idl_cache_read,
        }
    }

    pub fn write(&self) -> IdlArcSqliteWriteTransaction {
        // IMPORTANT! Always take entrycache FIRST
        let entry_cache_write = self.entry_cache.write();
        let idl_cache_write = self.idl_cache.write();
        let db_write = self.db.write();
        IdlArcSqliteWriteTransaction {
            db: db_write,
            entry_cache: entry_cache_write,
            idl_cache: idl_cache_write,
        }
    }
}
