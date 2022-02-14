
use thor::cli::{Alias,CliController};
use crate::cli::command::Error;
use jcli_lib::key::read_bech32;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Wallets {
    /// recover wallet funds from mnemonic
    Use {
        #[structopt(name = "ALIAS")]
        alias: Alias,
    },
    /// recover wallet funds from qr code
    Import {
        #[structopt(short, long)]
        alias: Alias,

        #[structopt(subcommand)] // Note that we mark a field as a subcommand
        cmd: WalletAddSubcommand,
    },
    Delete {
        #[structopt(name = "ALIAS")]
        alias: Alias,
    },
    List,
}

#[derive(StructOpt, Debug)]
pub struct WalletAddSubcommand {

        #[structopt(name = "SECRET")]
        secret: PathBuf,

        #[structopt(short, long)]
        password: String,

        #[structopt(short, long)]
        testing: bool,
}

impl WalletAddSubcommand {
    pub fn add_wallet(
        self,
        mut controller: CliController,
        alias: Alias,
    ) -> Result<(), Error> {
        let (_, data, _) = read_bech32(Some(&self.secret))?;
        controller.wallets_mut().add_wallet(alias, self.testing, data, &self.password)?;
        controller.save_config().map_err(Into::into)
    }
}

impl Wallets {
    pub fn exec(self, mut model: CliController) -> Result<(), Error> {
        match self {
            Self::Use { alias } => {
                model.wallets_mut().set_default_alias(alias)?;
                model.save_config().map_err(Into::into)
            }
            Self::Import { alias, cmd } => cmd.add_wallet(model, alias),
            Self::Delete { alias } => {
                model.wallets_mut().remove_wallet(alias)?;
                model.save_config().map_err(Into::into)
            }
            Self::List => {
                for (idx, (alias, wallet)) in model.wallets().iter().enumerate() {
                    if Some(alias) == model.wallets().default_alias() {
                        println!("[Default]{}.\t{}\t{}", idx + 1, alias, wallet.public_key);
                    } else {
                        println!("{}.\t{}\t{}", idx + 1, alias, wallet.public_key);
                    }
                }
                Ok(())
            }
        }
    }
}
