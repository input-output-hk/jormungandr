use super::persistent_sequence::PersistentSequence;
use imhamt;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;

use crate::blockcfg::{Block, BlockDate, ChainLength, Epoch, Fragment, FragmentId, HeaderHash};
use chain_addr::{Address, Discrimination};
use chain_core::property::Block as _;
use chain_core::property::Fragment as _;
use chain_impl_mockchain::block::Proof;
use chain_impl_mockchain::certificate::{Certificate, PoolId};
use chain_impl_mockchain::leadership::bft;
use chain_impl_mockchain::transaction::{InputEnum, TransactionSlice, Witness};
use chain_impl_mockchain::value::Value;
use std::convert::TryInto;

use cardano_legacy_address::Addr as OldAddress;

pub type Hamt<K, V> = imhamt::Hamt<DefaultHasher, K, V>;

pub type Transactions = Hamt<FragmentId, HeaderHash>;
pub type Blocks = Hamt<HeaderHash, ExplorerBlock>;
pub type ChainLengths = Hamt<ChainLength, HeaderHash>;

pub type Addresses = Hamt<ExplorerAddress, PersistentSequence<FragmentId>>;
pub type Epochs = Hamt<Epoch, EpochData>;

pub type StakePools = Hamt<PoolId, PersistentSequence<HeaderHash>>;

/// Block with unified inputs the metadata needed in the queries
#[derive(Clone)]
pub struct ExplorerBlock {
    /// The HashMap allows for easy search when querying transactions by id
    pub transactions: HashMap<FragmentId, ExplorerTransaction>,
    pub id: HeaderHash,
    pub date: BlockDate,
    pub chain_length: ChainLength,
    pub parent_hash: HeaderHash,
    pub producer: BlockProducer,
}

#[derive(Clone)]
pub enum BlockProducer {
    None,
    StakePool(PoolId),
    BftLeader(bft::LeaderId),
}

#[derive(Clone)]
pub struct ExplorerTransaction {
    pub id: FragmentId,
    pub inputs: Vec<ExplorerInput>,
    pub outputs: Vec<ExplorerOutput>,
    pub certificate: Option<Certificate>,
    pub offset_in_block: u32,
}

/// Unified Input representation for utxo and account inputs as used in the graphql API
#[derive(Clone)]
pub struct ExplorerInput {
    pub address: ExplorerAddress,
    pub value: Value,
}

#[derive(Clone)]
pub struct ExplorerOutput {
    pub address: ExplorerAddress,
    pub value: Value,
}

#[derive(Clone)]
pub struct EpochData {
    pub first_block: HeaderHash,
    pub last_block: HeaderHash,
    pub total_blocks: u32,
}

#[derive(Eq, PartialEq, Clone)]
pub enum ExplorerAddress {
    New(Address),
    Old(OldAddress),
}

// TODO: derive Hash in legacy address?
use std::hash::{Hash, Hasher};
impl Hash for ExplorerAddress {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ExplorerAddress::New(addr) => addr.hash(state),
            ExplorerAddress::Old(addr) => addr.as_ref().hash(state),
        }
    }
}

