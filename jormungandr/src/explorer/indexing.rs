use super::{persistent_sequence::PersistentSequence, stable_storage::StableIndexShared};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;

use crate::blockcfg::{Block, BlockDate, ChainLength, Epoch, Fragment, FragmentId, HeaderHash};
use cardano_legacy_address::Addr as OldAddress;
use chain_addr::{Address, Discrimination};
use chain_core::property::Block as _;
use chain_impl_mockchain::block::Proof;
use chain_impl_mockchain::certificate::{
    Certificate, ExternalProposalId, PoolId, PoolRegistration, PoolRetirement, VotePlanId,
};
use chain_impl_mockchain::key::BftLeaderId;
use chain_impl_mockchain::transaction::{InputEnum, TransactionSlice, Witness};
use chain_impl_mockchain::value::Value;
use chain_impl_mockchain::vote::{
    Choice, EncryptedVote, Options, PayloadType, ProofOfCorrectVote, Weight,
};
use futures::stream::{self, StreamExt};
use std::{convert::TryInto, sync::Arc};

pub type Hamt<K, V> = imhamt::Hamt<DefaultHasher, K, Arc<V>>;

pub type Transactions = Hamt<FragmentId, HeaderHash>;
pub type Blocks = Hamt<HeaderHash, ExplorerBlock>;
pub type ChainLengths = Hamt<ChainLength, HeaderHash>;

pub type Addresses = Hamt<ExplorerAddress, PersistentSequence<FragmentId>>;
pub type Epochs = Hamt<Epoch, EpochData>;

pub type StakePoolBlocks = Hamt<PoolId, PersistentSequence<HeaderHash>>;
pub type StakePool = Hamt<PoolId, StakePoolData>;

pub type VotePlans = Hamt<VotePlanId, ExplorerVotePlan>;

#[derive(Clone)]
pub struct StakePoolData {
    pub registration: PoolRegistration,
    pub retirement: Option<PoolRetirement>,
    // TODO: Track updates here too?
}

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
    pub total_input: Value,
    pub total_output: Value,
}

#[derive(Clone)]
pub enum BlockProducer {
    None,
    StakePool(PoolId),
    BftLeader(BftLeaderId),
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

#[derive(Eq, PartialEq, Clone, Hash)]
pub enum ExplorerAddress {
    New(Address),
    Old(OldAddress),
}

#[derive(Clone)]
pub struct ExplorerVotePlan {
    pub id: VotePlanId,
    pub vote_start: BlockDate,
    pub vote_end: BlockDate,
    pub committee_end: BlockDate,
    pub payload_type: PayloadType,
    pub proposals: Vec<ExplorerVoteProposal>,
}

#[derive(Clone)]
pub enum ExplorerVote {
    Public(Choice),
    Private {
        proof: ProofOfCorrectVote,
        encrypted_vote: EncryptedVote,
    },
}

#[derive(Clone)]
pub struct ExplorerVoteProposal {
    pub proposal_id: ExternalProposalId,
    pub options: Options,
    pub tally: Option<ExplorerVoteTally>,
    pub votes: Hamt<ExplorerAddress, ExplorerVote>,
}

// TODO do proper vote tally
#[derive(Clone)]
pub enum ExplorerVoteTally {
    Public {
        results: Vec<Weight>,
        options: Options,
    },
    Private {
        results: Option<Vec<Weight>>,
        options: Options,
    },
}

pub struct ExplorerBlockBuildingContext<'a> {
    pub discrimination: Discrimination,
    pub prev_transactions: &'a Transactions,
    pub prev_blocks: &'a Blocks,
    pub stable_storage: StableIndexShared,
}

