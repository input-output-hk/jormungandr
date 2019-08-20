use super::blockchain::{Blockchain, Ref};
use crate::blockcfg::Block;
use crate::blockcfg::ChainLength;
use crate::intercom::ExplorerMsg;
use crate::utils::task::{Input, ThreadServiceInfo};
use chain_impl_mockchain::multiverse::{GCRoot, Multiverse};
use futures::Future;
use futures::IntoFuture;
use std::collections::{HashMap, HashSet};

use chain_core::property::Block as _;

pub struct Process {
    multiverse: Multiverse<Ref>,
    // This is kind of the same thing the multiverse holds (with Ref instead of BlockId)
    chain_length_to_hash: HashMap<ChainLength, Vec<Ref>>,
}

impl Process {
    pub fn new() -> Self {
        Self {
            multiverse: Multiverse::<Ref>::new(),
            chain_length_to_hash: HashMap::new(),
        }
    }

    pub fn handle_input(
        &mut self,
        info: &ThreadServiceInfo,
        blockchain: &Blockchain,
        input: Input<ExplorerMsg>,
    ) {
        let logger = info.logger();
        let bquery = match input {
            Input::Shutdown => {
                return;
            }
            Input::Input(msg) => msg,
        };

        match bquery {
            ExplorerMsg::NewBlock(new_block_ref) => {
                let _gcroot = self.store_ref(new_block_ref);
            }
        };
    }

    fn store_ref(&mut self, new_block_ref: Ref) -> GCRoot {
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
