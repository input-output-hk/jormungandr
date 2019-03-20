extern crate cfg_if;
extern crate wasm_bindgen;

mod utils;

use cardano::util::hex;
use cfg_if::cfg_if;
use chain_addr as addr;
use chain_core::property::Serialize;
use chain_crypto::{algorithms::Ed25519Extended, SecretKey};
use chain_impl_mockchain::key;
use chain_impl_mockchain::transaction as tx;
use chain_impl_mockchain::value;
use std::str::FromStr;
use wasm_bindgen::prelude::*;


cfg_if! {
    // When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
    // allocator.
    if #[cfg(feature = "wee_alloc")] {
        extern crate wee_alloc;
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
    }
}

#[wasm_bindgen]
pub struct PrivateKey(key::SpendingSecretKey);

#[wasm_bindgen]
/// Private key.
impl PrivateKey {
    /// Generate a new private key.
    pub fn generate() -> Self {
        let rng = rand::thread_rng();
        PrivateKey(key::SpendingSecretKey::generate(rng))
    }

    /*
    /// Convert private key to hex.
    pub fn to_hex(&self) -> String {
        use cardano::util::hex::encode;
        encode((self.0).0.as_ref())
    } */

    /// Read private key form hex.
    pub fn from_hex(input: &str) -> Result<PrivateKey, JsValue> {
        use cardano::util::hex::decode;
        decode(input)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .and_then(|bytes| {
                SecretKey::<Ed25519Extended>::from_binary(&bytes)
                    .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
                    .map(PrivateKey)
            })
    }

    /// Extract public key.
    pub fn public(&self) -> PublicKey {
        PublicKey(self.0.to_public())
    }
}

#[wasm_bindgen]
pub struct PublicKey(key::SpendingPublicKey);

#[wasm_bindgen]
/// Public key wrapper.
impl PublicKey {
    /// Show public key as hex string.
    pub fn to_hex(&self) -> String {
        use cardano::util::hex::encode;
        encode(self.0.as_ref())
    }

    /// Read public key from hex string.
    pub fn from_hex(input: &str) -> Result<PublicKey, JsValue> {
        use cardano::util::hex::decode;
        let bytes = decode(input).unwrap();
        key::deserialize_public_key(std::io::Cursor::new(bytes))
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .map(PublicKey)
    }

    /// Get address.
    pub fn address(&self) -> Address {
        Address(addr::Address(
            addr::Discrimination::Test,
            addr::Kind::Single((self.0).clone()),
        ))
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct Address(addr::Address);

#[wasm_bindgen]
impl Address {
    pub fn from_hex(input: &str) -> Result<Address, JsValue> {
        let bytes = hex::decode(input).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        let address = addr::Address::from_bytes(&bytes)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        Ok(Address(address))
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0.to_bytes())
    }

    pub fn to_bech32(&self) -> String {
        addr::AddressReadable::from_address(&self.0)
            .as_string()
            .to_string()
    }

    pub fn from_bech32(input: String) -> Result<Address, JsValue> {
        let addr = addr::AddressReadable::from_string(&input)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        Ok(Address(addr.to_address()))
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct UtxoPointer(tx::UtxoPointer);

#[wasm_bindgen]
impl UtxoPointer {
    pub fn new(tx_id: TransactionId, output_index: u32, value: u64) -> UtxoPointer {
        UtxoPointer(tx::UtxoPointer {
            transaction_id: tx_id.0,
            output_index,
            value: value::Value(value),
        })
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct TransactionId(tx::TransactionId);

#[wasm_bindgen]
impl TransactionId {
    pub fn from_bytes(bytes: &[u8]) -> TransactionId {
        TransactionId(tx::TransactionId::hash_bytes(bytes))
    }

    pub fn from_hex(input: &str) -> Result<TransactionId, JsValue> {
        tx::TransactionId::from_str(input)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .map(TransactionId)
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct Transaction(tx::Transaction);

#[wasm_bindgen]
#[derive(Debug)]
pub struct TransactionBuilder(tx::Transaction);

#[wasm_bindgen]
impl TransactionBuilder {
    pub fn new() -> TransactionBuilder {
        TransactionBuilder(tx::Transaction {
            inputs: vec![],
            outputs: vec![],
        })
    }

    pub fn add_input(&mut self, utxo: UtxoPointer) {
        self.0.inputs.push(utxo.0)
    }

    pub fn add_output(&mut self, address: Address, value: u64) {
        self.0
            .outputs
            .push(tx::Output(address.0, value::Value(value)))
    }

    pub fn finalize(self) -> TransactionFinalizer {
        TransactionFinalizer(tx::SignedTransaction {
            transaction: self.0,
            witnesses: vec![],
        })
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct TransactionFinalizer(tx::SignedTransaction);

#[wasm_bindgen]
impl TransactionFinalizer {
    pub fn sign(&mut self, pk: &PrivateKey) {
        use chain_core::property::Transaction;
        let tx_id = self.0.transaction.id();
        let witness = tx::Witness::new(&tx_id, &pk.0);
        self.0.witnesses.push(witness);
    }
    pub fn build(self) -> SignedTransaction {
        SignedTransaction(self.0)
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct SignedTransaction(tx::SignedTransaction);

#[wasm_bindgen]
impl SignedTransaction {
    pub fn to_hex(self) -> Result<String, JsValue> {
        let v = self
            .0
            .serialize_as_vec()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        Ok(hex::encode(&v))
    }
}

#[wasm_bindgen]
pub struct Output(tx::Output);

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, mjolnir!");
}

