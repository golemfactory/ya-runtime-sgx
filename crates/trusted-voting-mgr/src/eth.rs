pub use secp256k1::{Error, Message, PublicKey, SecretKey};
use tiny_keccak::{Hasher, Keccak};

pub struct EthHash([u8; 32]);

pub trait EthAddress {
    fn to_eth_address(&self) -> String;
}

impl EthHash {

    pub fn personal_message(message: impl AsRef<[u8]>) -> EthHash {
        let message = message.as_ref();
        let msg_size = message.len().to_string();
        let prefix = b"\x19Ethereum Signed Message:\n";
        eth_hash_parts(&[prefix.as_ref(), msg_size.as_ref(), message])
    }
}

fn eth_hash_parts(chunks: &[impl AsRef<[u8]>]) -> EthHash {
    let mut hasher = Keccak::v256();
    for chunk in chunks {
        hasher.update(chunk.as_ref());
    }
    let mut hash_bytes = [0u8; 32];
    hasher.finalize(&mut hash_bytes[..]);
    EthHash(hash_bytes)
}

impl EthAddress for PublicKey {
    fn to_eth_address(&self) -> String {
        let bytes = self.serialize();
        let hash = eth_hash_parts(&[&bytes[1..]]);
        format!("0x{}", hex::encode(&hash.0[12..]))
    }
}

impl EthAddress for SecretKey {
    fn to_eth_address(&self) -> String {
        PublicKey::from_secret_key(self).to_eth_address()
    }
}

pub struct RecoverableSignature {
    signature: secp256k1::Signature,
    recovery_id: secp256k1::RecoveryId,
}

impl RecoverableSignature {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != 65 {
            return Err(Error::InvalidInputLength);
        }

        let signature = secp256k1::Signature::parse_slice(&bytes[..64])?;
        let recovery_id = secp256k1::RecoveryId::parse_rpc(bytes[64]).unwrap();
        Ok(Self {
            signature,
            recovery_id,
        })
    }

    pub fn from_hex(mut hex: &str) -> Result<Self, Error> {
        if hex.starts_with("0x") {
            hex = &hex[2..];
        }
        Self::from_bytes(&hex::decode(hex).map_err(|_| Error::InvalidSignature)?)
    }

    pub fn recover_pub_key(&self, message_hash: &EthHash) -> Result<PublicKey, Error> {
        let message = Message::parse(&message_hash.0);

        secp256k1::recover(&message, &self.signature, &self.recovery_id)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(RecoverableSignature::from_hex("87aa6e272316599a56644df843cf9ecbb681750a2afe31a8750cdb1821c257035721b20e1b170f2f7b31ad16d0f3436706bf6669347791c8afdf4ea947de6f601b")?
            .recover_pub_key(&EthHash::personal_message("kot"))?
            .to_eth_address(),
            "0xc79c9d10d504f33c3def67d4284c10ec3691593d");
        //
        let message = "RegisterToVote\nContract: aea5db67524e02a263b9339fe6667d6b577f3d4c 1\nAddress: a7dab260472557b5eef526589a4f37a0f5f81569";
        assert_eq!(RecoverableSignature::from_hex("0x174ddb3fccb6009e13a1e6ad938b7704cfc9eae72f54579309e88f44242fa723011a6f61cb3be705448a5a716a4ccad5ef534d5b399f4e4cee34444ef645ada81c")?
            .recover_pub_key(&EthHash::personal_message(message))?
            .to_eth_address(),
        "0xc79c9d10d504f33c3def67d4284c10ec3691593d");
        Ok(())
    }
}
