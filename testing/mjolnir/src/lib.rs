extern crate cfg_if;
extern crate wasm_bindgen;
#[macro_use]
extern crate serde_derive;

mod utils;

use bech32::{Bech32, FromBase32};
use cardano::util::hex;
use cfg_if::cfg_if;
use chain_addr as addr;
use chain_core::{
    mempack::{ReadBuf, Readable},
    property::{FromStr},
};
use chain_crypto::{self as crypto, algorithms::Ed25519Extended, AsymmetricKey, SecretKey};
use chain_impl_mockchain::account;
use chain_impl_mockchain::fee;
use chain_impl_mockchain::key;
use chain_impl_mockchain::transaction as tx;
use chain_impl_mockchain::txbuilder as tb;
use chain_impl_mockchain::value;
use chain_impl_mockchain::block::message as msg;
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

    pub fn from_bench32(input: &str) -> Result<PrivateKey, JsValue> {
        let bech32: Bech32 = input
            .trim()
            .parse()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        if bech32.hrp() != Ed25519Extended::SECRET_BECH32_HRP {
            return Err(JsValue::from_str(
                "Private key should contain Ed25519 extended private key",
            ));
        }
        let bytes = Vec::<u8>::from_base32(bech32.data())
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        let private_key =
            SecretKey::from_bytes(&bytes).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        Ok(PrivateKey(private_key))
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
        decode(input)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .and_then(|bytes| {
                let mut reader = ReadBuf::from(&bytes);
                key::deserialize_public_key(&mut reader)
                    .map(PublicKey)
                    .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            })
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
    pub fn from_utxo(utxo_pointer: &UtxoPointer) -> Self {
        Input(tx::Input::from_utxo(utxo_pointer.0.clone()))
    }

    pub fn from_account(account: &Account, v: u64) -> Self {
        Input(tx::Input::from_account(
            account.account.clone(),
            value::Value(v),
        ))
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
pub struct Transaction(tx::Transaction<addr::Address, tx::NoExtra>);

#[wasm_bindgen]
pub struct TransactionBuilder(tb::TransactionBuilder<addr::Address, tx::NoExtra>);

#[wasm_bindgen]
pub struct Fee(FeeVariant);

#[wasm_bindgen]
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
impl Balance {
    pub fn get_sign(&self) -> JsValue {
        JsValue::from_str(match self.0 {
            tb::Balance::Positive(_) => "positive",
            tb::Balance::Negative(_) => "negative",
            tb::Balance::Zero => "zero",
        })
    }

    pub fn get_value(&self) -> Value {
        match self.0 {
            tb::Balance::Positive(v) => Value(v),
            tb::Balance::Negative(v) => Value(v),
            tb::Balance::Zero => Value(value::Value(0)),
        }
    }
}

#[wasm_bindgen]
pub struct Value(value::Value);

#[wasm_bindgen]
impl Value {
    pub fn as_u64(&self) -> u64 {
        (self.0).0
    }
}

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
pub struct Account {
    account: account::Identifier,
    public: crypto::PublicKey<Ed25519Extended>,
}

#[wasm_bindgen]
impl Account {
    /// From bench32.
    pub fn from_bech32(input: String) -> Result<Account, JsValue> {
        let bech32: Bech32 = input
            .trim()
            .parse()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        let bytes = Vec::<u8>::from_base32(bech32.data())
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        crypto::PublicKey::<Ed25519Extended>::from_bytes(&bytes)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .map(|x| Account {
                account: account::Identifier::from(x.clone()),
                public: x,
            })
    }

    /// Make an account from account public key.
    pub fn from_public(input: &PublicKey) -> Account {
        Account {
            account: account::Identifier::from(input.0.clone()),
            public: input.0.clone(),
        }
    }

    pub fn to_bech32(&self) -> Result<String, JsValue> {
        let address = chain_addr::Address(
            addr::Discrimination::Test,
            addr::Kind::Account(self.public.clone()),
        );
        Ok(format!(
            "{}",
            addr::AddressReadable::from_address(&address).to_string()
        ))
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
                    result: TransactionFinalizer(tb::TransactionFinalizer::new_trans(result)),
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

    pub fn build(self) -> Message {
        match self.0.build() {
            tb::GeneratedTransaction::Type1(tx) => 
                Message(msg::Message::Transaction(tx)),
            tb::GeneratedTransaction::Type2(_) => unimplemented!(),
        }
    }
}

#[derive(Serialize)]
pub struct SignedTxDescription {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub witnesses: Vec<String>,
}

fn describe_tx(this: &tx::AuthenticatedTransaction<addr::Address,tx::NoExtra>) -> SignedTxDescription {
    SignedTxDescription {
        inputs: this
            .transaction
            .inputs
            .iter()
            .map(|i| format!("{:?}", i))
            .collect(),
        outputs: this 
            .transaction
            .outputs
            .iter()
            .map(|o| format!("{:?}", o))
            .collect(),
        witnesses: this
            .witnesses
            .iter()
            .map(|w| format!("{:?}", w))
            .collect(),
    }
}

#[derive(Serialize)]
pub struct MessageDescription {
    transaction: Option<SignedTxDescription>,
}

#[wasm_bindgen]
pub struct Message(msg::Message);

#[wasm_bindgen]
impl Message {

    /// Get internal type of the message
    pub fn get_type(&self) -> String {
        match self.0 {
            msg::Message::OldUtxoDeclaration(_) => "old",
            msg::Message::Transaction(_) => "tx",
            msg::Message::Certificate(_) => "cert",
            msg::Message::Update(_) => "update",
        }.to_string()
    }

    /// Convert all the data to hex.
    pub fn to_hex(&self) -> String {
        let bytes = self.0.to_raw();
        let bb = hex::encode(&bytes.as_ref());
        let bytes1 = hex::decode(&bb).unwrap();
        let mut reader = ReadBuf::from(&bytes1);
        msg::Message::read(&mut reader).unwrap();
        hex::encode(&bytes.as_ref())
    }

    /// Convert message from hex
    pub fn from_hex(input: &str) -> Result<Message, JsValue> {
        hex::decode(input)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .and_then(|bytes| {
                let mut reader = ReadBuf::from(&bytes);
                msg::Message::read(&mut reader)
                    .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
                    .map(Message)
            })
    }

    /// Describe existing message.
    pub fn describe(&self) -> JsValue {
        let msg = match &self.0 {
            msg::Message::Transaction(msg) => MessageDescription {
                transaction: Some(describe_tx(&msg)),
            },
            _ => unimplemented!()
        };
        JsValue::from_serde(&msg).unwrap()
    }

}
