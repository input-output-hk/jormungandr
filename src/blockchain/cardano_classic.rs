use super::{Hash, Date, Block, ChainState, ChainStateDelta};
use cardano::block::types::{HeaderHash};
use cardano::address::{StakeholderId};
use cbor_event::{de::{self, RawCbor}, se, Len};
use std::iter::FromIterator;

impl Date for cardano::block::date::BlockDate {
    fn serialize(&self) -> u64 {
        match self {
            cardano::block::BlockDate::Boundary(epoch) => epoch << 16,
            cardano::block::BlockDate::Normal(s) => { assert!(s.slotid < 65535); ((s.epoch as u64) << 16) | ((s.slotid + 1) as u64) }
        }
    }

    fn deserialize(n: u64) -> Self {
        let epoch = n >> 16;
        let slot = n & 65535;
        if slot == 0 {
            cardano::block::BlockDate::Boundary(epoch)
        } else {
            cardano::block::BlockDate::Normal(
                cardano::block::EpochSlotId { epoch, slotid: (slot - 1) as u16 })
        }
    }
}

impl Block for cardano::block::Block {
    fn get_hash(&self) -> Hash {
        (*self.get_header().compute_hash()).into()
    }

    fn get_parent(&self) -> Hash {
        (*self.get_header().get_previous_header()).into()
    }

    type Date = cardano::block::date::BlockDate;

    fn get_date(&self) -> Self::Date {
        self.get_header().get_blockdate()
    }

    fn serialize(&self) -> Vec<u8> {
        cbor!(self).unwrap()
    }

    fn deserialize(bytes: &[u8]) -> Self {
        RawCbor::from(bytes).deserialize_complete().unwrap()
    }
}

impl ChainState for cardano::block::ChainState {
    type Block = cardano::block::Block;
    type Error = cardano::block::verify::Error;
    type GenesisData = cardano::config::GenesisData;

    fn new(genesis_data: &Self::GenesisData) -> Result<Self, Self::Error> {
        Ok(cardano::block::ChainState::new(&genesis_data))
    }

    fn apply_block(&mut self, block: &Self::Block) -> Result<(), Self::Error> {
        self.verify_block(&block.get_hash().into(), block)
    }

    fn get_last_block(&self) -> Hash {
        (*self.last_block.clone()).into()
    }

    fn get_chain_length(&self) -> u64 {
        self.chain_length
    }

    type Delta = ClassicChainStateDelta;

    fn diff(from: &Self, to: &Self) -> Result<Self::Delta, Self::Error> {
        assert_ne!(from, to);

        let (removed_utxos, added_utxos) =
            cardano_storage::chain_state::diff_maps(&from.utxos, &to.utxos);

        Ok(ClassicChainStateDelta {
            base: from.last_block.clone(),
            last_block: to.last_block.clone(),
            last_date: to.last_date.unwrap().clone(),
            last_boundary_block: to.last_boundary_block.clone().unwrap(),
            slot_leaders: to.slot_leaders.clone(),
            chain_length: to.chain_length,
            nr_transactions: to.nr_transactions,
            spent_txos: to.spent_txos,
            removed_utxos: removed_utxos.into_iter().map(|x| x.clone()).collect(),
            added_utxos: cardano::block::verify_chain::Utxos::from_iter(
                added_utxos.into_iter().map(|(n, v)| (n.clone(), v.clone())))
        })
    }

    fn apply_delta(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
        assert_eq!(self.last_block, delta.base);
        self.last_block = delta.last_block;
        self.last_date = Some(delta.last_date);
        self.last_boundary_block = Some(delta.last_boundary_block);
        self.chain_length = delta.chain_length;
        self.nr_transactions = delta.nr_transactions;
        self.spent_txos = delta.spent_txos;
        self.slot_leaders = delta.slot_leaders;

        for txo_ptr in &delta.removed_utxos {
            if self.utxos.remove(txo_ptr).is_none() {
                panic!("chain state delta removes non-existent utxo {}", txo_ptr);
            }
        }

        for (txo_ptr, txo) in delta.added_utxos {
            if self.utxos.insert(txo_ptr, txo).is_some() {
                panic!("chain state delta inserts duplicate utxo");
            }
        }

        Ok(())
    }
}

// FIXME: move to cardano-deps/cardano?
pub struct ClassicChainStateDelta {
    base: HeaderHash,
    last_block: HeaderHash,
    last_date: cardano::block::date::BlockDate,
    last_boundary_block: HeaderHash,
    chain_length: u64,
    nr_transactions: u64,
    spent_txos: u64,
    slot_leaders: Vec<StakeholderId>, // FIXME: get from last_boundary_block
    removed_utxos: Vec<cardano::tx::TxoPointer>,
    added_utxos: cardano::block::verify_chain::Utxos,
}

const NR_FIELDS: u64 = 10;

impl ChainStateDelta for ClassicChainStateDelta {
    fn serialize(&self) -> Vec<u8> {
        let mut data = vec![];
        {
            let serializer = se::Serializer::new(&mut data)
                .write_array(Len::Len(NR_FIELDS)).unwrap()
                .serialize(&self.base).unwrap()
                .serialize(&self.last_block).unwrap()
                .serialize(&self.last_date.serialize()).unwrap()
                .serialize(&self.last_boundary_block).unwrap()
                .serialize(&self.chain_length).unwrap()
                .serialize(&self.nr_transactions).unwrap()
                .serialize(&self.spent_txos).unwrap();
            let serializer = se::serialize_fixed_array(self.slot_leaders.iter(), serializer).unwrap();
            let serializer = se::serialize_fixed_array(self.removed_utxos.iter(), serializer).unwrap();
            se::serialize_fixed_map(self.added_utxos.iter(), serializer).unwrap();
        }
        data
    }

    fn deserialize(bytes: &[u8]) -> Self {
        let mut raw = de::RawCbor::from(bytes);

        raw.tuple(NR_FIELDS, "chain state delta").unwrap();
        let base = raw.deserialize().unwrap();
        let last_block = raw.deserialize().unwrap();
        let last_date = cardano::block::date::BlockDate::deserialize(raw.deserialize().unwrap());
        let last_boundary_block = raw.deserialize().unwrap();
        let chain_length = raw.deserialize().unwrap();
        let nr_transactions = raw.deserialize().unwrap();
        let spent_txos = raw.deserialize().unwrap();
        let slot_leaders = raw.deserialize().unwrap();
        let removed_utxos = raw.deserialize().unwrap();
        let added_utxos = raw.deserialize().unwrap();

        Self {
            base,
            last_block,
            last_date,
            last_boundary_block,
            slot_leaders,
            chain_length,
            nr_transactions,
            spent_txos,
            removed_utxos,
            added_utxos
        }
    }
}
