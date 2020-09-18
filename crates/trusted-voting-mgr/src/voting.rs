use secp256k1::{PublicKey, SecretKey};
use std::{
    collections::HashMap,
    convert::TryInto,
    error::Error,
    fmt, fs,
    io::{self, BufReader, Read, Write},
};
use tiny_keccak::{Hasher, Keccak};

#[derive(Debug)]
pub enum VotingError {
    AlreadyStarted,
    InvalidAddress,
    InvalidId,
    NotStarted,
}

impl fmt::Display for VotingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Error for VotingError {}

pub type EthAddress = [u8; 20];

pub struct Voting {
    secret: SecretKey,
    contract: EthAddress,
    voting_id: String,
    started: bool,
    voters: HashMap<EthAddress, PublicKey>,
}

fn pub_key_to_ethaddr(pub_key: &PublicKey) -> EthAddress {
    let bytes = pub_key.serialize();
    let mut keccak = Keccak::v256();
    let mut result = [0u8; 32];
    // This is uncompressed form; we need raw key bytes, thus skipping the first byte.
    keccak.update(&bytes[1..]);
    keccak.finalize(&mut result);
    result[12..].try_into().unwrap()
}

fn priv_key_to_ethaddr(key: &SecretKey) -> EthAddress {
    let pub_key = PublicKey::from_secret_key(key);
    pub_key_to_ethaddr(&pub_key)
}

pub fn unhex_ethaddr(addr: &str) -> Result<EthAddress, hex::FromHexError> {
    let mut addr_bytes: EthAddress = [0u8; 20];
    hex::decode_to_slice(addr, &mut addr_bytes)?;
    Ok(addr_bytes)
}

impl Voting {
    const SAVED_FILE_NAME: &'static str = "voting.bin";

    pub fn new(contract: EthAddress, voting_id: String) -> Self {
        let mut os_rng = wasi_rng::WasiRng::default();
        let secret = SecretKey::random(&mut os_rng);
        Self {
            secret,
            contract,
            voting_id,
            started: false,
            voters: HashMap::new(),
        }
    }

    pub fn operator_address(&self) -> EthAddress {
        priv_key_to_ethaddr(&self.secret)
    }

    pub fn save(&self) -> io::Result<()> {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(super::prv_path(Self::SAVED_FILE_NAME))?;

        f.write_all(&self.secret.serialize())?;

        f.write_all(self.contract.as_ref())?;

        let voting_id = self.voting_id.as_bytes();
        let str_len = (voting_id.len() as u32).to_le_bytes();
        f.write_all(str_len.as_ref())?;
        f.write_all(voting_id)?;

        let started = (self.started as u8).to_le_bytes();
        f.write_all(started.as_ref())?;

        let voters_len = (self.voters.len() as u32).to_le_bytes();
        f.write_all(voters_len.as_ref())?;
        for (_, v) in self.voters.iter() {
            f.write_all(&v.serialize_compressed())?;
        }

        Ok(())
    }

    pub fn load(
        requested_contract: &str,
        requested_voting_id: &str,
        operator_addr: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let mut f = BufReader::new(
            fs::OpenOptions::new()
                .read(true)
                .open(super::prv_path(Self::SAVED_FILE_NAME))?,
        );

        let mut secret_key_bytes = [0u8; 32];
        f.read_exact(secret_key_bytes.as_mut())?;
        let secret = SecretKey::parse(&secret_key_bytes)?;

        let mut contract = [0u8; 20];
        f.read_exact(contract.as_mut())?;

        let mut str_len = [0u8; 4];
        let mut strbytes = Vec::new();
        f.read_exact(str_len.as_mut())?;
        strbytes.resize(u32::from_le_bytes(str_len) as usize, 0);
        f.read_exact(strbytes.as_mut())?;
        let voting_id = String::from_utf8(strbytes)?;

        let mut started = [0u8];
        f.read_exact(started.as_mut())?;
        let started = u8::from_le_bytes(started) != 0;

        let mut voters_len = [0u8; 4];
        f.read_exact(voters_len.as_mut())?;
        let voters_len = u32::from_le_bytes(voters_len);
        let mut voters = HashMap::new();
        for _ in 0..voters_len {
            let mut voter = [0u8; 33];
            f.read_exact(voter.as_mut())?;
            let voter = PublicKey::parse_compressed(&voter)?;
            let voter_addr = pub_key_to_ethaddr(&voter);
            voters.insert(voter_addr, voter);
        }

        let requested_contract = unhex_ethaddr(requested_contract)?;
        if requested_contract != contract {
            return Err(VotingError::InvalidAddress.into());
        }
        if *requested_voting_id != voting_id {
            return Err(VotingError::InvalidId.into());
        }
        let operator_addr = unhex_ethaddr(operator_addr)?;
        if operator_addr != priv_key_to_ethaddr(&secret) {
            return Err(VotingError::InvalidAddress.into());
        }

        Ok(Self {
            secret,
            contract,
            voting_id,
            started,
            voters,
        })
    }
}

impl Voting {
    pub fn start(&mut self) -> Result<(), VotingError> {
        if self.started {
            return Err(VotingError::AlreadyStarted);
        }
        self.started = true;
        Ok(())
    }

    pub fn register(&mut self, signature_hex: &[u8]) -> Result<(), Box<dyn Error>> {
        if self.started {
            return Err(VotingError::AlreadyStarted.into());
        }

        let mut keccak = Keccak::v256();
        let mut result = [0u8; 32];
        keccak.update(&self.contract);
        keccak.update(self.voting_id.as_bytes());
        keccak.update(&self.operator_address());
        keccak.finalize(&mut result);
        let msg = secp256k1::Message::parse(&result);

        let mut signature_packed = [0u8; 65];
        hex::decode_to_slice(signature_hex, &mut signature_packed)?;
        let recovery_id = secp256k1::RecoveryId::parse(signature_packed[0])?;
        let signature = secp256k1::Signature::parse_slice(&signature_packed[1..])?;

        let voter_key = secp256k1::recover(&msg, &signature, &recovery_id)?;
        let voter_addr = pub_key_to_ethaddr(&voter_key);

        self.voters.insert(voter_addr, voter_key);

        Ok(())
    }

    pub fn vote(&mut self, sender: &str, encrypted_vote: &str) -> Result<(), Box<dyn Error>> {
        if !self.started {
            return Err(VotingError::NotStarted.into());
        }

        let sender_addr = unhex_ethaddr(sender)?;

        let sender_key = self
            .voters
            .get(&sender_addr)
            .ok_or(VotingError::InvalidAddress)?;

        let vote = hex::decode(encrypted_vote)?;

        println!(
            "Voter: {}\nVote: {:?}",
            hex::encode(sender_key.serialize().as_ref()),
            vote
        );

        Ok(())
    }
}
