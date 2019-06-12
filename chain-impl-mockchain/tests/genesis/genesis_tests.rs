use crate::common::ledger as ledger_mock;
use chain_crypto::KeyPair;
use chain_crypto::*;
use chain_impl_mockchain::leadership::genesis::*;
use chain_impl_mockchain::milli::Milli;

use chain_impl_mockchain::ledger::Ledger;
use chain_impl_mockchain::stake::PoolStakeDistribution;
use chain_impl_mockchain::stake::StakePoolId;
use chain_impl_mockchain::stake::StakePoolInfo;
use chain_impl_mockchain::value::*;
use std::collections::HashMap;

fn make_pool(ledger: &mut Ledger) -> (StakePoolId, SecretKey<Curve25519_2HashDH>) {
    let mut rng = rand::thread_rng();

    let pool_vrf_private_key = SecretKey::generate(&mut rng);
    let pool_kes: KeyPair<SumEd25519_12> = KeyPair::generate(&mut rng);
    let (_, pool_kes_public_key) = pool_kes.into_keys();

    let pool_info = StakePoolInfo {
        serial: 1234,
        owners: vec![],
        initial_key: GenesisPraosLeader {
            vrf_public_key: pool_vrf_private_key.to_public(),
            kes_public_key: pool_kes_public_key,
        },
    };

    let pool_id = pool_info.to_id();

    ledger.delegation().register_stake_pool(pool_info).unwrap();

    (pool_id, pool_vrf_private_key)
}

#[test]
#[ignore]
pub fn test_phi() {
    let slots_per_epoch = 200000;
    let active_slots_coeff = 0.1;

    let config_params = ledger_mock::ConfigBuilder::new()
        .with_slots_per_epoch(200000)
        .with_active_slots_coeff(Milli::from_millis(100))
        .build();

    let (_genesis_hash, mut ledger) =
        ledger_mock::create_initial_fake_ledger(&vec![], config_params);

    let mut pools = HashMap::<StakePoolId, (SecretKey<Curve25519_2HashDH>, u64, Value)>::new();

    let (big_pool_id, big_pool_vrf_private_key) = make_pool(&mut ledger);
    pools.insert(
        big_pool_id.clone(),
        (big_pool_vrf_private_key, 0, Value(1000)),
    );

    for _i in 0..10 {
        let (small_pool_id, small_pool_vrf_private_key) = make_pool(&mut ledger);
        pools.insert(
            small_pool_id.clone(),
            (small_pool_vrf_private_key, 0, Value(100)),
        );
    }

    let mut selection = GenesisLeaderSelection::new(0, &ledger);

    for (pool_id, (_, _, value)) in &pools {
        selection.distribution().to_pools.insert(
            pool_id.clone(),
            PoolStakeDistribution {
                total_stake: *value,
            },
        );
    }

    let mut date = ledger.date();

    let mut empty_slots = 0;

    let mut times_selected_small = 0;

    let nr_slots = slots_per_epoch;

    for _i in 0..nr_slots {
        let mut any_found = false;
        let mut any_small = false;
        for (pool_id, (pool_vrf_private_key, times_selected, value)) in pools.iter_mut() {
            match selection
                .leader(&pool_id, &pool_vrf_private_key, date)
                .unwrap()
            {
                None => {}
                Some(_witness) => {
                    any_found = true;
                    *times_selected += 1;
                    if value.0 == 100 {
                        any_small = true;
                    }
                }
            }
        }
        if !any_found {
            empty_slots += 1;
        }
        if any_small {
            times_selected_small += 1;
        }
        date = date.next(&ledger.settings().era);
    }

    for (pool_id, (_pool_vrf_private_key, times_selected, stake)) in pools.iter_mut() {
        println!(
            "pool id={} stake={} slots={}",
            pool_id, stake.0, times_selected
        );
    }
    println!("empty slots = {}", empty_slots);
    println!("small stake slots = {}", times_selected_small);
    let times_selected_big = pools[&big_pool_id].1;
    println!("big stake slots = {}", times_selected_big);

    // Check that we got approximately the correct number of active slots.
    assert!(empty_slots > (nr_slots as f64 * (1.0 - active_slots_coeff - 0.01)) as u32);
    assert!(empty_slots < (nr_slots as f64 * (1.0 - active_slots_coeff + 0.01)) as u32);

    // Check that splitting a stake doesn't have a big effect on
    // the chance of becoming slot leader.
    assert!((times_selected_big as f64 / times_selected_small as f64) > 0.98);
    assert!((times_selected_big as f64 / times_selected_small as f64) < 1.02);
}
