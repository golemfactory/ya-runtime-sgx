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
        keccak.update(&bytes[1..]);
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

#[cfg(test)]
mod test {
    use secp256k1::{recover, verify, Message, PublicKey, PublicKeyFormat, RecoveryId, Signature};
    use std::error::Error;
    use tiny_keccak::{Hasher, Keccak, Sha3};

    #[test]
    fn test_recover() {
        let prefix = b"\x19Ethereum Signed Message:\n3kot";

        let mut hash = Keccak::v256();
        hash.update(prefix.as_ref());
        let mut hash_bytes = [0u8; 32];
        hash.finalize(&mut hash_bytes);

        //let signature = "929a3d941a05e84e18e531912c930f556cf61b497a9a947ef8ebe29334cb709c6bc0d847514bc6e3f66ea5a3da8636b110863f9b5d49923b65d7a141d51db4901b";
        let signature = "87aa6e272316599a56644df843cf9ecbb681750a2afe31a8750cdb1821c257035721b20e1b170f2f7b31ad16d0f3436706bf6669347791c8afdf4ea947de6f601b";
        let signature_bytes = hex::decode(signature).unwrap();
        let message = Message::parse(&hash_bytes);
        let message_sign = Signature::parse_slice(&signature_bytes[..64]).unwrap();
        /*function calculateSigRecovery(v: number, chainId?: number): number {
            return chainId ? v - (2 * chainId + 35) : v - 27
        }*/
        let recover_id = RecoveryId::parse_rpc(signature_bytes[64]).unwrap();
        let pub_key = recover(&message, &message_sign, &recover_id).unwrap();
        let b = pub_key.serialize();
        eprintln!("pub_key: {}", hex::encode(b.as_ref()));
        let b = pub_key.serialize_compressed();
        eprintln!("pub_key: {}", hex::encode(b.as_ref()));

        eprintln!("v: {}", verify(&message, &message_sign, &pub_key));
        eprintln!("pub_key = {:?}", pub_key);
        let address = pub_to_hex2(&pub_key);
        assert_eq!(address, "c79c9d10d504f33c3def67d4284c10ec3691593d");

        pub fn pub_to_hex2(pub_key: &PublicKey) -> String {
            let bytes = pub_key.serialize();
            let mut keccak = Keccak::v256();
            let mut result = [0u8; 32];
            keccak.update(&bytes[1..]);
            keccak.finalize(&mut result);
            hex::encode(&result[12..])
        }
    }

    fn pub_to_hex(bytes: &[u8]) -> String {
        let mut keccak = Keccak::v256();
        let mut result = [0u8; 32];
        keccak.update(&bytes);
        keccak.finalize(&mut result);
        hex::encode(&result[12..])
    }

    pub fn pub_to_hex2(pub_key: &PublicKey) -> String {
        let bytes = pub_key.serialize();
        let mut keccak = Keccak::v256();
        let mut result = [0u8; 32];
        keccak.update(&bytes[1..]);
        keccak.finalize(&mut result);
        hex::encode(&result[12..])
    }

    fn with_tag(tag: u8, bytes: &[u8]) -> [u8; 33] {
        let mut output = [0; 33];
        output[0] = tag;
        (&mut output[1..]).copy_from_slice(bytes);
        eprintln!("tag={:?}", output.as_ref());
        output
    }

    #[test]
    fn test_addr() {
        let signature = "0xc6f0fc349b81ffe33589db56d67ff88159babd6d492ccc7e92e599c14a7e85bd029514024977d51094206f8173120300e7c1bb99d3ea7aade8d6616a241af6051c";
        let address = "0xc79c9d10d504f33c3def67d4284c10ec3691593d";
        let pub_key = base64::decode("HSR42sLQp5qUmGqMgw7LTDSxuNDCWcYq8ISFPRSDu2Y=").unwrap();
        println!("{}", hex::encode(&pub_key));
        //let pk = PublicKey::parse_compressed(&with_tag(4, &pub_key)).unwrap();
        eprintln!("len={} addre={}", pub_key.len(), pub_to_hex(&pub_key));

        /*
                RegisterToVote
        Contract: aea5db67524e02a263b9339fe6667d6b577f3d4c 0
        Address: eef21c55ed403689fa93fa407fc1e122e91df519
                 */

        //let signature = hex::decode(&signature[2..]).unwrap();
        //let pub_key: PublicKey = pub_key.parse().unwrap();
        /*{
            let mut pub_key = Vec::new();
            pub_key.push(0x1);
            let bytes = base64::decode_config(&pub_key, base64::BINHEX).unwrap();
            pub_key.extend(&bytes);
        }*/
        /*
        let mut bytes = [0; 33];
        //bytes.copy_from_slice(&pub_key);
        //let pub_key: PublicKey = pub_key.parse().unwrap();
        let pub_key_addr = {
            let bytes = pub_key.serialize();
            let mut keccak = Keccak::v256();
            let mut result = [0u8; 32];
            keccak.update(&bytes);
            keccak.finalize(&mut result);
            hex::encode(&result[12..])
        };*/
    }

