use crate::voting::{unhex_ethaddr, Voting};

use std::error::Error;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

pub fn prv_path(f: &str) -> PathBuf {
    AsRef::<Path>::as_ref("/private").join(f)
}

mod eth;
mod voting;

#[derive(StructOpt)]
enum Args {
    /// initalizes voting log.
    Init {
        /// Example: c73b910e58cb19341ec86111a054547d536d0448
        contract: String,
        voting_id: String,
    },
    /// registers a voter
    Register {
        contract: String,
        voting_id: String,
        operator_addr: String,
        /// sender signed keccak256 for register(contract, voting_id, operator_addr)
        signature: String,
    },
    /// starts the voting
    Start {
        contract: String,
        voting_id: String,
        operator_addr: String,
    },
    /// adds an encrypted vote
    Vote {
        contract: String,
        voting_id: String,
        operator_addr: String,
        sender: String,
        encrypted_vote: String,
    },
    /// prints voting summary
    Report {
        contract: String,
        voting_id: String,
        operator_addr: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    match Args::from_args() {
        Args::Init {
            contract,
            voting_id,
        } => {
            let contract_addr = unhex_ethaddr(contract.as_str())?;
            let v = Voting::new(contract_addr, voting_id);
            v.save()?;
            let op_addr = hex::encode(&v.operator_address());
            println!("INIT: OK {}", op_addr);
        }
        Args::Start {
            contract,
            voting_id,
            operator_addr,
        } => {
            let mut v = Voting::load(&contract, &voting_id, &operator_addr)?;
            v.start()?;
            v.save()?;
            println!("START: OK");
        }
        Args::Register {
            contract,
            voting_id,
            operator_addr,
            signature,
        } => {
            let mut v = Voting::load(&contract, &voting_id, &operator_addr)?;
            v.register(signature.as_bytes())?;
            v.save()?;
            println!("REGISTER: OK");
        }
        Args::Vote {
            contract,
            voting_id,
            operator_addr,
            sender,
            encrypted_vote,
        } => {
            let mut v = Voting::load(&contract, &voting_id, &operator_addr)?;
            v.vote(&sender, &encrypted_vote)?;
            v.save()?;
            println!("VOTE: OK");
        }
        Args::Report {
            contract,
            voting_id,
            operator_addr,
        } => {
            let v = Voting::load(&contract, &voting_id, &operator_addr)?;
            let results = v.report()?;
            println!("Results:");
            for (option, votes) in results.iter() {
                println!("{}: {}", option, votes);
            }
            println!("REPORT: OK");
        }
    }
    Ok(())
}
