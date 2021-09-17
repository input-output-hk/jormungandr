use super::{
    chain_storable::{AccountId, Address, BlockId, FragmentId, PoolId, Stake},
    endian::L64,
    pair::Pair,
    SanakirjaMutTx, SeqNum,
};
use super::{
    chain_storable::{ChainLength, ProposalId},
    error::DbError,
};
use chain_impl_mockchain::{transaction, value::Value};
use sanakirja::{
    btree::{self, Db},
    direct_repr, Storable, UnsizedStorable,
};

pub type StakeControl = Db<AccountId, Stake>;
pub type BlocksInBranch = Db<ChainLength, BlockId>;

pub type AddressId = SeqNum;
pub type AddressIds = Db<Address, AddressId>;
pub type AddressTransactions = Db<AddressId, Pair<SeqNum, FragmentId>>;
pub type Votes = Db<ProposalId, Pair<SeqNum, FragmentId>>;

// a typed (and in-memory) version of SerializedStateRef
pub struct StateRef {
    pub stake_control: StakeControl,
    pub blocks: BlocksInBranch,
    pub address_id: AddressIds,
    pub address_transactions: AddressTransactions,
    pub votes: Votes,
    // cached field, this gets written back by `finish`
    next_address_id: Option<SeqNum>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct SerializedStateRef {
    pub stake_control: L64,
    pub blocks: L64,
    pub address_id: L64,
    pub addresses: L64,
    pub votes: L64,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PoolIdEntry {
    pool: PoolId,
    seq: SeqNum,
}

direct_repr!(PoolIdEntry);

impl From<SerializedStateRef> for StateRef {
    fn from(ser: SerializedStateRef) -> Self {
        StateRef {
            stake_control: Db::from_page(ser.stake_control.get()),
            blocks: Db::from_page(ser.blocks.get()),
            address_id: Db::from_page(ser.address_id.get()),
            address_transactions: Db::from_page(ser.addresses.get()),
            votes: Db::from_page(ser.votes.get()),
            next_address_id: None,
        }
    }
}

impl StateRef {
    pub fn new_empty<T>(txn: &mut T) -> Result<Self, DbError>
    where
        T: ::sanakirja::AllocPage
            + ::sanakirja::LoadPage<Error = ::sanakirja::Error>
            + ::sanakirja::RootPage,
    {
        let mut empty = Self {
            stake_control: btree::create_db_(txn).unwrap(),
            blocks: btree::create_db_(txn).unwrap(),
            address_id: btree::create_db_(txn).unwrap(),
            address_transactions: btree::create_db_(txn).unwrap(),
            votes: btree::create_db_(txn).unwrap(),

            next_address_id: None,
        };

        // TODO: extract [0u8; 65] to an Address::sigil function
        btree::put(
            txn,
            &mut empty.address_id,
            &Address([0u8; 65]),
            &SeqNum::new(0),
        )?;

        Ok(empty)
    }

    pub fn finish(mut self, txn: &mut SanakirjaMutTx) -> Result<SerializedStateRef, DbError> {
        // if the sequence counter for addresses was incremented previously, rewrite it
        if let Some(next_seq) = self.next_address_id {
            btree::del(txn, &mut self.address_id, &Address([0u8; 65]), None)?;

            debug_assert!(btree::put(
                txn,
                &mut self.address_id,
                &Address([0u8; 65]),
                &next_seq.next(),
            )?);
        }

        Ok(SerializedStateRef {
            stake_control: L64::new(self.stake_control.db),
            blocks: L64::new(self.blocks.db),
            address_id: L64::new(self.address_id.db),
            addresses: L64::new(self.address_transactions.db),
            votes: L64::new(self.votes.db),
        })
    }

    pub fn apply_vote(
        &mut self,
        _txn: &mut SanakirjaMutTx,
        _fragment_id: &FragmentId,
        _proposal_id: &ProposalId,
    ) -> Result<(), DbError> {
        todo!()
    }

    /// Add the given transaction to address at the end of the list
    /// This function *only* checks the last fragment to avoid repetition when a transaction has more
    /// than one (input|output) with the same address (eg: utxo input and change).
    pub fn add_transaction_to_address(
        &mut self,
        _txn: &mut SanakirjaMutTx,
        _fragment_id: &FragmentId,
        _address: &Address,
    ) -> Result<(), DbError> {
        todo!()
    }

    pub fn add_block_to_blocks(
        &mut self,
        _txn: &mut SanakirjaMutTx,
        _chain_length: &ChainLength,
        _block_id: &BlockId,
    ) -> Result<(), DbError> {
        todo!()
    }

    pub fn apply_output_to_stake_control(
        &mut self,
        _txn: &mut SanakirjaMutTx,
        _output: &transaction::Output<chain_addr::Address>,
    ) -> Result<(), DbError> {
        todo!()
    }

    pub fn add_stake_to_account(
        &mut self,
        _txn: &mut SanakirjaMutTx,
        _account: &chain_crypto::PublicKey<chain_crypto::Ed25519>,
        _value: Value,
    ) -> Result<(), DbError> {
        todo!()
    }

    pub fn substract_stake_from_account(
        &mut self,
        _txn: &mut SanakirjaMutTx,
        _account: &chain_crypto::PublicKey<chain_crypto::Ed25519>,
        _value: Value,
    ) -> Result<(), DbError> {
        todo!()
    }

    /// gc this fork so the allocated pages can be re-used
    ///
    /// # Safety
    ///
    /// It's important that any references to this particular state are not used anymore. For the
    /// current use-case, callers need to ensure that this snapshot is not referenced anymore in
    /// the `States` btree.
    pub unsafe fn drop(self, txn: &mut SanakirjaMutTx) -> Result<(), DbError> {
        let StateRef {
            stake_control,
            blocks,
            address_transactions,
            address_id,
            votes,
            next_address_id: _,
        } = self;

        btree::drop(txn, stake_control)?;
        btree::drop(txn, blocks)?;
        btree::drop(txn, address_id)?;
        btree::drop(txn, address_transactions)?;
        btree::drop(txn, votes)?;

        Ok(())
    }
}

impl SerializedStateRef {
    pub fn fork(&self, txn: &mut SanakirjaMutTx) -> Result<StateRef, DbError> {
        Ok(StateRef {
            stake_control: btree::fork_db(txn, &Db::from_page(self.stake_control.get()))?,
            blocks: btree::fork_db(txn, &Db::from_page(self.blocks.get()))?,
            address_id: btree::fork_db(txn, &Db::from_page(self.address_id.get()))?,
            address_transactions: btree::fork_db(txn, &Db::from_page(self.addresses.get()))?,
            votes: btree::fork_db(txn, &Db::from_page(self.votes.get()))?,
            next_address_id: None,
        })
    }
}

direct_repr!(SerializedStateRef);
