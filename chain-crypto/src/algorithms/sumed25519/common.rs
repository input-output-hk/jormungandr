use ed25519_dalek as ed25519;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hash([u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seed([u8; 32]);

impl AsRef<[u8]> for Seed {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Seed {
    pub const SIZE: usize = 32;

    pub fn zero() -> Seed {
        Seed([0u8; Self::SIZE])
    }

    pub fn set_zero(&mut self) {
        self.0.copy_from_slice(&[0u8; Self::SIZE])
    }

    pub fn from_bytes(b: [u8; Self::SIZE]) -> Seed {
        Seed(b)
    }

    pub fn from_slice(b: &[u8]) -> Seed {
        assert_eq!(b.len(), Self::SIZE);
        let mut out = [0u8; Self::SIZE];
        out.copy_from_slice(b);
        Seed(out)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Depth(pub usize);

impl Depth {
    pub fn total(&self) -> usize {
        usize::pow(2, self.0 as u32)
    }

    pub fn half(&self) -> usize {
        assert!(self.0 > 0);
        usize::pow(2, (self.0 - 1) as u32)
    }

    pub fn decr(&self) -> Self {
        assert!(self.0 > 0);
        Depth(self.0 - 1)
    }
    pub fn incr(&self) -> Self {
        Depth(self.0 + 1)
    }
}

pub fn split_seed(r: &Seed) -> (Seed, Seed) {
    use ed25519_dalek::Digest;
    let mut hleft = sha2::Sha256::default();
    let mut hright = sha2::Sha256::default();

    hleft.input(&[1]);
    hleft.input(&r.0);

    hright.input(&[2]);
    hright.input(&r.0);

    let o1 = hleft.result();
    let o2 = hright.result();
    let s1 = Seed::from_slice(&o1);
    let s2 = Seed::from_slice(&o2);
    (s1, s2)
}

pub fn keygen_1(r: &Seed) -> ed25519::Keypair {
    let sk = ed25519::SecretKey::from_bytes(&r.0).unwrap();
    let pk = (&sk).into();
    ed25519::Keypair {
        secret: sk,
        public: pk,
    }
}
