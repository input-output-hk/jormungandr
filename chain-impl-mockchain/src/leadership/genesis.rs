use crate::block::{BlockDate, Message, SignedBlock};
use crate::certificate;
use crate::key::PublicKey;
use crate::leadership::bft::BftRoundRobinIndex;
use crate::ledger::Ledger;
use crate::setting::{self, Settings};
use crate::stake::*;
use crate::transaction::Value;
use crate::update::ValueDiff;

use chain_addr::Kind;
use chain_core::property::{self, Block, LeaderSelection, Update};

use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct GenesisLeaderSelection {
    ledger: Arc<RwLock<Ledger>>,
    settings: Arc<RwLock<Settings>>,

    bft_leaders: Vec<PublicKey>,

    stake_keys: HashMap<StakeKeyId, StakeKeyInfo>,

    stake_pools: HashMap<StakePoolId, StakePoolInfo>,

    /// The stake distribution at the end of the previous epoch.
    stake_snapshot_n_minus_1: StakeDistribution,

    /// The stake distribution at the end of the previous previous
    /// epoch.
    stake_snapshot_n_minus_2: StakeDistribution,

    pos: Pos,
}

#[derive(Debug, Clone)]
struct Pos {
    next_date: BlockDate,
    next_bft_leader_index: BftRoundRobinIndex,
    bft_blocks: usize,
    genesis_blocks: usize, // FIXME: "genesis block" is rather ambiguous...
}

#[derive(Debug, PartialEq)]
pub enum Error {
    BlockHasInvalidLeader(PublicKey, PublicKey),
    BlockSignatureIsInvalid,
    UpdateHasInvalidCurrentLeader(PublicKey, PublicKey),
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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::BlockHasInvalidLeader(expected, found) => write!(
                f,
                "Invalid block leader, expected {:?} but the given block was signed by {:?}",
                expected, found
            ),
            Error::BlockSignatureIsInvalid => write!(f, "The block signature is not valid"),
            Error::UpdateHasInvalidCurrentLeader(current, found) => write!(
                f,
                "Update has an incompatible leader, we expect to update from {:?} but we are at {:?}",
                found, current
            ),
            Error::UpdateIsInvalid => write!(
                f,
                "Update does not apply to current state"
            ),
            Error::StakeKeyRegistrationSigIsInvalid => write!(
                f,
                "Block has a stake key registration certificate with an invalid signature"
            ),
            Error::StakeKeyDeregistrationSigIsInvalid => write!(
                f,
                "Block has a stake key deregistration certificate with an invalid signature"
            ),
            Error::StakeDelegationSigIsInvalid => write!(
                f,
                "Block has a stake delegation certificate with an invalid signature"
            ),
            Error::StakeDelegationStakeKeyIsInvalid(stake_key_id) => write!(
                f,
                "Block has a stake delegation certificate that delegates from a stake key '{:?} that does not exist",
                stake_key_id
            ),
            Error::StakeDelegationPoolKeyIsInvalid(pool_id) => write!(
                f,
                "Block has a stake delegation certificate that delegates to a pool '{:?} that does not exist",
                pool_id
            ),
            Error::StakePoolRegistrationPoolSigIsInvalid => write!(
                f,
                "Block has a pool registration certificate with an invalid pool signature"
            ),
            Error::StakePoolAlreadyExists(pool_id) => write!(
                f,
                "Block attempts to register pool '{:?}' which already exists",
                pool_id
            ),
            Error::StakePoolRetirementSigIsInvalid => write!(
                f,
                "Block has a pool retirement certificate with an invalid pool signature"
            ),
            Error::StakePoolDoesNotExist(pool_id) => write!(
                f,
                "Block references a pool '{:?}' which does not exist",
                pool_id
            ),
        }
    }
}

impl std::error::Error for Error {}

