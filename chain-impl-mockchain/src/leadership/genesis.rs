use crate::block::{Block, BlockDate, Message, Proof};
use crate::certificate;
use crate::date::Epoch;
use crate::leadership::{BftLeader, Error, ErrorKind, GenesisPraosLeader, PublicLeader, Update};
use crate::ledger::Ledger;
use crate::setting::{self, Settings};
use crate::stake::*;
use crate::update::ValueDiff;
use crate::value::Value;

use chain_core::property::Block as _;
use chain_core::property::{self, LeaderSelection, Update as _};

use rand::{Rng, SeedableRng};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct GenesisLeaderSelection {
    settings: Arc<RwLock<Settings>>,

    bft_leaders: Vec<BftLeader>,

    delegation_state: DelegationState,

    /// Stake snapshots for recent epochs. They contain the stake
    /// distribution at the *start* of the corresponding epoch
    /// (i.e. before applying any blocks in that epoch). Thus
    /// `stake_snapshots[0]` denotes the initial stake distribution.
    stake_snapshots: BTreeMap<Epoch, StakeDistribution>,

    pos: Pos,
}

#[derive(Debug, Clone)]
struct Pos {
    next_date: BlockDate,
    bft_blocks: usize,
    genesis_blocks: usize, // FIXME: "genesis block" is rather ambiguous...
}

#[derive(Debug, PartialEq)]
pub enum GenesisPraosError {
    BlockHasInvalidLeader(PublicLeader, PublicLeader),
    BlockSignatureIsInvalid,
    UpdateHasInvalidCurrentLeader(PublicLeader, PublicLeader),
    UpdateIsInvalid, // FIXME: add specific errors for all fields?
    StakeKeyRegistrationSigIsInvalid,
    StakeKeyDeregistrationSigIsInvalid,
    StakeDelegationSigIsInvalid,
    StakeDelegationStakeKeyIsInvalid(StakeKeyId),
    StakeDelegationPoolKeyIsInvalid(StakePoolId),
    StakePoolRegistrationPoolSigIsInvalid,
    StakePoolAlreadyExists(StakePoolId),
    StakePoolRetirementSigIsInvalid,
    StakePoolDoesNotExist(StakePoolId),
}

impl std::fmt::Display for GenesisPraosError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GenesisPraosError::BlockHasInvalidLeader(expected, found) => write!(
                f,
                "Invalid block leader, expected {:?} but the given block was signed by {:?}",
                expected, found
            ),
            GenesisPraosError::BlockSignatureIsInvalid => write!(f, "The block signature is not valid"),
            GenesisPraosError::UpdateHasInvalidCurrentLeader(current, found) => write!(
                f,
                "Update has an incompatible leader, we expect to update from {:?} but we are at {:?}",
                found, current
            ),
            GenesisPraosError::UpdateIsInvalid => write!(
                f,
                "Update does not apply to current state"
            ),
            GenesisPraosError::StakeKeyRegistrationSigIsInvalid => write!(
                f,
                "Block has a stake key registration certificate with an invalid signature"
            ),
            GenesisPraosError::StakeKeyDeregistrationSigIsInvalid => write!(
                f,
                "Block has a stake key deregistration certificate with an invalid signature"
            ),
            GenesisPraosError::StakeDelegationSigIsInvalid => write!(
                f,
                "Block has a stake delegation certificate with an invalid signature"
            ),
            GenesisPraosError::StakeDelegationStakeKeyIsInvalid(stake_key_id) => write!(
                f,
                "Block has a stake delegation certificate that delegates from a stake key '{:?} that does not exist",
                stake_key_id
            ),
            GenesisPraosError::StakeDelegationPoolKeyIsInvalid(pool_id) => write!(
                f,
                "Block has a stake delegation certificate that delegates to a pool '{:?} that does not exist",
                pool_id
            ),
            GenesisPraosError::StakePoolRegistrationPoolSigIsInvalid => write!(
                f,
                "Block has a pool registration certificate with an invalid pool signature"
            ),
            GenesisPraosError::StakePoolAlreadyExists(pool_id) => write!(
                f,
                "Block attempts to register pool '{:?}' which already exists",
                pool_id
            ),
            GenesisPraosError::StakePoolRetirementSigIsInvalid => write!(
                f,
                "Block has a pool retirement certificate with an invalid pool signature"
            ),
            GenesisPraosError::StakePoolDoesNotExist(pool_id) => write!(
                f,
                "Block references a pool '{:?}' which does not exist",
                pool_id
            ),
        }
    }
}

