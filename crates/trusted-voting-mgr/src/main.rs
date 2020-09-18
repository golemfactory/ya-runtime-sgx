use crate::idenity::Identity;

use std::error::Error;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

pub fn prv_path(f: &str) -> PathBuf {
    AsRef::<Path>::as_ref("/private").join(f)
}

mod eth;
mod idenity;

#[derive(StructOpt)]
#[allow(unused)]
enum Args {
    /// initalizes voteing log.
    Init {
        /// Example: c73b910e58cb19341ec86111a054547d536d0448
        contract: String,
        voting_id: String,
    },
    Register {
        contract: String,
        voting_id: String,
        mgr_addr: String,
        sender: String,
        /// keccak256 for register(contract, voting_id, mgr_addr, sender)
        signature: Option<String>,
    },
    Start {
        contract: String,
        voting_id: String,
        mgr_addr: String,
    },
    Vote {
        contract: String,
        voting_id: String,
        mgr_addr: String,
        sender: String,
        voters: u64,
        encrypted_vote: String,
    },
    Report {
        contract: String,
        voting_id: String,
        mgr_addr: String,
    },
    // for debug
    Debug {},
}

fn main() -> Result<(), Box<dyn Error>> {
    match Args::from_args() {
        Args::Init {
            contract,
            voting_id,
        } => {
            let mut contract_bytes: idenity::EthAddress = [0u8; 20];
            hex::decode_to_slice(contract, &mut contract_bytes)?;
            let id = idenity::Identity::new(contract_bytes, voting_id);
            id.save()?;
            println!("OK {}", id.address())
        }
        Args::Debug {} => {
            let id = Identity::from_storage()?;
            println!("OK {} {} {}", id.address(), id.contract(), id.vote_id());
        }
        _ => unimplemented!(),
    }
    Ok(())
}
