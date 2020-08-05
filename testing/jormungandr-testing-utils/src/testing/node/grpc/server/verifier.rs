use super::{logger::MethodType, MockLogger};

pub struct MockVerifier {
    logger: MockLogger,
}

impl MockVerifier {
    pub fn new(logger: MockLogger) -> Self {
        Self { logger }
    }

    pub fn method_executed_at_least_once(&self, method: MethodType) -> bool {
        self.logger.executed_at_least_once(method)
    }
}
