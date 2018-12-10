mod stake;
mod params;
mod vrf;
mod delegation;
mod identity;

pub use self::params::*;
pub use self::stake::*;
pub use self::vrf::*;
pub use self::identity::*;
pub use self::delegation::*;

/// Federal settings
pub struct Leaders(Vec<StakerIdentity>);
