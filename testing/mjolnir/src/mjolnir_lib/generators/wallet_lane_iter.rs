use chain_impl_mockchain::{accounting::account::SpendingCounterIncreasing, testing::WitnessMode};

/// A wrapper for `Vec<Wallet>` that allows iteration over individual lanes of each wallet
pub(super) struct SplitLaneIter {
    split_marker: usize,
    next_lane: usize,
}

impl SplitLaneIter {
    pub fn new() -> Self {
        Self {
            split_marker: 1,
            next_lane: 0,
        }
    }

    /// Similar to `Iterator::next`, except it takes the length of the wallet vector each
    /// iteration, since it can change
    pub fn next(&mut self, wallet_count: usize) -> (usize, WitnessMode) {
        self.next_lane += 1;
        self.next_lane = match self.next_lane {
            // If all lanes used, reset count and increment wallet
            SpendingCounterIncreasing::LANES.. => {
                self.split_marker += 1;
                if self.split_marker >= wallet_count - 1 {
                    self.split_marker = 1;
                }
                0
            }
            i => i,
        };

        (
            self.split_marker,
            WitnessMode::Account {
                lane: self.next_lane,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract_lane(mode: WitnessMode) -> usize {
        match mode {
            WitnessMode::Account { lane } => lane,
            _ => unreachable!(),
        }
    }

    #[test]
    fn produces_elements_in_correct_order() {
        let mut iter = SplitLaneIter {
            next_lane: 0,
            split_marker: 1,
        };

        let mut elements = vec![];
        for _ in 0..24 {
            let (index, mode) = iter.next(5);
            elements.push((index, extract_lane(mode)));
        }

        assert_eq!(
            elements,
            vec![
                (1, 1),
                (1, 2),
                (1, 3),
                (1, 4),
                (1, 5),
                (1, 6),
                (1, 7),
                (2, 0),
                (2, 1),
                (2, 2),
                (2, 3),
                (2, 4),
                (2, 5),
                (2, 6),
                (2, 7),
                (3, 0),
                (3, 1),
                (3, 2),
                (3, 3),
                (3, 4),
                (3, 5),
                (3, 6),
                (3, 7),
                (1, 0),
            ]
        );
    }
}
