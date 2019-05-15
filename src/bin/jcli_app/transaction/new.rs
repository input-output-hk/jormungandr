use structopt::StructOpt;

use jcli_app::transaction::{
    common,
    staging::{Staging, StagingError},
};

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct New {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,
}

custom_error! {pub NewError
    WriteTransaction { source: StagingError } = "cannot create new transaction"
}

impl New {
    pub fn exec(self) -> Result<(), NewError> {
        let staging = Staging::new();
        Ok(self.common.store(&staging)?)
    }
}

#[cfg(test)]
mod tests {

    use self::common::CommonTransaction;
    use super::*;
    use jcli_app::utils::io;

    #[test]
    pub fn test_staging_file_is_created() {
        let temp_staging_file = io::get_path_in_temp("staging_file.tx").unwrap();

        let new = New {
            common: CommonTransaction {
                staging_file: Some(temp_staging_file.clone()),
            },
        };
        new.exec().expect(" error while executing New action");;

        assert_eq!(
            temp_staging_file.is_file(),
            true,
            "staging file {:?} not created",
            &temp_staging_file
        );
    }
}
