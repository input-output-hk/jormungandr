use std::collections::BTreeMap;

use crate::blockcfg::{self as property, BlockConfig};
use crate::secure;

use cardano::{
    block::{chain_state::ChainState, verify::Error},
    tx::{TxOut, TxoPointer},
    util::try_from_slice::TryFromSlice,
};

pub type GenesisData = ::cardano::config::GenesisData;
pub type TransactionId = ::cardano::tx::TxId;
pub type Transaction = ::cardano::tx::TxAux;
pub type BlockDate = ::cardano::block::BlockDate;
pub type BlockHash = ::cardano::block::HeaderHash;
pub type Block = ::cardano::block::Block;
pub type Header = ::cardano::block::BlockHeader;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cardano;
impl BlockConfig for Cardano {
    type Block = Block;
    type BlockDate = BlockDate;
    type BlockHash = BlockHash;
    type BlockHeader = Header;
    type Transaction = Transaction;
    type TransactionId = TransactionId;
    type GenesisData = GenesisData;
    type Ledger = ::cardano::block::chain_state::ChainState;

    fn make_block(
        secret_key: &secure::NodeSecret,
        public_key: &secure::NodePublic,
        ledger: &Self::Ledger,
        block_date: <Self::Block as property::Block>::Date,
        transactions: Vec<Self::Transaction>,
    ) -> Self::Block {
        use cardano::block::*;
        use cardano::hash::Blake2b256;
        use cbor_event::Value;

        let previous_hash = &ledger.last_block;

        match block_date {
            BlockDate::Boundary(_) => unimplemented!(),
            BlockDate::Normal(block_id) => {
                let pm = ledger.protocol_magic;
                let bver = BlockVersion::new(1, 0, 0);
                let sver = SoftwareVersion::new(env!("CARGO_PKG_NAME"), 1).unwrap();

                let sig = secret_key.sign_block();

                let body = normal::Body {
                    tx: normal::TxPayload::new(transactions),
                    ssc: normal::SscPayload::fake(),
                    delegation: normal::DlgPayload(Value::U64(0)),
                    update: update::UpdatePayload {
                        proposal: None,
                        votes: Vec::new(),
                    },
                };
                let extra = Value::U64(0);

                let body_proof = normal::BodyProof::generate_from_body(&body);
                let extra_bytes = cbor!(&extra).unwrap();

                let hdr = normal::BlockHeader {
                    protocol_magic: pm,
                    previous_header: previous_hash.clone(),
                    body_proof: body_proof,
                    consensus: normal::Consensus {
                        slot_id: block_id,
                        leader_key: public_key.block_publickey.clone(),
                        chain_difficulty: ChainDifficulty::from(0u64),
                        block_signature: sig,
                    },
                    extra_data: HeaderExtraData {
                        block_version: bver,
                        software_version: sver,
                        attributes: BlockHeaderAttributes(Value::U64(0)),
                        extra_data_proof: Blake2b256::new(&extra_bytes),
                    },
                };
                let b = normal::Block {
                    header: hdr,
                    body: body,
                    extra: extra,
                };

                Block::MainBlock(b)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diff {
    spent_outputs: BTreeMap<TxoPointer, TxOut>,
    new_unspent_outputs: BTreeMap<TxoPointer, TxOut>,
}
impl Diff {
    fn new() -> Self {
        Diff {
            spent_outputs: BTreeMap::new(),
            new_unspent_outputs: BTreeMap::new(),
        }
    }

    fn extend(&mut self, other: Self) {
        self.new_unspent_outputs.extend(other.new_unspent_outputs);
        self.spent_outputs.extend(other.spent_outputs);
    }
}

impl property::Ledger<Transaction> for ChainState {
    type Update = Diff;
    type Error = Error;

    fn diff_transaction(&self, transaction: &Transaction) -> Result<Self::Update, Self::Error> {
        use cardano::block::verify::Verify;

        let id = transaction.tx.id();
        let mut diff = Diff::new();

        // 1. verify the transaction is valid (self valid)
        transaction.verify(self.protocol_magic)?;

        for (input, witness) in transaction.tx.inputs.iter().zip(transaction.witness.iter()) {
            if let Some(output) = self.utxos.get(&input) {
                if !witness.verify_address(&output.address) {
                    return Err(Error::AddressMismatch);
                }
                if let Some(_output) = diff.spent_outputs.insert(input.clone(), output.clone()) {
                    return Err(Error::DuplicateInputs);
                }
            } else {
                return Err(Error::MissingUtxo);
            }
        }

        // 2. prepare to add the new outputs
        for (index, output) in transaction.tx.outputs.iter().enumerate() {
            diff.new_unspent_outputs
                .insert(TxoPointer::new(id, index as u32), output.clone());
        }

        Ok(diff)
    }
    fn apply(&mut self, diff: Self::Update) -> Result<&mut Self, Self::Error> {
        for spent_output in diff.spent_outputs.keys() {
            if let None = self.utxos.remove(spent_output) {
                return Err(Error::MissingUtxo);
            }
        }

        for (input, output) in diff.new_unspent_outputs {
            if let Some(_original_output) = self.utxos.insert(input, output) {
                return Err(Error::DuplicateTxo);
            }
        }

        Ok(self)
    }
}
