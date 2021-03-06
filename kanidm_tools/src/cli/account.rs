use crate::common::CommonOpt;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct AccountCommonOpt {
    #[structopt()]
    account_id: String,
}

#[derive(Debug, StructOpt)]
pub struct AccountCredentialSet {
    #[structopt(flatten)]
    aopts: AccountCommonOpt,
    #[structopt()]
    application_id: Option<String>,
    #[structopt(flatten)]
    copt: CommonOpt,
}

#[derive(Debug, StructOpt)]
pub struct AccountNamedOpt {
    #[structopt(flatten)]
    aopts: AccountCommonOpt,
    #[structopt(flatten)]
    copt: CommonOpt,
}

#[derive(Debug, StructOpt)]
pub struct AccountNamedTagOpt {
    #[structopt(flatten)]
    aopts: AccountCommonOpt,
    #[structopt(flatten)]
    copt: CommonOpt,
    #[structopt(name = "tag")]
    tag: String,
}

#[derive(Debug, StructOpt)]
pub struct AccountNamedTagPKOpt {
    #[structopt(flatten)]
    aopts: AccountCommonOpt,
    #[structopt(flatten)]
    copt: CommonOpt,
    #[structopt(name = "tag")]
    tag: String,
    #[structopt(name = "pubkey")]
    pubkey: String,
}

#[derive(Debug, StructOpt)]
pub struct AccountCreateOpt {
    #[structopt(flatten)]
    aopts: AccountCommonOpt,
    #[structopt(name = "display_name")]
    display_name: String,
    #[structopt(flatten)]
    copt: CommonOpt,
}

#[derive(Debug, StructOpt)]
pub enum AccountCredential {
    #[structopt(name = "set_password")]
    SetPassword(AccountCredentialSet),
    #[structopt(name = "generate_password")]
    GeneratePassword(AccountCredentialSet),
}

#[derive(Debug, StructOpt)]
pub enum AccountRadius {
    #[structopt(name = "show_secret")]
    Show(AccountNamedOpt),
    #[structopt(name = "generate_secret")]
    Generate(AccountNamedOpt),
    #[structopt(name = "delete_secret")]
    Delete(AccountNamedOpt),
}

#[derive(Debug, StructOpt)]
pub struct AccountPosixOpt {
    #[structopt(flatten)]
    aopts: AccountCommonOpt,
    #[structopt(long = "gidnumber")]
    gidnumber: Option<u32>,
    #[structopt(long = "shell")]
    shell: Option<String>,
    #[structopt(flatten)]
    copt: CommonOpt,
}

#[derive(Debug, StructOpt)]
pub enum AccountPosix {
    #[structopt(name = "show")]
    Show(AccountNamedOpt),
    #[structopt(name = "set")]
    Set(AccountPosixOpt),
    #[structopt(name = "set_password")]
    SetPassword(AccountNamedOpt),
}

#[derive(Debug, StructOpt)]
pub enum AccountSsh {
    #[structopt(name = "list_publickeys")]
    List(AccountNamedOpt),
    #[structopt(name = "add_publickey")]
    Add(AccountNamedTagPKOpt),
    #[structopt(name = "delete_publickey")]
    Delete(AccountNamedTagOpt),
}

#[derive(Debug, StructOpt)]
pub enum AccountOpt {
    #[structopt(name = "credential")]
    Credential(AccountCredential),
    #[structopt(name = "radius")]
    Radius(AccountRadius),
    #[structopt(name = "posix")]
    Posix(AccountPosix),
    #[structopt(name = "ssh")]
    Ssh(AccountSsh),
    #[structopt(name = "list")]
    List(CommonOpt),
    #[structopt(name = "get")]
    Get(AccountNamedOpt),
    #[structopt(name = "create")]
    Create(AccountCreateOpt),
    #[structopt(name = "delete")]
    Delete(AccountNamedOpt),
}

impl AccountOpt {
    pub fn debug(&self) -> bool {
        match self {
            AccountOpt::Credential(acopt) => match acopt {
                AccountCredential::SetPassword(acs) => acs.copt.debug,
                AccountCredential::GeneratePassword(acs) => acs.copt.debug,
            },
            AccountOpt::Radius(acopt) => match acopt {
                AccountRadius::Show(aro) => aro.copt.debug,
                AccountRadius::Generate(aro) => aro.copt.debug,
                AccountRadius::Delete(aro) => aro.copt.debug,
            },
            AccountOpt::Posix(apopt) => match apopt {
                AccountPosix::Show(apo) => apo.copt.debug,
                AccountPosix::Set(apo) => apo.copt.debug,
                AccountPosix::SetPassword(apo) => apo.copt.debug,
            },
            AccountOpt::Ssh(asopt) => match asopt {
                AccountSsh::List(ano) => ano.copt.debug,
                AccountSsh::Add(ano) => ano.copt.debug,
                AccountSsh::Delete(ano) => ano.copt.debug,
            },
            AccountOpt::List(copt) => copt.debug,
            AccountOpt::Get(aopt) => aopt.copt.debug,
            AccountOpt::Delete(aopt) => aopt.copt.debug,
            AccountOpt::Create(aopt) => aopt.copt.debug,
        }
    }

