mod preferred_list;
mod rings;

pub use self::preferred_list::PreferredListConfig;
pub(super) use self::preferred_list::PreferredListLayer;
pub(super) use self::rings::Rings;
pub use self::rings::{ParseError, RingsConfig};

#[derive(Clone)]
pub struct LayersConfig {
    pub preferred_list: PreferredListConfig,
    pub rings: RingsConfig,
}
