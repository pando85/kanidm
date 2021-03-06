use crate::common::{CommonOpt, Named};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub enum RecycleOpt {
    #[structopt(name = "list")]
    /// List objects that are in the recycle bin
    List(CommonOpt),
    #[structopt(name = "get")]
    /// Display an object from the recycle bin
    Get(Named),
    #[structopt(name = "revive")]
    /// Revive a recycled object into a live (accessible) state - this is the opposite of "delete"
    Revive(Named),
}

impl RecycleOpt {
    pub fn debug(&self) -> bool {
        match self {
            RecycleOpt::List(copt) => copt.debug,
            RecycleOpt::Get(nopt) => nopt.copt.debug,
            RecycleOpt::Revive(nopt) => nopt.copt.debug,
        }
    }

    pub fn exec(&self) {
        match self {
            RecycleOpt::List(copt) => {
                let client = copt.to_client();
                let r = client.recycle_bin_list().unwrap();
                for e in r {
                    println!("{:?}", e);
                }
            }
            RecycleOpt::Get(nopt) => {
                let client = nopt.copt.to_client();
                let e = client.recycle_bin_get(nopt.name.as_str()).unwrap();
                println!("{:?}", e);
            }
            RecycleOpt::Revive(nopt) => {
                let client = nopt.copt.to_client();
                client.recycle_bin_revive(nopt.name.as_str()).unwrap();
            }
        }
    }
}