impl GenesisLeaderSelection {
    /// Create a new Genesis leadership
    pub fn new(
        bft_leaders: Vec<PublicKey>,
        ledger: Arc<RwLock<Ledger>>,
        settings: Arc<RwLock<Settings>>,
        initial_stake_pools: HashSet<StakePoolId>,
        initial_stake_keys: HashMap<StakeKeyId, Option<StakePoolId>>,
    ) -> Option<Self> {
        if bft_leaders.len() == 0 {
            return None;
        }

        let mut stake_pools: HashMap<StakePoolId, StakePoolInfo> = initial_stake_pools
            .into_iter()
            .map(|pool_id| {
                (
                    pool_id,
                    StakePoolInfo {
                        members: HashSet::new(),
                    },
                )
            })
            .collect();

        let mut stake_keys = HashMap::new();
        for (stake_key_id, pool_id) in initial_stake_keys {
            if let Some(pool_id) = &pool_id {
                if let Some(pool) = stake_pools.get_mut(&pool_id) {
                    pool.members.insert(stake_key_id.clone());
                } else {
                    panic!("Pool '{:?}' does not exist.", pool_id)
                }
            }
            stake_keys.insert(stake_key_id, StakeKeyInfo { pool: pool_id });
        }

        let mut result = GenesisLeaderSelection {
            ledger,
            settings,
            bft_leaders: bft_leaders,
            pos: Pos {
                next_date: BlockDate::first(),
                next_bft_leader_index: BftRoundRobinIndex(0),
                bft_blocks: 0,
                genesis_blocks: 0,
            },
            stake_keys,
            stake_pools,
            stake_snapshot_n_minus_1: HashMap::new(),
            stake_snapshot_n_minus_2: HashMap::new(),
        };

        result.stake_snapshot_n_minus_1 = result.get_stake_distribution();
        result.stake_snapshot_n_minus_2 = result.stake_snapshot_n_minus_1.clone();

        Some(result)
    }

    fn advance_to(&self, to_date: BlockDate) -> (Pos, PublicKey) {
        let mut now = self.pos.clone();

        let d = self.settings.read().unwrap().bootstrap_key_slots_percentage;

        loop {
            assert!(now.next_date <= to_date);

            let done = now.next_date == to_date;

            now.next_date = now.next_date.next();

            // FIXME: handle the case were we're advancing so far
            // (i.e. crossing an epoch) that we have to use
            // stake_snapshot_n_minus_1, or even calculate a new
            // snapshot.
            let stake_snapshot = &self.stake_snapshot_n_minus_2;

            // If we didn't have eligible stake pools in the epoch
            // used for sampling, then we have to use BFT rules.
            // FIXME: require a certain minimum number of stake pools?
            let have_stakeholders = !stake_snapshot.is_empty();

            let is_bft_slot = d == setting::SLOTS_PERCENTAGE_RANGE
                || !have_stakeholders
                || now.bft_blocks * (setting::SLOTS_PERCENTAGE_RANGE as usize)
                    < (d as usize) * (now.bft_blocks + now.genesis_blocks);

            if is_bft_slot {
                now.bft_blocks += 1;
                let bft_leader_index = now.next_bft_leader_index.0;
                now.next_bft_leader_index =
                    BftRoundRobinIndex((bft_leader_index + 1) % self.bft_leaders.len());
                if done {
                    return (now, self.bft_leaders[bft_leader_index].clone());
                }
            } else {
                now.genesis_blocks += 1;
                if done {
                    // FIXME: the following is a placeholder for a
                    // proper VRF-based leader selection.

                    // Calculate the total stake.
                    let total_stake: Value = stake_snapshot
                        .iter()
                        .map(|(_, (pool_stake, _))| pool_stake)
                        .fold(Value(0), |sum, &x| sum + x);

                    assert!(total_stake.0 > 0);

                    // Pick a random point in the range [0, total_stake).
                    let mut rng: rand::rngs::StdRng =
                        SeedableRng::seed_from_u64(u64::from(&to_date));
                    let mut point = rng.gen_range(0, total_stake.0);

                    // Sort the pools by public key.
                    let mut pools_sorted: Vec<_> = stake_snapshot
                        .iter()
                        .map(|(pool_id, (pool_stake, _))| (pool_id, pool_stake))
                        .collect();

                    pools_sorted.sort();

                    for (pool_id, pool_stake) in pools_sorted {
                        if point < pool_stake.0 {
                            return (now, pool_id.0.clone());
                        }
                        point -= pool_stake.0
                    }

                    unreachable!();
                }
            }
        }
    }

