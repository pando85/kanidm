use crate::internal::FsType;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
pub struct KubidmdCli {
    #[clap(
        env = "KANIDM_LOG_LEVEL",
        global = true,
        help = "Specify the log level (info, debug, trace)"
    )]
    pub log_level: Option<sketching::LogLevel>,

    #[clap(
        env = "KANIDM_OTEL_GRPC_ENDPOINT",
        global = true,
        help = "Specify the OpenTelemetry gRPC endpoint (ip/hostname:port)"
    )]
    pub otel_grpc_endpoint: Option<String>,

    #[clap(env = "KANIDM_DOMAIN", global = true, help = "Specify the domain")]
    pub domain: Option<String>,

    #[clap(env = "KANIDM_ORIGIN", global = true, help = "Specify the origin URL")]
    pub origin: Option<url::Url>,

    #[clap(env = "KANIDM_ROLE", global = true, help = "Specify the server role")]
    pub role: Option<crate::config::ServerRole>,

    #[clap(
        env = "KANIDM_DB_PATH",
        global = true,
        help = "Specify the database path"
    )]
    pub db_path: Option<std::path::PathBuf>,

    #[clap(
        env = "KANIDM_DB_FS_TYPE",
        global = true,
        help = "Specify the database filesystem type, either zfs or generic"
    )]
    pub db_fs_type: Option<FsType>,

    #[clap(
        env = "KANIDM_DB_ARC_SIZE",
        global = true,
        help = "Specify the database ARC size in bytes"
    )]
    pub db_arc_size: Option<usize>,

    #[clap(
        env = "KANIDM_ADMIN_BIND_PATH",
        global = true,
        help = "Specify the admin bind path"
    )]
    pub admin_bind_path: Option<String>,

    // TLS
    #[clap(
        env = "KANIDM_TLS_CHAIN",
        global = true,
        help = "Specify the TLS chain file path"
    )]
    pub tls_chain: Option<std::path::PathBuf>,
    #[clap(
        env = "KANIDM_TLS_KEY",
        global = true,
        help = "Specify the TLS key file path"
    )]
    pub tls_key: Option<std::path::PathBuf>,

    #[clap(
        env = "KANIDM_TLS_CLIENT_CA",
        global = true,
        help = "Specify the TLS client CA file path"
    )]
    pub tls_client_ca: Option<std::path::PathBuf>,

    // networking
    #[clap(
        env = "KANIDM_BINDADDRESS",
        global = true,
        help = "Specify the HTTPS server bind address(es)"
    )]
    pub bindaddress: Option<String>,

    #[clap(
        env = "KANIDM_LDAPBINDADDRESS",
        global = true,
        help = "Specify the LDAP bind address(es)"
    )]
    pub ldapbindaddress: Option<String>,

    #[clap(
        global = true,
        hide = true,
        env = "KANIDM_TRUST_X_FORWARDED_FOR",
        help = "Whether to blindly trust the X-Forwarded-For header, regardless of source IP"
    )]
    pub trust_all_x_forwarded_for: Option<bool>,

    // replication
    #[clap(
        env = "KANIDM_REPLICATION_ORIGIN",
        global = true,
        help = "Specify the replication origin URL"
    )]
    pub replication_origin: Option<url::Url>,

    #[clap(
        env = "KANIDM_REPLICATION_BINDADDRESS",
        global = true,
        help = "Specify the replication bind address"
    )]
    pub replication_bindaddress: Option<std::net::SocketAddr>,

    #[clap(
        env = "KANIDM_REPLICATION_TASK_POLL_INTERVAL",
        global = true,
        help = "Specify the replication task poll interval in seconds"
    )]
    pub replication_task_poll_interval: Option<u64>,

    // backup things
    #[clap(
        env = "KANIDM_ONLINE_BACKUP_PATH",
        global = true,
        help = "Specify the online backup path"
    )]
    pub online_backup_path: Option<std::path::PathBuf>,

    #[clap(
        global = true,
        env = "KANIDM_ONLINE_BACKUP_VERSIONS",
        help = "Number of online backup versions to keep"
    )]
    pub online_backup_versions: Option<usize>,

    #[clap(
        global = true,
        env = "KANIDM_ONLINE_BACKUP_SCHEDULE",
        help = "Cron schedule for online backups",
        last = true
    )]
    pub online_backup_schedule: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerRole;
    use std::str::FromStr;

    fn empty_cli() -> KubidmdCli {
        KubidmdCli {
            log_level: None,
            otel_grpc_endpoint: None,
            domain: None,
            origin: None,
            role: None,
            db_path: None,
            db_fs_type: None,
            db_arc_size: None,
            admin_bind_path: None,
            tls_chain: None,
            tls_key: None,
            tls_client_ca: None,
            bindaddress: None,
            ldapbindaddress: None,
            trust_all_x_forwarded_for: None,
            replication_origin: None,
            replication_bindaddress: None,
            replication_task_poll_interval: None,
            online_backup_path: None,
            online_backup_versions: None,
            online_backup_schedule: None,
        }
    }

    #[test]
    fn test_kubidmd_cli_default_fields() {
        let cli = empty_cli();
        assert!(cli.log_level.is_none());
        assert!(cli.otel_grpc_endpoint.is_none());
        assert!(cli.domain.is_none());
        assert!(cli.origin.is_none());
        assert!(cli.role.is_none());
        assert!(cli.db_path.is_none());
        assert!(cli.db_fs_type.is_none());
        assert!(cli.db_arc_size.is_none());
        assert!(cli.admin_bind_path.is_none());
        assert!(cli.tls_chain.is_none());
        assert!(cli.tls_key.is_none());
        assert!(cli.tls_client_ca.is_none());
        assert!(cli.bindaddress.is_none());
        assert!(cli.ldapbindaddress.is_none());
        assert!(cli.trust_all_x_forwarded_for.is_none());
        assert!(cli.replication_origin.is_none());
        assert!(cli.replication_bindaddress.is_none());
        assert!(cli.replication_task_poll_interval.is_none());
        assert!(cli.online_backup_path.is_none());
        assert!(cli.online_backup_versions.is_none());
        assert!(cli.online_backup_schedule.is_none());
    }

    #[test]
    fn test_kubidmd_cli_debug() {
        let cli = empty_cli();
        let debug = format!("{:?}", cli);
        assert!(debug.contains("KubidmdCli"));
        assert!(debug.contains("log_level"));
        assert!(debug.contains("domain"));
        assert!(debug.contains("role"));
        assert!(debug.contains("bindaddress"));
    }

    #[test]
    fn test_kubidmd_cli_clone() {
        let cli = empty_cli();
        let cloned = cli.clone();
        assert!(cloned.log_level.is_none());
        assert!(cloned.domain.is_none());
        assert!(cloned.role.is_none());
    }

    #[test]
    fn test_kubidmd_cli_with_fields() {
        let cli = KubidmdCli {
            domain: Some("example.com".to_string()),
            role: Some(ServerRole::WriteReplicaNoUI),
            bindaddress: Some("127.0.0.1:8443".to_string()),
            db_arc_size: Some(2048),
            trust_all_x_forwarded_for: Some(true),
            ..empty_cli()
        };
        assert_eq!(cli.domain.as_deref(), Some("example.com"));
        assert_eq!(cli.role, Some(ServerRole::WriteReplicaNoUI));
        assert_eq!(cli.bindaddress.as_deref(), Some("127.0.0.1:8443"));
        assert_eq!(cli.db_arc_size, Some(2048));
        assert_eq!(cli.trust_all_x_forwarded_for, Some(true));
    }

    #[test]
    fn test_kubidmd_cli_field_types_serde() {
        let role: ServerRole = serde_json::from_str("\"WriteReplicaNoUI\"").unwrap();
        assert_eq!(role, ServerRole::WriteReplicaNoUI);
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, "\"WriteReplicaNoUI\"");

        let fs_type: FsType = serde_json::from_str("\"zfs\"").unwrap();
        assert_eq!(fs_type, FsType::Zfs);

        let fs_type: FsType = serde_json::from_str("\"generic\"").unwrap();
        assert_eq!(fs_type, FsType::Generic);
    }

    #[test]
    fn test_kubidmd_cli_role_field_from_str() {
        assert_eq!(
            ServerRole::from_str("write_replica").unwrap(),
            ServerRole::WriteReplica
        );
        assert_eq!(
            ServerRole::from_str("read_only_replica").unwrap(),
            ServerRole::ReadOnlyReplica
        );
    }
}
