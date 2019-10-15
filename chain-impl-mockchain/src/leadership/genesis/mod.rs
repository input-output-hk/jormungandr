mod vrfeval;

use crate::{
    block::{BlockDate, Header, Proof},
    certificate::PoolId,
    date::Epoch,
    key::{deserialize_public_key, verify_signature},
    leadership::{Error, ErrorKind, Verification},
    ledger::Ledger,
    stake::{self, StakeDistribution},
    value::Value,
};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_crypto::Verification as SigningVerification;
use chain_crypto::{
    digest::DigestOf, Blake2b256, Curve25519_2HashDH, PublicKey, SecretKey, SumEd25519_12,
};
use typed_bytes::ByteBuilder;
pub(crate) use vrfeval::witness_to_nonce;
pub use vrfeval::{ActiveSlotsCoeff, ActiveSlotsCoeffError, Nonce, Witness, WitnessOutput};
use vrfeval::{PercentStake, VrfEvaluator};

/// Praos Leader consisting of the KES public key and VRF public key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenesisPraosLeader {
    pub kes_public_key: PublicKey<SumEd25519_12>,
    pub vrf_public_key: PublicKey<Curve25519_2HashDH>,
}

impl GenesisPraosLeader {
    pub fn digest(&self) -> DigestOf<Blake2b256, Self> {
        DigestOf::digest_byteslice(
            &ByteBuilder::new()
                .bytes(self.vrf_public_key.as_ref())
                .bytes(self.kes_public_key.as_ref())
                .finalize()
                .as_byteslice(),
        )
    }
}

/// Genesis Praos leadership data for a specific epoch
pub struct LeadershipData {
    epoch_nonce: Nonce,
    nodes: stake::PoolTable,
    distribution: StakeDistribution,
    // the epoch this leader selection is valid for
    epoch: Epoch,
    active_slots_coeff: ActiveSlotsCoeff,
}

custom_error! {GenesisError
    InvalidEpoch { expected: Epoch, actual: Epoch } = "Wrong epoch, expected epoch {expected} but received block at epoch {actual}",
    TotalStakeIsZero = "Total stake is null",
}

impl LeadershipData {
    pub fn new(epoch: Epoch, ledger: &Ledger) -> Self {
        LeadershipData {
            epoch_nonce: ledger.settings.consensus_nonce.clone(),
            nodes: ledger.delegation.stake_pools.clone(),
            distribution: ledger.get_stake_distribution(),
            epoch,
            active_slots_coeff: ledger.settings.active_slots_coeff,
        }
    }

    pub fn distribution(&self) -> &StakeDistribution {
        &self.distribution
    }

    pub fn nodes(&self) -> &stake::PoolTable {
        &self.nodes
    }

    pub fn leader(
        &self,
        pool_id: &PoolId,
        vrf_key: &SecretKey<Curve25519_2HashDH>,
        date: BlockDate,
    ) -> Result<Option<Witness>, Error> {
        if date.epoch != self.epoch {
            return Err(Error::new_(
                ErrorKind::Failure,
                GenesisError::InvalidEpoch {
                    actual: date.epoch,
                    expected: self.epoch,
                },
            ));
        }

        let stake_snapshot = &self.distribution;

        match stake_snapshot.get_stake_for(pool_id) {
            None => Ok(None),
            Some(stake) => {
                // Calculate the total stake.
                let total_stake: Value = stake_snapshot.total_stake();

                if total_stake == Value::zero() {
                    return Err(Error::new_(
                        ErrorKind::Failure,
                        GenesisError::TotalStakeIsZero,
                    ));
                }

                let percent_stake = PercentStake {
                    stake: stake,
                    total: total_stake,
                };

                let evaluator = VrfEvaluator {
                    stake: percent_stake,
                    nonce: &self.epoch_nonce,
                    slot_id: date.slot_id,
                    active_slots_coeff: self.active_slots_coeff,
                };
                Ok(evaluator.evaluate(vrf_key))
            }
        }
    }

