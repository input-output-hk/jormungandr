#![cfg(test)]
use crate::{
    accounting::account::LedgerError::ValueError,
    date::BlockDate,
    header::ChainLength,
    ledger::{ledger::Error::Account, Error as LedgerError},
    testing::{
        builders::{GenesisPraosBlockBuilder, TestTxBuilder},
        scenario::{prepare_scenario, wallet},
    },
    value::{Value, ValueError::NegativeAmount},
};

#[test]
pub fn apply_block_increases_leaders_log() {
    let (mut ledger, controller) = prepare_scenario()
        .with_initials(vec![wallet("Bob").with(1_000).owns("stake_pool")])
        .build()
        .unwrap();

    let stake_pool = controller.stake_pool("stake_pool").unwrap();
    let date = BlockDate {
        epoch: 1,
        slot_id: 0,
    };
    let block = GenesisPraosBlockBuilder::new()
        .with_date(date)
        .with_chain_length(ledger.chain_length())
        .with_parent_id(ledger.block0_hash)
        .build(&stake_pool, ledger.era());

    assert!(ledger.apply_block(block).is_ok());
    assert_eq!(
        ledger.leaders_log().total(),
        1,
        "record should be increased by 1"
    );
    assert!(
        ledger
            .leaders_log()
            .iter()
            .find(|x| *x.0 == stake_pool.id())
            .is_some(),
        "pool should appear in record"
    );
}

#[test]
pub fn apply_block_wrong_chain_length() {
    let (mut ledger, controller) = prepare_scenario()
        .with_initials(vec![wallet("Bob").with(1_000).owns("stake_pool")])
        .build()
        .unwrap();

    let stake_pool = controller.stake_pool("stake_pool").unwrap();
    let date = BlockDate {
        epoch: 0,
        slot_id: 1,
    };
    let block = GenesisPraosBlockBuilder::new()
        .with_date(date)
        .with_chain_length(ChainLength(10))
        .with_parent_id(ledger.block0_hash)
        .build(&stake_pool, ledger.era());

    assert_err!(
        LedgerError::WrongChainLength {
            actual: ChainLength(11),
            expected: ChainLength(1),
        },
        ledger.apply_block(block)
    );
}

#[test]
pub fn apply_block_wrong_date() {
    let (mut ledger, controller) = prepare_scenario()
        .with_initials(vec![wallet("Bob").with(1_000).owns("stake_pool")])
        .build()
        .unwrap();

    let stake_pool = controller.stake_pool("stake_pool").unwrap();
    let date = BlockDate {
        epoch: 0,
        slot_id: 0,
    };

    ledger.set_date(BlockDate {
        epoch: 0,
        slot_id: 3,
    });

    let block = GenesisPraosBlockBuilder::new()
        .with_date(date.clone())
        .with_chain_length(ChainLength(0))
        .with_parent_id(ledger.block0_hash)
        .build(&stake_pool, ledger.era());

    assert_err!(
        LedgerError::NonMonotonicDate {
            block_date: BlockDate {
                epoch: 0,
                slot_id: 1,
            },
            chain_date: ledger.date().clone(),
        },
        ledger.apply_block(block)
    );
}

#[test]
#[should_panic]
pub fn apply_block_epoch_transition_without_rewards_distribution() {
    let (mut ledger, controller) = prepare_scenario()
        .with_initials(vec![wallet("Bob").with(1_000).owns("stake_pool")])
        .build()
        .unwrap();

    let stake_pool = controller.stake_pool("stake_pool").unwrap();
    let date = BlockDate {
        epoch: 1,
        slot_id: 0,
    };
    let block = GenesisPraosBlockBuilder::new()
        .with_date(date.clone())
        .with_chain_length(ChainLength(1))
        .with_parent_id(ledger.block0_hash)
        .build(&stake_pool, ledger.era());

    ledger.increase_leader_log(&stake_pool.id());
    ledger.apply_block(block).unwrap();
}

#[test]
pub fn apply_block_incorrect_fragment() {
    let (mut ledger, controller) = prepare_scenario()
        .with_initials(vec![
            wallet("Bob").with(1_000).owns("stake_pool"),
            wallet("Alice").with(1_000),
        ])
        .build()
        .unwrap();

    let stake_pool = controller.stake_pool("stake_pool").unwrap();
    let alice = controller.wallet("Alice").unwrap();
    let bob = controller.wallet("Bob").unwrap();

    let date = BlockDate {
        epoch: 1,
        slot_id: 0,
    };

    let fragment = TestTxBuilder::new(&ledger.block0_hash)
        .move_funds(
            &mut ledger,
            &bob.as_account(),
            &alice.as_account(),
            &Value(10_000),
        )
        .get_fragment();

    let block = GenesisPraosBlockBuilder::new()
        .with_date(date.clone())
        .with_fragment(fragment)
        .with_chain_length(ChainLength(0))
        .with_parent_id(ledger.block0_hash)
        .build(&stake_pool, ledger.era());

    assert_err!(
        Account {
            source: ValueError {
                source: NegativeAmount
            }
        },
        ledger.apply_block(block)
    );
}
