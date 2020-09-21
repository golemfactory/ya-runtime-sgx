use crate::eth::{EthAddress, EthHash, RecoverableSignature, ToEthAddress};
use secp256k1::{PublicKey, SecretKey};
use std::{
    collections::{hash_map, HashMap},
    convert::TryInto,
    error::Error,
    fmt, fs,
    io::{self, BufReader, Read, Write},
};

#[derive(Debug)]
pub enum VotingError {
    AlreadyStarted,
    AlreadyVoted,
    InvalidAddress,
    InvalidId,
    NotFinished,
    NotStarted,
}

impl fmt::Display for VotingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Error for VotingError {}

pub struct Voting {
    secret: SecretKey,
    contract: EthAddress,
    voting_id: String,
    started: bool,
    voters: HashMap<EthAddress, (PublicKey, bool)>,
    results: HashMap<u32, u32>,
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
            results: HashMap::new(),
        }
    }

    pub fn operator_address(&self) -> EthAddress {
        self.secret.to_eth_address()
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
        for (_, (v, b)) in self.voters.iter() {
            f.write_all(&v.serialize_compressed())?;
            f.write_all((*b as u8).to_le_bytes().as_ref())?;
        }

        let results_len = (self.results.len() as u32).to_le_bytes();
        f.write_all(results_len.as_ref())?;
        for (k, v) in self.results.iter() {
            f.write_all(k.to_le_bytes().as_ref())?;
            f.write_all(v.to_le_bytes().as_ref())?;
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
        let mut voters = HashMap::with_capacity(voters_len as usize);
        for _ in 0..voters_len {
            let mut voter = [0u8; 33];
            f.read_exact(voter.as_mut())?;
            let voter = PublicKey::parse_compressed(&voter)?;
            let voter_addr = voter.to_eth_address();

            let mut voted = [0u8];
            f.read_exact(voted.as_mut())?;
            let voted = u8::from_le_bytes(voted);

            voters.insert(voter_addr, (voter, voted != 0));
        }

        let mut results_len = [0u8; 4];
        f.read_exact(results_len.as_mut())?;
        let results_len = u32::from_le_bytes(results_len);
        let mut results = HashMap::with_capacity(results_len as usize);
        for _ in 0..results_len {
            let mut k = [0u8; 4];
            let mut v = [0u8; 4];
            f.read_exact(k.as_mut())?;
            f.read_exact(v.as_mut())?;
            let k = u32::from_le_bytes(k);
            let v = u32::from_le_bytes(v);
            results.insert(k, v);
        }

        let contract = EthAddress::new(contract);

        let requested_contract = EthAddress::from_hex(requested_contract)?;
        if requested_contract != contract {
            return Err(VotingError::InvalidAddress.into());
        }
        if *requested_voting_id != voting_id {
            return Err(VotingError::InvalidId.into());
        }
        let operator_addr = EthAddress::from_hex(operator_addr)?;
        if operator_addr != secret.to_eth_address() {
            return Err(VotingError::InvalidAddress.into());
        }

        Ok(Self {
            secret,
            contract,
            voting_id,
            started,
            voters,
            results,
        })
    }
}

impl Voting {
    pub fn start(&mut self) -> Result<String, VotingError> {
        if self.started {
            return Err(VotingError::AlreadyStarted);
        }
        self.started = true;
        let voters = self
            .voters
            .keys()
            .map(|addr| addr.to_hex_string())
            .collect::<Vec<_>>()
            .join(" ");
        Ok(voters)
    }

    pub fn register(
        &mut self,
        sender: &str,
        signature_hex: &str,
        session_pub_key: &str,
    ) -> Result<String, Box<dyn Error>> {
        use crate::eth;
        if self.started {
            return Err(VotingError::AlreadyStarted.into());
        }
        let sender = eth::EthAddress::from_hex(sender)?;
        let signature = RecoverableSignature::from_hex(signature_hex)?;
        let session_pub_key = PublicKey::parse_slice(&hex::decode(&session_pub_key)?, None)?;

        let mesagage_hash = EthHash::personal_message(
            format!("\nSgxRegister\nContract: {contract:x} {voting_id}\nAddress: {operator:x}\nSession: {session_addr:x}",
                contract= self.contract,
                voting_id = self.voting_id,
                operator = self.operator_address(),
                session_addr = session_pub_key.to_eth_address()
            ),
        );
        if signature.recover_pub_key(&mesagage_hash)?.to_eth_address() != sender {
            return Err(VotingError::InvalidAddress.into());
        }
        let hash = EthHash::new("SgxVotingTicket(address,bytes,address)")
            .add(&self.contract)
            .add(&self.voting_id)
            .add(&sender)
            .build();
        self.voters.insert(sender, (session_pub_key, false));

        let signature = hash.sign_by(&self.secret);

        Ok(signature.to_hex())
    }

    pub fn vote(&mut self, sender: &str, encrypted_vote: &str) -> Result<String, Box<dyn Error>> {
        if !self.started {
            return Err(VotingError::NotStarted.into());
        }

        let sender_addr = EthAddress::from_hex(sender)?;

        let (_session_key, voted_already) = self
            .voters
            .get(&sender_addr)
            .ok_or(VotingError::InvalidAddress)?;
        if *voted_already {
            return Err(VotingError::AlreadyVoted.into());
        }

        let vote = hex::decode(encrypted_vote)?;

        // TODO decrypt vote

        let vote = u32::from_le_bytes(vote[..4].try_into()?);

        *self.results.entry(vote).or_insert(0) += 1;

        match self.voters.entry(sender_addr) {
            hash_map::Entry::Occupied(mut e) => {
                e.get_mut().1 = true;
            }
            hash_map::Entry::Vacant(_) => {
                panic!("impossible");
            }
        }

        // TODO encrypt response
        Ok("ACCPETED".to_string())
    }

    pub fn report(&self) -> Result<Vec<(u32, u32)>, Box<dyn Error>> {
        if !self.started {
            return Err(VotingError::NotStarted.into());
        }

        for (_, (_, voted)) in self.voters.iter() {
            if !voted {
                return Err(VotingError::NotFinished.into());
            }
        }

        let mut results = Vec::with_capacity(self.results.len());

        for (k, v) in self.results.iter() {
            results.push((*k, *v));
        }

        Ok(results)
    }
}