impl ExplorerBlock {
    /// Map the given `Block` to the `ExplorerBlock`, transforming all the transactions
    /// using the previous state to transform the utxo inputs to the form (Address, Amount)
    /// and mapping the account inputs to addresses with the given discrimination
    /// This function relies on the given block to be validated previously, and will panic
    /// otherwise
    pub async fn resolve_from(
        block: &Block,
        context: ExplorerBlockBuildingContext<'_>,
    ) -> ExplorerBlock {
        let fragments = block.contents.iter();
        let id = block.id();
        let chain_length = block.chain_length();
        let transactions: HashMap<FragmentId, ExplorerTransaction> = {
            let mut current_block_txs = HashMap::new();
            for (offset, fragment) in fragments.enumerate() {
                let fragment_id = fragment.hash();
                let offset: u32 = offset.try_into().unwrap();
                let metx = match fragment {
                    Fragment::Transaction(tx) => {
                        let tx = tx.as_slice();
                        Some(
                            ExplorerTransaction::from(
                                &context,
                                fragment_id,
                                &tx,
                                None,
                                offset.clone(),
                                &current_block_txs,
                            )
                            .await,
                        )
                    }
                    Fragment::OwnerStakeDelegation(tx) => {
                        let tx = tx.as_slice();
                        Some(
                            ExplorerTransaction::from(
                                &context,
                                fragment_id,
                                &tx,
                                Some(Certificate::OwnerStakeDelegation(
                                    tx.payload().into_payload(),
                                )),
                                offset.clone(),
                                &current_block_txs,
                            )
                            .await,
                        )
                    }
                    Fragment::StakeDelegation(tx) => {
                        let tx = tx.as_slice();
                        Some(
                            ExplorerTransaction::from(
                                &context,
                                fragment_id,
                                &tx,
                                Some(Certificate::StakeDelegation(tx.payload().into_payload())),
                                offset.clone(),
                                &current_block_txs,
                            )
                            .await,
                        )
                    }
                    Fragment::PoolRegistration(tx) => {
                        let tx = tx.as_slice();
                        Some(
                            ExplorerTransaction::from(
                                &context,
                                fragment_id.clone(),
                                &tx,
                                Some(Certificate::PoolRegistration(tx.payload().into_payload())),
                                offset.clone(),
                                &current_block_txs,
                            )
                            .await,
                        )
                    }
                    Fragment::PoolRetirement(tx) => {
                        let tx = tx.as_slice();
                        Some(
                            ExplorerTransaction::from(
                                &context,
                                fragment_id.clone(),
                                &tx,
                                Some(Certificate::PoolRetirement(tx.payload().into_payload())),
                                offset.clone(),
                                &current_block_txs,
                            )
                            .await,
                        )
                    }
                    Fragment::PoolUpdate(tx) => {
                        let tx = tx.as_slice();
                        Some(
                            ExplorerTransaction::from(
                                &context,
                                fragment_id.clone(),
                                &tx,
                                Some(Certificate::PoolUpdate(tx.payload().into_payload())),
                                offset.clone(),
                                &current_block_txs,
                            )
                            .await,
                        )
                    }
                    Fragment::VotePlan(tx) => {
                        let tx = tx.as_slice();
                        Some(
                            ExplorerTransaction::from(
                                &context,
                                fragment_id.clone(),
                                &tx,
                                Some(Certificate::VotePlan(tx.payload().into_payload())),
                                offset.clone(),
                                &current_block_txs,
                            )
                            .await,
                        )
                    }
                    Fragment::VoteCast(tx) => {
                        let tx = tx.as_slice();
                        Some(
                            ExplorerTransaction::from(
                                &context,
                                fragment_id.clone(),
                                &tx,
                                Some(Certificate::VoteCast(tx.payload().into_payload())),
                                offset.clone(),
                                &current_block_txs,
                            )
                            .await,
                        )
                    }
                    Fragment::VoteTally(tx) => {
                        let tx = tx.as_slice();
                        Some(
                            ExplorerTransaction::from(
                                &context,
                                fragment_id.clone(),
                                &tx,
                                Some(Certificate::VoteTally(tx.payload().into_payload())),
                                offset.clone(),
                                &current_block_txs,
                            )
                            .await,
                        )
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
                            offset_in_block: offset,
                        })
                    }
                    _ => None,
                };

                if let Some(etx) = metx {
                    current_block_txs.insert(fragment_id, etx);
                }
            }

