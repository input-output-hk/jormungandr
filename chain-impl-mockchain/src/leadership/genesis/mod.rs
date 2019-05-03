mod vrfeval;

use crate::{
    block::{BlockDate, Header, Proof},
    date::Epoch,
    key::verify_signature,
    leadership::{Error, ErrorKind, Verification},
    ledger::Ledger,
    stake::{self, StakeDistribution, StakePoolId},
    value::Value,
};
use chain_crypto::Verification as SigningVerification;
use chain_crypto::{Curve25519_2HashDH, FakeMMM, PublicKey, SecretKey};
pub use vrfeval::{ActiveSlotsCoeff, ActiveSlotsCoeffError, Witness};
use vrfeval::{Nonce, PercentStake, VrfEvaluator};

/// Praos Leader consisting of the KES public key and VRF public key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenesisPraosLeader {
    pub kes_public_key: PublicKey<FakeMMM>,
    pub vrf_public_key: PublicKey<Curve25519_2HashDH>,
}

pub struct GenesisLeaderSelection {
    epoch_nonce: Nonce,
    nodes: stake::PoolTable,
    distribution: StakeDistribution,
    // the epoch this leader selection is valid for
    epoch: Epoch,
    active_slots_coeff: ActiveSlotsCoeff,
}

impl GenesisLeaderSelection {
    pub fn new(epoch: Epoch, ledger: &Ledger) -> Self {
        GenesisLeaderSelection {
            epoch_nonce: Nonce::zero(),
            nodes: ledger.delegation.stake_pools.clone(),
            distribution: ledger.get_stake_distribution(),
            epoch,
            active_slots_coeff: ledger.settings.active_slots_coeff,
        }
    }

