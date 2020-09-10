use secp256k1::{PublicKey, SecretKey};
use std::error::Error;
use std::io::{BufReader, Read, Write};
use std::{fs, io};
use tiny_keccak::{Hasher, Keccak};

pub type EthAddress = [u8; 20];

pub struct Identity {
    secret: SecretKey,
    contract: EthAddress,
    vote_id: String,
}

impl Identity {
    pub fn new(contract: EthAddress, vote_id: String) -> Self {
        let mut os_rng = wasi_rng::WasiRng::default();
        let secret = SecretKey::random(&mut os_rng);
        Identity {
            secret,
            contract,
            vote_id,
        }
    }

    pub fn address(&self) -> String {
        let pub_key = PublicKey::from_secret_key(&self.secret);
        let bytes = pub_key.serialize();
        let mut keccak = Keccak::v256();
        let mut result = [0u8; 32];
        keccak.update(&bytes);
        keccak.finalize(&mut result);
        hex::encode(&result[12..])
    }

    pub fn contract(&self) -> String {
        hex::encode(self.contract.as_ref())
    }

    pub fn vote_id(&self) -> &str {
        &self.vote_id
    }

    pub fn save(&self) -> io::Result<()> {
        let mut f = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(super::prv_path("id.bin"))?;
        f.write_all(self.secret.serialize().as_ref())?;
        f.write_all(self.contract.as_ref())?;
        let vote_id = self.vote_id.as_bytes();
        let str_len = (vote_id.len() as u32).to_le_bytes();
        f.write_all(str_len.as_ref())?;
        f.write_all(vote_id)?;
        Ok(())
    }

    pub fn from_storage() -> Result<Self, Box<dyn Error>> {
        let mut f = BufReader::new(
            fs::OpenOptions::new()
                .read(true)
                .open(super::prv_path("id.bin"))?,
        );
        let mut secret_key_bytes = [0u8; 32];
        let mut contract = [0u8; 20];
        let mut str_len = [0u8; 4];
        f.read_exact(secret_key_bytes.as_mut())?;
        f.read_exact(contract.as_mut())?;
        f.read_exact(str_len.as_mut())?;
        let mut strbytes = Vec::new();
        strbytes.resize(u32::from_le_bytes(str_len) as usize, 0);
        f.read_exact(strbytes.as_mut())?;

        let secret = SecretKey::parse(&secret_key_bytes)?;
        let vote_id = String::from_utf8(strbytes)?;

        Ok(Self {
            secret,
            contract,
            vote_id,
        })
    }
}
