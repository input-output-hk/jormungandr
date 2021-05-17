use chain_impl_mockchain::fee::LinearFee;
use std::path::Path;
use std::process::Command;

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
            .arg(&tx_id)
            .arg(tx_index.to_string())
            .arg(amount)
            .arg("--staging")
            .arg(staging_file.as_ref());
        self
    }

    pub fn add_account<P: AsRef<Path>>(
        mut self,
        account_addr: &str,
        spending_counter: u32,
        amount: &str,
        staging_file: P,
    ) -> Self {
        self.command
            .arg("add-account")
            .arg(account_addr.to_string())
            .arg(spending_counter.to_string())
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
            .arg(&addr)
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
        addr_type: &str,
        witness_file: P,
        witness_key: Q,
    ) -> Self {
        self.command
            .arg("make-witness")
            .arg("--genesis-block-hash")
            .arg(block0_hash)
            .arg("--type")
            .arg(&addr_type)
            .arg(&tx_id)
            .arg(witness_file.as_ref())
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
            .arg(&signing_key.as_ref());
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

    pub fn build(self) -> Command {
        println!("{:?}", self.command);
        self.command
    }
}