    pub(crate) fn verify(&self, block_header: &Header) -> Verification {
        if block_header.block_date().epoch != self.epoch {
            return Verification::Failure(Error::new_(
                ErrorKind::Failure,
                GenesisError::InvalidEpoch {
                    expected: self.epoch,
                    actual: block_header.block_date().epoch,
                },
            ));
        }

        let stake_snapshot = &self.distribution;

        match &block_header.proof() {
            Proof::GenesisPraos(ref genesis_praos_proof) => {
                let node_id = &genesis_praos_proof.node_id;
                match (
                    stake_snapshot.get_stake_for(node_id),
                    self.nodes.lookup(node_id),
                ) {
                    (Some(stake), Some(pool_info)) => {
                        // Calculate the total stake.
                        let total_stake: Value = stake_snapshot.total_stake();

                        let percent_stake = PercentStake {
                            stake: stake,
                            total: total_stake,
                        };

                        let _ = VrfEvaluator {
                            stake: percent_stake,
                            nonce: &self.epoch_nonce,
                            slot_id: block_header.block_date().slot_id,
                            active_slots_coeff: self.active_slots_coeff,
                        }
                        .verify(
                            &pool_info.keys.vrf_public_key,
                            &genesis_praos_proof.vrf_proof,
                        );

                        let valid = verify_signature(
                            &genesis_praos_proof.kes_proof.0,
                            &pool_info.keys.kes_public_key,
                            &block_header.common,
                        );

                        if valid == SigningVerification::Failed {
                            Verification::Failure(Error::new(ErrorKind::InvalidLeaderSignature))
                        } else {
                            Verification::Success
                        }
                    }
                    (_, _) => Verification::Failure(Error::new(ErrorKind::InvalidBlockMessage)),
                }
            }
            _ => Verification::Failure(Error::new(ErrorKind::InvalidLeaderSignature)),
        }
    }
}

