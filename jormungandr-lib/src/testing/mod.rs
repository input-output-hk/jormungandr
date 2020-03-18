/// Module contains cross project test utils
mod measurement;

pub use measurement::{
    benchmark_consumption, benchmark_efficiency, benchmark_endurance, benchmark_speed,
    EfficiencyBenchmarkDef, EfficiencyBenchmarkFinish, EfficiencyBenchmarkRun, Endurance,
    EnduranceBenchmarkDef, EnduranceBenchmarkFinish, EnduranceBenchmarkRun, Speed,
    SpeedBenchmarkDef, SpeedBenchmarkFinish, SpeedBenchmarkRun, Thresholds, Timestamp,
};
