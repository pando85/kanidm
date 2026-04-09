use kubidm_proto::constants::DEFAULT_CLIENT_CONFIG_PATH;
pub const DEFAULT_IPA_CONFIG_PATH: &str = "/etc/kubidm/ipa-sync";

#[derive(Debug, clap::Parser)]
#[clap(about = "Kubidm FreeIPA Sync Driver")]
pub struct Opt {
    /// Enable debugging of the sync driver
    #[clap(short, long, env = "KANIDM_DEBUG")]
    pub debug: bool,
    /// Path to the client config file.
    #[clap(value_parser, short, long, default_value_os_t = DEFAULT_CLIENT_CONFIG_PATH.into())]
    pub client_config: PathBuf,

    /// Path to the ipa-sync config file.
    #[clap(value_parser, short, long, env = "KANIDM_IPA_SYNC_CONFIG", default_value_os_t = DEFAULT_IPA_CONFIG_PATH.into())]
    pub ipa_sync_config: PathBuf,

    /// Dump the ldap protocol inputs, as well as the scim outputs. This can be used
    /// to create test cases for testing the parser.
    ///
    /// No actions are taken on the kubidm instance, this is purely a dump of the
    /// state in/out.
    #[clap(short, long, hide = true)]
    pub proto_dump: bool,

    /// Read entries from ipa, and check the connection to kubidm, but take no actions against
    /// kubidm that would change state.
    #[clap(short = 'n')]
    pub dry_run: bool,

    /// Run in scheduled mode, where the sync tool will periodically attempt to sync between
    /// FreeIPA and Kubidm.
    #[clap(long = "schedule")]
    pub schedule: bool,

    /// Skip the root user permission check.
    #[clap(short, long, hide = true)]
    pub skip_root_check: bool,
}
