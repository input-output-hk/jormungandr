use structopt::StructOpt;

use crate::jcli_app::transaction::{common, staging::Staging, Error};

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct New {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,
}

impl New {
    pub fn exec(self) -> Result<(), Error> {
        let staging = Staging::new();
        self.common.store(&staging)
    }
}

#[cfg(test)]
mod tests {

    use self::common::CommonTransaction;
    use super::*;

    #[test]
    pub fn test_staging_file_is_created() {
        let tempfile = mktemp::Temp::new_file().unwrap();

        let temp_staging_file = tempfile.to_path_buf();

        let new = New {
            common: CommonTransaction {
                staging_file: Some(temp_staging_file.clone()),
            },
        };
        new.exec().expect(" error while executing New action");

        assert_eq!(
            temp_staging_file.is_file(),
            true,
            "staging file {:?} not created",
            &temp_staging_file
        );
    }
}
