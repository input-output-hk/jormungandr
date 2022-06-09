use crate::jcli_lib::transaction::{common, staging::Staging, Error};
use structopt::StructOpt;

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
    use assert_fs::{prelude::*, NamedTempFile};
    use predicates::prelude::*;

    #[test]
    pub fn test_staging_file_is_created() {
        let tempfile = NamedTempFile::new("staging").unwrap();

        let new = New {
            common: CommonTransaction {
                staging_file: Some(tempfile.path().into()),
            },
        };
        new.exec().expect(" error while executing New action");

        tempfile.assert(predicate::path::is_file());
    }
}