impl std::error::Error for GenesisPraosError {}

impl GenesisLeaderSelection {
    /// Create a new Genesis leadership
    pub fn new(
        bft_leaders: Vec<BftLeader>,
        ledger: Arc<RwLock<Ledger>>,
        settings: Arc<RwLock<Settings>>,
        initial_stake_pools: Vec<StakePoolInfo>,
        initial_stake_keys: HashMap<StakeKeyId, Option<StakePoolId>>,
    ) -> Option<Self> {
        if bft_leaders.len() == 0 {
            return None;
        }

        let mut result = GenesisLeaderSelection {
            ledger,
            settings,
            bft_leaders: bft_leaders,
            pos: Pos {
                next_date: BlockDate::first(),
                bft_blocks: 0,
                genesis_blocks: 0,
            },
            delegation_state: DelegationState::new(initial_stake_pools, initial_stake_keys),
            stake_snapshots: BTreeMap::new(),
        };

        result
            .stake_snapshots
            .insert(0, result.get_stake_distribution());

        Some(result)
    }

    fn advance_to(&self, to_date: BlockDate) -> (Pos, PublicLeader) {
        let state_epoch = if self.pos.next_date.slot_id == 0 && self.pos.next_date.epoch > 0 {
            self.pos.next_date.epoch - 1
        } else {
            self.pos.next_date.epoch
        };

        let mut now = self.pos.clone();

        let d = self.settings.read().unwrap().bootstrap_key_slots_percentage;

        loop {
            assert!(now.next_date <= to_date);

            let done = now.next_date == to_date;

            let cur_epoch = now.next_date.epoch;

            now.next_date = now.next_date.next();

            // Base leadership selection on the stake distribution at
            // the start of the previous epoch.
            let stake_snapshot = if cur_epoch == 0 {
                // We're still in the first epoch, so use the initial stake distribution.
                Cow::Borrowed(&self.stake_snapshots[&0])
            } else if cur_epoch == state_epoch || cur_epoch == state_epoch + 1 {
                if let Some(snapshot) = self.stake_snapshots.get(&(cur_epoch - 1)) {
                    // Use the stake distribution at the start of the previous epoch.
                    Cow::Borrowed(snapshot)
                } else {
                    // We don't have the stake distribution at the
                    // start of the previous epoch, which can happen
                    // if the last block skipped a whole epoch. So use
                    // the distribution at the start of that block's
                    // epoch. The distribution in the epoch before
                    // must be the same, since there were no blocks
                    // that could have changed it.
                    assert_eq!(self.stake_snapshots.len(), 1);
                    Cow::Borrowed(&self.stake_snapshots[&(cur_epoch)])
                }
            } else if cur_epoch > state_epoch + 1 {
                // We've advanced so far that we to use the current
                // snapshot. FIXME: cache this across the loop.
                Cow::Owned(self.get_stake_distribution())
            } else {
                unreachable!()
            };

            // If we didn't have eligible stake pools in the epoch
            // used for sampling, then we have to use BFT rules.
            // FIXME: require a certain minimum number of stake pools?
            let have_stakeholders = stake_snapshot.eligible_stake_pools() > 0;

            let is_bft_slot = d == setting::SLOTS_PERCENTAGE_RANGE
                || !have_stakeholders
                || now.bft_blocks * (setting::SLOTS_PERCENTAGE_RANGE as usize)
                    < (d as usize) * (now.bft_blocks + now.genesis_blocks);

            if is_bft_slot {
                let cur_bft_leader = now.bft_blocks;
                now.bft_blocks += 1;
                if done {
                    return (
                        now,
                        PublicLeader::Bft(
                            self.bft_leaders[cur_bft_leader % self.bft_leaders.len()].clone(),
                        ),
                    );
                }
            } else {
                now.genesis_blocks += 1;
                if done {
                    // FIXME: the following is a placeholder for a
                    // proper VRF-based leader selection.

                    // Calculate the total stake.
                    let total_stake: Value = stake_snapshot.total_stake();

                    assert!(total_stake.0 > 0);

                    // Pick a random point in the range [0, total_stake).
                    let mut rng: rand::rngs::StdRng = SeedableRng::seed_from_u64(
                        (to_date.epoch as u64) << 32 | to_date.slot_id as u64,
                    );
                    let point = rng.gen_range(0, total_stake.0);

                    // Select the stake pool containing the point we
                    // picked.
                    let pool_id = stake_snapshot.select_pool(point).unwrap();
                    let pool_info = self
                        .delegation_state
                        .get_stake_pools()
                        .get(&pool_id)
                        .unwrap();
                    let keys = GenesisPraosLeader {
                        kes_public_key: pool_info.kes_public_key.clone(),
                        vrf_public_key: pool_info.vrf_public_key.clone(),
                    };
                    return (now, PublicLeader::GenesisPraos(keys));
                }
            }
        }
    }

