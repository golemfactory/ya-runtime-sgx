use aes_gcm::{
    Aes256Gcm,
    aead::{
        Aead,
        NewAead,
        generic_array::GenericArray,
    },
};
use rand::Rng;
use secp256k1::{PublicKey, SecretKey};
use sha2::Sha256;
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
        mgr_addr: String,
    },
    EncryptVote {
        mgr_key: String,
        vote: u32,
    },
}

fn read_key() -> Result<SecretKey, Box<dyn Error>> {
    let mut f = fs::OpenOptions::new().read(true).open(KEY_PATH)?;
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
            mgr_addr,
        } => {
            let key = read_key()?;
            let pkey = PublicKey::from_secret_key(&key);

            let msg = format!("\nSgxRegister\nContract: {} {}\nAddress: {}\nSession: {}",
                contract,
                voting_id,
                mgr_addr,
                hex::encode(pub_key_to_ethaddr(&pkey)),
            );
            let mut keccak = Keccak::v256();
            let mut result = [0u8; 32];
            keccak.update(b"\x19Ethereum Signed Message:\n".as_ref());
            keccak.update(msg.len().to_string().as_ref());
            keccak.update(msg.as_ref());
            keccak.finalize(&mut result);

            let msg = secp256k1::Message::parse(&result);

            let (sig, rid) = secp256k1::sign(&msg, &key);

            let mut sig_packed = hex::encode(sig.serialize().as_ref());
            sig_packed.push_str(&hex::encode(&[rid.serialize() + 27])); // +27 to match manager format


            println!("KEY {}", hex::encode(&pkey.serialize().as_ref()));
            println!("ADDR: {}", hex::encode(pub_key_to_ethaddr(&pkey)));
            println!("OK {}", sig_packed);
        }
        Args::EncryptVote {
            mgr_key,
            vote,
        } => {
            let key = read_key()?;

            let mut mgr_key_bytes = [0u8; 65];
            hex::decode_to_slice(&mgr_key, &mut mgr_key_bytes)?;
            let mgr_key = PublicKey::parse(&mgr_key_bytes)?;

            let shared_sec = secp256k1::SharedSecret::<Sha256>::new(&mgr_key, &key)?;
            let shared_key = GenericArray::from_slice(shared_sec.as_ref());
            let cipher = Aes256Gcm::new(&shared_key);

            let mut rng = rand::thread_rng();
            let nonce: [u8; 12] = rng.gen();
            let nonce = GenericArray::from_slice(&nonce);

            let msg = vote.to_le_bytes();

            let ct = cipher.encrypt(nonce, msg.as_ref()).map_err(|e| format!("Encryption error: {}", e).to_string())?;

            println!("CT: {}{}", hex::encode(nonce.as_slice()), hex::encode(&ct));
        }
    }
    Ok(())
}
