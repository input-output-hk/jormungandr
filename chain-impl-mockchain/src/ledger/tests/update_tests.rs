use crate::{
    block::Block,
    fragment::Contents,
    header::{BlockVersion, HeaderId, HeaderBuilderNew},
    date::BlockDate,
    ledger::ledger::Ledger,
    testing::arbitrary::update_proposal::UpdateProposalData,
    testing::ledger as mock_ledger,
};
use chain_core::property::ChainLength as ChainLengthProperty;
use chain_crypto::{Ed25519, SecretKey};
use quickcheck::{TestResult};
use quickcheck_macros::quickcheck;

#[quickcheck]
pub fn ledger_adopt_settings_from_update_proposal(
    update_proposal_data: UpdateProposalData,
) -> TestResult {
    let config = mock_ledger::ConfigBuilder::new()
        .with_leaders(&update_proposal_data.leaders_ids())
        .build();

    let (block0_hash, mut ledger) = mock_ledger::create_initial_fake_ledger(&[], config).unwrap();

    // apply proposal
    let date = ledger.date();
    ledger = ledger
        .apply_update_proposal(
            update_proposal_data.proposal_id,
            &update_proposal_data.proposal,
            date,
        )
        .unwrap();

    // apply votes
    for vote in update_proposal_data.votes.iter() {
        ledger = ledger.apply_update_vote(&vote).unwrap();
    }

    // trigger proposal process (build block)
    let block = build_block(
        &ledger,
        block0_hash,
        date.next_epoch(),
        &update_proposal_data.block_signing_key,
    );
    let header_meta = block.header.to_content_eval_context();
    ledger = ledger
        .apply_block(
            &ledger.get_ledger_parameters(),
            block.contents.iter(),
            &header_meta,
        )
        .unwrap();

    // assert
    let actual_params = ledger.settings.to_config_params();
    let expected_params = update_proposal_data.proposal_settings();

    let mut all_settings_equal = true;
    for expected_param in expected_params.iter() {
        if !actual_params.iter().any(|x| x == expected_param) {
            all_settings_equal = false;
            break;
        }
    }

    if !ledger.updates.proposals.is_empty() {
        return TestResult::error(format!("Error: proposal collection should be empty but contains:{:?}",
                                ledger.updates.proposals));
    }

    match all_settings_equal {
            false => TestResult::error(format!("Error: proposed update reached required votes, but proposal was NOT updated, Expected: {:?} vs Actual: {:?}",
                                expected_params,actual_params)),
            true => TestResult::passed(),
        }
}

fn build_block(
    ledger: &Ledger,
    block0_hash: HeaderId,
    date: BlockDate,
    block_signing_key: &SecretKey<Ed25519>,
) -> Block {
    let contents = Contents::empty();
    let header = HeaderBuilderNew::new(BlockVersion::Ed25519Signed, &contents)
        .set_parent(&block0_hash, ledger.chain_length.next())
        .set_date(date.next_epoch())
        .to_bft_builder()
        .unwrap()
        .sign_using(block_signing_key)
        .generalize();
    Block { header, contents }
}
