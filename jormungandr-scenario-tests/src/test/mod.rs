pub mod comm;
pub mod network;
pub mod utils;

use jormungandr_lib::interfaces::FragmentStatus;

error_chain! {
    links {
        Node(crate::node::Error, crate::node::ErrorKind);
        Wallet(crate::wallet::Error, crate::wallet::ErrorKind);
        Scenario(crate::scenario::Error, crate::scenario::ErrorKind);
    }

    errors {
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