    pub fn get_stake_distribution(&self) -> StakeDistribution {
        self.delegation_state
            .get_stake_distribution(&self.ledger.read().unwrap())
    }

    pub fn get_delegation_state(&self) -> &DelegationState {
        &self.delegation_state
    }
}

#[derive(Debug, Clone)]
pub struct GenesisSelectionDiff {
    next_date: ValueDiff<BlockDate>,
    bft_blocks: ValueDiff<usize>,
    genesis_blocks: ValueDiff<usize>,
    stake_key_registrations: HashSet<StakeKeyId>,
    stake_key_deregistrations: HashSet<StakeKeyId>,
    new_stake_pools: HashMap<StakePoolId, certificate::StakePoolRegistration>,
    retired_stake_pools: HashSet<StakePoolId>,
    delegations: HashMap<StakeKeyId, StakePoolId>,
    stake_snapshots: Option<BTreeMap<Epoch, StakeDistribution>>,
}

impl property::Update for GenesisSelectionDiff {
    fn empty() -> Self {
        GenesisSelectionDiff {
            next_date: ValueDiff::None,
            bft_blocks: ValueDiff::None,
            genesis_blocks: ValueDiff::None,
            stake_key_registrations: HashSet::new(),
            stake_key_deregistrations: HashSet::new(),
            new_stake_pools: HashMap::new(),
            retired_stake_pools: HashSet::new(),
            delegations: HashMap::new(),
            stake_snapshots: None,
        }
    }
    fn inverse(self) -> Self {
        unimplemented!()
    }
    fn union(&mut self, other: Self) -> &mut Self {
        self.next_date.union(other.next_date);
        self.bft_blocks.union(other.bft_blocks);
        self.genesis_blocks.union(other.genesis_blocks);
        self.stake_key_registrations
            .extend(other.stake_key_registrations);
        self.stake_key_deregistrations
            .extend(other.stake_key_deregistrations);

        // TODO: if a pool was registered in self and retired in
        // other, then we need to remove it from both new_stake_pools
        // and retired_stake_pools. However, if this was a
        // re-registration, then we need to keep the entry in
        // retired_stake_pools.
        self.new_stake_pools.extend(other.new_stake_pools);
        self.retired_stake_pools.extend(other.retired_stake_pools);

        // Note: this overwrites delegations, so the most recent
        // delegation takes precedence.
        self.delegations.extend(other.delegations);

        if let Some(stake_snapshots) = other.stake_snapshots {
            self.stake_snapshots = Some(stake_snapshots);
        }

        self
    }
}

