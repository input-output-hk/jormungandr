use super::blockchain::{Blockchain, Ref};
use crate::blockcfg::ChainLength;
use crate::blockchain::Multiverse;
use crate::intercom::ExplorerMsg;
use crate::utils::task::{Input, TokioServiceInfo};
use chain_impl_mockchain::multiverse::GCRoot;
use std::collections::HashMap;
use std::convert::Infallible;
use tokio::prelude::Future;

#[derive(Clone)]
pub struct Process {
    multiverse: Multiverse<Ref>,
    // This is kind of the same thing the multiverse holds (with Ref instead of BlockId)
    // FIXME: The constructor of `ChainLength` is private, so querying this thing could be
    // a problem
    chain_length_to_hash: HashMap<ChainLength, Vec<Ref>>,
    blockchain: Blockchain,
}

impl Process {
    pub fn new(blockchain: Blockchain) -> Self {
        Self {
            multiverse: Multiverse::<Ref>::new(),
            chain_length_to_hash: HashMap::new(),
            blockchain,
        }
    }

    pub fn handle_input(
        &mut self,
        info: &TokioServiceInfo,
        input: Input<ExplorerMsg>,
    ) -> Result<(), ()> {
        let _logger = info.logger();
        let bquery = match input {
            Input::Shutdown => {
                return Ok(());
            }
            Input::Input(msg) => msg,
        };

        match bquery {
            ExplorerMsg::NewBlock(new_block_ref) => {
                let _gcroot = self.store_ref(new_block_ref);
            }
        };

        Ok(())
    }

    fn store_ref(&mut self, new_block_ref: Ref) -> impl Future<Item = GCRoot, Error = Infallible> {
        let chain_length = new_block_ref.chain_length();
        let header_hash = new_block_ref.hash();

        self.chain_length_to_hash
            .entry(chain_length)
            .or_insert(Vec::new())
            .push(new_block_ref.clone());

        let gc_root = self
            .multiverse
            .insert(chain_length, header_hash, new_block_ref.clone());

        gc_root
    }
}
