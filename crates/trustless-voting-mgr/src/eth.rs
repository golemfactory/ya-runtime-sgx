#![allow(unused)]
pub use secp256k1::{Error, Message, PublicKey, SecretKey};
use std::fmt;
use std::fmt::{Debug, Formatter};
use tiny_keccak::{Hasher, Keccak};

type DynError = Box<dyn std::error::Error>;

pub struct EthHash([u8; 32]);

impl EthHash {
    pub fn personal_message(message: impl AsRef<[u8]>) -> EthHash {
        let message = message.as_ref();
        let msg_size = message.len().to_string();
        let prefix = b"\x19Ethereum Signed Message:\n";
        eth_hash_parts(&[prefix.as_ref(), msg_size.as_ref(), message])
    }

    pub fn new(signature: &str) -> EthHashBuilder {
        let sig = signature_hash(signature);
        let mut hasher = Keccak::v256();
        hasher.update(sig.as_ref());
        EthHashBuilder(hasher)
    }

    pub fn sign_by(&self, secret: &SecretKey) -> RecoverableSignature {
        let message = Message::parse(&self.0);
        let (signature, recovery_id) = secp256k1::sign(&message, secret);
        RecoverableSignature {
            signature,
            recovery_id,
        }
    }
}

impl AsRef<[u8]> for EthHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::LowerHex for EthHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

pub fn signature_hash(signature: &str) -> EthHash {
    eth_hash_parts(&[signature.as_bytes()])
}

pub struct EthHashBuilder(Keccak);

impl EthHashBuilder {
    pub fn add(mut self, content: impl AsRef<[u8]>) -> Self {
        self.0.update(content.as_ref());
        self
    }

    pub fn build(self) -> EthHash {
        let mut bytes = [0; 32];
        self.0.finalize(&mut bytes[..]);
        EthHash(bytes)
    }
}

#[derive(Eq, PartialEq, Hash)]
pub struct EthAddress([u8; 20]);

impl AsRef<[u8]> for EthAddress {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8; 20]> for EthAddress {
    fn as_ref(&self) -> &[u8; 20] {
        &self.0
    }
}

pub trait ToEthAddress {
    fn to_eth_address(&self) -> EthAddress;
}

impl fmt::LowerHex for EthAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..]))
    }
}

impl Debug for EthAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}", self)
    }
}

impl EthAddress {
    pub fn new(inner: [u8; 20]) -> Self {
        EthAddress(inner)
    }

    pub fn to_hex_string(&self) -> String {
        format!("{:x}", self)
    }

    pub fn from_hex(bytes: impl AsRef<[u8]>) -> Result<Self, hex::FromHexError> {
        let mut inner = [0; 20];
        hex::decode_to_slice(bytes.as_ref(), &mut inner[..])?;
        Ok(EthAddress(inner))
    }

