mod delegation;
mod identity;
mod params;
mod stake;
mod vrf;

pub use self::delegation::*;
pub use self::identity::*;
pub use self::params::*;
pub use self::stake::*;
pub use self::vrf::*;

/// Federal settings
pub struct Leaders(Vec<StakerIdentity>);