    pub fn leader(
        &self,
        pool_id: &StakePoolId,
        vrf_key: &SecretKey<Curve25519_2HashDH>,
        date: BlockDate,
    ) -> Result<Option<Witness>, Error> {
        if date.epoch != self.epoch {
            // TODO: add more error details: invalid Date
            return Err(Error::new(ErrorKind::Failure));
        }

        let stake_snapshot = &self.distribution;

        match stake_snapshot.get_stake_for(&pool_id) {
            None => Ok(None),
            Some(stake) => {
                // Calculate the total stake.
                let total_stake: Value = stake_snapshot.total_stake();

                if total_stake == Value::zero() {
                    // TODO: give more info about the error here...
                    return Err(Error::new(ErrorKind::Failure));
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
            // TODO: add more error details: invalid Date
            return Verification::Failure(Error::new(ErrorKind::Failure));
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
                            &pool_info.initial_key.vrf_public_key,
                            &genesis_praos_proof.vrf_proof,
                        );

                        let valid = verify_signature(
                            &genesis_praos_proof.kes_proof.0,
                            &pool_info.initial_key.kes_public_key,
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

/*
#[cfg(test)]
mod test {

    use super::*;
    use crate::block::{
        Block, BlockContents, Common, BLOCK_VERSION_CONSENSUS_BFT,
        BLOCK_VERSION_CONSENSUS_GENESIS_PRAOS,
    };
    use crate::key::Hash;
    use crate::leadership::Leader;
    use crate::ledger::test::make_key;
    use crate::transaction::*;
    use chain_addr::{Address, Discrimination, Kind};
    use chain_core::property::Ledger as L;
    use chain_core::property::Settings as S;
    use chain_core::property::Transaction as T;
    use chain_core::property::{BlockId, HasTransaction};
    use chain_crypto::{
        algorithms::{Ed25519, Ed25519Extended, FakeMMM},
        SecretKey,
    };
    use quickcheck::{Arbitrary, StdGen};
    use rand::rngs::{StdRng, ThreadRng};

    struct TestState {
        g: StdGen<ThreadRng>,
        bft_leaders: Vec<SecretKey<Ed25519>>,
        pool_private_keys: Vec<SecretKey<Ed25519>>,
        ledger: Arc<RwLock<Ledger>>,
        settings: Arc<RwLock<Settings>>,
        cur_date: BlockDate,
        prev_hash: Hash,
        leader_selection: GenesisLeaderSelection,
        faucet_utxo: UtxoPointer,
        faucet_private_key: SecretKey<Ed25519Extended>,
        selected_leaders: HashMap<LeaderId, usize>,
    }

    impl TestState {
        fn faucet_value(&self) -> Value {
            self.faucet_utxo.value
        }
    }

    fn create_chain(
        initial_bootstrap_key_slots_percentage: u8,
        mut initial_utxos: HashMap<UtxoPointer, Output>,
        initial_stake_pools: Vec<SecretKey<Ed25519>>,
        initial_stake_keys: HashMap<StakeKeyId, Option<StakePoolId>>,
    ) -> TestState {
        let mut g = StdGen::new(rand::thread_rng(), 10);

        let bft_leaders: Vec<_> = (0..10_i32)
            .map(|_| crate::key::test::arbitrary_secret_key(&mut g))
            .collect();

        let faucet_utxo = UtxoPointer::new(
            TransactionId::hash_bytes("faucet".as_bytes()),
            0,
            Value(1000000000),
        );
        let (faucet_private_key, faucet_address) = make_key(123);

        initial_utxos.insert(faucet_utxo, Output(faucet_address, faucet_utxo.value));

        let ledger = Arc::new(RwLock::new(Ledger::new(initial_utxos)));

        let settings = Arc::new(RwLock::new(Settings::new()));
        settings.write().unwrap().bootstrap_key_slots_percentage =
            initial_bootstrap_key_slots_percentage;

        let leader_selection = GenesisLeaderSelection::new(
            bft_leaders
                .iter()
                .map(|k| LeaderId(k.to_public()))
                .collect(),
            ledger.clone(),
            settings.clone(),
            initial_stake_pools.iter().map(|x| x.into()).collect(),
            initial_stake_keys,
        )
        .unwrap();

        TestState {
            g,
            bft_leaders,
            pool_private_keys: initial_stake_pools,
            leader_selection,
            ledger,
            settings,
            cur_date: BlockDate::first(),
            prev_hash: Hash::zero(),
            faucet_utxo,
            faucet_private_key,
            selected_leaders: HashMap::new(),
        }
    }

    /// Create and apply a signed block with a single certificate.
    fn apply_signed_block(state: &mut TestState, blk: Block) -> Result<(), Error> {
        let settings_diff = state.settings.read().unwrap().diff(&blk).unwrap();
        let ledger_diff = state
            .ledger
            .read()
            .unwrap()
            .diff(blk.transactions())
            .unwrap();
        let leader_diff = state.leader_selection.diff(&blk)?;

        state
            .settings
            .write()
            .unwrap()
            .apply(settings_diff)
            .unwrap();
        state.ledger.write().unwrap().apply(ledger_diff).unwrap();
        state.leader_selection.apply(leader_diff.clone())?;

        // Applying the diff again should fail.
        state.leader_selection.apply(leader_diff).unwrap_err();

        state.prev_hash = blk.id();
        state.cur_date = state.cur_date.next();

        // Keep track of how often leaders were selected.
        *state
            .selected_leaders
            .entry(blk.header.proof.leader_id().unwrap())
            .or_insert(0) += 1;

        Ok(())
    }

    /// Create and apply a block with the specified contents.
    fn apply_block(state: &mut TestState, contents: Vec<Message>) -> Result<LeaderId, Error> {
        let leader_id = state
            .leader_selection
            .get_leader_at(state.cur_date)
            .unwrap();

        let (leader_private_key, block_version) = if let Some(leader_private_key) = state
            .bft_leaders
            .iter()
            .find(|k| LeaderId::from(*k) == leader_id)
        {
            (
                Leader::BftLeader(leader_private_key.clone()),
                BLOCK_VERSION_CONSENSUS_BFT,
            )
        } else if let Some(pool_private_key) = state
            .pool_private_keys
            .iter()
            .find(|k| LeaderId::from(*k) == leader_id)
        {
            let mut csprng: rand::OsRng = rand::OsRng::new().unwrap();
            let key = vrf::SecretKey::random(&mut csprng);
            let (point, seed) = key.verifiable_output(&[][..]);
            let scalar = vrf::Scalar::random(&mut csprng);
            let proof = key.proove(&scalar, point, seed);
            (
                Leader::GenesisPraos(key, pool_private_key.clone(), proof),
                BLOCK_VERSION_CONSENSUS_GENESIS_PRAOS,
            )
        } else {
            panic!();
        };

        let contents = BlockContents::new(contents);
        let (hash, size) = contents.compute_hash_size();
        let common = Common {
            block_version: block_version,
            block_date: state.cur_date.clone(),
            block_content_size: size as u32,
            block_content_hash: hash,
            block_parent_hash: state.prev_hash.clone(),
        };

        let blk = Block::new(contents, common, &leader_private_key);

        apply_signed_block(state, blk)?;

        Ok(leader_id)
    }

    /// Create and apply a signed block with a single message.
    fn apply_block1(state: &mut TestState, msg: Message) -> Result<LeaderId, Error> {
        apply_block(state, vec![msg])
    }

    #[test]
    pub fn pure_bft() {
        let mut state = create_chain(
            crate::setting::SLOTS_PERCENTAGE_RANGE,
            HashMap::new(),
            vec![],
            HashMap::new(),
        );

        assert_eq!(
            state
                .leader_selection
                .get_leader_at(BlockDate {
                    epoch: 0,
                    slot_id: 0
                })
                .unwrap(),
            (&state.bft_leaders[0]).into()
        );

        // Generate a bunch of blocks and check that all leaders are
        // picked an equal number of times.
        for i in 0..10 * state.bft_leaders.len() {
            let leader_id = apply_block(&mut state, vec![]).unwrap();
            assert_eq!(
                LeaderId::from(&state.bft_leaders[i % state.bft_leaders.len()]),
                leader_id
            );
        }

        // Applying a block with a wrong leader should fail.
        {
            let contents = BlockContents::new(Vec::new());
            let (hash, size) = contents.compute_hash_size();
            let common = Common {
                block_version: BLOCK_VERSION_CONSENSUS_BFT,
                block_date: state.cur_date.clone(),
                block_content_size: size as u32,
                block_content_hash: hash,
                block_parent_hash: state.prev_hash.clone(),
            };

            let signed_block = Block::new(
                contents,
                common,
                &(Leader::BftLeader(state.bft_leaders[1].clone())),
            );

            assert_eq!(
                state.leader_selection.diff(&signed_block).unwrap_err(),
                Error::BlockHasInvalidLeader(
                    (&state.bft_leaders[0]).into(),
                    (&state.bft_leaders[1]).into()
                )
            );
        }

        // Let's skip a few slots.
        {
            state.cur_date = state.cur_date.next().next().next();

            let contents = BlockContents::new(Vec::new());
            let (hash, size) = contents.compute_hash_size();
            let common = Common {
                block_version: BLOCK_VERSION_CONSENSUS_BFT,
                block_date: state.cur_date.clone(),
                block_content_size: size as u32,
                block_content_hash: hash,
                block_parent_hash: state.prev_hash.clone(),
            };

            let signed_block = Block::new(
                contents,
                common,
                &(Leader::BftLeader(state.bft_leaders[3].clone())),
            );

            apply_signed_block(&mut state, signed_block).unwrap();
        }
    }

    #[test]
    pub fn delegation() {
        let mut state = create_chain(
            crate::setting::SLOTS_PERCENTAGE_RANGE,
            HashMap::new(),
            vec![],
            HashMap::new(),
        );

        // Try to register a stake key with a wrong certificate.
        let sks0 = PrivateKey::arbitrary(&mut state.g);
        {
            let signer = PrivateKey::arbitrary(&mut state.g);
            assert_eq!(
                apply_block1(
                    &mut state,
                    (certificate::StakeKeyRegistration {
                        stake_key_id: (&sks0).into(),
                    })
                    .make_certificate(&signer)
                ),
                Err(Error::StakeKeyRegistrationSigIsInvalid)
            );
        }

        // Register a stake key.
        {
            assert_eq!(state.leader_selection.delegation_state.nr_stake_keys(), 0);
            apply_block1(
                &mut state,
                (certificate::StakeKeyRegistration {
                    stake_key_id: (&sks0).into(),
                })
                .make_certificate(&sks0),
            )
            .unwrap();
            assert_eq!(state.leader_selection.delegation_state.nr_stake_keys(), 1);
        }

        // Transfer some money to an address that delegates to the stake key.
        let (user_private_key, _user_address) = make_key(42);
        {
            let value = Value(10000);
            let change_value = state.faucet_value() - value;
            let transaction = Transaction {
                inputs: vec![state.faucet_utxo],
                outputs: vec![
                    Output(
                        Address(
                            Discrimination::Test,
                            Kind::Single(state.faucet_private_key.public().0),
                        ),
                        change_value,
                    ),
                    Output(
                        Address(
                            Discrimination::Test,
                            Kind::Group(user_private_key.public().0, sks0.public().0),
                        ),
                        value,
                    ),
                ],
            };
            let txid = transaction.id();
            let signed_tx = SignedTransaction {
                witnesses: vec![Witness::new(&transaction.id(), &state.faucet_private_key)],
                transaction,
            };
            apply_block1(&mut state, Message::Transaction(signed_tx)).unwrap();
            state.faucet_utxo = UtxoPointer::new(txid, 0, change_value);
        }

        // Try to register a pool with a wrong certificate.
        let pool0_private_key = PrivateKey::arbitrary(&mut state.g);
        //let owner0_private_key = PrivateKey::arbitrary(&mut g);
        {
            let signer = PrivateKey::arbitrary(&mut state.g);
            assert_eq!(
                apply_block1(
                    &mut state,
                    (certificate::StakePoolRegistration {
                        pool_id: (&pool0_private_key).into(),
                        //owner: owner0_private_key.public(),
                    })
                    .make_certificate(&signer)
                ),
                Err(Error::StakePoolRegistrationPoolSigIsInvalid)
            );
        }

        // Register a new pool.
        {
            assert_eq!(state.leader_selection.delegation_state.nr_stake_pools(), 0);
            apply_block1(
                &mut state,
                (certificate::StakePoolRegistration {
                    pool_id: (&pool0_private_key).into(),
                    //owner: owner0_private_key().public(),
                })
                .make_certificate(&pool0_private_key),
            )
            .unwrap();
            assert_eq!(state.leader_selection.delegation_state.nr_stake_pools(), 1);
            assert_eq!(
                state
                    .leader_selection
                    .get_stake_distribution()
                    .eligible_stake_pools(),
                0,
            );
        }

        // Try to delegate some stake with a wrong key.
        {
            assert_eq!(
                apply_block1(
                    &mut state,
                    (certificate::StakeDelegation {
                        stake_key_id: (&sks0).into(),
                        pool_id: (&pool0_private_key).into(),
                    })
                    .make_certificate(&pool0_private_key)
                ),
                Err(Error::StakeDelegationSigIsInvalid)
            );
        }

        // Delegate some stake to the pool.
        {
            apply_block1(
                &mut state,
                (certificate::StakeDelegation {
                    stake_key_id: (&sks0).into(),
                    pool_id: (&pool0_private_key).into(),
                })
                .make_certificate(&sks0),
            )
            .unwrap();
            assert_eq!(
                state
                    .leader_selection
                    .delegation_state
                    .nr_pool_members((&pool0_private_key).into()),
                1
            );
            let dist = state.leader_selection.get_stake_distribution();
            assert_eq!(
                dist.0,
                vec![(
                    (&pool0_private_key).into(),
                    PoolStakeDistribution {
                        total_stake: Value(10000),
                        member_stake: vec![((&sks0).into(), Value(10000))]
                            .iter()
                            .cloned()
                            .collect()
                    }
                )]
                .iter()
                .cloned()
                .collect()
            );
        }

        // Transfer some more money.
        {
            let value = Value(20000);
            let change_value = state.faucet_value() - value;
            let transaction = Transaction {
                inputs: vec![state.faucet_utxo],
                outputs: vec![
                    Output(
                        Address(
                            Discrimination::Test,
                            Kind::Single(state.faucet_private_key.public().0),
                        ),
                        change_value,
                    ),
                    Output(
                        Address(
                            Discrimination::Test,
                            Kind::Group(user_private_key.public().0, sks0.public().0),
                        ),
                        value,
                    ),
                ],
            };
            let txid = transaction.id();
            let signed_tx = SignedTransaction {
                witnesses: vec![Witness::new(&transaction.id(), &state.faucet_private_key)],
                transaction,
            };
            apply_block1(&mut state, Message::Transaction(signed_tx)).unwrap();
            state.faucet_utxo = UtxoPointer::new(txid, 0, change_value);
            assert_eq!(
                state.leader_selection.get_stake_distribution().0,
                vec![(
                    (&pool0_private_key).into(),
                    PoolStakeDistribution {
                        total_stake: Value(30000),
                        member_stake: vec![((&sks0).into(), Value(30000))]
                            .iter()
                            .cloned()
                            .collect()
                    }
                )]
                .iter()
                .cloned()
                .collect()
            );
        }

        // Register another stake key.
        let sks1 = PrivateKey::arbitrary(&mut state.g);
        {
            assert_eq!(state.leader_selection.delegation_state.nr_stake_keys(), 1);
            apply_block1(
                &mut state,
                (certificate::StakeKeyRegistration {
                    stake_key_id: (&sks1).into(),
                })
                .make_certificate(&sks1),
            )
            .unwrap();
            assert_eq!(state.leader_selection.delegation_state.nr_stake_keys(), 2);
        }

        // Register another pool.
        let pool1_private_key = PrivateKey::arbitrary(&mut state.g);
        {
            assert_eq!(state.leader_selection.delegation_state.nr_stake_pools(), 1);
            apply_block1(
                &mut state,
                (certificate::StakePoolRegistration {
                    pool_id: (&pool1_private_key).into(),
                })
                .make_certificate(&pool1_private_key),
            )
            .unwrap();
            assert_eq!(state.leader_selection.delegation_state.nr_stake_pools(), 2);
        }

        // Delegate some stake to the pool.
        {
            apply_block1(
                &mut state,
                (certificate::StakeDelegation {
                    stake_key_id: (&sks1).into(),
                    pool_id: (&pool1_private_key).into(),
                })
                .make_certificate(&sks1),
            )
            .unwrap();
            assert_eq!(
                state.leader_selection.get_stake_distribution().0,
                vec![(
                    (&pool0_private_key).into(),
                    PoolStakeDistribution {
                        total_stake: Value(30000),
                        member_stake: vec![((&sks0).into(), Value(30000))]
                            .iter()
                            .cloned()
                            .collect()
                    }
                )]
                .iter()
                .cloned()
                .collect()
            );
        }

        // Transfer some money, delegating it to the new stake key.
        let expected_stake_dist = vec![
            (
                (&pool0_private_key).into(),
                PoolStakeDistribution {
                    total_stake: Value(30000),
                    member_stake: vec![((&sks0).into(), Value(30000))]
                        .iter()
                        .cloned()
                        .collect(),
                },
            ),
            (
                (&pool1_private_key).into(),
                PoolStakeDistribution {
                    total_stake: Value(42000),
                    member_stake: vec![((&sks1).into(), Value(42000))]
                        .iter()
                        .cloned()
                        .collect(),
                },
            ),
        ]
        .iter()
        .cloned()
        .collect();

        {
            let value = Value(42000);
            let change_value = state.faucet_value() - value;
            let transaction = Transaction {
                inputs: vec![state.faucet_utxo],
                outputs: vec![
                    Output(
                        Address(
                            Discrimination::Test,
                            Kind::Single(state.faucet_private_key.public().0),
                        ),
                        change_value,
                    ),
                    Output(
                        Address(
                            Discrimination::Test,
                            Kind::Group(user_private_key.public().0, sks1.public().0),
                        ),
                        value,
                    ),
                ],
            };
            let txid = transaction.id();
            let signed_tx = SignedTransaction {
                witnesses: vec![Witness::new(&transaction.id(), &state.faucet_private_key)],
                transaction,
            };
            apply_block1(&mut state, Message::Transaction(signed_tx)).unwrap();
            state.faucet_utxo = UtxoPointer::new(txid, 0, change_value);
            assert_eq!(
                state.leader_selection.get_stake_distribution().0,
                expected_stake_dist
            );
        }

        // Skip to the next epoch. This should cause the stake
        // distribution snapshots to be updated.
        {
            assert_eq!(state.cur_date.epoch, 0);
            state.cur_date = state.cur_date.next_epoch();
            apply_block(&mut state, vec![]).unwrap();
            assert_eq!(state.cur_date.epoch, 1);
            assert_eq!(state.leader_selection.stake_snapshots[&0].0, HashMap::new());
            assert_eq!(
                state.leader_selection.stake_snapshots[&1].0,
                expected_stake_dist
            );
        }

        {
            state.cur_date = state.cur_date.next_epoch();
            apply_block(&mut state, vec![]).unwrap();
            assert_eq!(state.cur_date.epoch, 2);
            assert_eq!(
                state.leader_selection.stake_snapshots[&1].0,
                expected_stake_dist
            );
            assert_eq!(
                state.leader_selection.stake_snapshots[&2].0,
                expected_stake_dist
            );
        }

        // Redelegate to the new pool.
        {
            apply_block1(
                &mut state,
                (certificate::StakeDelegation {
                    stake_key_id: (&sks0).into(),
                    pool_id: (&pool1_private_key).into(),
                })
                .make_certificate(&sks0),
            )
            .unwrap();
            assert_eq!(
                state.leader_selection.get_stake_distribution().0,
                vec![(
                    (&pool1_private_key).into(),
                    PoolStakeDistribution {
                        total_stake: Value(72000),
                        member_stake: vec![
                            ((&sks0).into(), Value(30000)),
                            ((&sks1).into(), Value(42000))
                        ]
                        .iter()
                        .cloned()
                        .collect()
                    }
                )]
                .iter()
                .cloned()
                .collect()
            );
        }

        // Try to deregister a non-existent pool.
        {
            let pool1_private_key = PrivateKey::arbitrary(&mut state.g);
            //let owner1_private_key = PrivateKey::arbitrary(&mut state.g);

            assert_eq!(
                apply_block1(
                    &mut state,
                    (certificate::StakePoolRetirement {
                        pool_id: (&pool1_private_key).into(),
                    })
                    .make_certificate(&pool1_private_key)
                ),
                Err(Error::StakePoolDoesNotExist((&pool1_private_key).into()))
            );
        }

        // Try to deregister a pool with an incorrect certificate signature.
        {
            let signer = PrivateKey::arbitrary(&mut state.g);
            assert_eq!(
                apply_block1(
                    &mut state,
                    (certificate::StakePoolRetirement {
                        pool_id: (&pool0_private_key).into(),
                    })
                    .make_certificate(&signer)
                ),
                Err(Error::StakePoolRetirementSigIsInvalid)
            );
        }

        // Deregister a pool.
        {
            apply_block1(
                &mut state,
                (certificate::StakePoolRetirement {
                    pool_id: (&pool0_private_key).into(),
                })
                .make_certificate(&pool0_private_key),
            )
            .unwrap();
            assert_eq!(state.leader_selection.delegation_state.nr_stake_pools(), 1);
        }

        // Deregister a stake key.
        {
            apply_block1(
                &mut state,
                (certificate::StakeKeyDeregistration {
                    stake_key_id: (&sks0).into(),
                })
                .make_certificate(&sks0),
            )
            .unwrap();
            assert_eq!(state.leader_selection.delegation_state.nr_stake_keys(), 1);
        }

        // Change the 'd' parameter.
        {
            let mut proposal = setting::UpdateProposal::new();
            proposal.bootstrap_key_slots_percentage = Some(80);
            apply_block1(&mut state, Message::Update(proposal)).unwrap();
        }
    }

    pub fn run_mixed(initial_bootstrap_key_slots_percentage: u8) {
        let mut g = StdGen::<StdRng>::new(SeedableRng::seed_from_u64(42), 10);

        let pool0 = PrivateKey::arbitrary(&mut g);
        let pool1 = PrivateKey::arbitrary(&mut g);

        let initial_stake_pools = vec![pool0.clone(), pool1.clone()];

        let sks0 = PrivateKey::arbitrary(&mut g);
        let sks1 = PrivateKey::arbitrary(&mut g);
        let sks2 = PrivateKey::arbitrary(&mut g);

        let mut initial_stake_keys = HashMap::new();
        initial_stake_keys.insert((&sks0).into(), Some((&pool0).into()));
        initial_stake_keys.insert((&sks1).into(), Some((&pool0).into()));
        initial_stake_keys.insert((&sks2).into(), Some((&pool1).into()));

        let skp0 = PrivateKey::arbitrary(&mut g);

        let mut initial_utxos = HashMap::new();

        let utxo = UtxoPointer::new(TransactionId::hash_bytes("a0".as_bytes()), 0, Value(10000));
        initial_utxos.insert(
            utxo,
            Output(
                Address(
                    Discrimination::Test,
                    Kind::Group(skp0.public().0, sks0.public().0),
                ),
                utxo.value,
            ),
        );

        let utxo = UtxoPointer::new(TransactionId::hash_bytes("a1".as_bytes()), 0, Value(20000));
        initial_utxos.insert(
            utxo,
            Output(
                Address(
                    Discrimination::Test,
                    Kind::Group(skp0.public().0, sks1.public().0),
                ),
                utxo.value,
            ),
        );

        let utxo = UtxoPointer::new(TransactionId::hash_bytes("a2".as_bytes()), 0, Value(60000));
        initial_utxos.insert(
            utxo,
            Output(
                Address(
                    Discrimination::Test,
                    Kind::Group(skp0.public().0, sks2.public().0),
                ),
                utxo.value,
            ),
        );

        let mut state = create_chain(
            initial_bootstrap_key_slots_percentage,
            initial_utxos,
            initial_stake_pools,
            initial_stake_keys,
        );

        for _i in 0..1000 {
            apply_block(&mut state, vec![]).unwrap();
        }

        // Note: because genesis leader selection is random, the
        // number of times pool{0,1} are selected depends on the
        // PRNG. We're using a fixed seed but it might not be portable
        // or stable across releases...

        if initial_bootstrap_key_slots_percentage == 0 {
            assert_eq!(state.selected_leaders[&(&pool0).into()], 329);
            assert_eq!(state.selected_leaders[&(&pool1).into()], 671);
            for leader in &state.bft_leaders {
                assert!(!state.selected_leaders.contains_key(&leader.into()));
            }
        } else if initial_bootstrap_key_slots_percentage == 20 {
            assert_eq!(state.selected_leaders[&(&pool0).into()], 254);
            assert_eq!(state.selected_leaders[&(&pool1).into()], 546);
            for leader in &state.bft_leaders {
                assert_eq!(state.selected_leaders[&leader.into()], 20);
            }
        } else if initial_bootstrap_key_slots_percentage == 50 {
            assert_eq!(state.selected_leaders[&(&pool0).into()], 169);
            assert_eq!(state.selected_leaders[&(&pool1).into()], 331);
            for leader in &state.bft_leaders {
                assert_eq!(state.selected_leaders[&leader.into()], 50);
            }
        } else {
            unimplemented!();
        }

        // Skip one or more epochs.
        assert_eq!(state.cur_date.slot_id, 0);

        {
            state.cur_date = state.cur_date.next_epoch();
            apply_block(&mut state, vec![]).unwrap();
            apply_block(&mut state, vec![]).unwrap();
        }

        assert_ne!(state.cur_date.slot_id, 0);

        {
            state.cur_date = state.cur_date.next_epoch();
            apply_block(&mut state, vec![]).unwrap();
            apply_block(&mut state, vec![]).unwrap();
        }

        {
            state.cur_date = state.cur_date.next_epoch().next_epoch();
            apply_block(&mut state, vec![]).unwrap();
            apply_block(&mut state, vec![]).unwrap();
        }

        {
            state.cur_date = state.cur_date.next_epoch().next_epoch().next_epoch();
            apply_block(&mut state, vec![]).unwrap();
            apply_block(&mut state, vec![]).unwrap();
        }
    }

    #[test]
    pub fn pure_genesis() {
        run_mixed(0);
    }

    #[test]
    pub fn mixed_genesis_20() {
        run_mixed(20);
    }

    #[test]
    pub fn mixed_genesis_50() {
        run_mixed(50);
    }

}
*/
