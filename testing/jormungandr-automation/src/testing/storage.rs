use chain_storage::{
    test_utils::{Block, BlockId},
    BlockInfo, BlockStore,
};
use rand::RngCore;
use rand_core::OsRng;
use std::path::{Path, PathBuf};

pub enum BranchCount {
    Unlimited,
    Limited(u32),
}

pub enum StopCriteria {
    BlocksCount(u32),
    SizeInMb(u32),
}

impl StopCriteria {
    pub fn criterion_met(&self, iteration_counter: u32) -> bool {
        iteration_counter >= self.end()
    }

    pub fn end(&self) -> u32 {
        match self {
            StopCriteria::BlocksCount(count) => *count,
            StopCriteria::SizeInMb(size) => size * 333,
        }
    }
}

const BLOCK_DATA_LENGTH: usize = 1024;

pub struct StorageBuilder {
    path: PathBuf,
    branches: BranchCount,
    stop_criteria: StopCriteria,
}

impl StorageBuilder {
    pub fn new<P: AsRef<Path>>(
        branches: BranchCount,
        stop_criteria: StopCriteria,
        path: P,
    ) -> Self {
        Self {
            branches,
            stop_criteria,
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn build(&self) {
        let mut rng = OsRng;
        let mut block_data = [0; BLOCK_DATA_LENGTH];

        rng.fill_bytes(&mut block_data);
        let genesis_block = Block::genesis(Some(Box::new(block_data)));

        let store = BlockStore::file(self.path.clone(), BlockId(0).serialize_as_vec()).unwrap();

        let genesis_block_info = BlockInfo::new(
            genesis_block.id.serialize_as_vec(),
            genesis_block.parent.serialize_as_vec(),
            genesis_block.chain_length,
        );

        store
            .put_block(&genesis_block.serialize_as_vec(), genesis_block_info)
            .unwrap();

        let mut blocks = vec![genesis_block];

        let mut iterations_counter = 0;
        loop {
            if self.stop_criteria.criterion_met(iterations_counter) {
                break;
            }

            let last_block = {
                match self.branches {
                    BranchCount::Unlimited => {
                        blocks.get(rng.next_u32() as usize % blocks.len()).unwrap()
                    }
                    BranchCount::Limited(count) => {
                        let limit = std::cmp::min(count, blocks.len() as u32);
                        blocks.get(limit as usize).unwrap()
                    }
                }
            };

            if iterations_counter % 100 == 0 {
                println!("{}/{}", iterations_counter, self.stop_criteria.end())
            }

            rng.fill_bytes(&mut block_data);
            let block = last_block.make_child(Some(Box::new(block_data)));
            blocks.push(block.clone());

            let block_info = BlockInfo::new(
                block.id.serialize_as_vec(),
                block.parent.serialize_as_vec(),
                block.chain_length,
            );
            store
                .put_block(&block.serialize_as_vec(), block_info)
                .unwrap();
            iterations_counter += 1;
        }
    }
}
