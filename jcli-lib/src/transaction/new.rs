#[cfg(feature = "structopt")]
use structopt::StructOpt;

use crate::transaction::{common, staging::Staging, Error};

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub struct New {
    #[cfg_attr(feature = "structopt", structopt(flatten))]
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
    use assert_fs::prelude::*;
    use assert_fs::NamedTempFile;
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
