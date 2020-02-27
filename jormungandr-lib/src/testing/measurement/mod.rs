mod attribute;
mod benchmark;
mod marker;
mod status;
mod thresholds;

pub use attribute::{Efficiency, Endurance, Speed};
pub use benchmark::{
    benchmark_efficiency, benchmark_endurance, benchmark_speed, EfficiencyBenchmarkDef,
    EfficiencyBenchmarkFinish, EfficiencyBenchmarkRun, EnduranceBenchmarkDef,
    EnduranceBenchmarkFinish, EnduranceBenchmarkRun, SpeedBenchmarkDef, SpeedBenchmarkFinish,
    SpeedBenchmarkRun,
};
pub use marker::{Counter, Timestamp};
pub use status::Status;
pub use thresholds::Thresholds;
