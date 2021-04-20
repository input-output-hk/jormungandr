use structopt::StructOpt;
use crate::transaction;
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Send {
    input_address_sk: String,
    faucet_address: String,
    amount: u64,
    receiver_address: Option<String>,
}

impl Send {
    pub fn exec(&self) -> std::io::Result<()> {
        transaction::new::New
        Ok(())
    }
}
