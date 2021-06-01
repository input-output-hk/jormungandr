pub mod fail_fast;
pub mod statuses;
use jormungandr_lib::interfaces::FragmentStatus;

pub fn assert_in_block(fragment_status: &FragmentStatus) {
    match fragment_status {
        FragmentStatus::InABlock { .. } => (),
        _ => panic!("should be in block '{:?}'", fragment_status),
    }
}

pub fn assert_not_in_block(fragment_status: &FragmentStatus) {
    let in_block = matches!(fragment_status, FragmentStatus::InABlock { .. });
    assert!(!in_block, "should NOT be in block '{:?}'", fragment_status);
}
