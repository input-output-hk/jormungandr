use crate::common::io;
extern crate libjcli;

use libjcli::jcli_app::transaction::{common::CommonTransaction, new::New};

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
