use crate::voting::Voting;

use crate::eth::EthAddress;
use anyhow::Context;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

pub fn prv_path<P: AsRef<Path>>(f: P) -> PathBuf {
    AsRef::<Path>::as_ref("/private").join(f.as_ref())
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
        sender: String,
        signature: String,
        session_pub_key: String,
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

fn main() {
    match run() {
        Ok(()) => (),
        Err(e) => println!("ERROR v2 {:?}", e),
    }
}

fn run() -> anyhow::Result<()> {
    match Args::from_args() {
        Args::Init {
            contract,
            voting_id,
        } => {
            let contract_addr = EthAddress::from_hex(contract.as_str())?;
            let v = Voting::new(contract_addr, voting_id);
            v.save().context("init save")?;
            let op_addr = hex::encode(&v.operator_address());
            let op_pkey = hex::encode(v.operator_pubkey().as_ref());
            println!("OK {} {}", op_addr, op_pkey);
        }
        Args::Start {
            contract,
            voting_id,
            operator_addr,
        } => {
            let mut v = Voting::load(&contract, &voting_id, &operator_addr).context("load")?;
            let list = v.start().context("start")?;
            v.save().context("save")?;
            println!("OK {}", list);
        }
        Args::Register {
            contract,
            voting_id,
            operator_addr,
            sender,
            signature,
            session_pub_key,
        } => {
            let mut v = Voting::load(&contract, &voting_id, &operator_addr)
                .with_context(|| "loading state")?;
            let ticket = v
                .register(&sender, &signature, &session_pub_key)
                .context("register")?;
            v.save().context("save")?;
            println!("OK {}", ticket);
        }
        Args::Vote {
            contract,
            voting_id,
            operator_addr,
            sender,
            encrypted_vote,
        } => {
            let mut v = Voting::load(&contract, &voting_id, &operator_addr)?;
            let response = v.vote(&sender, &encrypted_vote)?;
            v.save()?;
            println!("OK {}", response);
        }
        Args::Report {
            contract,
            voting_id,
            operator_addr,
        } => {
            let v = Voting::load(&contract, &voting_id, &operator_addr)?;
            let (results, signature) = v.report()?;

            let formated_results = results
                .into_iter()
                .map(|(k, v)| format!("{:x} {:x}", k, v))
                .collect::<Vec<_>>()
                .join(" ");
            println!("OK {} {}", signature, formated_results);
        }
    }
    Ok(())
}
