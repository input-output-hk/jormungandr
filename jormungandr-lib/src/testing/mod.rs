/// Module contains cross project test utils
mod measurement;

pub use measurement::{
    benchmark_consumption, benchmark_efficiency, benchmark_endurance, benchmark_speed,
    EfficiencyBenchmarkDef, EfficiencyBenchmarkFinish, EfficiencyBenchmarkRun, Endurance,
    EnduranceBenchmarkDef, EnduranceBenchmarkFinish, EnduranceBenchmarkRun, ResourcesUsage, Speed,
    SpeedBenchmarkDef, SpeedBenchmarkFinish, SpeedBenchmarkRun, Thresholds, Timestamp,
};
