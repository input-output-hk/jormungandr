use super::number::Number;

/// FeeHistory
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeHistory {
    /// Lowest number block of the returned range.
    oldest_block: Number,
    /// An array of block base fees per gas.
    /// This includes the next block after the newest of the returned range,
    /// because this value can be derived from the newest block. Zeroes are
    /// returned for pre-EIP-1559 blocks.
    base_fee_per_gas: Vec<Number>,
    /// An array of effective priority fee per gas data points from a single
    /// block. All zeroes are returned if the block is empty.
    reward: Vec<Vec<Number>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_history_serialize() {
        let fee_history = FeeHistory {
            oldest_block: 0.into(),
            base_fee_per_gas: vec![0.into()],
            reward: vec![vec![0.into()]],
        };

        assert_eq!(
            serde_json::to_string(&fee_history).unwrap(),
            r#"{"oldestBlock":"0x0","baseFeePerGas":["0x0"],"reward":[["0x0"]]}"#
        );
    }
}
