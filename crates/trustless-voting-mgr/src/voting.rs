use crate::eth::{EthAddress, EthHash, RecoverableSignature, ToEthAddress};
use aes_gcm::{
    aead::{generic_array::GenericArray, Aead, NewAead},
    Aes256Gcm,
};
use anyhow::Context;
use rand_core::RngCore;
use secp256k1::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::PathBuf;
use std::{
    collections::{hash_map, HashMap},
    convert::TryInto,
    fs,
    io::BufReader,
};
use thiserror::Error;
use wasi_rng::WasiRng;

#[derive(Error, Debug)]
pub enum VotingError {
    #[error("voting already started")]
    AlreadyStarted,
    #[error("already voted")]
    AlreadyVoted,
    #[error("DecryptionError")]
    DecryptionError,
    #[error("InvalidAddress")]
    InvalidAddress,
    #[allow(unused)]
    #[error("InvalidId")]
    InvalidId,
    #[error("NotFinished")]
    NotFinished,
    #[error("NotStarted")]
    NotStarted,
}

pub struct Voting {
    secret: SecretKey,
    contract: EthAddress,
    voting_id: String,
    started: bool,
    voters: HashMap<EthAddress, (PublicKey, bool)>,
    results: HashMap<u32, u32>,
}

#[derive(Serialize, Deserialize)]
struct BinaryState {
    secret: Vec<u8>,
    contract: [u8; 20],
    voting_id: String,
    started: bool,
    voters: HashMap<[u8; 20], (Vec<u8>, bool)>,
    results: HashMap<u32, u32>,
}

impl BinaryState {
    fn from_voting(v: &Voting) -> Self {
        let secret = v.secret.serialize().into();
        let contract = v.contract.to_array();
        let voting_id = v.voting_id.clone();
        let started = v.started;
        let voters = v
            .voters
            .iter()
            .map(|(k, (p, v))| (k.to_array(), (Vec::from(p.serialize().as_ref()), *v)))
            .collect();
        let results = v.results.clone();
        Self {
            secret,
            contract,
            voting_id,
            started,
            voters,
            results,
        }
    }

    fn into_voting(self) -> anyhow::Result<Voting> {
        let secret = SecretKey::parse_slice(&self.secret)?;
        let contract = EthAddress::new(self.contract);
        let voting_id = self.voting_id;
        let started = self.started;
        let voters = self
            .voters
            .into_iter()
            .map(|(k, (p, v))| Ok((EthAddress::new(k), (PublicKey::parse_slice(&p, None)?, v))))
            .collect::<anyhow::Result<_>>()?;
        let results = self.results;
        Ok(Voting {
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

    pub fn operator_pubkey(&self) -> [u8; 65] {
        PublicKey::from_secret_key(&self.secret).serialize()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let file_name = Self::storage_path(&self.voting_id);
        let mut f = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&file_name)
            .with_context(|| format!("failed to open file: {}", file_name.display()))?;

        bincode::serialize_into(&mut f, &BinaryState::from_voting(self))?;

        Ok(())
    }

    pub fn load(
        _requested_contract: &str,
        requested_voting_id: &str,
        _operator_addr: &str,
    ) -> anyhow::Result<Self> {
        let file_name = Self::storage_path(requested_voting_id);
        let mut f = BufReader::new(
            fs::OpenOptions::new()
                .read(true)
                .open(&file_name)
                .with_context(|| format!("failed to open file {}", file_name.display()))?,
        );

        let state: BinaryState = bincode::deserialize_from(&mut f)?;
        Ok(state.into_voting()?)
    }

    fn storage_path(voting_id: &str) -> PathBuf {
        super::prv_path(format!("voting-{}.bin", voting_id))
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
    ) -> anyhow::Result<String> {
        use crate::eth;
        if self.started {
            return Err(VotingError::AlreadyStarted.into());
        }
        let sender = eth::EthAddress::from_hex(sender)
            .with_context(|| format!("invalid sender {}", sender))?;
        let signature = RecoverableSignature::from_hex(signature_hex)
            .with_context(|| format!("invalid signature format, {:?}", signature_hex))?;
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
            return Err(VotingError::InvalidAddress).context("invalid signature address");
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

    pub fn vote(&mut self, sender: &str, encrypted_vote: &str) -> anyhow::Result<String> {
        if !self.started {
            return Err(VotingError::NotStarted.into());
        }

        let sender_addr = EthAddress::from_hex(sender)?;

        let (session_key, voted_already) = self
            .voters
            .get(&sender_addr)
            .ok_or(VotingError::InvalidAddress)?;
        if *voted_already {
            return Err(VotingError::AlreadyVoted.into());
        }

        let vote = hex::decode(encrypted_vote)?;
        if vote.len() <= 12 {
            return Err(VotingError::DecryptionError.into());
        }

        let shared_sec = secp256k1::SharedSecret::<Sha256>::new(session_key, &self.secret)?;
        let shared_key = GenericArray::from_slice(shared_sec.as_ref());
        let cipher = Aes256Gcm::new(&shared_key);

        let nonce = GenericArray::from_slice(&vote[..12]);
        let enc_vote = &vote[12..];

        let vote = cipher
            .decrypt(nonce, enc_vote)
            .map_err(|e| anyhow::anyhow!("DecryptionError {}", e))
            .with_context(|| format!("fail to decrypt vote"))?;
        if vote.len() != 4 {
            return Err(VotingError::DecryptionError).context("invalid vote length");
        }

        let vote = u32::from_le_bytes(vote.as_slice().try_into()?);

        *self.results.entry(vote).or_insert(0) += 1;

        match self.voters.entry(sender_addr) {
            hash_map::Entry::Occupied(mut e) => {
                e.get_mut().1 = true;
            }
            hash_map::Entry::Vacant(_) => {
                panic!("impossible");
            }
        }
        let response = b"ACCEPTED";
        let mut rng = WasiRng::default();
        let mut iv = [0u8; 12];
        rng.fill_bytes(&mut iv);
        let encrypted_response = cipher
            .encrypt(&GenericArray::from(iv), response.as_ref())
            .map_err(|e| anyhow::anyhow!("EncryptionError: {}", e))?;

        Ok(format!(
            "{}{}",
            hex::encode(&iv),
            hex::encode(&encrypted_response)
        ))
    }

    pub fn report(&self) -> anyhow::Result<(Vec<(u32, u32)>, String)> {
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
        results.sort_by(|(k1, _), (k2, _)| Ord::cmp(k1, k2));

        let mut hasher = EthHash::new("SgxVotingResults(address,string,fixed32[])")
            .add(&self.contract)
            .add(&self.voting_id);
        for (k, v) in &results {
            hasher = hasher.add(u32::to_le_bytes(*k)).add(u32::to_le_bytes(*v))
        }
        let signature = hasher.build().sign_by(&self.secret);

        Ok((results, signature.to_hex()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decrypt() -> anyhow::Result<()> {
        let key = hex::decode("ba95ff8fdf43418d6653a1bfd542c5ef1c840892c0381ec1ebd89cf8bd29731b")?;
        let message =
            hex::decode("d380b132012072385152c432f75f3c3b46aefe25f00fe89404b4c40b6b334160")?;
        //e492c11aef6cf055e959c8d3960d96a737128f2cb20db2fd865d9f529b1097a4
        let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
        let nonce = &message[..12];
        let v = cipher
            .decrypt(nonce.into(), &message[12..])
            .map_err(|e| anyhow::anyhow!("decrypt {}", e))?;

        assert_eq!(v.len(), 4);
        println!("v={:?}", v);

        Ok(())
    }
}
