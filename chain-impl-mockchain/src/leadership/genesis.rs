use super::LeaderId;
use crate::block::{BlockDate, Message, SignedBlock};
use crate::certificate;
use crate::date::Epoch;
use crate::leadership::bft::BftRoundRobinIndex;
use crate::ledger::Ledger;
use crate::setting::{self, Settings};
use crate::stake::*;
use crate::transaction::Value;
use crate::update::ValueDiff;

use chain_core::property::{self, Block, LeaderSelection, Update};

use rand::{Rng, SeedableRng};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct GenesisLeaderSelection {
    ledger: Arc<RwLock<Ledger>>,
    settings: Arc<RwLock<Settings>>,

    bft_leaders: Vec<LeaderId>,

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
    next_bft_leader_index: BftRoundRobinIndex,
    bft_blocks: usize,
    genesis_blocks: usize, // FIXME: "genesis block" is rather ambiguous...
}

#[derive(Debug, PartialEq)]
pub enum Error {
    BlockHasInvalidLeader(LeaderId, LeaderId),
    BlockSignatureIsInvalid,
    UpdateHasInvalidCurrentLeader(LeaderId, LeaderId),
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
        bft_leaders: Vec<LeaderId>,
        ledger: Arc<RwLock<Ledger>>,
        settings: Arc<RwLock<Settings>>,
        initial_stake_pools: HashSet<StakePoolId>,
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
                next_bft_leader_index: BftRoundRobinIndex(0),
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

    fn advance_to(&self, to_date: BlockDate) -> (Pos, LeaderId) {
        let mut now = self.pos.clone();

        let d = self.settings.read().unwrap().bootstrap_key_slots_percentage;

        loop {
            assert!(now.next_date <= to_date);

            let done = now.next_date == to_date;

            let cur_epoch = now.next_date.epoch;

            now.next_date = now.next_date.next();

            // Base leadership selection on the stake distribution at
            // the start of the previous epoch. FIXME: handle the case
            // were we're advancing so far (i.e. crossing two or more
            // epochs) that we have to compute a snapshot not in
            // self.stake_snaphots.
            let epoch_for_leadership = if cur_epoch < 1 { 0 } else { cur_epoch - 1 };
            let stake_snapshot = &self.stake_snapshots[&epoch_for_leadership];

            // If we didn't have eligible stake pools in the epoch
            // used for sampling, then we have to use BFT rules.
            // FIXME: require a certain minimum number of stake pools?
            let have_stakeholders = stake_snapshot.eligible_stake_pools() > 0;

            let is_bft_slot = d == setting::SLOTS_PERCENTAGE_RANGE
                || !have_stakeholders
                || now.bft_blocks * (setting::SLOTS_PERCENTAGE_RANGE as usize)
                    < (d as usize) * (now.bft_blocks + now.genesis_blocks);

            if is_bft_slot {
                now.bft_blocks += 1;
                let bft_leader_index = now.next_bft_leader_index.0;
                now.next_bft_leader_index =
                    BftRoundRobinIndex((bft_leader_index + 1) % self.bft_leaders.len() as u64);
                if done {
                    return (now, self.bft_leaders[bft_leader_index as usize].clone());
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
                    return (now, stake_snapshot.select_pool(point).unwrap().into());
                }
            }
        }
    }

    pub fn get_stake_distribution(&self) -> StakeDistribution {
        self.delegation_state
            .get_stake_distribution(&self.ledger.read().unwrap())
    }
}

#[derive(Debug, Clone)]
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
    stake_snapshots: Option<BTreeMap<Epoch, StakeDistribution>>,
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
            stake_snapshots: None,
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

        if let Some(stake_snapshots) = other.stake_snapshots {
            self.stake_snapshots = Some(stake_snapshots);
        }

        self
    }
}

impl LeaderSelection for GenesisLeaderSelection {
    type Update = GenesisSelectionDiff;
    type Block = SignedBlock;
    type Error = Error;
    type LeaderId = LeaderId;

