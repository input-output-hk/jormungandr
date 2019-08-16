use super::blockchain::Blockchain;
use crate::blockcfg::Block;
use crate::blockcfg::ChainLength;
use crate::intercom::ExplorerMsg;
use crate::utils::task::{Input, ThreadServiceInfo};
use chain_impl_mockchain::multiverse::{GCRoot, Multiverse};
use futures::Future;
use futures::IntoFuture;

use chain_core::property::Block as _;

pub struct Process {
    multiverse: Multiverse<State>,
}

struct State {
    block: Block,
}

impl State {
    pub fn new(block: Block) -> Self {
        Self { block }
    }
}

impl Process {
    pub fn new() -> Self {
        Self {
            multiverse: Multiverse::<State>::new(),
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
            ExplorerMsg::NewBlock(header) => {
                //I don't really understand what's the purpose of the gcroot yet
                let _gcroot: Result<GCRoot, ()> = blockchain
                    .storage()
                    .get(header.hash())
                    .or_else(|storage_error| {
                        error!(logger, "{}", storage_error);
                        Err(())
                    })
                    .and_then(|block| {
                        debug!(
                            logger,
                            "storing blockhash: {} with chain_length: {}",
                            header.hash(),
                            header.chain_length()
                        );

                        block
                            .map(|some_block| self.store_block(some_block))
                            .unwrap_or_else(|| Err(error!(logger, "Block is not in storage")))
                    })
                    .wait();
            }
        };
    }

    fn store_block(&mut self, block: Block) -> Result<GCRoot, ()> {
        let header = &block.header;
        let _gcroot =
            self.multiverse
                .insert(header.chain_length(), header.hash(), State::new(block));

        Ok(_gcroot)
    }

    fn _get_block_by_chain_length(_length: ChainLength) {
        unimplemented!()
    }
}
