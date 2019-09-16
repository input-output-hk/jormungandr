use chain_storage::error::Error as StorageError;

error_chain! {
    foreign_links {
        StorageError(StorageError);
    }
    errors {
        BlockNotFound(hash: String) {
            description("block not found"),
            display("block '{}' cannot be found in the explorer", hash)
        }
        AncestorNotFound(hash: String) {
            description("ancestor of block is not in explorer"),
            display("ancestor of block '{}' cannot be found in the explorer", hash)
        }
        TransactionAlreadyExists(id: String) {
            description("tried to index already existing transaction")
            display("transaction '{}' is already indexed", id)
        }
        BlockAlreadyExists(id: String) {
            description("tried to index already indexed block")
            display("block '{}' is already indexed", id)
        }
        ChainLengthBlockAlreadyExists(chain_length: u32) {
            description("tried to index already indexed chainlength in the given branch")
            display("chain length: {} is already indexed", chain_length)
        }
        BootstrapError(msg: String) {
            description("failed to initialize explorer's database from storage")
            display("the explorer's database couldn't be initialized: {}", msg)
        }
    }
}
