mod preferred_list;

pub use self::preferred_list::{PreferredListConfig, PreferredListLayer};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LayersConfig {
    #[serde(default)]
    pub preferred_list: PreferredListConfig,
}
