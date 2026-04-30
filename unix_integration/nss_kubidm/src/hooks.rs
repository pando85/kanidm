use crate::core::{self, RequestOptions};
<<<<<<<< HEAD:unix_integration/nss_sparkle_common/src/hooks.rs
========
use kubidm_unix_common::constants::DEFAULT_CONFIG_PATH;
>>>>>>>> master:unix_integration/nss_kubidm/src/hooks.rs
use libnss::group::{Group, GroupHooks};
use libnss::interop::Response;
use libnss::passwd::{Passwd, PasswdHooks};
use kubidm_unix_common::constants::DEFAULT_CONFIG_PATH;

<<<<<<<< HEAD:unix_integration/nss_sparkle_common/src/hooks.rs
pub struct SparklePasswd;

impl PasswdHooks for SparklePasswd {
========
struct KubidmPasswd;
libnss_passwd_hooks!(kubidm, KubidmPasswd);

impl PasswdHooks for KubidmPasswd {
>>>>>>>> master:unix_integration/nss_kubidm/src/hooks.rs
    fn get_all_entries() -> Response<Vec<Passwd>> {
        let req_opt = RequestOptions::Main {
            config_path: DEFAULT_CONFIG_PATH,
        };

        core::get_all_user_entries(req_opt)
    }

    fn get_entry_by_uid(uid: libc::uid_t) -> Response<Passwd> {
        let req_opt = RequestOptions::Main {
            config_path: DEFAULT_CONFIG_PATH,
        };

        core::get_user_entry_by_uid(uid, req_opt)
    }

    fn get_entry_by_name(name: String) -> Response<Passwd> {
        let req_opt = RequestOptions::Main {
            config_path: DEFAULT_CONFIG_PATH,
        };

        core::get_user_entry_by_name(name, req_opt)
    }
}

<<<<<<<< HEAD:unix_integration/nss_sparkle_common/src/hooks.rs
pub struct SparkleGroup;

impl GroupHooks for SparkleGroup {
========
struct KubidmGroup;
libnss_group_hooks!(kubidm, KubidmGroup);

impl GroupHooks for KubidmGroup {
>>>>>>>> master:unix_integration/nss_kubidm/src/hooks.rs
    fn get_all_entries() -> Response<Vec<Group>> {
        let req_opt = RequestOptions::Main {
            config_path: DEFAULT_CONFIG_PATH,
        };

        core::get_all_group_entries(req_opt)
    }

    fn get_entry_by_gid(gid: libc::gid_t) -> Response<Group> {
        let req_opt = RequestOptions::Main {
            config_path: DEFAULT_CONFIG_PATH,
        };

        core::get_group_entry_by_gid(gid, req_opt)
    }

    fn get_entry_by_name(name: String) -> Response<Group> {
        let req_opt = RequestOptions::Main {
            config_path: DEFAULT_CONFIG_PATH,
        };

        core::get_group_entry_by_name(name, req_opt)
    }
}
