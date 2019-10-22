
/// Inputs & Outputs for a transaction being built
pub struct InputOutputBuilder {
    inputs: Vec<Input>,
    outputs: Vec<Output<Address>>,
}

/// Inputs & Outputs for a built transaction
pub struct InputOutput {
    inputs: Box<[Input]>,
    outputs: Box<[Outputs]>,
}

impl InputOutputBuilder {
    /// Build the InputOutput from the Builder
    pub fn build(self) -> InputOutput {
        InputOutput {
            inputs: self.input.into(),
            outputs: self.output.into(),
        }
    }
}
