//! this module defines all the different static values used
//! in the block0 configuration.

/// default active slot coefficient in milli `0.100`
pub const DEFAULT_ACTIVE_SLOT_COEFFICIENT: u64 = 0_100;
/// minimum active slot coefficient in milli `0.001`
pub const MINIMUM_ACTIVE_SLOT_COEFFICIENT: u64 = 0_001;
/// maximum active slot coefficient in milli `1.000`
pub const MAXIMUM_ACTIVE_SLOT_COEFFICIENT: u64 = 1_000;

/// default BFT Slot ratio in milli `0.220`
pub const DEFAULT_BFT_SLOTS_RATIO: u64 = 0_220;
/// minimum BFT Slot ratio in milli `0.000`
pub const MINIMUM_BFT_SLOTS_RATIO: u64 = 0_000;
/// maximum BFT Slot ratio in milli `1.000`
pub const MAXIMUM_BFT_SLOTS_RATIO: u64 = 1_000;

/// default KES Update speed (in seconds): 12hours
pub const DEFAULT_KES_SPEED_UPDATE: u32 = 12 * 3600;
/// minimum KES Update speed (in seconds): 1minute
pub const MINIMUM_KES_SPEED_UPDATE_IN_SECONDS: u32 = 60;
/// maximum KES Update speed (in seconds): about one year
pub const MAXIMUM_KES_SPEED_UPDATE_IN_SECONDS: u32 = 365 * 24 * 3600;

/// default number of slots per epoch
pub const DEFAULT_NUMBER_OF_SLOTS_PER_EPOCH: u32 = 720;
/// minimum number of slots per epoch
pub const MINIMUM_NUMBER_OF_SLOTS_PER_EPOCH: u32 = 1;
/// maximum number of slots per epoch
pub const MAXIMUM_NUMBER_OF_SLOTS_PER_EPOCH: u32 = 1_000_000;

/// default slot duration in seconds
pub const DEFAULT_SLOT_DURATION: u8 = 5;
/// minimum slot duration in seconds
pub const MINIMUM_SLOT_DURATION: u8 = 1;
/// maximum slot duration in seconds (here is it max of u8: 255)
pub const MAXIMUM_SLOT_DURATION: u8 = u8::max_value();

/// when generating arbitrary values for property testing this will be the maximum
/// number of entries we will generate in an `Initial` fragment. This is in order
/// to avoid testing too large values that may not make sense. Updating this value
/// should only affect execution time of the tests
#[cfg(test)]
pub const ARBITRARY_MAX_NUMBER_ENTRIES_PER_INITIAL_FRAGMENT: usize = 8;
/// when generating arbitrary values for property testing this will be the maximum
/// number `Initial` fragment in the block0 configuration. This is in order
/// to avoid testing too large values that may not make sense. Updating this value
/// should only affect execution time of the tests
#[cfg(test)]
pub const ARBITRARY_MAX_NUMBER_INITIAL_FRAGMENTS: usize = 64;
