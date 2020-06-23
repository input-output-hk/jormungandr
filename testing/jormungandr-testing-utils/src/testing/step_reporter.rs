pub struct StepReporter {
    step_name: Option<String>,
}

impl StepReporter {
    pub fn new() -> Self {
        Self { step_name: None }
    }

    pub fn step<S: Into<String>>(&mut self, step_name: S) {
        self.end_step();
        self.begin_step(step_name);
    }

    pub fn begin_step<S: Into<String>>(&mut self, step_name: S) {
        self.step_name = Some(step_name.into());
        println!("Step '{}' started.", self.step_name.clone().unwrap());
    }

    pub fn end_step(&mut self) {
        if self.step_name.is_none() {
            return;
        }
        println!("Step '{}' finished.", self.step_name.clone().unwrap());
    }
}
