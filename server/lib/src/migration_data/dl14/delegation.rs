use crate::entry::EntryInitNew;
use crate::prelude::*;

#[derive(Clone, Debug, Default)]
pub struct BuiltinDelegatedRole {
    pub name: &'static str,
    pub description: &'static str,
    pub uuid: uuid::Uuid,
    pub entry_managed_by: Option<uuid::Uuid>,
}

impl From<BuiltinDelegatedRole> for EntryInitNew {
    fn from(val: BuiltinDelegatedRole) -> Self {
        let mut entry = EntryInitNew::new();

        entry.add_ava(Attribute::Name, Value::new_iname(val.name));
        entry.add_ava(Attribute::Description, Value::new_utf8s(val.description));
        entry.set_ava(
            Attribute::Class,
            vec![EntryClass::Object.into(), EntryClass::DelegatedRole.into()],
        );

        if let Some(entry_manager) = val.entry_managed_by {
            entry.add_ava(Attribute::EntryManagedBy, Value::Refer(entry_manager));
        }

        entry.add_ava(Attribute::Uuid, Value::Uuid(val.uuid));
        entry
    }
}

pub static BUILTIN_DELEGATED_ROLE_HELPDESK: LazyLock<BuiltinDelegatedRole> =
    LazyLock::new(|| BuiltinDelegatedRole {
        name: "idm_delegated_helpdesk",
        description: "Builtin Delegated Helpdesk Role - allows password resets for users in scope",
        uuid: UUID_IDM_DELEGATED_HELPDESK,
        entry_managed_by: Some(UUID_IDM_ADMINS),
    });

pub static BUILTIN_DELEGATED_ROLE_USER_ADMIN: LazyLock<BuiltinDelegatedRole> =
    LazyLock::new(|| BuiltinDelegatedRole {
        name: "idm_delegated_user_admin",
        description: "Builtin Delegated User Admin Role - allows managing users in scope",
        uuid: UUID_IDM_DELEGATED_USER_ADMIN,
        entry_managed_by: Some(UUID_IDM_ADMINS),
    });

pub static BUILTIN_DELEGATED_ROLE_GROUP_ADMIN: LazyLock<BuiltinDelegatedRole> =
    LazyLock::new(|| BuiltinDelegatedRole {
        name: "idm_delegated_group_admin",
        description: "Builtin Delegated Group Admin Role - allows managing groups in scope",
        uuid: UUID_IDM_DELEGATED_GROUP_ADMIN,
        entry_managed_by: Some(UUID_IDM_ADMINS),
    });
