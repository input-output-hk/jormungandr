pub mod comm;
pub mod network;
pub mod non_functional;
pub mod utils;

use jormungandr_lib::interfaces::FragmentStatus;

use std::time::Duration;

error_chain! {
    links {
        Node(crate::node::Error, crate::node::ErrorKind);
        Wallet(crate::wallet::Error, crate::wallet::ErrorKind);
        Scenario(crate::scenario::Error, crate::scenario::ErrorKind);
    }

    errors {
        SyncTimeoutOccurred(info: String, timeout: Duration) {
            description("synchronization for nodes has failed"),
            display("synchronization for nodes has failed. {}. Timeout was: {} s", info, timeout.as_secs()),
        }

        AssertionFailed(info: String) {
            description("assertion has failed"),
            display("{}", info),
        }
        TransactionNotInBlock(node: String, status: FragmentStatus) {
            description("transaction not in block"),
            display("transaction should be 'In Block'. status: {:?}, node: {}", status, node),
        }


    }
}
