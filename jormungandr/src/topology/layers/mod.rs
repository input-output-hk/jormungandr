mod preferred_list;
mod rings;

pub use self::{
    preferred_list::PreferredListConfig,
    rings::{ParseError, RingsConfig},
};
pub(super) use self::{preferred_list::PreferredListLayer, rings::Rings};

#[derive(Clone)]
pub struct LayersConfig {
    pub preferred_list: PreferredListConfig,
    pub rings: RingsConfig,
}