    /*
        {
        "blockHash": "0x6bfe17e25aac566660d532b64d7c933eca236099da4201fdb01f8afeb1e61229",
        "blockNumber": "0x6e2497",
        "from": "0xc79c9d10d504f33c3def67d4284c10ec3691593d",
        "gas": "0x5208",
        "gasPrice": "0x2b8a118e00",
        "hash": "0xd33865cb8acad05d705f5df1fb016c761eb809dca91bf0b4f22f15be2ccfda61",
        "input": "0x",
        "nonce": "0x1",
        "r": "0xf6e20660b9e2e2cd22757702cd1ed86cba99b79d8daa829ce4279adaa98272b3",
        "s": "0x1791a3fb52322f5583ed38306ce29fb690228a95740104f54199a01ea45d1a52",
        "to": "0x8e423850a3b37ecfc17c0ca55a2da11dc1e45df9",
        "transactionIndex": "0x0",
        "v": "0x2b",
        "value": "0x38d7ea4c68000"
    }
         */

    fn recover_pub_key(hash: &str, r: &str, s: &str, v: u8) -> Result<PublicKey, Box<dyn Error>> {
        let hash = hex::decode(hash).unwrap();
        let r = hex::decode(r).unwrap();
        let s = hex::decode(s).unwrap();
        let mut bytes = Vec::new();
        bytes.extend(&r);
        bytes.extend(&s);
        let s = Signature::parse_slice(&bytes)?;
        let pk = recover(&Message::parse_slice(&hash)?, &s, &RecoveryId::parse(v)?)?;
        Ok(pk)
    }

    #[test]
    fn test2() -> Result<(), Box<dyn Error>> {
        let pk1 = recover_pub_key(
            "e2fe6d74cbfb9677e4af5417d7d3705cd8cac2f721c0e9d113b2ab5900f64fb1",
            "ce4d8e920d3f7b5531d801fa2b3e17b95c6e9bc874e75f42f7ed12cc6ebfa34a",
            "32432228fbd307b9cbc51bc5e7a2604562c572a8e832bd189c5404bc1c8ca7bd",
            1,
        )?;
        let pk2 = recover_pub_key(
            "d33865cb8acad05d705f5df1fb016c761eb809dca91bf0b4f22f15be2ccfda61",
            "f6e20660b9e2e2cd22757702cd1ed86cba99b79d8daa829ce4279adaa98272b3",
            "1791a3fb52322f5583ed38306ce29fb690228a95740104f54199a01ea45d1a52",
            1,
        )?;
        assert_eq!(pk1.serialize().as_ref(), pk2.serialize().as_ref());
        Ok(())
    }

    #[test]
    fn test3() -> Result<(), Box<dyn Error>> {
        fn pub_to_hex(bytes: &[u8]) -> String {
            let mut keccak = Keccak::v256();
            let mut result = [0u8; 32];
            keccak.update(&bytes);
            keccak.finalize(&mut result);
            hex::encode(&result[12..])
        }

        let bytes = hex::decode("9984d59017a9537632d911726e98ae0b89583b3be9f55d37bdb465e8d68bb7f1340898bdefa8b8733a77b590e8530cd0f76e5b043a337e195c08897884dd0355")?;
        assert_eq!(
            "c79c9d10d504f33c3def67d4284c10ec3691593d",
            pub_to_hex(&bytes)
        );
        let pk = PublicKey::parse_slice(&bytes, Some(PublicKeyFormat::Raw))?;
        let xbytes = pk.serialize();
        assert_eq!(
            "c79c9d10d504f33c3def67d4284c10ec3691593d",
            pub_to_hex(&xbytes[1..])
        );
        Ok(())
    }
}
