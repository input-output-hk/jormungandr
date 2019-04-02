use crate::blockcfg::{Message, MessageId};
use crate::blockchain::BlockchainR;
use crate::intercom::{do_stream_reply, TransactionMsg};
use crate::rest::v0::node::stats::StatsCounter;
use crate::transaction::TPool;
use crate::utils::task::{Input, ThreadServiceInfo};
use chain_core::property::Message as _;
use std::sync::{Arc, RwLock};

#[allow(type_alias_bounds)]
pub type TPoolR = Arc<RwLock<TPool<MessageId, Message>>>;

pub fn handle_input(
    _info: &ThreadServiceInfo,
    blockchain: &BlockchainR,
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
            let blockchain = blockchain.lock_read();
            let chain_state = blockchain.multiverse.get(&blockchain.tip).unwrap();
            let parameters = chain_state.get_ledger_parameters();

            // this will test the transaction is valid within the current
            // state of the local state of the global ledger.
            //
            // We don't want to keep transactions that are not valid within
            // our state of the blockchain as we will not be able to add them
            // in the blockchain.
            if let Err(error) = chain_state.apply_block(&parameters, txs.iter()) {
                warn!("Received transactions where some are invalid, {}", error);
            // TODO
            } else {
                stats_counter.add_tx_recv_cnt(txs.len());
                for tx in txs {
                    tpool.add(tx.id(), tx);
                }
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
