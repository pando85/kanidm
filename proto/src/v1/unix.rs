use serde::{Deserialize, Serialize};
use sshkey_attest::proto::PublicKey as SshPublicKey;
use sshkeys::{KeyType, KeyTypeKind, PublicKeyKind};
use std::fmt::{self, Display};
use utoipa::ToSchema;
use uuid::Uuid;

use serde_with::skip_serializing_none;

use crate::constants::{ATTR_GROUP, ATTR_LDAP_SSHPUBLICKEY};

#[allow(dead_code)]
#[derive(ToSchema)]
#[schema(as = KeyTypeKind, value_type = String)]
pub struct KeyTypeKindSchema(KeyTypeKind);

#[derive(ToSchema)]
#[schema(as = KeyType)]
pub struct KeyTypeSchema {
    pub name: &'static str,
    pub short_name: &'static str,
    pub is_cert: bool,
    pub is_sk: bool,
    #[schema(value_type = String)]
    pub kind: KeyTypeKind,
    pub plain: &'static str,
}

#[allow(dead_code)]
#[derive(ToSchema)]
#[schema(as = PublicKeyKind, value_type = String)]
pub struct PublicKeyKindSchema(PublicKeyKind);

#[derive(ToSchema)]
#[schema(as = SshPublicKey)]
pub struct SshPublicKeySchema {
    #[schema(value_type = String)]
    pub key_type: KeyType,
    #[schema(value_type = String)]
    pub kind: PublicKeyKind,
    pub comment: Option<String>,
}

/// A token representing the details of a unix group
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UnixGroupToken {
    pub name: String,
    pub spn: String,
    pub uuid: Uuid,
    pub gidnumber: u32,
}

impl Display for UnixGroupToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[ spn: {}, gidnumber: {}, name: {}, uuid: {} ]",
            self.spn, self.gidnumber, self.name, self.uuid
        )
    }
}

/// Request addition of unix attributes to a group.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct GroupUnixExtend {
    pub gidnumber: Option<u32>,
}

/// A token representing the details of a unix user
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UnixUserToken {
    pub name: String,
    pub spn: String,
    pub displayname: String,
    pub gidnumber: u32,
    pub uuid: Uuid,
    pub shell: Option<String>,
    pub groups: Vec<UnixGroupToken>,
    #[schema(value_type = Vec<String>)]
    pub sshkeys: Vec<SshPublicKey>,
    // The default value of bool is false.
    #[serde(default)]
    pub valid: bool,
}

impl Display for UnixUserToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "---")?;
        writeln!(f, "spn: {}", self.spn)?;
        writeln!(f, "name: {}", self.name)?;
        writeln!(f, "displayname: {}", self.displayname)?;
        writeln!(f, "uuid: {}", self.uuid)?;
        writeln!(f, "gidnumber: {}", self.gidnumber)?;
        match &self.shell {
            Some(s) => writeln!(f, "shell: {s}")?,
            None => writeln!(f, "shell: <none>")?,
        }
        self.sshkeys
            .iter()
            .try_for_each(|s| writeln!(f, "{ATTR_LDAP_SSHPUBLICKEY}: {s}"))?;
        self.groups
            .iter()
            .try_for_each(|g| writeln!(f, "{ATTR_GROUP}: {g}"))
    }
}

/// Request addition of unix attributes to an account
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AccountUnixExtend {
    pub gidnumber: Option<u32>,
    // TODO: rename shell to loginshell everywhere we can find
    /// The internal attribute is "loginshell" but we use shell in the API currently
    #[serde(alias = "loginshell")]
    pub shell: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_group_token_display() {
        let token = UnixGroupToken {
            name: "test_group".to_string(),
            spn: "test_group@example.com".to_string(),
            uuid: Uuid::nil(),
            gidnumber: 1000,
        };
        let display = format!("{}", token);
        assert!(display.contains("test_group"));
        assert!(display.contains("1000"));
        assert!(display.contains("spn:"));
        assert!(display.contains("gidnumber:"));
    }

    #[test]
    fn test_unix_group_token_serialization() {
        let token = UnixGroupToken {
            name: "test_group".to_string(),
            spn: "test_group@example.com".to_string(),
            uuid: Uuid::nil(),
            gidnumber: 1000,
        };
        let json = serde_json::to_string(&token).expect("Failed to serialize");
        let deserialized: UnixGroupToken =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.name, "test_group");
        assert_eq!(deserialized.gidnumber, 1000);
    }

    #[test]
    fn test_group_unix_extend_serialization() {
        let extend = GroupUnixExtend {
            gidnumber: Some(2000),
        };
        let json = serde_json::to_string(&extend).expect("Failed to serialize");
        let deserialized: GroupUnixExtend =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.gidnumber, Some(2000));
    }

    #[test]
    fn test_group_unix_extend_none() {
        let extend = GroupUnixExtend { gidnumber: None };
        let json = serde_json::to_string(&extend).expect("Failed to serialize");
        let deserialized: GroupUnixExtend =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.gidnumber, None);
    }

    #[test]
    fn test_unix_user_token_serialization() {
        let token = UnixUserToken {
            name: "testuser".to_string(),
            spn: "testuser@example.com".to_string(),
            displayname: "Test User".to_string(),
            gidnumber: 1000,
            uuid: Uuid::nil(),
            shell: Some("/bin/bash".to_string()),
            groups: vec![],
            sshkeys: vec![],
            valid: true,
        };
        let json = serde_json::to_string(&token).expect("Failed to serialize");
        let deserialized: UnixUserToken =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.name, "testuser");
        assert_eq!(deserialized.shell, Some("/bin/bash".to_string()));
        assert!(deserialized.valid);
    }

    #[test]
    fn test_unix_user_token_display() {
        let token = UnixUserToken {
            name: "testuser".to_string(),
            spn: "testuser@example.com".to_string(),
            displayname: "Test User".to_string(),
            gidnumber: 1000,
            uuid: Uuid::nil(),
            shell: None,
            groups: vec![],
            sshkeys: vec![],
            valid: false,
        };
        let display = format!("{}", token);
        assert!(display.contains("testuser"));
        assert!(display.contains("shell: <none>"));
    }

    #[test]
    fn test_unix_user_token_default_valid() {
        // Test that valid defaults to false when deserializing
        let json = r#"{"name":"u","spn":"u@e","displayname":"U","gidnumber":1,"uuid":"00000000-0000-0000-0000-000000000000","groups":[],"sshkeys":[]}"#;
        let token: UnixUserToken = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(!token.valid);
    }

    #[test]
    fn test_account_unix_extend_serialization() {
        let extend = AccountUnixExtend {
            gidnumber: Some(3000),
            shell: Some("/bin/zsh".to_string()),
        };
        let json = serde_json::to_string(&extend).expect("Failed to serialize");
        let deserialized: AccountUnixExtend =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.gidnumber, Some(3000));
        assert_eq!(deserialized.shell, Some("/bin/zsh".to_string()));
    }

    #[test]
    fn test_account_unix_extend_loginshell_alias() {
        // Test that "loginshell" is accepted as an alias for "shell"
        let json = r#"{"gidnumber":3000,"loginshell":"/bin/bash"}"#;
        let extend: AccountUnixExtend =
            serde_json::from_str(json).expect("Failed to deserialize with loginshell alias");
        assert_eq!(extend.shell, Some("/bin/bash".to_string()));
    }

    #[test]
    fn test_account_unix_extend_deny_unknown_fields() {
        // Test that unknown fields are rejected
        let json = r#"{"gidnumber":3000,"unknown_field":"value"}"#;
        let result: Result<AccountUnixExtend, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