impl ExplorerBlock {
    /// Map the given `Block` to the `ExplorerBlock`, transforming all the transactions
    /// using the previous state to transform the utxo inputs to the form (Address, Amount)
    /// and mapping the account inputs to addresses with the given discrimination
    /// This function relies on the given block to be validated previously, and will panic
    /// otherwise
    pub fn resolve_from(
        block: &Block,
        discrimination: Discrimination,
        prev_transactions: &Transactions,
        prev_blocks: &Blocks,
    ) -> ExplorerBlock {
        let fragments = block.contents.iter();
        let id = block.id();
        let chain_length = block.chain_length();

        let transactions = fragments
            .enumerate()
            .filter_map(|(offset, fragment)| {
                let fragment_id = fragment.id();
                let metx = match fragment {
                    Fragment::Transaction(tx) => {
                        let tx = tx.as_slice();
                        Some(ExplorerTransaction::from(
                            &fragment_id,
                            &tx,
                            discrimination,
                            prev_transactions,
                            prev_blocks,
                            None,
                            offset.try_into().unwrap(),
                        ))
                    }
                    Fragment::OwnerStakeDelegation(tx) => {
                        let tx = tx.as_slice();
                        Some(ExplorerTransaction::from(
                            &fragment_id,
                            &tx,
                            discrimination,
                            prev_transactions,
                            prev_blocks,
                            Some(Certificate::OwnerStakeDelegation(
                                tx.payload().into_payload(),
                            )),
                            offset.try_into().unwrap(),
                        ))
                    }
                    Fragment::StakeDelegation(tx) => {
                        let tx = tx.as_slice();
                        Some(ExplorerTransaction::from(
                            &fragment_id,
                            &tx,
                            discrimination,
                            prev_transactions,
                            prev_blocks,
                            Some(Certificate::StakeDelegation(tx.payload().into_payload())),
                            offset.try_into().unwrap(),
                        ))
                    }
                    Fragment::PoolRegistration(tx) => {
                        let tx = tx.as_slice();
                        Some(ExplorerTransaction::from(
                            &fragment_id,
                            &tx,
                            discrimination,
                            prev_transactions,
                            prev_blocks,
                            Some(Certificate::PoolRegistration(tx.payload().into_payload())),
                            offset.try_into().unwrap(),
                        ))
                    }
                    Fragment::PoolRetirement(tx) => {
                        let tx = tx.as_slice();
                        Some(ExplorerTransaction::from(
                            &fragment_id,
                            &tx,
                            discrimination,
                            prev_transactions,
                            prev_blocks,
                            Some(Certificate::PoolRetirement(tx.payload().into_payload())),
                            offset.try_into().unwrap(),
                        ))
                    }
                    Fragment::PoolUpdate(tx) => {
                        let tx = tx.as_slice();
                        Some(ExplorerTransaction::from(
                            &fragment_id,
                            &tx,
                            discrimination,
                            prev_transactions,
                            prev_blocks,
                            Some(Certificate::PoolUpdate(tx.payload().into_payload())),
                            offset.try_into().unwrap(),
                        ))
                    }
                    Fragment::OldUtxoDeclaration(decl) => {
                        let outputs = decl
                            .addrs
                            .iter()
                            .map(|(old_address, value)| ExplorerOutput {
                                address: ExplorerAddress::Old(old_address.clone()),
                                value: *value,
                            })
                            .collect();
                        Some(ExplorerTransaction {
                            id: fragment_id,
                            inputs: vec![],
                            outputs,
                            certificate: None,
                            offset_in_block: offset.try_into().unwrap(),
                        })
                    }
                    _ => None,
                };
                metx.map(|etx| (fragment_id, etx))
            })
            .collect();

        let producer = match block.header.proof() {
            Proof::GenesisPraos(_proof) => {
                // Unwrap is safe in this pattern match
                BlockProducer::StakePool(block.header.get_stakepool_id().unwrap().clone())
            }
            // TODO: I think there are no accesors for this
            Proof::Bft(_proof) => unimplemented!(),
            Proof::None => BlockProducer::None,
        };

        ExplorerBlock {
            id,
            transactions,
            chain_length,
            date: block.header.block_date(),
            parent_hash: block.parent_id(),
            producer,
        }
    }

    pub fn id(&self) -> HeaderHash {
        self.id
    }

    pub fn date(&self) -> BlockDate {
        self.date
    }

    pub fn chain_length(&self) -> ChainLength {
        self.chain_length
    }

    pub fn producer(&self) -> &BlockProducer {
        &self.producer
    }
}

impl ExplorerTransaction {
    /// Map the given AuthenticatedTransaction to the ExplorerTransaction API representation
    /// type.
    /// the fragment id is the associated to the given AuthenticatedTransaction before 'unwrapping'
    /// The discrimination is needed to get addresses from account inputs.
    /// The transactions and blocks are used to resolve utxo inputs
    pub fn from<'a, T>(
        id: &FragmentId,
        tx: &TransactionSlice<'a, T>,
        discrimination: Discrimination,
        transactions: &Transactions,
        blocks: &Blocks,
        certificate: Option<Certificate>,
        offset_in_block: u32,
    ) -> ExplorerTransaction {
        let outputs = tx.outputs().iter();
        let inputs = tx.inputs().iter();
        let witnesses = tx.witnesses().iter();

        let new_outputs = outputs
            .map(|output| ExplorerOutput {
                address: ExplorerAddress::New(output.address.clone()),
                value: output.value,
            })
            .collect();

        let new_inputs = inputs
            .map(|i| i.to_enum())
            .zip(witnesses)
            .filter_map(|input_with_witness| match input_with_witness {
                (InputEnum::AccountInput(id, value), Witness::Account(_)) => {
                    let kind = chain_addr::Kind::Account(
                        id.to_single_account()
                            .expect("the input to be validated")
                            .into(),
                    );
                    let address = ExplorerAddress::New(Address(discrimination, kind));
                    Some(ExplorerInput { address, value })
                }
                (InputEnum::AccountInput(_id, _value), Witness::Multisig(_)) => {
                    // TODO
                    None
                }
                (InputEnum::UtxoInput(utxo_pointer), _witness) => {
                    let tx = utxo_pointer.transaction_id;
                    let index = utxo_pointer.output_index;

                    let block_id = transactions.lookup(&tx).expect("the input to be validated");

                    let block = blocks.lookup(&block_id).expect("the input to be validated");

                    let output = &block.transactions[&tx].outputs[index as usize];

                    Some(ExplorerInput {
                        address: output.address.clone(),
                        value: output.value,
                    })
                }
                _ => None,
            })
            .collect();

        ExplorerTransaction {
            id: *id,
            inputs: new_inputs,
            outputs: new_outputs,
            certificate,
            offset_in_block,
        }
    }

    pub fn id(&self) -> FragmentId {
        self.id
    }

    pub fn inputs(&self) -> &Vec<ExplorerInput> {
        &self.inputs
    }

    pub fn outputs(&self) -> &Vec<ExplorerOutput> {
        &self.outputs
    }
}
