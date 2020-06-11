mod consumption_benchmark;
mod efficiency_benchmark;
mod endurance_benchmark;
mod speed_benchmark;

pub use efficiency_benchmark::{
    EfficiencyBenchmarkDef, EfficiencyBenchmarkFinish, EfficiencyBenchmarkRun,
};
pub use endurance_benchmark::{
    EnduranceBenchmarkDef, EnduranceBenchmarkFinish, EnduranceBenchmarkRun,
};

pub use consumption_benchmark::{
    ConsumptionBenchmarkDef, ConsumptionBenchmarkError, ConsumptionBenchmarkFinish,
    ConsumptionBenchmarkRun,
};

pub use speed_benchmark::{SpeedBenchmarkDef, SpeedBenchmarkFinish, SpeedBenchmarkRun};

pub fn benchmark_efficiency<S: Into<String>>(name: S) -> EfficiencyBenchmarkDef {
    EfficiencyBenchmarkDef::new(name.into())
}
pub fn benchmark_endurance<S: Into<String>>(name: S) -> EnduranceBenchmarkDef {
    EnduranceBenchmarkDef::new(name.into())
}
pub fn benchmark_speed<S: Into<String>>(name: S) -> SpeedBenchmarkDef {
    SpeedBenchmarkDef::new(name.into())
}

pub fn benchmark_consumption<S: Into<String>>(name: S) -> ConsumptionBenchmarkDef {
    ConsumptionBenchmarkDef::new(name.into())
}
