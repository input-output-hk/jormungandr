use crate::cli::command::Error;
use jcli_lib::key::read_bech32;
use std::path::PathBuf;
use structopt::StructOpt;
use thor::cli::{Alias, CliController};

#[derive(StructOpt, Debug)]
pub enum Wallets {
    /// recover wallet funds from mnemonic
    Use {
        #[structopt(name = "ALIAS")]
        alias: Alias,
    },
    /// recover wallet funds from qr code
    Import {
        #[structopt(name = "SECRET")]
        secret: PathBuf,

        #[structopt(short, long)]
        password: String,

        #[structopt(short, long)]
        testing: bool,

        #[structopt(short, long)]
        alias: Alias,
    },
    Delete {
        #[structopt(name = "ALIAS")]
        alias: Alias,
    },
    List,
}

impl Wallets {
    pub fn exec(self, mut model: CliController) -> Result<(), Error> {
        match self {
            Self::Use { alias } => {
                model.wallets_mut().set_default_alias(alias)?;
                model.save_config().map_err(Into::into)
            }
            Self::Import {
                secret,
                alias,
                testing,
                password,
            } => {
                let (_, data, _) = read_bech32(Some(&secret))?;
                model
                    .wallets_mut()
                    .add_wallet(alias, testing, data, &password)?;
                model.save_config().map_err(Into::into)
            }
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
