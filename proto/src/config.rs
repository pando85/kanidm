use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Display, str::FromStr};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, Eq, PartialEq)]
pub enum ServerRole {
    #[default]
    WriteReplica,
    WriteReplicaNoUI,
    ReadOnlyReplica,
}

impl Display for ServerRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerRole::WriteReplica => f.write_str("write replica"),
            ServerRole::WriteReplicaNoUI => f.write_str("write replica (no ui)"),
            ServerRole::ReadOnlyReplica => f.write_str("read only replica"),
        }
    }
}

impl FromStr for ServerRole {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "write_replica" => Ok(ServerRole::WriteReplica),
            "write_replica_no_ui" => Ok(ServerRole::WriteReplicaNoUI),
            "read_only_replica" => Ok(ServerRole::ReadOnlyReplica),
            _ => Err("Must be one of write_replica, write_replica_no_ui, read_only_replica"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_server_role_serde_roundtrip() {
        let variants = [
            ServerRole::WriteReplica,
            ServerRole::WriteReplicaNoUI,
            ServerRole::ReadOnlyReplica,
        ];
        for role in variants {
            let json = serde_json::to_string(&role).unwrap();
            let back: ServerRole = serde_json::from_str(&json).unwrap();
            assert_eq!(role, back);
        }
    }

    #[test]
    fn test_server_role_deserialize() {
        assert_eq!(
            serde_json::from_str::<ServerRole>("\"WriteReplica\"").unwrap(),
            ServerRole::WriteReplica
        );
        assert_eq!(
            serde_json::from_str::<ServerRole>("\"WriteReplicaNoUI\"").unwrap(),
            ServerRole::WriteReplicaNoUI
        );
        assert_eq!(
            serde_json::from_str::<ServerRole>("\"ReadOnlyReplica\"").unwrap(),
            ServerRole::ReadOnlyReplica
        );
    }

    #[test]
    fn test_server_role_default() {
        assert_eq!(ServerRole::default(), ServerRole::WriteReplica);
    }

    #[test]
    fn test_server_role_display() {
        assert_eq!(ServerRole::WriteReplica.to_string(), "write replica");
        assert_eq!(
            ServerRole::WriteReplicaNoUI.to_string(),
            "write replica (no ui)"
        );
        assert_eq!(ServerRole::ReadOnlyReplica.to_string(), "read only replica");
    }

    #[test]
    fn test_server_role_from_str_valid() {
        assert_eq!(
            ServerRole::from_str("write_replica").unwrap(),
            ServerRole::WriteReplica
        );
        assert_eq!(
            ServerRole::from_str("write_replica_no_ui").unwrap(),
            ServerRole::WriteReplicaNoUI
        );
        assert_eq!(
            ServerRole::from_str("read_only_replica").unwrap(),
            ServerRole::ReadOnlyReplica
        );
    }

    #[test]
    fn test_server_role_from_str_invalid() {
        assert!(ServerRole::from_str("invalid").is_err());
        assert!(ServerRole::from_str("").is_err());
        assert!(ServerRole::from_str("WriteReplica").is_err());
        assert!(ServerRole::from_str("WRITE_REPLICA").is_err());
        assert!(ServerRole::from_str("write-replica").is_err());
        assert!(ServerRole::from_str("read_only").is_err());
    }
}
