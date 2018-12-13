use super::identity::{StakerIdentity, StakerSignature};

pub struct Certificate {
    _delegatee: StakerIdentity,
    _delegator: StakerIdentity,
    _signature: StakerSignature,
}
