use kubidm_proto::constants::DEFAULT_CLIENT_CONFIG_PATH;
pub const DEFAULT_MAIL_SENDER_CONFIG_PATH: &str = "/etc/kubidm/mail-sender";

#[derive(Debug, clap::Parser, Clone)]
#[clap(about = "Kubidm Mail Sender")]
pub struct Opt {
    /// Enable debugging of the sender
    #[clap(short, long, env = "KUBIDM_DEBUG")]
    pub debug: bool,
    /// Path to the client config file.
    #[clap(short, long, value_parser, default_value_os_t = DEFAULT_CLIENT_CONFIG_PATH.into())]
    pub client_config: PathBuf,

    /// Path to the mail-sender config file.
    #[clap(short, long, value_parser, default_value_os_t = DEFAULT_MAIL_SENDER_CONFIG_PATH.into())]
    pub mail_sender_config: PathBuf,

    /// Skip the root user permission check.
    #[clap(short, long, hide = true)]
    pub skip_root_check: bool,

    /// Send a single test email and then exit.
    #[clap(short, long)]
    pub test_email: Option<String>,
}
