#[cfg(not(feature = "optimized-node"))]
mod reference;

#[cfg(not(feature = "optimized-node"))]
pub use reference::*;

#[cfg(feature = "optimized-node")]
mod optimized;

#[cfg(feature = "optimized-node")]
pub use optimized::*;
