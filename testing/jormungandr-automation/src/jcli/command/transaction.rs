use crate::jcli::WitnessType;
use chain_impl_mockchain::{account::SpendingCounter, fee::LinearFee};
use std::{path::Path, process::Command};

#[derive(Debug)]
pub struct TransactionCommand {
    command: Command,
}

impl TransactionCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn new_transaction<P: AsRef<Path>>(mut self, staging_file: P) -> Self {
        self.command
            .arg("new")
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn add_input<P: AsRef<Path>>(
        mut self,
        tx_id: &str,
        tx_index: u8,
        amount: &str,
        staging_file: P,
    ) -> Self {
        self.command
            .arg("add-input")
            .arg(tx_id)
            .arg(tx_index.to_string())
            .arg(amount)
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn add_account<P: AsRef<Path>>(
        mut self,
        account_addr: &str,
        amount: &str,
        staging_file: P,
    ) -> Self {
        self.command
            .arg("add-account")
            .arg(account_addr)
            .arg(amount)
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn add_certificate<S: Into<String>, P: AsRef<Path>>(
        mut self,
        certificate: S,
        staging_file: P,
    ) -> Self {
        self.command
            .arg("add-certificate")
            .arg(certificate.into())
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn add_output<P: AsRef<Path>>(mut self, addr: &str, amount: &str, staging_file: P) -> Self {
        self.command
            .arg("add-output")
            .arg(addr)
            .arg(amount)
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn finalize<P: AsRef<Path>>(mut self, staging_file: P) -> Self {
        self.command
            .arg("finalize")
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn finalize_with_fee<P: AsRef<Path>>(
        mut self,
        address: &str,
        linear_fees: &LinearFee,
        staging_file: P,
    ) -> Self {
        self.command
            .arg("finalize")
            .arg(address)
            .arg("--fee-certificate")
            .arg(linear_fees.certificate.to_string())
            .arg("--fee-coefficient")
            .arg(linear_fees.coefficient.to_string())
            .arg("--fee-constant")
            .arg(linear_fees.constant.to_string())
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn make_witness<P: AsRef<Path>, Q: AsRef<Path>>(
        mut self,
        block0_hash: &str,
        tx_id: &str,
        addr_type: WitnessType,
        account_spending_counter: Option<SpendingCounter>,
        witness_file: P,
        witness_key: Q,
    ) -> Self {
        let spending_counter = account_spending_counter.unwrap_or_else(SpendingCounter::zero);
        self.command
            .arg("make-witness")
            .arg("--genesis-block-hash")
            .arg(block0_hash)
            .arg("--type")
            .arg(addr_type.to_string())
            .arg(tx_id)
            .arg(witness_file.as_ref())
            .arg("--account-spending-counter")
            .arg(spending_counter.unlaned_counter().to_string())
            .arg("--account-spending-counter-lane")
            .arg(spending_counter.lane().to_string())
            .arg(witness_key.as_ref());
        self
    }

    pub fn add_witness<P: AsRef<Path>, Q: AsRef<Path>>(
        mut self,
        witness_file: P,
        staging_file: Q,
    ) -> Self {
        self.command
            .arg("add-witness")
            .arg(witness_file.as_ref())
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn seal<P: AsRef<Path>>(mut self, staging_file: P) -> Self {
        self.command
            .arg("seal")
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn auth<P: AsRef<Path>, Q: AsRef<Path>>(mut self, signing_key: P, staging_file: Q) -> Self {
        self.command
            .arg("auth")
            .arg("--staging")
            .arg(staging_file.as_ref())
            .arg("--key")
            .arg(signing_key.as_ref());
        self
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_message<P: AsRef<Path>>(mut self, staging_file: P) -> Self {
        self.command
            .arg("to-message")
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn id<P: AsRef<Path>>(mut self, staging_file: P) -> Self {
        self.command
            .arg("data-for-witness")
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn info<P: AsRef<Path>>(mut self, format: &str, staging_file: P) -> Self {
        self.command
            .arg("info")
            .arg("--format")
            .arg(format)
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn make_transaction(
        mut self,
        host: String,
        sender: jormungandr_lib::interfaces::Address,
        receiver: Option<jormungandr_lib::interfaces::Address>,
        value: jormungandr_lib::interfaces::Value,
        block0_hash: String,
        expiry_date: jormungandr_lib::interfaces::BlockDate,
        secret: impl AsRef<Path>,
        staging_file: impl AsRef<Path>,
        post: bool,
    ) -> Self {
        self.command
            .arg("make-transaction")
            .arg("--secret")
            .arg(secret.as_ref())
            .arg("--staging")
            .arg(staging_file.as_ref())
            .arg("--host")
            .arg(host)
            .arg("--block0-hash")
            .arg(block0_hash)
            .arg("--valid-until")
            .arg(&expiry_date.to_string())
            .arg("--force");

        if post {
            self.command.arg("--post");
        }
        if let Some(receiver) = receiver {
            self.command.arg("--receiver").arg(receiver.to_string());
        };

        self.command.arg(sender.to_string()).arg(value.to_string());
        self
    }

    pub fn set_expiry_date<P: AsRef<Path>>(mut self, expiry_date: &str, staging_file: P) -> Self {
        self.command
            .arg("set-expiry-date")
            .arg(expiry_date)
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn build(self) -> Command {
        println!("{:?}", self.command);
        self.command
    }
}
