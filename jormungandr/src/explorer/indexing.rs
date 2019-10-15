use super::set::HamtSet as Set;
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
use chain_impl_mockchain::transaction::{AuthenticatedTransaction, InputEnum, Witness};
use chain_impl_mockchain::value::Value;

pub type Hamt<K, V> = imhamt::Hamt<DefaultHasher, K, V>;

pub type Transactions = Hamt<FragmentId, HeaderHash>;
pub type Blocks = Hamt<HeaderHash, ExplorerBlock>;
pub type ChainLengths = Hamt<ChainLength, HeaderHash>;

pub type Addresses = Hamt<Address, Set<FragmentId>>;
pub type Epochs = Hamt<Epoch, EpochData>;

// Use a Hamt to store a sequence, the indexes can be used for pagination
#[derive(Clone)]
pub struct PersistentSequence<T> {
    // Could be usize, but it's only used to store blocks for now
    len: u32,
    elements: Hamt<u32, T>,
}

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
}

/// Unified Input representation for utxo and account inputs as used in the graphql API
#[derive(Clone)]
pub struct ExplorerInput {
    pub address: Address,
    pub value: Value,
}

#[derive(Clone)]
pub struct ExplorerOutput {
    pub address: Address,
    pub value: Value,
}

#[derive(Clone)]
pub struct EpochData {
    pub first_block: HeaderHash,
    pub last_block: HeaderHash,
    pub total_blocks: u32,
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
            .filter_map(|fragment| {
                let fragment_id = fragment.id();
                match fragment {
                    Fragment::Transaction(auth_tx) => Some((
                        fragment_id,
                        ExplorerTransaction::from(
                            &fragment_id,
                            auth_tx,
                            discrimination,
                            prev_transactions,
                            prev_blocks,
                            //certificate
                            None,
                        ),
                    )),
                    Fragment::OwnerStakeDelegation(auth_tx) => Some((
                        fragment_id,
                        ExplorerTransaction::from(
                            &fragment_id,
                            auth_tx,
                            discrimination,
                            prev_transactions,
                            prev_blocks,
                            Some(Certificate::OwnerStakeDelegation(
                                auth_tx.transaction.extra.clone(),
                            )),
                        ),
                    )),
                    Fragment::StakeDelegation(auth_tx) => Some((
                        fragment_id,
                        ExplorerTransaction::from(
                            &fragment_id,
                            auth_tx,
                            discrimination,
                            prev_transactions,
                            prev_blocks,
                            Some(Certificate::StakeDelegation(
                                auth_tx.transaction.extra.clone(),
                            )),
                        ),
                    )),
                    Fragment::PoolRegistration(auth_tx) => Some((
                        fragment_id,
                        ExplorerTransaction::from(
                            &fragment_id,
                            auth_tx,
                            discrimination,
                            prev_transactions,
                            prev_blocks,
                            Some(Certificate::PoolRegistration(
                                auth_tx.transaction.extra.clone(),
                            )),
                        ),
                    )),
                    Fragment::PoolManagement(auth_tx) => Some((
                        fragment_id,
                        ExplorerTransaction::from(
                            &fragment_id,
                            auth_tx,
                            discrimination,
                            prev_transactions,
                            prev_blocks,
                            Some(Certificate::PoolManagement(
                                auth_tx.transaction.extra.clone(),
                            )),
                        ),
                    )),
                    _ => None,
                }
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
            date: *block.header.block_date(),
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
    pub fn from<T>(
        id: &FragmentId,
        auth_tx: &AuthenticatedTransaction<Address, T>,
        discrimination: Discrimination,
        transactions: &Transactions,
        blocks: &Blocks,
        certificate: Option<Certificate>,
    ) -> ExplorerTransaction {
        let outputs = auth_tx.transaction.outputs.iter();
        let inputs = auth_tx.transaction.inputs.iter();
        let witnesses = auth_tx.witnesses.iter();

        let new_outputs = outputs
            .map(|output| ExplorerOutput {
                address: output.address.clone(),
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
                    let address = Address(discrimination, kind);
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

impl<T> PersistentSequence<T> {
    pub fn new() -> Self {
        PersistentSequence {
            len: 0,
            elements: Hamt::new(),
        }
    }

    pub fn append(&self, t: T) -> Self {
        let len = self.len + 1;
        PersistentSequence {
            len,
            elements: self.elements.insert(len - 1, t).unwrap(),
        }
    }

    pub fn get(&self, i: u32) -> Option<&T> {
        self.elements.lookup(&i)
    }

    pub fn len(&self) -> u32 {
        self.len
    }
}
