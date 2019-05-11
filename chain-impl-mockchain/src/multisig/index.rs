use std::cmp::Ordering;

pub const LEVEL_MAXLIMIT: usize = 8;

/// The Index is really just 3 bits and has a hardbound linked to the LEVEL_MAXLIMIT
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Index(u8);

impl Index {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v as usize > LEVEL_MAXLIMIT {
            None
        } else {
            Some(Index(v))
        }
    }

    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeIndex {
    D1(Index),
    D2(Index, Index),
}

impl PartialOrd for TreeIndex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TreeIndex {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (TreeIndex::D1(a), TreeIndex::D1(b)) => a.cmp(b),
            (TreeIndex::D1(a), TreeIndex::D2(b1, _)) => a.cmp(b1).then(Ordering::Less),
            (TreeIndex::D2(a1, _), TreeIndex::D1(b)) => a1.cmp(b).then(Ordering::Greater),
            (TreeIndex::D2(a1, a2), TreeIndex::D2(b1, b2)) => a1.cmp(b1).then(a2.cmp(b2)),
        }
    }
}

const TREEINDEX_TAG_DEPTH1: u16 = 1;
const TREEINDEX_TAG_DEPTH2: u16 = 2;

impl TreeIndex {
    pub fn indices(&self) -> Vec<Index> {
        match self {
            TreeIndex::D1(a) => vec![*a],
            TreeIndex::D2(a, b) => vec![*a, *b],
        }
    }
    pub fn depth(&self) -> usize {
        match self {
            TreeIndex::D1(_) => 0,
            TreeIndex::D2(_, _) => 1,
        }
    }

    pub fn pack(&self) -> u16 {
        match self {
            TreeIndex::D1(Index(a)) => (TREEINDEX_TAG_DEPTH1 << 12) + ((*a as u16) << 9),
            TreeIndex::D2(Index(a), Index(b)) => {
                (TREEINDEX_TAG_DEPTH2 << 12) + ((*a as u16) << 9) + ((*b as u16) << 6)
            }
        }
    }

    pub fn unpack(v: u16) -> Option<Self> {
        let tag = (v >> 12) & 0b1111;
        let a = (v >> 9) & 0b111;
        let b = (v >> 6) & 0b111;
        let c = (v >> 3) & 0b111;
        let d = v & 0b111;

        if c != 0 || d != 0 {
            return None;
        }

        match tag {
            TREEINDEX_TAG_DEPTH1 => {
                if b != 0 {
                    return None;
                }
                Some(TreeIndex::D1(Index(a as u8)))
            }
            TREEINDEX_TAG_DEPTH2 => Some(TreeIndex::D2(Index(a as u8), Index(b as u8))),
            _ => None,
        }
    }
}