    pub fn exec(&self) {
        match self {
            // id/cred/primary/set
            AccountOpt::Credential(acopt) => match acopt {
                AccountCredential::SetPassword(acsopt) => {
                    let client = acsopt.copt.to_client();
                    let password = rpassword::prompt_password_stderr(
                        format!("Enter new password for {}: ", acsopt.aopts.account_id).as_str(),
                    )
                    .unwrap();

                    client
                        .idm_account_primary_credential_set_password(
                            acsopt.aopts.account_id.as_str(),
                            password.as_str(),
                        )
                        .unwrap();
                }
                AccountCredential::GeneratePassword(acsopt) => {
                    let client = acsopt.copt.to_client();

                    let npw = client
                        .idm_account_primary_credential_set_generated(
                            acsopt.aopts.account_id.as_str(),
                        )
                        .unwrap();
                    println!(
                        "Generated password for {}: {}",
                        acsopt.aopts.account_id, npw
                    );
                }
            }, // end AccountOpt::Credential
            AccountOpt::Radius(aropt) => match aropt {
                AccountRadius::Show(aopt) => {
                    let client = aopt.copt.to_client();

                    let rcred = client
                        .idm_account_radius_credential_get(aopt.aopts.account_id.as_str())
                        .unwrap();

                    match rcred {
                        Some(s) => println!("Radius secret: {}", s),
                        None => println!("NO Radius secret"),
                    }
                }
                AccountRadius::Generate(aopt) => {
                    let client = aopt.copt.to_client();
                    client
                        .idm_account_radius_credential_regenerate(aopt.aopts.account_id.as_str())
                        .unwrap();
                }
                AccountRadius::Delete(aopt) => {
                    let client = aopt.copt.to_client();
                    client
                        .idm_account_radius_credential_delete(aopt.aopts.account_id.as_str())
                        .unwrap();
                }
            }, // end AccountOpt::Radius
            AccountOpt::Posix(apopt) => match apopt {
                AccountPosix::Show(aopt) => {
                    let client = aopt.copt.to_client();
                    let token = client
                        .idm_account_unix_token_get(aopt.aopts.account_id.as_str())
                        .unwrap();
                    println!("{:?}", token);
                }
                AccountPosix::Set(aopt) => {
                    let client = aopt.copt.to_client();
                    client
                        .idm_account_unix_extend(
                            aopt.aopts.account_id.as_str(),
                            aopt.gidnumber,
                            aopt.shell.as_deref(),
                        )
                        .unwrap();
                }
                AccountPosix::SetPassword(aopt) => {
                    let client = aopt.copt.to_client();
                    let password =
                        rpassword::prompt_password_stderr("Enter new unix (sudo) password: ")
                            .unwrap();
                    client
                        .idm_account_unix_cred_put(
                            aopt.aopts.account_id.as_str(),
                            password.as_str(),
                        )
                        .unwrap();
                }
            }, // end AccountOpt::Posix
            AccountOpt::Ssh(asopt) => match asopt {
                AccountSsh::List(aopt) => {
                    let client = aopt.copt.to_client();

                    let pkeys = client
                        .idm_account_get_ssh_pubkeys(aopt.aopts.account_id.as_str())
                        .unwrap();

                    for pkey in pkeys {
                        println!("{}", pkey)
                    }
                }
                AccountSsh::Add(aopt) => {
                    let client = aopt.copt.to_client();
                    client
                        .idm_account_post_ssh_pubkey(
                            aopt.aopts.account_id.as_str(),
                            aopt.tag.as_str(),
                            aopt.pubkey.as_str(),
                        )
                        .unwrap();
                }
                AccountSsh::Delete(aopt) => {
                    let client = aopt.copt.to_client();
                    client
                        .idm_account_delete_ssh_pubkey(
                            aopt.aopts.account_id.as_str(),
                            aopt.tag.as_str(),
                        )
                        .unwrap();
                }
            }, // end AccountOpt::Ssh
            AccountOpt::List(copt) => {
                let client = copt.to_client();
                let r = client.idm_account_list().unwrap();
                for e in r {
                    println!("{:?}", e);
                }
            }
            AccountOpt::Get(aopt) => {
                let client = aopt.copt.to_client();
                let e = client
                    .idm_account_get(aopt.aopts.account_id.as_str())
                    .unwrap();
                println!("{:?}", e);
            }
            AccountOpt::Delete(aopt) => {
                let client = aopt.copt.to_client();
                client
                    .idm_account_delete(aopt.aopts.account_id.as_str())
                    .unwrap();
            }
            AccountOpt::Create(acopt) => {
                let client = acopt.copt.to_client();
                client
                    .idm_account_create(
                        acopt.aopts.account_id.as_str(),
                        acopt.display_name.as_str(),
                    )
                    .unwrap();
            }
        }
    }
}
