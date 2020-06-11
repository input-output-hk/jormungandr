mod attribute;
mod benchmark;
mod marker;
mod status;
mod thresholds;

pub use attribute::{Efficiency, Endurance, NamedProcess, Speed};
pub use benchmark::{
    benchmark_consumption, benchmark_efficiency, benchmark_endurance, benchmark_speed,
    ConsumptionBenchmarkError, ConsumptionBenchmarkRun, EfficiencyBenchmarkDef,
    EfficiencyBenchmarkFinish, EfficiencyBenchmarkRun, EnduranceBenchmarkDef,
    EnduranceBenchmarkFinish, EnduranceBenchmarkRun, SpeedBenchmarkDef, SpeedBenchmarkFinish,
    SpeedBenchmarkRun,
};
pub use marker::{Counter, ResourcesUsage, Timestamp};
pub use status::Status;
pub use thresholds::Thresholds;
