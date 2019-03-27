use self::tx_data::TxData;
use cardano::util::hex;
use std::fs::File;
use std::io::{self, Write as _};
use std::path::PathBuf;
use structopt::StructOpt;

mod tx_data;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Build {
    #[structopt(flatten)]
    tx_data: TxData,
    /// create or update transaction builder state file
    #[structopt(short, long)]
    file: Option<PathBuf>,
    /// do not generate final transaction
    #[structopt(short, long)]
    draft: bool,
}

impl Build {
    pub fn exec(mut self) {
        self.sync_tx_data_file();
        self.print_tx();
    }

    fn sync_tx_data_file(&mut self) {
        let path = match &self.file {
            Some(path) => path,
            None => return,
        };
        if path.exists() {
            let reader = File::open(path).unwrap();
            let tx_data = serde_yaml::from_reader(reader).unwrap();
            self.tx_data.merge_old(tx_data);
        }
        let writer = File::create(path).unwrap();
        serde_yaml::to_writer(writer, &self.tx_data).unwrap();
    }

    fn print_tx(&self) {
        if self.draft {
            return;
        }
        let tx = self.tx_data.build_tx();
        let tx_hex = hex::encode(&tx);
        write!(io::stdout(), "{}\n", tx_hex).unwrap();
    }
}