impl LeaderSelection for GenesisLeaderSelection {
    type Block = Block;
    type Error = Error;
    type LeaderId = PublicLeader;
    /*
        fn diff(&self, input: &Self::Block) -> Result<Self::Update, Self::Error> {
            let mut update = <Self::Update as property::Update>::empty();

            let date = input.date();

            let (new_pos, leader) = self.advance_to(date);

            assert_eq!(new_pos.next_date, date.next());

            match &input.header.proof() {
                Proof::None => unimplemented!(),
                Proof::GenesisPraos(genesis_praos_proof) => {
                    if let PublicLeader::GenesisPraos(genesis_praos_leader) = &leader {
                        if &genesis_praos_proof.vrf_public_key != &genesis_praos_leader.vrf_public_key
                            || &genesis_praos_proof.kes_public_key
                                != &genesis_praos_leader.kes_public_key
                        {
                            return Err(Error {
                                kind: ErrorKind::InvalidLeader,
                                cause: Some(Box::new(GenesisPraosError::BlockHasInvalidLeader(
                                    leader.clone(),
                                    PublicLeader::GenesisPraos(GenesisPraosLeader {
                                        kes_public_key: genesis_praos_proof.kes_public_key.clone(),
                                        vrf_public_key: genesis_praos_proof.vrf_public_key.clone(),
                                    }),
                                ))),
                            });
                        }
                    } else {
                        // TODO: error, we would expect a GENESIS leader in the case of a GenesisPraos proof
                    }
                }
                Proof::Bft(bft_proof) => {
                    if let PublicLeader::Bft(bft_leader) = &leader {
                        if &bft_proof.leader_id != bft_leader {
                            return Err(Error {
                                kind: ErrorKind::InvalidLeader,
                                cause: Some(Box::new(GenesisPraosError::BlockHasInvalidLeader(
                                    leader.clone(),
                                    PublicLeader::Bft(bft_proof.leader_id.clone()),
                                ))),
                            });
                        }
                    } else {
                        // TODO: this is an error, we need to only accept BFT leader in the
                        // case of a BFT proof
                    }
                }
            }

            if !input.verify() {
                return Err(Error {
                    kind: ErrorKind::InvalidLeaderSignature,
                    cause: Some(Box::new(GenesisPraosError::BlockSignatureIsInvalid)),
                });
            }

            // If we crossed into a new epoch, then update the stake
            // distribution snapshots. NOTE: this is a snapshot of the
            // stake *before* applying the ledger changes in this block
            // (because this is the stake distribution at the very start
            // of the epoch). TODO: When we merge leadership and ledger,
            // we need to take care that we don't break this.
            if date.epoch != self.pos.next_date.epoch
                || (self.pos.next_date.slot_id == 0 && self.pos.next_date.epoch > 0)
            {
                let mut snapshots: BTreeMap<Epoch, StakeDistribution> = self
                    .stake_snapshots
                    .iter()
                    .filter(|(epoch, _snapshot)| *epoch + 1 >= date.epoch)
                    .map(|(epoch, snapshot)| (*epoch, snapshot.clone()))
                    .collect();
                snapshots.insert(date.epoch, self.get_stake_distribution());
                assert!(snapshots.len() <= 2);
                update.genesis.stake_snapshots = Some(snapshots);
            }

            for msg in input.contents.iter() {
                match msg {
                    Message::StakeKeyRegistration(reg) => {
                        if crate::key::verify_signature(&reg.sig, &reg.data.stake_key_id.0, &reg.data)
                            == chain_crypto::Verification::Failed
                        {
                            return Err(Error {
                                kind: ErrorKind::InvalidBlockMessage,
                                cause: Some(Box::new(
                                    GenesisPraosError::StakeKeyRegistrationSigIsInvalid,
                                )),
                            });
                        }

                        // FIXME: should it be an error to register an
                        // already registered stake key?
                        if !self
                            .delegation_state
                            .stake_key_exists(&reg.data.stake_key_id)
                        {
                            // FIXME: need to handle a block that both
                            // deregisters *and* re-registers a stake
                            // key. Probably that should void the reward
                            // account (rather than be a no-op).
                            assert!(!update
                                .genesis
                                .stake_key_deregistrations
                                .contains(&reg.data.stake_key_id));

                            update
                                .genesis
                                .stake_key_registrations
                                .insert(reg.data.stake_key_id.clone());
                        }
                    }

                    Message::StakeKeyDeregistration(reg) => {
                        if crate::key::verify_signature(&reg.sig, &reg.data.stake_key_id.0, &reg.data)
                            == chain_crypto::Verification::Failed
                        {
                            return Err(Error {
                                kind: ErrorKind::InvalidBlockMessage,
                                cause: Some(Box::new(
                                    GenesisPraosError::StakeKeyDeregistrationSigIsInvalid,
                                )),
                            });
                        }

                        if self
                            .delegation_state
                            .stake_key_exists(&reg.data.stake_key_id)
                        {
                            // FIXME: for now, ban registrations and
                            // deregistrations of a key in the same
                            // block.
                            assert!(!update
                                .genesis
                                .stake_key_registrations
                                .contains(&reg.data.stake_key_id));
                            update
                                .genesis
                                .stake_key_deregistrations
                                .insert(reg.data.stake_key_id.clone());
                        }
                    }

                    Message::StakeDelegation(reg) => {
                        if crate::key::verify_signature(&reg.sig, &reg.data.stake_key_id.0, &reg.data)
                            == chain_crypto::Verification::Failed
                        {
                            return Err(Error {
                                kind: ErrorKind::InvalidBlockMessage,
                                cause: Some(Box::new(GenesisPraosError::StakeDelegationSigIsInvalid)),
                            });
                        }

                        // FIXME: should it be allowed to register a stake
                        // key and delegate from it in the same
                        // transaction? Probably yes.
                        if !self
                            .delegation_state
                            .stake_key_exists(&reg.data.stake_key_id)
                        {
                            return Err(Error {
                                kind: ErrorKind::InvalidBlockMessage,
                                cause: Some(Box::new(
                                    GenesisPraosError::StakeDelegationStakeKeyIsInvalid(
                                        reg.data.stake_key_id.clone(),
                                    ),
                                )),
                            });
                        }

                        // FIXME: should it be allowed to create a stake
                        // pool and delegate to it in the same
                        // transaction?
                        if !self.delegation_state.stake_pool_exists(&reg.data.pool_id) {
                            return Err(Error {
                                kind: ErrorKind::InvalidBlockMessage,
                                cause: Some(Box::new(
                                    GenesisPraosError::StakeDelegationPoolKeyIsInvalid(
                                        reg.data.pool_id.clone(),
                                    ),
                                )),
                            });
                        }

                        update
                            .genesis
                            .delegations
                            .insert(reg.data.stake_key_id.clone(), reg.data.pool_id.clone());
                    }

                    Message::StakePoolRegistration(reg) => {
                        if crate::key::verify_signature(&reg.sig, &reg.data.pool_id.0, &reg.data)
                            == chain_crypto::Verification::Failed
                        {
                            return Err(Error {
                                kind: ErrorKind::InvalidBlockMessage,
                                cause: Some(Box::new(
                                    GenesisPraosError::StakePoolRegistrationPoolSigIsInvalid,
                                )),
                            });
                        }

                        if self.delegation_state.stake_pool_exists(&reg.data.pool_id)
                            || update
                                .genesis
                                .new_stake_pools
                                .contains_key(&reg.data.pool_id)
                        {
                            // FIXME: support re-registration to change pool parameters.
                            return Err(Error {
                                kind: ErrorKind::InvalidBlockMessage,
                                cause: Some(Box::new(GenesisPraosError::StakePoolAlreadyExists(
                                    reg.data.pool_id.clone(),
                                ))),
                            });
                        }

                        // FIXME: check owner_sig

                        // FIXME: should pool_id be a previously registered stake key?

                        update
                            .genesis
                            .new_stake_pools
                            .insert(reg.data.pool_id.clone(), reg.data.clone());
                    }

                    Message::StakePoolRetirement(ret) => {
                        if self.delegation_state.stake_pool_exists(&ret.data.pool_id) {
                            if crate::key::verify_signature(&ret.sig, &ret.data.pool_id.0, &ret.data)
                                == chain_crypto::Verification::Failed
                            {
                                return Err(Error {
                                    kind: ErrorKind::InvalidBlockMessage,
                                    cause: Some(Box::new(
                                        GenesisPraosError::StakePoolRetirementSigIsInvalid,
                                    )),
                                });
                            }
                            update
                                .genesis
                                .retired_stake_pools
                                .insert(ret.data.pool_id.clone());
                        } else {
                            return Err(Error {
                                kind: ErrorKind::InvalidBlockMessage,
                                cause: Some(Box::new(GenesisPraosError::StakePoolDoesNotExist(
                                    ret.data.pool_id.clone(),
                                ))),
                            });
                        }
                    }

                    _ => {}
                }
            }

            update.genesis.next_date = ValueDiff::Replace(self.pos.next_date, new_pos.next_date);
            update.genesis.bft_blocks = ValueDiff::Replace(self.pos.bft_blocks, new_pos.bft_blocks);
            update.genesis.genesis_blocks =
                ValueDiff::Replace(self.pos.genesis_blocks, new_pos.genesis_blocks);

            Ok(update)
        }

        fn apply(&mut self, update: Self::Update) -> Result<(), Self::Error> {
            if !update.genesis.next_date.check(&self.pos.next_date)
                || !update.genesis.bft_blocks.check(&self.pos.bft_blocks)
                || !update
                    .genesis
                    .genesis_blocks
                    .check(&self.pos.genesis_blocks)
            {
                return Err(Error {
                    kind: ErrorKind::InvalidStateUpdate,
                    cause: Some(Box::new(GenesisPraosError::UpdateIsInvalid)),
                });
            }

            for stake_key_id in update.genesis.stake_key_registrations {
                self.delegation_state.register_stake_key(stake_key_id);
            }

            for stake_key_id in update.genesis.stake_key_deregistrations {
                self.delegation_state.deregister_stake_key(&stake_key_id);
            }

            for (pool_id, new_stake_pool) in update.genesis.new_stake_pools {
                self.delegation_state.register_stake_pool(
                    pool_id,
                    new_stake_pool.kes_public_key,
                    new_stake_pool.vrf_public_key,
                );
            }

            for (stake_key_id, pool_id) in update.genesis.delegations {
                self.delegation_state.delegate_stake(stake_key_id, pool_id);
            }

            // FIXME: the pool should be retired at the end of a specified epoch.
            for pool_id in update.genesis.retired_stake_pools {
                self.delegation_state.deregister_stake_pool(&pool_id);
            }

            update.genesis.next_date.apply_to(&mut self.pos.next_date);
            update.genesis.bft_blocks.apply_to(&mut self.pos.bft_blocks);
            update
                .genesis
                .genesis_blocks
                .apply_to(&mut self.pos.genesis_blocks);

            if let Some(stake_snapshots) = update.genesis.stake_snapshots {
                self.stake_snapshots = stake_snapshots;
            }

            Ok(())
        }
    */
    fn get_leader_at(
        &self,
        date: <Self::Block as property::Block>::Date,
    ) -> Result<Self::LeaderId, Self::Error> {
        let (_pos, stakeholder_id) = self.advance_to(date);
        Ok(stakeholder_id)
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