impl Readable for GenesisPraosLeader {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let vrf_public_key = deserialize_public_key(buf)?;
        let kes_public_key = deserialize_public_key(buf)?;
        Ok(GenesisPraosLeader {
            vrf_public_key,
            kes_public_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::certificate::PoolId;
    use crate::ledger::Ledger;
    use crate::milli::Milli;
    use crate::stake::{PoolStakeDistribution, PoolStakeInformation, PoolStakeTotal};
    use crate::testing::{
        builders::{GenesisPraosBlockBuilder, StakePoolBuilder},
        ledger as ledger_mock,
    };
    use crate::value::*;

    use chain_crypto::*;
    use lazy_static::lazy_static;
    use quickcheck::{Arbitrary, Gen};
    use rand_core;
    use std::collections::HashMap;

    fn make_pool(ledger: &mut Ledger) -> (PoolId, SecretKey<Curve25519_2HashDH>) {
        let stake_pool = StakePoolBuilder::new().build();
        ledger
            .delegation()
            .register_stake_pool(stake_pool.info())
            .expect("cannot add stake pool to ledger");
        (stake_pool.id(), stake_pool.vrf().private_key().clone())
    }

    #[derive(Clone, Debug)]
    pub struct LeaderElectionParameters {
        slots_per_epoch: u32,
        active_slots_coeff: f32,
        pools_count: usize,
        value: Value,
    }

    impl Arbitrary for GenesisPraosLeader {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand_core::SeedableRng;
            lazy_static! {
                static ref PK_KES: PublicKey<SumEd25519_12> = {
                    let sk: SecretKey<SumEd25519_12> =
                        SecretKey::generate(&mut rand_chacha::ChaChaRng::from_seed([0; 32]));
                    sk.to_public()
                };
            }

            let tcg = testing::TestCryptoGen::arbitrary(g);
            let mut rng = tcg.get_rng(0);
            let vrf_sk: SecretKey<Curve25519_2HashDH> = SecretKey::generate(&mut rng);
            GenesisPraosLeader {
                vrf_public_key: vrf_sk.to_public(),
                kes_public_key: PK_KES.clone(),
            }
        }
    }

    impl LeaderElectionParameters {
        pub fn new() -> Self {
            // Those values are arbitrary. Generated by one of quickcheck test case
            // Converted it to 'standard' test case due to test case extended duration
            let pools_count = 5;
            let active_slots_coeff = 0.18;

            LeaderElectionParameters {
                slots_per_epoch: 1700,
                active_slots_coeff: active_slots_coeff,
                pools_count: pools_count,
                value: Value(100),
            }
        }

        pub fn active_slots_coeff_as_milli(&self) -> Milli {
            Milli::from_millis((self.active_slots_coeff * 1000.0) as u64)
        }
    }

    type Pools = HashMap<PoolId, (SecretKey<Curve25519_2HashDH>, u64, Value)>;

    fn make_leadership_with_pools(ledger: &Ledger, pools: &Pools) -> LeadershipData {
        let mut selection = LeadershipData::new(0, &ledger);

        for (pool_id, (_, _, value)) in pools {
            update_stake_pool_total_value(&mut selection, pool_id, value);
        }
        selection
    }

    fn update_stake_pool_total_value(
        selection: &mut LeadershipData,
        pool_id: &PoolId,
        value: &Value,
    ) {
        selection.distribution.to_pools.insert(
            pool_id.clone(),
            PoolStakeInformation {
                total: PoolStakeTotal {
                    total_stake: *value,
                },
                stake_owners: PoolStakeDistribution {
                    accounts: HashMap::new(),
                },
            },
        );
    }

    #[test]
    pub fn test_leader_election_is_consistent_with_stake_distribution() {
        let leader_election_parameters = LeaderElectionParameters::new();

        let config_params = ledger_mock::ConfigBuilder::new()
            .with_slots_per_epoch(leader_election_parameters.slots_per_epoch)
            .with_active_slots_coeff(leader_election_parameters.active_slots_coeff_as_milli())
            .build();

        let (_genesis_hash, mut ledger) =
            ledger_mock::create_initial_fake_ledger(&vec![], config_params).unwrap();

        let mut pools = HashMap::<PoolId, (SecretKey<Curve25519_2HashDH>, u64, Value)>::new();

        for _i in 0..leader_election_parameters.pools_count {
            let (pool_id, pool_vrf_private_key) = make_pool(&mut ledger);
            pools.insert(
                pool_id.clone(),
                (
                    pool_vrf_private_key,
                    0,
                    leader_election_parameters.value.clone(),
                ),
            );
        }

        let selection = make_leadership_with_pools(&ledger, &pools);

        let mut empty_slots = 0;
        let mut date = ledger.date();
        for _i in 0..leader_election_parameters.slots_per_epoch {
            let mut any_found = false;
            for (pool_id, (pool_vrf_private_key, times_selected, _)) in pools.iter_mut() {
                match selection
                    .leader(&pool_id, &pool_vrf_private_key, date)
                    .unwrap()
                {
                    None => {}
                    Some(_) => {
                        any_found = true;
                        *times_selected += 1;
                    }
                }
            }
            if !any_found {
                empty_slots += 1;
            }
            date = date.next(&ledger.era());
        }

        println!("Calculating percentage of election per pool....");
        println!("parameters = {:?}", leader_election_parameters);
        println!("empty slots = {}", empty_slots);
        let total_election_count: u64 = pools.iter().map(|(_, y)| y.1).sum();
        let ideal_election_count_per_pool: f32 =
            total_election_count as f32 / leader_election_parameters.pools_count as f32;
        let ideal_election_percentage =
            ideal_election_count_per_pool as f32 / total_election_count as f32;
        let grace_percentage: f32 = 0.05;
        println!(
            "ideal percentage: {:.2}, grace_percentage: {:.2}",
            ideal_election_percentage, grace_percentage
        );

        for (pool_id, (_pool_vrf_private_key, times_selected, stake)) in pools.iter_mut() {
            let pool_election_percentage = (*times_selected as f32) / (total_election_count as f32);
            println!(
                "pool id={}, stake={}, slots %={}",
                pool_id, stake.0, pool_election_percentage
            );

            assert!(
                (pool_election_percentage - ideal_election_percentage).abs() - grace_percentage
                    < 0.01,
                "Incorrect percentage {:.2} is out of correct range [{:.2} {:.2} ]",
                pool_election_percentage,
                ideal_election_percentage - grace_percentage,
                ideal_election_percentage + grace_percentage
            );
        }
    }

    #[test]
    #[ignore]
    pub fn test_phi() {
        let slots_per_epoch = 200000;
        let active_slots_coeff = 0.1;
        let active_slots_coeff_as_milli = Milli::from_millis((active_slots_coeff * 1000.0) as u64);
        let config_params = ledger_mock::ConfigBuilder::new()
            .with_slots_per_epoch(slots_per_epoch)
            .with_active_slots_coeff(active_slots_coeff_as_milli)
            .build();

        let (_genesis_hash, mut ledger) =
            ledger_mock::create_initial_fake_ledger(&vec![], config_params).unwrap();

        let mut pools = Pools::new();

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

        let selection = make_leadership_with_pools(&ledger, &pools);

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
            date = date.next(&ledger.era());
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

    #[test]
    pub fn leadership_leader_different_epoch() {
        let selection_epoch = 0;
        let date = BlockDate {
            epoch: 1u32,
            slot_id: 0u32,
        };
        let (_genesis_hash, ledger) = ledger_mock::create_initial_fake_ledger(
            &vec![],
            ledger_mock::ConfigBuilder::new().build(),
        )
        .unwrap();

        let stake_pool = StakePoolBuilder::new().build();
        ledger
            .delegation()
            .register_stake_pool(stake_pool.info())
            .expect("cannot register stake pool");
        let mut selection = LeadershipData::new(selection_epoch, &ledger);
        update_stake_pool_total_value(&mut selection, &stake_pool.id(), &Value(100));

        assert!(selection
            .leader(&stake_pool.id(), &stake_pool.vrf().private_key(), date)
            .is_err());
    }

    #[test]
    pub fn leadership_leader_no_stake() {
        let date = BlockDate::first();
        let (_genesis_hash, ledger) = ledger_mock::create_initial_fake_ledger(
            &vec![],
            ledger_mock::ConfigBuilder::new().build(),
        )
        .unwrap();

        let stake_pool = StakePoolBuilder::new().build();
        ledger
            .delegation()
            .register_stake_pool(stake_pool.info())
            .expect("cannot register stake pool");
        let selection = LeadershipData::new(date.epoch, &ledger);

        assert_eq!(
            selection
                .leader(&stake_pool.id(), &stake_pool.vrf().private_key(), date)
                .unwrap(),
            None
        );
    }

    #[test]
    pub fn leadership_leader_zero_stake() {
        let date = BlockDate::first();
        let (_genesis_hash, ledger) = ledger_mock::create_initial_fake_ledger(
            &vec![],
            ledger_mock::ConfigBuilder::new().build(),
        )
        .unwrap();

        let stake_pool = StakePoolBuilder::new().build();
        ledger
            .delegation()
            .register_stake_pool(stake_pool.info())
            .expect("cannot register stake pool");
        let mut selection = LeadershipData::new(date.epoch, &ledger);
        update_stake_pool_total_value(&mut selection, &stake_pool.id(), &Value(0));

        assert!(selection
            .leader(&stake_pool.id(), &stake_pool.vrf().private_key(), date)
            .is_err());
    }

    use crate::block::{BlockBuilder, Contents};
    use chain_core::property::ChainLength;

    #[test]
    pub fn leadership_verify_different_epoch() {
        let date = BlockDate {
            epoch: 1,
            slot_id: 0,
        };
        let (genesis_hash, ledger) = ledger_mock::create_initial_fake_ledger(
            &vec![],
            ledger_mock::ConfigBuilder::new().build(),
        )
        .unwrap();

        let stake_pool = StakePoolBuilder::new().build();
        ledger
            .delegation()
            .register_stake_pool(stake_pool.info())
            .expect("cannot register stake pool");
        let selection = LeadershipData::new(0, &ledger);

        let block = GenesisPraosBlockBuilder::new()
            .with_date(date)
            .with_chain_length(ledger.chain_length())
            .with_parent_id(genesis_hash)
            .build(&stake_pool, ledger.era());

        assert!(selection.verify(&block.header).failure());
    }

    #[test]
    pub fn leadership_verify_different_proof() {
        let date = BlockDate {
            epoch: 1,
            slot_id: 0,
        };
        let (_genesis_hash, ledger) = ledger_mock::create_initial_fake_ledger(
            &vec![],
            ledger_mock::ConfigBuilder::new().build(),
        )
        .unwrap();

        let stake_pool = StakePoolBuilder::new().build();
        ledger
            .delegation()
            .register_stake_pool(stake_pool.info())
            .expect("cannot register stake pool");
        let selection = LeadershipData::new(date.epoch, &ledger);
        let rng = rand_os::OsRng::new().unwrap();
        let mut block_builder = BlockBuilder::new(Contents::empty());
        block_builder.date(date);
        block_builder.chain_length(ledger.chain_length().next());
        let block = block_builder.make_bft_block(&SecretKey::generate(rng));

        assert!(selection.verify(&block.header).failure());
    }

    #[test]
    pub fn leadership_verify_no_stake() {
        let date = BlockDate::first();
        let (genesis_hash, ledger) = ledger_mock::create_initial_fake_ledger(
            &vec![],
            ledger_mock::ConfigBuilder::new().build(),
        )
        .unwrap();

        let stake_pool = StakePoolBuilder::new().build();
        ledger
            .delegation()
            .register_stake_pool(stake_pool.info())
            .expect("cannot register stake pool");
        let selection = LeadershipData::new(date.epoch, &ledger);

        let block = GenesisPraosBlockBuilder::new()
            .with_date(date)
            .with_chain_length(ledger.chain_length())
            .with_parent_id(genesis_hash)
            .build(&stake_pool, ledger.era());
        assert!(selection.verify(&block.header).failure());
    }

    #[test]
    pub fn leadership_verify_zero_stake() {
        let date = BlockDate::first();
        let (genesis_hash, ledger) = ledger_mock::create_initial_fake_ledger(
            &vec![],
            ledger_mock::ConfigBuilder::new().build(),
        )
        .unwrap();

        let stake_pool = StakePoolBuilder::new().build();
        ledger
            .delegation()
            .register_stake_pool(stake_pool.info())
            .expect("cannot register stake pool");
        let mut selection = LeadershipData::new(date.epoch, &ledger);
        update_stake_pool_total_value(&mut selection, &stake_pool.id(), &Value(0));

        let block = GenesisPraosBlockBuilder::new()
            .with_date(date)
            .with_chain_length(ledger.chain_length())
            .with_parent_id(genesis_hash)
            .build(&stake_pool, ledger.era());

        assert!(selection.verify(&block.header).failure());
    }

    #[test]
    pub fn leadership_verify_non_existing_pool() {
        let date = BlockDate::first();
        let (genesis_hash, ledger) = ledger_mock::create_initial_fake_ledger(
            &vec![],
            ledger_mock::ConfigBuilder::new().build(),
        )
        .unwrap();

        let stake_pool = StakePoolBuilder::new().build();
        let selection = LeadershipData::new(date.epoch, &ledger);

        let block = GenesisPraosBlockBuilder::new()
            .with_date(date)
            .with_chain_length(ledger.chain_length())
            .with_parent_id(genesis_hash)
            .build(&stake_pool, ledger.era());

        assert!(selection.verify(&block.header).failure());
    }
}
