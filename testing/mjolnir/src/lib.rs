extern crate cfg_if;
extern crate wasm_bindgen;

mod utils;

use cfg_if::cfg_if;
use chain_addr as addr;
use chain_impl_mockchain::key;
use chain_impl_mockchain::transaction as tx;
use rand::Rng;
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
#[derive(Debug)]
pub struct PrivateKey(key::PrivateKey);

#[wasm_bindgen]
/// Private key.
impl PrivateKey {
    /// Generate a new private key.
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        PrivateKey(key::PrivateKey::normalize_bytes(bytes))
    }

    /// Convert private key to hex.
    pub fn to_hex(&self) -> String {
        use cardano::util::hex::encode;
        encode(self.0.as_ref())
    }

    /// Read private key form hex.
    pub fn from_hex(input: &str) -> Result<PrivateKey, JsValue> {
        key::PrivateKey::from_hex(input)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .map(PrivateKey)
    }

    /// Extract public key.
    pub fn public(&self) -> PublicKey {
        PublicKey(self.0.public())
    }
}

#[wasm_bindgen]
pub struct PublicKey(key::PublicKey);

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
        key::PublicKey::from_hex(input)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
            .map(PublicKey)
    }

    /// Get address.
    pub fn address(&self) -> Address {
        Address(addr::Address(
            addr::Discrimination::Test,
            addr::Kind::Single((self.0).0),
        ))
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct Address(addr::Address);

#[wasm_bindgen]
#[derive(Debug)]
pub struct UtxoPointer(tx::UtxoPointer);

#[wasm_bindgen]
impl UtxoPointer {
    pub fn new(tx_id: TransactionId, output_index: u32, value: u64) -> UtxoPointer {
        UtxoPointer(tx::UtxoPointer {
            transaction_id: tx_id.0,
            output_index,
            value: tx::Value(value),
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
        self.0.outputs.push(tx::Output(address.0, tx::Value(value)))
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
pub struct Output(tx::Output);

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, mjolnir!");
}

#[test]
fn build_transaction_case() {
    // make private keys.
    let priv0 =
        PrivateKey::from_hex("A012881282b611ad8ce4fbf833831eeafea85f474e0b4d5bcaccf84749555459")
            .unwrap();
    let priv1 =
        PrivateKey::from_hex("8012881282b611ad8ce4fbf833831eeafea85f474e0b4d5bcaccf84749555459")
            .unwrap();
    // make public keys.
    let pub0 = priv0.public();
    let pub1 = priv1.public();
    // make addresses.
    //let addr0 = pub0.address();
    let addr1 = pub1.address();
    // transaction id
    let txid1 = TransactionId::from_bytes(&[0]);
    assert_eq!("TransactionId(Hash(Blake2b256(0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314)))"
              , format!("{:?}", txid1));

    // build transaction
    let utx0 = UtxoPointer::new(txid1.clone(), 0, 10);
    let utx1 = UtxoPointer::new(txid1.clone(), 1, 20);
    let mut tx = TransactionBuilder::new();
    tx.add_input(utx0);
    tx.add_input(utx1);
    tx.add_output(addr1, 30);
    let mut txf = tx.finalize();
    assert_eq!("TransactionFinalizer(SignedTransaction { transaction: Transaction { inputs: [UtxoPointer { transaction_id: Hash(Blake2b256(0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314)), output_index: 0, value: Value(10) }, UtxoPointer { transaction_id: Hash(Blake2b256(0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314)), output_index: 1, value: Value(20) }], outputs: [Output(Address(Test, Single(08fdb9dbf6daec725179fbd0c9da1f6b88e758221af4c6319b17211907eddac3)), Value(30))] }, witnesses: [] })", format!("{:?}", txf));
    txf.sign(&priv0);
    txf.sign(&priv1);
    let signed_tx = txf.build();
    assert_eq!("SignedTransaction(SignedTransaction { transaction: Transaction { inputs: [UtxoPointer { transaction_id: Hash(Blake2b256(0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314)), output_index: 0, value: Value(10) }, UtxoPointer { transaction_id: Hash(Blake2b256(0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314)), output_index: 1, value: Value(20) }], outputs: [Output(Address(Test, Single(08fdb9dbf6daec725179fbd0c9da1f6b88e758221af4c6319b17211907eddac3)), Value(30))] }, witnesses: [Witness(Signature(ff16ae19b0419f7225328f094ee413a59fa8875c3037ecbeb9f317bdbbeb45b2cf111fc7424e6b7d3fd172e96da4f2e929b70955d907daeae3c814e135903c03)), Witness(Signature(fc7270e8b213543db085ab644e10f7a68ac94232b91b4cc23bad45c2159763a25eb0a67a65636ef590d31bcec5e90be09174d4c8d67e2adafb6523f003eb2f06))] })", format!("{:?}", signed_tx));
}
