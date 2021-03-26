#[derive(Debug, Clone)]
pub enum MeasurementReportInterval {
    Standard,
    Long,
}

impl From<MeasurementReportInterval> for u32 {
    fn from(interval: MeasurementReportInterval) -> u32 {
        match interval {
            MeasurementReportInterval::Standard => 20,
            MeasurementReportInterval::Long => 100,
        }
    }
}

pub struct MeasurementReporter {
    interval: u32,
    counter: u32,
}

impl MeasurementReporter {
    pub fn new(interval: MeasurementReportInterval) -> Self {
        Self {
            interval: interval.into(),
            counter: 0u32,
        }
    }

    pub fn is_interval_reached(&self) -> bool {
        self.counter >= self.interval
    }

    pub fn do_if_interval_reached<F: std::marker::Send>(&self, method: F)
    where
        F: Fn(),
    {
        if self.is_interval_reached() {
            method();
        }
    }

    pub fn do_if_interval_reached_and_inc<F: std::marker::Send>(&mut self, method: F)
    where
        F: Fn(),
    {
        if self.is_interval_reached() {
            method();
            self.counter = 0;
        } else {
            self.increment();
        }
    }

    pub fn increment(&mut self) {
        self.counter += 1;
    }
}
