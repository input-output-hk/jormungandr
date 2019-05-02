use crate::blockcfg::{Message, MessageId};
use crate::blockchain::BlockchainR;
use crate::intercom::{do_stream_reply, TransactionMsg};
use crate::rest::v0::node::stats::StatsCounter;
use crate::transaction::TPool;
use crate::utils::task::{Input, ThreadServiceInfo};
use chain_core::property::{ChainLength as _, Message as _};
use std::sync::{Arc, RwLock};

#[allow(type_alias_bounds)]
pub type TPoolR = Arc<RwLock<TPool<MessageId, Message>>>;

pub fn handle_input(
    _info: &ThreadServiceInfo,
    _: &BlockchainR,
    tpool: &TPoolR,
    stats_counter: &StatsCounter,
    input: Input<TransactionMsg>,
) {
    let tquery = match input {
        Input::Shutdown => return,
        Input::Input(msg) => msg,
    };
    match tquery {
        TransactionMsg::ProposeTransaction(txids, reply) => {
            let tpool = tpool.read().unwrap();
            let rep: Vec<_> = txids.into_iter().map(|txid| tpool.exist(&txid)).collect();
            reply.reply_ok(rep);
        }
        TransactionMsg::SendTransaction(txs) => {
            let mut tpool = tpool.write().unwrap();

            // Note that we cannot use apply_block here, since we don't have a valid context to which to apply
            // those blocks. one valid tx in a given context, could be invalid in another. for example
            // fee calculations, existence utxo / account solvency.

            // FIXME/TODO check that the txs are valid within themselves with basic requirements (e.g. inputs >= outputs).
            // we also want to keep a basic capability to filter away repetitive queries or definitely discarded txid.

            // This interface only makes sense for messages coming from arbitrary users (like transaction, certificates),
            // for other message we don't want to receive them through this interface, and possibly
            // put them in another pool.

            stats_counter.add_tx_recv_cnt(txs.len());
            for tx in txs {
                tpool.add(tx.id(), tx);
            }
        }
        TransactionMsg::GetTransactions(txids, handler) => do_stream_reply(handler, |handler| {
            let tpool = tpool.read().unwrap();
            for id in txids {
                match tpool.get(&id) {
                    Some(tx) => handler.send(tx),
                    None => (),
                }
            }
            Ok(())
        }),
    }
}
