extern crate cfg_if;
extern crate wasm_bindgen;

mod utils;

use cardano::util::hex;
use cfg_if::cfg_if;
use chain_addr as addr;
use chain_core::property::FromStr;
use chain_core::property::Serialize;
use chain_crypto::{algorithms::Ed25519Extended, SecretKey};
use chain_impl_mockchain::fee;
use chain_impl_mockchain::key;
use chain_impl_mockchain::transaction as tx;
use chain_impl_mockchain::txbuilder as tb;
use chain_impl_mockchain::value;
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

    /// Read private key from hex representation.
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

    //TODO: introduce from bench32 representation.

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

    // TODO introduce from bench

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
pub struct Input(tx::Input);

#[wasm_bindgen]
impl Input {
    pub fn from_utxo(utxo_pointer: UtxoPointer) -> Self {
        Input(tx::Input::from_utxo(utxo_pointer.0))
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct UtxoPointer(tx::UtxoPointer);

#[wasm_bindgen]
impl UtxoPointer {
    pub fn new(tx_id: TransactionId, output_index: u8, value: u64) -> UtxoPointer {
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
pub struct Transaction(tx::Transaction<addr::Address>);

#[wasm_bindgen]
pub struct TransactionBuilder(tb::TransactionBuilder<addr::Address>);

#[wasm_bindgen]
pub struct Fee(FeeVariant);

impl Fee {
    pub fn linear_fee(a: u64, b: u64) -> Fee {
        Fee(FeeVariant::Linear(fee::LinearFee::new(a, b, 0)))
    }
}

pub enum FeeVariant {
    Linear(fee::LinearFee),
}

#[wasm_bindgen]
pub struct OutputPolicy(tb::OutputPolicy);

#[wasm_bindgen]
impl OutputPolicy {
    pub fn one(address: Address) -> Self {
        OutputPolicy(tb::OutputPolicy::One(address.0))
    }

    pub fn forget() -> Self {
        OutputPolicy(tb::OutputPolicy::Forget)
    }
}

#[wasm_bindgen]
pub struct Balance(tb::Balance);

#[wasm_bindgen]
pub struct Value(value::Value);

#[wasm_bindgen]
pub struct FinalizationResult {
    balance: Balance,
    result: TransactionFinalizer,
}

#[wasm_bindgen]
impl FinalizationResult {
    pub fn get_balance(self) -> Balance {
        self.balance // TODO: fixme
    }

    pub fn get_result(self) -> TransactionFinalizer {
        self.result
    }
}

#[wasm_bindgen]
impl TransactionBuilder {
    pub fn new() -> TransactionBuilder {
        TransactionBuilder(tb::TransactionBuilder::new())
    }

    pub fn add_input(&mut self, input: &Input) {
        self.0.add_input(&input.0)
    }

    pub fn add_output(&mut self, address: Address, value: u64) {
        self.0.add_output(address.0, value::Value(value))
    }

    pub fn get_balance(&mut self, fee_algorithm: &Fee) -> Result<Balance, JsValue> {
        match fee_algorithm.0 {
            FeeVariant::Linear(linear) => self
                .0
                .get_balance(linear)
                .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
                .map(Balance),
        }
    }

    pub fn estimate_fee(&mut self, fee_algorithm: &Fee) -> Result<Value, JsValue> {
        match fee_algorithm.0 {
            FeeVariant::Linear(linear) => self
                .0
                .estimate_fee(linear)
                .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
                .map(Value),
        }
    }

    pub fn finalize(
        self,
        fee_algorithm: &Fee,
        policy: OutputPolicy,
    ) -> Result<FinalizationResult, JsValue> {
        match fee_algorithm.0 {
            FeeVariant::Linear(linear) => self
                .0
                .finalize(linear, policy.0)
                .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
                .map(|(balance, result)| FinalizationResult {
                    balance: Balance(balance),
                    result: TransactionFinalizer(result),
                }),
        }
    }
}

#[wasm_bindgen]
pub struct TransactionFinalizer(tb::TransactionFinalizer);

#[wasm_bindgen]
impl TransactionFinalizer {
    pub fn sign(&mut self, pk: &PrivateKey) {
        self.0.sign(&pk.0)
    }

    pub fn build(self) -> SignedTransaction {
        match self.0.build() {
            tb::GeneratedTransaction::Type1(tx) => SignedTransaction(tx),
            tb::GeneratedTransaction::Type2(_) => unimplemented!(),
        }
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct SignedTransaction(tx::SignedTransaction<addr::Address>);

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
