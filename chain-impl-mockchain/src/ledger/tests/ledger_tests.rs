#![cfg(test)]

use crate::fee::FeeAlgorithm;
use crate::{
    config::ConfigParam,
    fragment::{config::ConfigParams, Fragment},
    ledger::{
        Ledger,
        ledger::{Block0Error,Error::{Block0,ExpectingInitialMessage}}
    },
    testing::{
        arbitrary::{
            AccountStatesVerifier, ArbitraryValidTransactionData, UtxoVerifier,
        },
        data::AddressDataValue,
        ledger::{ConfigBuilder, LedgerBuilder},
        builders::{OldAddressBuilder,TestTxBuilder},
        TestGen,
    },
};
use chain_addr::Discrimination;
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

#[quickcheck]
pub fn ledger_accepts_correct_transaction(
    faucet: AddressDataValue,
    receiver: AddressDataValue,
) -> TestResult {

    let mut ledger = LedgerBuilder::from_config(ConfigBuilder::new(0)).initial_fund(&faucet).build().unwrap();
    let fragment = TestTxBuilder::new(&ledger.block0_hash).move_funds(&mut ledger,&faucet,&receiver,&faucet.value).get_fragment();
    let total_funds_before = ledger.total_funds();
    let result = ledger.apply_transaction(fragment);

    if result.is_err() {
        return TestResult::error(format!("Error from ledger: {}", result.err().unwrap()));
    }
    let total_funds_after = ledger.total_funds();
    match total_funds_before == total_funds_after {
        false => TestResult::error(format!(
                "Total funds in ledger before and after transaction is not equal {} <> {} ",
                total_funds_before, total_funds_after
        )),
        true => TestResult::passed(),
    }
}

#[quickcheck]
pub fn total_funds_are_const_in_ledger(
    transaction_data: ArbitraryValidTransactionData,
) -> TestResult {

    let config = ConfigBuilder::new(0)
        .with_discrimination(Discrimination::Test)
        .with_fee(transaction_data.fee.clone());

    let mut ledger = LedgerBuilder::from_config(config).initial_funds(&transaction_data.addresses).build().unwrap();
    let signed_tx = TestTxBuilder::new(&ledger.block0_hash).move_funds_multiple(&mut ledger,&transaction_data.input_addresses,&transaction_data.output_addresses);
    let total_funds_before = ledger.total_funds();
    let result = ledger.apply_transaction(signed_tx.clone().get_fragment());

    if result.is_err() {
        return TestResult::error(format!("Error from ledger: {:?}", result.err()));
    }

    let total_funds_after = ledger.total_funds();
    let fee = transaction_data
                .fee
                .calculate_tx(&signed_tx.get_tx().as_slice());

    if total_funds_before != (total_funds_after + fee).unwrap() {
        return TestResult::error(format!(
            "Total funds in ledger before and (after transaction + fee) is not equal {} <> {} (fee: {:?})",
            total_funds_before, (total_funds_after + fee).unwrap(),transaction_data.fee
        ));
    }

            let utxo_verifier = UtxoVerifier::new(transaction_data.clone());
            let utxo_verification_result = utxo_verifier.verify(&ledger);
            if utxo_verification_result.is_err() {
                return TestResult::error(format!("{}", utxo_verification_result.err().unwrap()));
            }

            let account_state_verifier = AccountStatesVerifier::new(transaction_data.clone());
            let account_state_verification_result =
                account_state_verifier.verify(ledger.accounts());
            if account_state_verification_result.is_err() {
                return TestResult::error(format!(
                    "{}",
                    account_state_verification_result.err().unwrap()
                ));
            }
            TestResult::passed()

}

#[test]
pub fn test_first_initial_fragment_empty() {
    let header_id = TestGen::hash();
    let content = Vec::new();
    assert_eq!(Ledger::new(header_id,content).err().unwrap(),Block0 {
        source: Block0Error::InitialMessageMissing,
    });
}

#[test]
pub fn test_first_initial_fragment_wrong_type() {
    let header_id = TestGen::hash();
    let fragment = Fragment::OldUtxoDeclaration(OldAddressBuilder::build_utxo_declaration(Some(1)));
    assert_eq!(Ledger::new(header_id,&vec![fragment]).err().unwrap(),ExpectingInitialMessage); 
}

#[test]
pub fn ledger_new_no_block_start_time() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.leader_id));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KESUpdateSpeed(3600));

    assert_eq!(Ledger::new(header_id,vec![&Fragment::Initial(ie)]).err().unwrap(),
            Block0 {
                source: Block0Error::InitialMessageNoDate,
            });
}

#[test]
pub fn ledger_new_no_discrimination() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::AddBftLeader(leader_pair.leader_id));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KESUpdateSpeed(3600));

    assert_eq!(Ledger::new(header_id,vec![&Fragment::Initial(ie)]).err().unwrap(),
        Block0 {
            source: Block0Error::InitialMessageNoDiscrimination,
    });
}

#[test]
pub fn ledger_new_no_slot_duration() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.leader_id));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KESUpdateSpeed(3600));

    assert_eq!(Ledger::new(header_id,vec![&Fragment::Initial(ie)]).err().unwrap(),
        Block0 {
            source: Block0Error::InitialMessageNoSlotDuration,
    });
}

#[test]
pub fn ledger_new_no_slots_per_epoch() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.leader_id));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::KESUpdateSpeed(3600));

    assert_eq!(Ledger::new(header_id,vec![&Fragment::Initial(ie)]).err().unwrap(),
        Block0 {
            source: Block0Error::InitialMessageNoSlotsPerEpoch,
    });
}

#[test]
pub fn ledger_new_no_kes_update_speed() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.leader_id));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));

    assert_eq!(Ledger::new(header_id,vec![&Fragment::Initial(ie)]).err().unwrap(),
        Block0 {
            source: Block0Error::InitialMessageNoKesUpdateSpeed,
    });
}

#[test]
pub fn ledger_new_no_bft_leader() {
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KESUpdateSpeed(3600));

    assert_eq!(Ledger::new(header_id,vec![&Fragment::Initial(ie)]).err().unwrap(),
        Block0 {
            source: Block0Error::InitialMessageNoConsensusLeaderId,
    });
}