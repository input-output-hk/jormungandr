use std::sync::{
    Arc, RwLock,
    mpsc::{Receiver},
};

use crate::transaction::{TPool};
use crate::blockcfg::{BlockConfig, ledger::{Transaction}};
use crate::intercom::{TransactionMsg};

#[allow(type_alias_bounds)]
pub type TPoolR<B: BlockConfig> = Arc<RwLock<TPool<B::TransactionId, B::Transaction>>>;

pub fn transaction_task<B>(tpool: TPoolR<B>, r: Receiver<TransactionMsg<B>>)
    -> !
    where B: BlockConfig
        , B::TransactionId: Eq+std::hash::Hash
{
    loop {
        let tquery = r.recv().unwrap();

        match tquery {
            TransactionMsg::ProposeTransaction(txids, mut reply) => {
                let tpool = tpool.read().unwrap();
                let rep : Vec<_> = txids.into_iter().map(|txid| tpool.exist(&txid)).collect();
                reply.reply_ok(rep);
            }
            TransactionMsg::SendTransaction(txs) => {
                let mut tpool = tpool.write().unwrap();
                for tx in txs {
                    tpool.add(tx.id(), tx);
                }
            }
        }
    }

}