    pub fn to_array(&self) -> [u8; 20] {
        self.0
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

impl ToEthAddress for PublicKey {
    fn to_eth_address(&self) -> EthAddress {
        let bytes = self.serialize();
        let hash = eth_hash_parts(&[&bytes[1..]]);
        let mut address = [0; 20];
        address.copy_from_slice(&hash.0[12..]);
        EthAddress(address)
    }
}

impl ToEthAddress for SecretKey {
    fn to_eth_address(&self) -> EthAddress {
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
        let r = bytes[64];
        let recovery_id = if r >= 0x1b {
            secp256k1::RecoveryId::parse_rpc(bytes[64])?
        } else {
            secp256k1::RecoveryId::parse(bytes[64])?
        };

        Ok(Self {
            signature,
            recovery_id,
        })
    }

    pub fn to_hex(&self) -> String {
        let sig = self.signature.serialize();
        let r = self.recovery_id.serialize();
        format!("{}{:02x}", hex::encode(sig.as_ref()), r)
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
    use sha2::digest::generic_array::GenericArray;
    use sha2::{Digest, Sha256};
    use std::io::Write;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(RecoverableSignature::from_hex("87aa6e272316599a56644df843cf9ecbb681750a2afe31a8750cdb1821c257035721b20e1b170f2f7b31ad16d0f3436706bf6669347791c8afdf4ea947de6f601b")?
            .recover_pub_key(&EthHash::personal_message("kot"))?
            .to_eth_address().to_hex_string(),
            "c79c9d10d504f33c3def67d4284c10ec3691593d");
        //
        let message = "RegisterToVote\nContract: aea5db67524e02a263b9339fe6667d6b577f3d4c 1\nAddress: a7dab260472557b5eef526589a4f37a0f5f81569";
        assert_eq!(RecoverableSignature::from_hex("0x174ddb3fccb6009e13a1e6ad938b7704cfc9eae72f54579309e88f44242fa723011a6f61cb3be705448a5a716a4ccad5ef534d5b399f4e4cee34444ef645ada81c")?
            .recover_pub_key(&EthHash::personal_message(message))?
            .to_eth_address().to_hex_string(),
        "c79c9d10d504f33c3def67d4284c10ec3691593d");
        Ok(())
    }

    #[test]
    fn recover_test() -> Result<(), Box<dyn std::error::Error>> {
        let sender = EthAddress::from_hex("c79c9d10d504f33c3def67d4284c10ec3691593d")?;
        let signature = RecoverableSignature::from_hex("cb048ee6660c407395aa0df8512cb6e8f07a8a1af8dc980c594fbd56d451414024306c2984e40f8395f900e4c1ae1c7b660d1b4dfc17684e4831086eb0ab6c351b")?;

        let contract = EthAddress::from_hex("aea5db67524e02a263b9339fe6667d6b577f3d4c")?;
        let voting_id = "1";
        let message_hash = EthHash::new("SgxVotingTicket(address,bytes,address)")
            .add(contract)
            .add(voting_id)
            .add(&sender)
            .build();
        eprintln!("hash= {:x}", &message_hash);
        eprintln!(
            "empty= {:x}",
            &EthHash::new("SgxVotingTicket(address,bytes,address)").build()
        );
        assert_eq!(
            signature.recover_pub_key(&message_hash)?.to_eth_address(),
            EthAddress::from_hex("0440e6762cb37ba01b2f39336f4d1a05399367e1")?
        );
        Ok(())
    }

    #[derive(Default)]
    struct FakeDigest(Vec<u8>);

    impl Digest for FakeDigest {
        type OutputSize = <Sha256 as Digest>::OutputSize;

        fn new() -> Self {
            FakeDigest(Vec::new())
        }

        fn input<B: AsRef<[u8]>>(&mut self, data: B) {
            self.0.extend_from_slice(data.as_ref())
        }

        fn chain<B: AsRef<[u8]>>(mut self, data: B) -> Self
        where
            Self: Sized,
        {
            self.0.extend_from_slice(data.as_ref());
            self
        }

        fn result(self) -> GenericArray<u8, Self::OutputSize> {
            println!("H: {}", hex::encode(&self.0));
            GenericArray::default()
        }

        fn result_reset(&mut self) -> GenericArray<u8, Self::OutputSize> {
            println!("H: {}", hex::encode(&self.0));
            GenericArray::default()
        }

        fn reset(&mut self) {
            unimplemented!()
        }

        fn output_size() -> usize {
            Sha256::output_size()
        }

        fn digest(data: &[u8]) -> GenericArray<u8, Self::OutputSize> {
            unimplemented!()
        }
    }

    #[test]
    fn test_shared_secret() -> Result<(), Box<dyn std::error::Error>> {
        let secret1 = SecretKey::parse_slice(&hex::decode(
            "361577b05f29be9253e775cb6f332a75d4d5fa311856cad9c04585d687a6625a",
        )?)?;
        let public1 = PublicKey::parse_slice(&hex::decode("040ddcd46801ea93b00b422324d6c217883d6f62155da92a3efdd9e50d06f848cf6f5fffb582ab7528b0481b25ece2e5b150b4d6c9099097d7878be94fcbdf600d")?, None)?;
        let address1 = "0a2f48cafc23acffb02218a30a25298ceace99ac";
        let shared1 =
            hex::decode("3f9e53507881a7e80e16e110ad5e3d091e582da3cd4b8425bf09acc2d5275c05")?;
        println!("a");

        println!("b");
        let secret2 = SecretKey::parse_slice(&hex::decode(
            "480540ef5faf644fe69c8ce0024d57f0dab3a55f4aeb97c0cbb8d4186aee28d9",
        )?)?;
        let public2 = PublicKey::parse_slice(&hex::decode("0496cdf0cc5f16bb357e33d8f750e5407ad39a7a4d5cecc5d4d69775143ee9b42c99d34e5c11c65bbc6874e43cba7a88cec7bffdebcd0bd6b66b7d9cb101a20195")?, None)?;
        let address2 = "61e8272ee4b61458bc3c1fe4c2d2829b8cbf2766";
        let shared2 = hex::decode("01000680087000000130005919000995468608014047")?;

        assert_eq!(PublicKey::from_secret_key(&secret1), public1);
        assert_eq!(PublicKey::from_secret_key(&secret2), public2);
        assert_eq!(secret1.to_eth_address().to_hex_string(), address1);
        assert_eq!(secret2.to_eth_address().to_hex_string(), address2);

        let ss = secp256k1::SharedSecret::<Sha256>::new(&public1, &secret2)?;
        eprintln!("p1s2 {}", hex::encode(ss.as_ref()));
        let ss = secp256k1::SharedSecret::<Sha256>::new(&public2, &secret1)?;
        eprintln!("p2a1 {}", hex::encode(ss.as_ref()));
        eprintln!(
            "hhhh {:x}",
            Sha256::digest(&hex::decode(
                "0250a33eb53df82e355261930ca892e1107b43d5c650389a62bae1f572b9b512e8"
            )?)
        );

        let sx = secp256k1::SharedSecret::<FakeDigest>::new(&public1, &secret2)?;

        let h = Sha256::digest(shared1.as_ref());

        Ok(())
    }
}
