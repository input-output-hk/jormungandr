#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitmapIndex(u128, u128);

impl BitmapIndex {
    pub fn new() -> Self {
        Self(0u128, 0u128)
    }

    #[inline]
    fn get_mask_and_part(&self, idx: u8) -> (u128, u8) {
        let mask: u128 = 1 << (idx % 128);
        let part: u8 = idx / 128;
        (mask, part)
    }

    #[inline]
    pub fn get_index(&self, idx: u8) -> bool {
        let (mask, part) = self.get_mask_and_part(idx);

        if part == 0 {
            (self.0 & mask) != 0
        } else {
            (self.1 & mask) != 0
        }
    }

    #[inline]
    pub fn set_index(&mut self, idx: u8) {
        let (mask, part) = self.get_mask_and_part(idx);

        if part == 0 {
            self.0 |= mask;
        } else {
            self.1 |= mask;
        }
    }

    #[inline]
    pub fn remove_index(&mut self, idx: u8) {
        let (mask, part) = self.get_mask_and_part(idx);

        if part == 0 {
            self.0 &= !mask;
        } else {
            self.1 &= !mask;
        }
    }

    #[inline]
    #[cfg_attr(target_arch = "x86_64", target_feature(enable = "popcnt"))]
    unsafe fn get_real_index_impl(&self, idx: u8) -> Option<u8> {
        if !self.get_index(idx) {
            return None;
        }

        let (mask, part) = self.get_mask_and_part(idx);
        let mask = mask - 1;

        if part == 0 {
            Some((self.0 & mask).count_ones() as u8)
        } else {
            let count = self.0.count_ones() + (self.1 & mask).count_ones();
            Some(count as u8)
        }
    }

    #[inline]
    pub fn get_real_index(&self, idx: u8) -> Option<u8> {
        unsafe { self.get_real_index_impl(idx) }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0 && self.1 == 0
    }

    #[inline]
    // #[cfg_attr(target_arch = "x86_64", target_feature(enable = "bmi1"))]
    unsafe fn get_first_index_impl(&self) -> Option<u8> {
        let trailing_zeros0 = self.0.trailing_zeros();
        let trailing_zeros1 = self.1.trailing_zeros();
        if trailing_zeros0 < 128 {
            Some(trailing_zeros0 as u8)
        } else if trailing_zeros1 < 128 {
            Some(128u8 + trailing_zeros1 as u8)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_first_index(&self) -> Option<u8> {
        unsafe { self.get_first_index_impl() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_empty_when_created_test() {
        let bitmap = BitmapIndex::new();
        assert!(bitmap.is_empty());
        assert!(bitmap.get_first_index().is_none());
    }

    #[quickcheck]
    fn set_index_test(indices: Vec<u8>) -> bool {
        let mut bitmap = BitmapIndex::new();
        for idx in indices.iter() {
            bitmap.set_index(*idx);
        }
        indices.iter().all(|idx| bitmap.get_index(*idx))
            && !(bitmap.is_empty() && indices.len() > 0)
    }

    #[quickcheck]
    fn remove_indextest(indices: Vec<u8>) -> bool {
        // this test will not work correctly if there are two same numbers in
        // both splits (see below)
        let mut indices = indices;
        indices.sort();
        indices.dedup();

        let mut bitmap = BitmapIndex::new();
        for idx in indices.iter() {
            bitmap.set_index(*idx);
        }

        // split indices vector in two and remove elements only from the first
        // vector
        let (to_remove, to_set) = indices.split_at(indices.len() / 2);
        for idx in to_remove.iter() {
            bitmap.remove_index(*idx);
        }
        to_remove.iter().all(|idx| !bitmap.get_index(*idx))
            && to_set.iter().all(|idx| bitmap.get_index(*idx))
    }

    #[quickcheck]
    fn get_real_index_test(indices: Vec<u8>) -> bool {
        let mut indices = indices;
        indices.sort();
        indices.dedup();
        let mut bitmap = BitmapIndex::new();
        for idx in indices.iter() {
            bitmap.set_index(*idx);
        }
        indices
            .iter()
            .enumerate()
            .all(|(expected, idx)| bitmap.get_real_index(*idx) == Some(expected as u8))
    }

    #[quickcheck]
    fn get_first_index_test(idx: u8) -> bool {
        let mut bitmap = BitmapIndex::new();
        bitmap.set_index(idx);
        bitmap.get_first_index() == Some(idx)
    }
}