            current_block_txs
        };

        let producer = match block.header.proof() {
            Proof::GenesisPraos(_proof) => {
                // Unwrap is safe in this pattern match
                BlockProducer::StakePool(block.header.get_stakepool_id().unwrap())
            }
            Proof::Bft(_proof) => {
                BlockProducer::BftLeader(block.header.get_bft_leader_id().unwrap())
            }
            Proof::None => BlockProducer::None,
        };

        let total_input = Value::sum(
            transactions
                .values()
                .flat_map(|tx| tx.inputs.iter().map(|i| i.value)),
        )
        .expect("Couldn't compute block's total input");

        let total_output = Value::sum(
            transactions
                .values()
                .flat_map(|tx| tx.outputs.iter().map(|o| o.value)),
        )
        .expect("Couldn't compute block's total output");

        ExplorerBlock {
            id,
            transactions,
            chain_length,
            date: block.header.block_date(),
            parent_hash: block.parent_id(),
            producer,
            total_input,
            total_output,
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

    // TODO: The signature of this got too long, using a builder may be a good idea
    // It's called only from one place, though, so it is not that bothersome
    pub async fn from<'transaction, T>(
        context: &ExplorerBlockBuildingContext<'transaction>,
        id: FragmentId,
        tx: &TransactionSlice<'transaction, T>,
        certificate: Option<Certificate>,
        offset_in_block: u32,
        transactions_in_current_block: &HashMap<FragmentId, ExplorerTransaction>,
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

        let new_inputs = stream::iter(inputs.map(|i| i.to_enum()).zip(witnesses))
            .filter_map(|input_with_witness| async {
                match input_with_witness {
                    (InputEnum::AccountInput(id, value), Witness::Account(_)) => {
                        let kind = chain_addr::Kind::Account(
                            id.to_single_account()
                                .expect("the input to be validated")
                                .into(),
                        );
                        let address = ExplorerAddress::New(Address(context.discrimination, kind));
                        Some(ExplorerInput { address, value })
                    }
                    (InputEnum::AccountInput(id, value), Witness::Multisig(_)) => {
                        let kind = chain_addr::Kind::Multisig(
                            id.to_multi_account()
                                .as_ref()
                                .try_into()
                                .expect("multisig identifier size doesn't match address kind"),
                        );
                        let address = ExplorerAddress::New(Address(context.discrimination, kind));
                        Some(ExplorerInput { address, value })
                    }
                    (InputEnum::UtxoInput(utxo_pointer), _witness) => {
                        let tx = utxo_pointer.transaction_id;
                        let index = utxo_pointer.output_index;

                        let output = match context
                            .prev_transactions
                            .lookup(&tx)
                            .and_then(|block_id| {
                                context
                                    .prev_blocks
                                    .lookup(&block_id)
                                    .map(|block| &block.transactions[&tx].outputs[index as usize])
                            })
                            .or_else(|| {
                                transactions_in_current_block
                                    .get(&tx)
                                    .map(|fragment| &fragment.outputs[index as usize])
                            }) {
                            Some(x) => Some(x.clone()),
                            None => {
                                // TODO: maybe this function should return an error
                                // if any of this things panics, the error is
                                // most likely a programmer error, at least now
                                // that the stable storage is in memory, but in
                                // the future this could be caused by a compromised database
                                let storage = context.stable_storage.read().await;
                                let block_id = storage.transaction_to_block(&tx).unwrap();
                                let block = storage.get_block(block_id).unwrap();
                                let tx = block.transactions.get(&tx).unwrap();

                                Some(tx.outputs[index as usize].clone())
                            }
                        }
                        .expect("transaction not found for utxo input");

                        Some(ExplorerInput {
                            address: output.address.clone(),
                            value: output.value,
                        })
                    }
                    _ => None,
                }
            })
            .collect();

        ExplorerTransaction {
            id,
            inputs: new_inputs.await,
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
