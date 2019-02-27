//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use mjolnir::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn pass() {
    assert_eq!(1 + 1, 2);
}

#[wasm_bindgen_test]
fn simple_case() {
    let input = "8012881282b611ad8ce4fbf833831eeafea85f474e0b4d5bcaccf84749555459";
    let x = PrivateKey::from_hex(input);;
    let pk = x.unwrap();
    assert_eq!(input, pk.to_hex());
    assert_eq!(
        "08fdb9dbf6daec725179fbd0c9da1f6b88e758221af4c6319b17211907eddac3",
        pk.public().to_hex()
    );
    assert_eq!("Address(Address(Test, Single(08fdb9dbf6daec725179fbd0c9da1f6b88e758221af4c6319b17211907eddac3)))", format!("{:?}",pk.public().address()));
}

#[wasm_bindgen_test]
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
