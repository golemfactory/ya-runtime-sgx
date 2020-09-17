use secp256k1::{PublicKey, SecretKey};
use std::{
    convert::TryInto,
    error::Error,
    fs,
    io::{Read, Write},
};
use structopt::StructOpt;
use tiny_keccak::{Hasher, Keccak};

const KEY_PATH: &str = "key.bin";

#[derive(StructOpt)]
enum Args {
    /// generates a new key
    GenKey {},
    /// generates signature for a register request
    SignRegister {
        contract: String,
        voting_id: String,
        operator_addr: String,
    },
    EncryptVote {
    },
}

fn read_key() -> Result<SecretKey, Box<dyn Error>> {
    let mut f = fs::OpenOptions::new()
        .read(true)
        .open(KEY_PATH)?;
    let mut key_bytes = [0u8; 32];
    f.read_exact(key_bytes.as_mut())?;
    Ok(SecretKey::parse(&key_bytes)?)
}

pub fn unhex_ethaddr(addr: &str) -> Result<[u8; 20], hex::FromHexError> {
    let mut addr_bytes = [0u8; 20];
    hex::decode_to_slice(addr, &mut addr_bytes)?;
    Ok(addr_bytes)
}

fn pub_key_to_ethaddr(pub_key: &PublicKey) -> [u8; 20] {
    let bytes = pub_key.serialize();
    let mut keccak = Keccak::v256();
    let mut result = [0u8; 32];
    // This is uncompressed form; we need raw key bytes, thus skipping the first byte.
    keccak.update(&bytes[1..]);
    keccak.finalize(&mut result);
    result[12..].try_into().unwrap()
}

fn main() -> Result<(), Box<dyn Error>> {
    match Args::from_args() {
        Args::GenKey {} => {
            let mut rng = rand::thread_rng();
            let key = SecretKey::random(&mut rng);
            let mut f = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(KEY_PATH)?;
            f.write_all(&key.serialize())?;
        }
        Args::SignRegister {
            contract,
            voting_id,
            operator_addr,
        } => {
            let contract = unhex_ethaddr(&contract)?;
            let operator_addr = unhex_ethaddr(&operator_addr)?;

            let mut keccak = Keccak::v256();
            let mut result = [0u8; 32];
            keccak.update(&contract);
            keccak.update(voting_id.as_bytes());
            keccak.update(&operator_addr);
            keccak.finalize(&mut result);

            let msg = secp256k1::Message::parse(&result);
            let key = read_key()?;

            let (sig, rid) = secp256k1::sign(&msg, &key);

            let mut sig_packed = hex::encode(&[rid.serialize()]);
            sig_packed.push_str(&hex::encode(sig.serialize().as_ref()));

            println!("OK {}", sig_packed);
        }
        Args::EncryptVote {
        } => {
            let key = read_key()?;
            let pk = secp256k1::PublicKey::from_secret_key(&key);
            println!("ADDR: {}", hex::encode(pub_key_to_ethaddr(&pk)));
        }
    }
    Ok(())
}