    pub fn get_stake_distribution(&self) -> StakeDistribution {
        let mut dist = StakeDistribution::new();

        for (ptr, output) in self.ledger.read().unwrap().unspent_outputs.iter() {
            assert_eq!(ptr.value, output.1);

            // We're only interested in "group" addresses
            // (i.e. containing a spending key and a stake key).
            if let Kind::Group(_spending_key, stake_key) = output.0.kind() {
                // Grmbl.
                let stake_key = PublicKey(stake_key.clone()).into();

                // Do we have a stake key for this spending key?
                if let Some(stake_key_info) = self.stake_keys.get(&stake_key) {
                    // Is this stake key a member of a stake pool?
                    if let Some(pool_id) = &stake_key_info.pool {
                        let pool = &self.stake_pools[pool_id];
                        debug_assert!(pool.members.contains(&stake_key));
                        let stake_pool_dist = dist
                            .entry(pool_id.clone())
                            .or_insert((Value(0), HashMap::new()));
                        stake_pool_dist.0 += ptr.value;
                        let member_dist = stake_pool_dist
                            .1
                            .entry(stake_key.clone())
                            .or_insert(Value(0));
                        *member_dist += ptr.value;
                    }
                }
            }
        }

        dist
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GenesisSelectionDiff {
    next_date: ValueDiff<BlockDate>,
    next_bft_leader_index: ValueDiff<BftRoundRobinIndex>,
    bft_blocks: ValueDiff<usize>,
    genesis_blocks: ValueDiff<usize>,
    stake_key_registrations: HashSet<StakeKeyId>,
    stake_key_deregistrations: HashSet<StakeKeyId>,
    new_stake_pools: HashMap<StakePoolId, certificate::StakePoolRegistration>,
    retired_stake_pools: HashSet<StakePoolId>,
    delegations: HashMap<StakeKeyId, StakePoolId>,
    stake_snapshot_n_minus_1: Option<StakeDistribution>,
    stake_snapshot_n_minus_2: Option<StakeDistribution>,
}

impl Update for GenesisSelectionDiff {
    fn empty() -> Self {
        GenesisSelectionDiff {
            next_date: ValueDiff::None,
            next_bft_leader_index: ValueDiff::None,
            bft_blocks: ValueDiff::None,
            genesis_blocks: ValueDiff::None,
            stake_key_registrations: HashSet::new(),
            stake_key_deregistrations: HashSet::new(),
            new_stake_pools: HashMap::new(),
            retired_stake_pools: HashSet::new(),
            delegations: HashMap::new(),
            stake_snapshot_n_minus_1: None,
            stake_snapshot_n_minus_2: None,
        }
    }
    fn inverse(self) -> Self {
        unimplemented!()
    }
    fn union(&mut self, other: Self) -> &mut Self {
        self.next_date.union(other.next_date);
        self.next_bft_leader_index
            .union(other.next_bft_leader_index);
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

        if let Some(snapshot) = other.stake_snapshot_n_minus_1 {
            self.stake_snapshot_n_minus_1 = Some(snapshot);
        }

        if let Some(snapshot) = other.stake_snapshot_n_minus_2 {
            self.stake_snapshot_n_minus_2 = Some(snapshot);
        }

        self
    }
}

impl LeaderSelection for GenesisLeaderSelection {
    type Update = GenesisSelectionDiff;
    type Block = SignedBlock;
    type Error = Error;
    type LeaderId = PublicKey;

    fn diff(&self, input: &Self::Block) -> Result<Self::Update, Self::Error> {
        let mut update = <Self::Update as property::Update>::empty();

        let date = input.date();

        let (new_pos, leader) = self.advance_to(date);

        assert_eq!(new_pos.next_date, date.next());

        if leader != input.public_key {
            return Err(Error::BlockHasInvalidLeader(
                leader,
                input.public_key.clone(),
            ));
        }

        if !input.verify() {
            return Err(Error::BlockSignatureIsInvalid);
        }

        // If we crossed into a new epoch, then update the stake
        // distribution snapshots.
        if date.epoch != self.pos.next_date.epoch || self.pos.next_date.slot_id == 0 {
            update.stake_snapshot_n_minus_1 = Some(self.get_stake_distribution());
            let n = date - self.pos.next_date;
            // Shift the snapshot for the previous epoch, keeping in
            // mind the case where we jumped more than an epoch.
            if n < crate::date::EPOCH_DURATION {
                update.stake_snapshot_n_minus_2 = Some(self.stake_snapshot_n_minus_1.clone());
            } else {
                // FIXME: add test case.
                update.stake_snapshot_n_minus_2 = update.stake_snapshot_n_minus_1.clone();
            }
        }

        for msg in input.block.contents.iter() {
            match msg {
                Message::StakeKeyRegistration(reg) => {
                    if !reg
                        .data
                        .stake_key_id
                        .0
                        .serialize_and_verify(&reg.data, &reg.sig)
                    {
                        return Err(Error::StakeKeyRegistrationSigIsInvalid);
                    }

                    // FIXME: should it be an error to register an
                    // already registered stake key?
                    if !self.stake_keys.contains_key(&reg.data.stake_key_id) {
                        // FIXME: need to handle a block that both
                        // deregisters *and* re-registers a stake
                        // key. Probably that should void the reward
                        // account (rather than be a no-op).
                        assert!(!update
                            .stake_key_deregistrations
                            .contains(&reg.data.stake_key_id));

                        update
                            .stake_key_registrations
                            .insert(reg.data.stake_key_id.clone());
                    }
                }

                Message::StakeKeyDeregistration(reg) => {
                    if !reg
                        .data
                        .stake_key_id
                        .0
                        .serialize_and_verify(&reg.data, &reg.sig)
                    {
                        return Err(Error::StakeKeyDeregistrationSigIsInvalid);
                    }

                    if self.stake_keys.contains_key(&reg.data.stake_key_id) {
                        // FIXME: for now, ban registrations and
                        // deregistrations of a key in the same
                        // block.
                        assert!(!update
                            .stake_key_registrations
                            .contains(&reg.data.stake_key_id));
                        update
                            .stake_key_deregistrations
                            .insert(reg.data.stake_key_id.clone());
                    }
                }

                Message::StakeDelegation(reg) => {
                    if !reg
                        .data
                        .stake_key_id
                        .0
                        .serialize_and_verify(&reg.data, &reg.sig)
                    {
                        return Err(Error::StakeDelegationSigIsInvalid);
                    }

                    // FIXME: should it be allowed to register a stake
                    // key and delegate from it in the same
                    // transaction? Probably yes.
                    if !self.stake_keys.contains_key(&reg.data.stake_key_id) {
                        return Err(Error::StakeDelegationStakeKeyIsInvalid(
                            reg.data.stake_key_id.clone(),
                        ));
                    }

                    // FIXME: should it be allowed to create a stake
                    // pool and delegate to it in the same
                    // transaction?
                    if !self.stake_pools.contains_key(&reg.data.pool_id) {
                        return Err(Error::StakeDelegationPoolKeyIsInvalid(
                            reg.data.pool_id.clone(),
                        ));
                    }

                    update
                        .delegations
                        .insert(reg.data.stake_key_id.clone(), reg.data.pool_id.clone());
                }

                Message::StakePoolRegistration(reg) => {
                    if !reg.data.pool_id.0.serialize_and_verify(&reg.data, &reg.sig) {
                        return Err(Error::StakePoolRegistrationPoolSigIsInvalid);
                    }

                    if self.stake_pools.contains_key(&reg.data.pool_id)
                        || update.new_stake_pools.contains_key(&reg.data.pool_id)
                    {
                        // FIXME: support re-registration to change pool parameters.
                        return Err(Error::StakePoolAlreadyExists(reg.data.pool_id.clone()));
                    }

                    // FIXME: check owner_sig

                    // FIXME: should pool_id be a previously registered stake key?

                    update
                        .new_stake_pools
                        .insert(reg.data.pool_id.clone(), reg.data.clone());
                }

                Message::StakePoolRetirement(ret) => {
                    match self.stake_pools.get(&ret.data.pool_id) {
                        None => {
                            return Err(Error::StakePoolDoesNotExist(ret.data.pool_id.clone()));
                        }
                        Some(_stake_pool) => {
                            if !ret.data.pool_id.0.serialize_and_verify(&ret.data, &ret.sig) {
                                return Err(Error::StakePoolRetirementSigIsInvalid);
                            }
                            update.retired_stake_pools.insert(ret.data.pool_id.clone());
                        }
                    }
                }

                _ => {}
            }
        }

        update.next_date = ValueDiff::Replace(self.pos.next_date, new_pos.next_date);
        update.next_bft_leader_index = ValueDiff::Replace(
            self.pos.next_bft_leader_index,
            new_pos.next_bft_leader_index,
        );
        update.bft_blocks = ValueDiff::Replace(self.pos.bft_blocks, new_pos.bft_blocks);
        update.genesis_blocks = ValueDiff::Replace(self.pos.genesis_blocks, new_pos.genesis_blocks);

        Ok(update)
    }

    fn apply(&mut self, update: Self::Update) -> Result<(), Self::Error> {
        if !update.next_date.check(&self.pos.next_date)
            || !update
                .next_bft_leader_index
                .check(&self.pos.next_bft_leader_index)
            || !update.bft_blocks.check(&self.pos.bft_blocks)
            || !update.genesis_blocks.check(&self.pos.genesis_blocks)
        {
            return Err(Error::UpdateIsInvalid);
        }

        for stake_key_id in update.stake_key_registrations {
            let inserted = !self
                .stake_keys
                .insert(stake_key_id, StakeKeyInfo { pool: None })
                .is_some();
            assert!(inserted);
        }

        for stake_key_id in update.stake_key_deregistrations {
            let stake_key_info = self.stake_keys.remove(&stake_key_id).unwrap();

            // Remove this stake key from its pool, if any.
            if let Some(pool_id) = stake_key_info.pool {
                self.stake_pools
                    .get_mut(&pool_id)
                    .unwrap()
                    .members
                    .remove(&stake_key_id);
            }
        }

        for (pool_id, _new_stake_pool) in update.new_stake_pools {
            assert!(!self.stake_pools.contains_key(&pool_id));
            self.stake_pools.insert(
                pool_id,
                StakePoolInfo {
                    //owners: new_stake_pool.owners
                    members: HashSet::new(),
                },
            );
        }

        for (stake_key_id, pool_id) in update.delegations {
            let stake_key = self.stake_keys.get_mut(&stake_key_id).unwrap();

            // If this is a redelegation, remove the stake key from its previous pool.
            if let Some(prev_stake_pool) = &stake_key.pool {
                let removed = self
                    .stake_pools
                    .get_mut(&prev_stake_pool)
                    .unwrap()
                    .members
                    .remove(&stake_key_id);
                assert!(removed);
            }

            let stake_pool = self.stake_pools.get_mut(&pool_id).unwrap();
            stake_key.pool = Some(pool_id);
            stake_pool.members.insert(stake_key_id);
        }

        // FIXME: the pool should be retired at the end of a specified epoch.
        for pool_id in update.retired_stake_pools {
            let pool_info = self.stake_pools.remove(&pool_id).unwrap();

            // Remove all pool members.
            for member in pool_info.members {
                let stake_key_info = self.stake_keys.get_mut(&member).unwrap();
                assert_eq!(stake_key_info.pool.as_ref().unwrap(), &pool_id);
                stake_key_info.pool = None;
            }
        }

        update.next_date.apply_to(&mut self.pos.next_date);
        update
            .next_bft_leader_index
            .apply_to(&mut self.pos.next_bft_leader_index);
        update.bft_blocks.apply_to(&mut self.pos.bft_blocks);
        update.genesis_blocks.apply_to(&mut self.pos.genesis_blocks);

        if let Some(dist) = update.stake_snapshot_n_minus_1 {
            self.stake_snapshot_n_minus_1 = dist;
        }

        if let Some(dist) = update.stake_snapshot_n_minus_2 {
            self.stake_snapshot_n_minus_2 = dist;
        }

        Ok(())
    }

    fn get_leader_at(
        &self,
        date: <Self::Block as property::Block>::Date,
    ) -> Result<Self::LeaderId, Self::Error> {
        let (_pos, stakeholder_id) = self.advance_to(date);
        Ok(stakeholder_id)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::block::{Block, SignedBlock};
    use crate::key::{Hash, PrivateKey};
    use crate::ledger::test::make_key;
    use crate::transaction::*;
    use chain_addr::{Address, Discrimination};
    use chain_core::property::Block as B;
    use chain_core::property::Ledger as L;
    use chain_core::property::Settings as S;
    use chain_core::property::Transaction as T;
    use chain_core::property::{Deserialize, HasTransaction, Serialize};
    use quickcheck::{Arbitrary, StdGen};
    use rand::rngs::{StdRng, ThreadRng};

    struct TestState {
        g: StdGen<ThreadRng>,
        bft_leaders: Vec<PrivateKey>,
        pool_private_keys: Vec<PrivateKey>,
        ledger: Arc<RwLock<Ledger>>,
        settings: Arc<RwLock<Settings>>,
        cur_date: BlockDate,
        prev_hash: Hash,
        leader_selection: GenesisLeaderSelection,
        faucet_utxo: UtxoPointer,
        faucet_private_key: PrivateKey,
        selected_leaders: HashMap<PublicKey, usize>,
    }

    impl TestState {
        fn faucet_value(&self) -> Value {
            self.faucet_utxo.value
        }
    }

    fn create_chain(
        initial_bootstrap_key_slots_percentage: u8,
        mut initial_utxos: HashMap<UtxoPointer, Output>,
        initial_stake_pools: Vec<PrivateKey>,
        initial_stake_keys: HashMap<StakeKeyId, Option<StakePoolId>>,
    ) -> TestState {
        let mut g = StdGen::new(rand::thread_rng(), 10);

        let genesis_hash = Hash::hash_bytes("abc".as_bytes());

        let bft_leaders: Vec<PrivateKey> =
            (0..10_i32).map(|_| PrivateKey::arbitrary(&mut g)).collect();

        let faucet_utxo = UtxoPointer::new(
            TransactionId::hash_bytes("faucet".as_bytes()),
            0,
            Value(1000000000),
        );
        let (faucet_private_key, faucet_address) = make_key(123);

        initial_utxos.insert(faucet_utxo, Output(faucet_address, faucet_utxo.value));

        let ledger = Arc::new(RwLock::new(Ledger::new(initial_utxos)));

        let settings = Arc::new(RwLock::new(Settings::new(genesis_hash)));
        settings.write().unwrap().bootstrap_key_slots_percentage =
            initial_bootstrap_key_slots_percentage;

        let leader_selection = GenesisLeaderSelection::new(
            bft_leaders.iter().map(|k| k.public()).collect(),
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
            prev_hash: genesis_hash.clone(),
            faucet_utxo,
            faucet_private_key,
            selected_leaders: HashMap::new(),
        }
    }

    /// Create and apply a signed block with a single certificate.
    fn apply_signed_block(state: &mut TestState, blk: SignedBlock) -> Result<(), Error> {
        // Test whether we can round-trip this block.
        let mut codec = chain_core::packer::Codec::from(vec![]);
        blk.serialize(&mut codec).unwrap();
        assert_eq!(
            blk,
            SignedBlock::deserialize(&codec.into_inner()[..]).unwrap()
        );

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
        *state.selected_leaders.entry(blk.public_key).or_insert(0) += 1;

        Ok(())
    }

    /// Create and apply a block with the specified contents.
    fn apply_block(state: &mut TestState, contents: Vec<Message>) -> Result<PublicKey, Error> {
        let leader_public_key = state
            .leader_selection
            .get_leader_at(state.cur_date)
            .unwrap();

        let leader_private_key = if let Some(leader_private_key) = state
            .bft_leaders
            .iter()
            .find(|k| k.public() == leader_public_key)
        {
            leader_private_key
        } else if let Some(pool_private_key) = state
            .pool_private_keys
            .iter()
            .find(|k| k.public() == leader_public_key)
        {
            pool_private_key
        } else {
            panic!();
        };

        let blk = SignedBlock::new(
            Block {
                slot_id: state.cur_date.clone(),
                parent_hash: state.prev_hash.clone(),
                contents,
            },
            leader_private_key,
        );

        apply_signed_block(state, blk)?;

        Ok(leader_public_key)
    }

    /// Create and apply a signed block with a single message.
    fn apply_block1(state: &mut TestState, msg: Message) -> Result<PublicKey, Error> {
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
            state.bft_leaders[0].public()
        );

        // Generate a bunch of blocks and check that all leaders are
        // picked an equal number of times.
        for i in 0..10 * state.bft_leaders.len() {
            let leader_public_key = apply_block(&mut state, vec![]).unwrap();
            assert_eq!(
                state.bft_leaders[i % state.bft_leaders.len()].public(),
                leader_public_key
            );
        }

        // Applying a block with a wrong leader should fail.
        {
            let signed_block = SignedBlock::new(
                Block {
                    slot_id: state.cur_date,
                    parent_hash: state.prev_hash,
                    contents: vec![],
                },
                &state.bft_leaders[1],
            );

            assert_eq!(
                state.leader_selection.diff(&signed_block).unwrap_err(),
                Error::BlockHasInvalidLeader(
                    state.bft_leaders[0].public(),
                    state.bft_leaders[1].public()
                )
            );
        }

        // Let's skip a few slots.
        {
            state.cur_date = state.cur_date.next().next().next();
            let signed_block = SignedBlock::new(
                Block {
                    slot_id: state.cur_date,
                    parent_hash: state.prev_hash,
                    contents: vec![],
                },
                &state.bft_leaders[3],
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
            assert_eq!(state.leader_selection.stake_keys.len(), 0);
            apply_block1(
                &mut state,
                (certificate::StakeKeyRegistration {
                    stake_key_id: (&sks0).into(),
                })
                .make_certificate(&sks0),
            )
            .unwrap();
            assert_eq!(state.leader_selection.stake_keys.len(), 1);
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
            assert_eq!(state.leader_selection.stake_pools.len(), 0);
            apply_block1(
                &mut state,
                (certificate::StakePoolRegistration {
                    pool_id: (&pool0_private_key).into(),
                    //owner: owner0_private_key().public(),
                })
                .make_certificate(&pool0_private_key),
            )
            .unwrap();
            assert_eq!(state.leader_selection.stake_pools.len(), 1);
            assert!(state.leader_selection.get_stake_distribution().is_empty());
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
                state.leader_selection.stake_pools[&(&pool0_private_key).into()]
                    .members
                    .len(),
                1
            );
            let dist = state.leader_selection.get_stake_distribution();
            assert_eq!(
                dist,
                vec![(
                    (&pool0_private_key).into(),
                    (
                        Value(10000),
                        vec![((&sks0).into(), Value(10000))]
                            .iter()
                            .cloned()
                            .collect()
                    )
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
                state.leader_selection.get_stake_distribution(),
                vec![(
                    (&pool0_private_key).into(),
                    (
                        Value(30000),
                        vec![((&sks0).into(), Value(30000))]
                            .iter()
                            .cloned()
                            .collect()
                    )
                )]
                .iter()
                .cloned()
                .collect()
            );
        }

        // Register another stake key.
        let sks1 = PrivateKey::arbitrary(&mut state.g);
        {
            assert_eq!(state.leader_selection.stake_keys.len(), 1);
            apply_block1(
                &mut state,
                (certificate::StakeKeyRegistration {
                    stake_key_id: (&sks1).into(),
                })
                .make_certificate(&sks1),
            )
            .unwrap();
            assert_eq!(state.leader_selection.stake_keys.len(), 2);
        }

        // Register another pool.
        let pool1_private_key = PrivateKey::arbitrary(&mut state.g);
        {
            assert_eq!(state.leader_selection.stake_pools.len(), 1);
            apply_block1(
                &mut state,
                (certificate::StakePoolRegistration {
                    pool_id: (&pool1_private_key).into(),
                })
                .make_certificate(&pool1_private_key),
            )
            .unwrap();
            assert_eq!(state.leader_selection.stake_pools.len(), 2);
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
                state.leader_selection.get_stake_distribution(),
                vec![(
                    (&pool0_private_key).into(),
                    (
                        Value(30000),
                        vec![((&sks0).into(), Value(30000))]
                            .iter()
                            .cloned()
                            .collect()
                    )
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
                (
                    Value(30000),
                    vec![((&sks0).into(), Value(30000))]
                        .iter()
                        .cloned()
                        .collect(),
                ),
            ),
            (
                (&pool1_private_key).into(),
                (
                    Value(42000),
                    vec![((&sks1).into(), Value(42000))]
                        .iter()
                        .cloned()
                        .collect(),
                ),
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
                state.leader_selection.get_stake_distribution(),
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
            assert_eq!(
                state.leader_selection.stake_snapshot_n_minus_1,
                expected_stake_dist
            );
            assert_eq!(
                state.leader_selection.stake_snapshot_n_minus_2,
                HashMap::new()
            );
        }

        {
            state.cur_date = state.cur_date.next_epoch();
            apply_block(&mut state, vec![]).unwrap();
            assert_eq!(state.cur_date.epoch, 2);
            assert_eq!(
                state.leader_selection.stake_snapshot_n_minus_1,
                expected_stake_dist
            );
            assert_eq!(
                state.leader_selection.stake_snapshot_n_minus_2,
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
                state.leader_selection.get_stake_distribution(),
                vec![(
                    (&pool1_private_key).into(),
                    (
                        Value(72000),
                        vec![
                            ((&sks0).into(), Value(30000)),
                            ((&sks1).into(), Value(42000))
                        ]
                        .iter()
                        .cloned()
                        .collect()
                    )
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
            assert_eq!(state.leader_selection.stake_pools.len(), 1);
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
            assert_eq!(state.leader_selection.stake_keys.len(), 1);
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
            assert_eq!(state.selected_leaders[&pool0.public()], 335);
            assert_eq!(state.selected_leaders[&pool1.public()], 665);
            for leader in state.bft_leaders {
                assert!(!state.selected_leaders.contains_key(&leader.public()));
            }
        } else if initial_bootstrap_key_slots_percentage == 20 {
            assert_eq!(state.selected_leaders[&pool0.public()], 270);
            assert_eq!(state.selected_leaders[&pool1.public()], 530);
            for leader in state.bft_leaders {
                assert_eq!(state.selected_leaders[&leader.public()], 20);
            }
        } else if initial_bootstrap_key_slots_percentage == 50 {
            assert_eq!(state.selected_leaders[&pool0.public()], 152);
            assert_eq!(state.selected_leaders[&pool1.public()], 348);
            for leader in state.bft_leaders {
                assert_eq!(state.selected_leaders[&leader.public()], 50);
            }
        } else {
            unimplemented!();
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
