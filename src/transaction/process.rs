use std::sync::{mpsc::Receiver, Arc, RwLock};

use crate::blockcfg::{BlockConfig, Ledger, Transaction};
use crate::blockchain::BlockchainR;
use crate::intercom::TransactionMsg;
use crate::transaction::TPool;

#[allow(type_alias_bounds)]
pub type TPoolR<B: BlockConfig> = Arc<RwLock<TPool<B::TransactionId, B::Transaction>>>;

pub fn transaction_task<B>(
    blockchain: BlockchainR<B>,
    tpool: TPoolR<B>,
    r: Receiver<TransactionMsg<B>>,
) -> !
where
    B: BlockConfig,
    B::TransactionId: Eq + std::hash::Hash,
{
    loop {
        let tquery = r.recv().unwrap();

        match tquery {
            TransactionMsg::ProposeTransaction(txids, reply) => {
                let tpool = tpool.read().unwrap();
                let rep: Vec<_> = txids.into_iter().map(|txid| tpool.exist(&txid)).collect();
                reply.reply_ok(rep);
            }
            TransactionMsg::SendTransaction(txs) => {
                let mut tpool = tpool.write().unwrap();
                let blockchain = blockchain.read().unwrap();
                let chain_state = &blockchain.ledger;

                // this will test the transaction is valid within the current
                // state of the local state of the global ledger.
                //
                // We don't want to keep transactions that are not valid within
                // our state of the blockchain as we will not be able to add them
                // in the blockchain.
                if let Err(error) = chain_state.diff(txs.iter()) {
                    warn!("Received transactions where some are invalid, {}", error);
                // TODO
                } else {
                    for tx in txs {
                        tpool.add(tx.id(), tx);
                    }
                }
            }
        }
    }
}
