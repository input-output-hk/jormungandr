pub mod era;
pub mod timeframe;
pub mod timeline;
pub mod units;

pub use era::{Epoch, TimeEra};
pub use timeframe::{Slot, SlotDuration, TimeFrame};
pub use timeline::{TimeOffsetSeconds, Timeline};
pub use units::DurationSeconds;

#[macro_use]
extern crate cfg_if;

cfg_if! {
   if #[cfg(test)] {
        pub mod testing;
    } else if #[cfg(feature = "property-test-api")] {
        pub mod testing;
    }
}
