use assert_fs::{fixture::PathChild, TempDir};
use chain_core::property::BlockDate as _;
use chain_impl_mockchain::{
    header::BlockDate,
    testing::VoteTestGen,
    tokens::{identifier::TokenIdentifier as TokenIdentifierLib, minting_policy::MintingPolicy},
    vote::Choice,
};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Starter},
    testing::{time::wait_for_epoch, VotePlanBuilder},
};
use jormungandr_lib::interfaces::{InitialToken, Tally, TallyResult, VotePlanStatus};
use std::{collections::BTreeSet, path::Path, str::FromStr};
use thor::{vote_plan_cert, FragmentSender, FragmentSenderSetup, Wallet};

const INITIAL_FUND_PER_WALLET: u64 = 1_000_000;
const SLOTS_PER_EPOCH: u32 = 10;
const SLOT_DURATION: u8 = 4;

#[test]
pub fn merge_two_voteplans() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = Wallet::default();
    let mut bob = Wallet::default();

    let minting_policy = MintingPolicy::new();

    let first_token = TokenIdentifierLib::from_str(
        "00000000000000000000000000000000000000000000000000000000.00000000",
    )
    .unwrap();

    let second_token = TokenIdentifierLib::from_str(
        "00000000000000000000000000000000000000000000000000000000.00000001",
    )
    .unwrap();

    let proposal_external_id = VoteTestGen::external_proposal_id();
    let vote_plan_statuses_file = temp_dir.child("vote_plan_status.json");

    let first_vote_plan = VotePlanBuilder::new()
        .proposals_external_ids(vec![proposal_external_id.clone()])
        .vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .tally_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_end(BlockDate::from_epoch_slot_id(2, 0))
        .voting_token(first_token.clone())
        .build();

    let second_vote_plan = VotePlanBuilder::new()
        .proposals_external_ids(vec![proposal_external_id.clone()])
        .vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .tally_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_end(BlockDate::from_epoch_slot_id(2, 0))
        .voting_token(second_token.clone())
        .build();

    let vote_plan_certs: Vec<_> = vec![first_vote_plan.clone(), second_vote_plan.clone()]
        .iter()
        .map(|vp| {
            vote_plan_cert(
                &alice,
                chain_impl_mockchain::block::BlockDate {
                    epoch: 1,
                    slot_id: 0,
                },
                vp,
            )
            .into()
        })
        .collect();

    let wallets = [&alice, &bob];

    let config = ConfigurationBuilder::new()
        .with_funds(
            wallets
                .iter()
                .map(|x| x.to_initial_fund(INITIAL_FUND_PER_WALLET))
                .collect(),
        )
        .with_token(InitialToken {
            token_id: first_token.into(),
            policy: minting_policy.clone().into(),
            to: vec![alice.to_initial_token(INITIAL_FUND_PER_WALLET)],
        })
        .with_token(InitialToken {
            token_id: second_token.into(),
            policy: minting_policy.into(),
            to: vec![bob.to_initial_token(INITIAL_FUND_PER_WALLET)],
        })
        .with_committees(&[alice.to_committee_id()])
        .with_slots_per_epoch(SLOTS_PER_EPOCH)
        .with_certs(vote_plan_certs)
        .with_slot_duration(SLOT_DURATION)
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        chain_impl_mockchain::block::BlockDate::first()
            .next_epoch()
            .into(),
        FragmentSenderSetup::resend_3_times(),
    );

    transaction_sender
        .send_vote_cast(
            &mut alice,
            &first_vote_plan,
            0,
            &Choice::new(0),
            &jormungandr,
        )
        .unwrap();

    transaction_sender
        .send_vote_cast(
            &mut bob,
            &second_vote_plan,
            0,
            &Choice::new(0),
            &jormungandr,
        )
        .unwrap();

    wait_for_epoch(1, jormungandr.rest());

    let transaction_sender =
        transaction_sender.set_valid_until(chain_impl_mockchain::block::BlockDate {
            epoch: 3,
            slot_id: 0,
        });

    let vote_plan_statuses = jormungandr.rest().vote_plan_statuses().unwrap();

    transaction_sender
        .send_public_vote_tally(&mut alice, &first_vote_plan, &jormungandr)
        .unwrap();

    transaction_sender
        .send_public_vote_tally(&mut alice, &second_vote_plan, &jormungandr)
        .unwrap();

    wait_for_epoch(2, jormungandr.rest());

    write_vote_plan_statuses(vote_plan_statuses, vote_plan_statuses_file.path());

    let merged_vote_plans = JCli::default()
        .votes()
        .tally()
        .merge_results(vote_plan_statuses_file.path())
        .unwrap();

    let merged_vote_plan = merged_vote_plans.get(0).unwrap();
    let mut ids: BTreeSet<jormungandr_lib::crypto::hash::Hash> = BTreeSet::new();
    ids.insert(first_vote_plan.to_id().into());
    ids.insert(second_vote_plan.to_id().into());

    assert_eq!(merged_vote_plan.ids, ids);

    let merged_proposal = merged_vote_plan.proposals.get(0).unwrap();
    assert_eq!(merged_proposal.proposal_id, proposal_external_id.into());
    assert_eq!(merged_proposal.votes_cast, 2);
    assert_eq!(
        merged_proposal.tally,
        Tally::Public {
            result: TallyResult {
                options: 0..3,
                results: vec![2000000, 0, 0]
            }
        }
    );
}

pub fn write_vote_plan_statuses<P: AsRef<Path>>(vote_plan_statuses: Vec<VotePlanStatus>, path: P) {
    use std::io::Write;
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(
        serde_json::to_string(&vote_plan_statuses)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
}