    fn diff(&self, input: &Self::Block) -> Result<Self::Update, Self::Error> {
        let mut update = <Self::Update as property::Update>::empty();

        let date = input.date();

        let (new_pos, leader) = self.advance_to(date);

        assert_eq!(new_pos.next_date, date.next());

        if leader != input.leader_id {
            return Err(Error::BlockHasInvalidLeader(
                leader,
                input.leader_id.clone(),
            ));
        }

        if !input.verify() {
            return Err(Error::BlockSignatureIsInvalid);
        }

        // If we crossed into a new epoch, then update the stake
        // distribution snapshots.
        if date.epoch != self.pos.next_date.epoch
            || (self.pos.next_date.slot_id == 0 && self.pos.next_date.epoch > 0)
        {
            let mut snapshots = self.stake_snapshots.clone();
            if date.epoch >= 2 {
                // Expire snapshots that we don't need anymore.
                snapshots.remove(&date.epoch.checked_sub(2).unwrap());
            }
            snapshots.insert(date.epoch, self.get_stake_distribution());
            assert!(snapshots.len() <= 2);
            update.stake_snapshots = Some(snapshots);
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
                    if !self
                        .delegation_state
                        .stake_key_exists(&reg.data.stake_key_id)
                    {
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

                    if self
                        .delegation_state
                        .stake_key_exists(&reg.data.stake_key_id)
                    {
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
                    if !self
                        .delegation_state
                        .stake_key_exists(&reg.data.stake_key_id)
                    {
                        return Err(Error::StakeDelegationStakeKeyIsInvalid(
                            reg.data.stake_key_id.clone(),
                        ));
                    }

                    // FIXME: should it be allowed to create a stake
                    // pool and delegate to it in the same
                    // transaction?
                    if !self.delegation_state.stake_pool_exists(&reg.data.pool_id) {
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

                    if self.delegation_state.stake_pool_exists(&reg.data.pool_id)
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
                    if self.delegation_state.stake_pool_exists(&ret.data.pool_id) {
                        if !ret.data.pool_id.0.serialize_and_verify(&ret.data, &ret.sig) {
                            return Err(Error::StakePoolRetirementSigIsInvalid);
                        }
                        update.retired_stake_pools.insert(ret.data.pool_id.clone());
                    } else {
                        return Err(Error::StakePoolDoesNotExist(ret.data.pool_id.clone()));
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
            self.delegation_state.register_stake_key(stake_key_id);
        }

        for stake_key_id in update.stake_key_deregistrations {
            self.delegation_state.deregister_stake_key(&stake_key_id);
        }

        for (pool_id, _new_stake_pool) in update.new_stake_pools {
            self.delegation_state.register_stake_pool(pool_id);
        }

        for (stake_key_id, pool_id) in update.delegations {
            self.delegation_state.delegate_stake(stake_key_id, pool_id);
        }

        // FIXME: the pool should be retired at the end of a specified epoch.
        for pool_id in update.retired_stake_pools {
            self.delegation_state.deregister_stake_pool(&pool_id);
        }

        update.next_date.apply_to(&mut self.pos.next_date);
        update
            .next_bft_leader_index
            .apply_to(&mut self.pos.next_bft_leader_index);
        update.bft_blocks.apply_to(&mut self.pos.bft_blocks);
        update.genesis_blocks.apply_to(&mut self.pos.genesis_blocks);

        if let Some(stake_snapshots) = update.stake_snapshots {
            self.stake_snapshots = stake_snapshots;
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
    use chain_addr::{Address, Discrimination, Kind};
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
            bft_leaders.iter().map(|k| k.into()).collect(),
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
        *state.selected_leaders.entry(blk.leader_id).or_insert(0) += 1;

        Ok(())
    }

    /// Create and apply a block with the specified contents.
    fn apply_block(state: &mut TestState, contents: Vec<Message>) -> Result<LeaderId, Error> {
        let leader_id = state
            .leader_selection
            .get_leader_at(state.cur_date)
            .unwrap();

        let leader_private_key = if let Some(leader_private_key) = state
            .bft_leaders
            .iter()
            .find(|k| LeaderId::from(*k) == leader_id)
        {
            leader_private_key
        } else if let Some(pool_private_key) = state
            .pool_private_keys
            .iter()
            .find(|k| LeaderId::from(*k) == leader_id)
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
                    (&state.bft_leaders[0]).into(),
                    (&state.bft_leaders[1]).into()
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
