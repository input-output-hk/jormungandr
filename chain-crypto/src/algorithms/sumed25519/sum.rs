use super::common::{self, Depth, Seed};
use ed25519_dalek as ed25519;
use ed25519_dalek::Digest;
//use std::hash::Hash;

#[derive(Debug, Clone)]
pub enum Error {
    Ed25519SignatureError(ed25519::SignatureError),
    InvalidSecretKeySize(usize),
    InvalidPublicKeySize(usize),
    InvalidSignatureSize(usize),
    InvalidSignatureCount(usize, Depth),
    KeyCannotBeUpdatedMore,
}

impl From<ed25519::SignatureError> for Error {
    fn from(sig: ed25519::SignatureError) -> Error {
        Error::Ed25519SignatureError(sig)
    }
}

type PeriodSerialized = u32;
const PERIOD_SERIALIZE_SIZE: usize = 4;

const INDIVIDUAL_SECRET_SIZE: usize = 32; // ED25519 secret key size
const INDIVIDUAL_PUBLIC_SIZE: usize = 32; // ED25519 public key size
const SIGMA_SIZE: usize = 64; // ED25519 signature size

const PUBLIC_KEY_SIZE: usize = 32;

/// Secret Key in the binary tree sum composition of the ed25519 scheme
///
/// Serialization:
/// * period
/// * keypair : ED25519 keypair
/// * pks : depth size of left and right public keys
/// * rs : Stack of right seed for updates
#[derive(Clone)]
pub struct SecretKey {
    depth: Depth,
    data: Vec<u8>,
}

impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

// doesn't contains the seeds
pub const fn minimum_secretkey_size(depth: Depth) -> usize {
    PERIOD_SERIALIZE_SIZE
        + INDIVIDUAL_SECRET_SIZE + INDIVIDUAL_PUBLIC_SIZE // keypair
        + depth.0 * 2 * PUBLIC_KEY_SIZE
}

pub struct MerklePublicKeys<'a>(&'a [u8]);

impl<'a> MerklePublicKeys<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        assert_eq!(data.len() % (PUBLIC_KEY_SIZE * 2), 0);
        MerklePublicKeys(data)
    }
}

impl<'a> Iterator for MerklePublicKeys<'a> {
    type Item = (PublicKey, PublicKey);

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.len() == 0 {
            None
        } else {
            let mut datl = [0u8; PUBLIC_KEY_SIZE];
            let mut datr = [0u8; PUBLIC_KEY_SIZE];
            datl.copy_from_slice(&self.0[0..PUBLIC_KEY_SIZE]);
            datr.copy_from_slice(&self.0[PUBLIC_KEY_SIZE..PUBLIC_KEY_SIZE * 2]);
            *self = MerklePublicKeys::new(&self.0[PUBLIC_KEY_SIZE * 2..]);
            Some((PublicKey(datl), PublicKey(datr)))
        }
    }
}

impl<'a> DoubleEndedIterator for MerklePublicKeys<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.0.len() == 0 {
            None
        } else {
            let mut datl = [0u8; PUBLIC_KEY_SIZE];
            let mut datr = [0u8; PUBLIC_KEY_SIZE];
            let last_offset = self.0.len() - (PUBLIC_KEY_SIZE * 2);
            datl.copy_from_slice(&self.0[last_offset..last_offset + PUBLIC_KEY_SIZE]);
            datr.copy_from_slice(
                &self.0[last_offset + PUBLIC_KEY_SIZE..last_offset + PUBLIC_KEY_SIZE * 2],
            );
            *self = MerklePublicKeys::new(&self.0[0..last_offset]);
            Some((PublicKey(datl), PublicKey(datr)))
        }
    }
}

impl<'a> ExactSizeIterator for MerklePublicKeys<'a> {
    fn len(&self) -> usize {
        self.0.len() / (PUBLIC_KEY_SIZE * 2)
    }
}

pub struct Seeds<'a>(&'a [u8]);

impl<'a> Iterator for Seeds<'a> {
    type Item = Seed;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.len() == 0 {
            None
        } else {
            let seed = Seed::from_slice(&self.0[0..Seed::SIZE]);
            *self = Seeds(&self.0[Seed::SIZE..]);
            Some(seed)
        }
    }
}

impl SecretKey {
    const T_OFFSET: usize = 0;
    const KEYPAIR_OFFSET: usize = Self::T_OFFSET + PERIOD_SERIALIZE_SIZE;
    const MERKLE_PKS_OFFSET: usize =
        Self::KEYPAIR_OFFSET + INDIVIDUAL_SECRET_SIZE + INDIVIDUAL_PUBLIC_SIZE;
    const fn seed_offset(depth: Depth) -> usize {
        Self::MERKLE_PKS_OFFSET + depth.0 * PUBLIC_KEY_SIZE * 2
    }

    // --------------------------------------
    // accessors
    pub fn t(&self) -> usize {
        let mut t = [0u8; PERIOD_SERIALIZE_SIZE];
        t.copy_from_slice(&self.data[0..PERIOD_SERIALIZE_SIZE]);
        PeriodSerialized::from_le_bytes(t) as usize
    }

    pub fn sk(&self) -> ed25519::Keypair {
        let bytes = &self.data[Self::KEYPAIR_OFFSET..Self::MERKLE_PKS_OFFSET];
        ed25519::Keypair::from_bytes(&bytes).expect("internal error: keypair invalid")
    }

    fn merkle_pks(&self) -> MerklePublicKeys {
        let bytes = &self.data[Self::MERKLE_PKS_OFFSET..Self::seed_offset(self.depth)];
        MerklePublicKeys::new(bytes)
    }

    fn rs(&self) -> Seeds {
        let bytes = &self.data[Self::seed_offset(self.depth)..];
        Seeds(bytes)
    }

    fn set_t(&mut self, t: usize) {
        let t_bytes = PeriodSerialized::to_le_bytes(t as PeriodSerialized);
        let out = &mut self.data[0..PERIOD_SERIALIZE_SIZE];
        out.copy_from_slice(&t_bytes)
    }

    fn set_sk(&mut self, sk: &ed25519::Keypair) {
        let out = &mut self.data[Self::KEYPAIR_OFFSET..Self::MERKLE_PKS_OFFSET];
        out.copy_from_slice(&sk.to_bytes());
    }

    fn set_merkle_pks(&mut self, n: usize, pks: &(PublicKey, PublicKey)) {
        let bytes = &mut self.data[Self::MERKLE_PKS_OFFSET..Self::seed_offset(self.depth)];
        let startl = n * PUBLIC_KEY_SIZE * 2;
        let startr = startl + PUBLIC_KEY_SIZE;
        let end = startr + PUBLIC_KEY_SIZE;
        bytes[startl..startr].copy_from_slice(pks.0.as_ref());
        bytes[startr..end].copy_from_slice(pks.1.as_ref());
    }

    fn get_merkle_pks(&self, n: usize) -> (PublicKey, PublicKey) {
        let bytes = &self.data[Self::MERKLE_PKS_OFFSET..Self::seed_offset(self.depth)];
        let startl = n * PUBLIC_KEY_SIZE * 2;
        let startr = startl + PUBLIC_KEY_SIZE;
        let end = startr + PUBLIC_KEY_SIZE;

        let mut datl = [0u8; PUBLIC_KEY_SIZE];
        let mut datr = [0u8; PUBLIC_KEY_SIZE];
        datl.copy_from_slice(&bytes[startl..startr]);
        datr.copy_from_slice(&bytes[startr..end]);
        (PublicKey(datl), PublicKey(datr))
    }

    pub fn compute_public(&self) -> PublicKey {
        let t = self.t();
        let mut got = PublicKey::from_ed25519_publickey(&self.sk().public);
        for (i, (pk_left, pk_right)) in self.merkle_pks().rev().enumerate() {
            let right = (t & (1 << i)) != 0;
            if right {
                got = hash(&pk_left, &got);
            } else {
                got = hash(&got, &pk_right);
            }
        }
        got
    }

    // --------------------------------------

    fn create(
        t: usize,
        keypair: ed25519::Keypair,
        pks: &[(PublicKey, PublicKey)],
        rs: &[Seed],
    ) -> Self {
        let depth = Depth(pks.len());
        let mut out = Vec::with_capacity(minimum_secretkey_size(depth) + rs.len() * Seed::SIZE);

        let t_bytes = PeriodSerialized::to_le_bytes(t as PeriodSerialized);
        out.extend_from_slice(&t_bytes);
        assert_eq!(out.len(), 4);
        out.extend_from_slice(&keypair.to_bytes());
        assert_eq!(out.len(), 68);
        for (pkl, pkr) in pks {
            out.extend_from_slice(&pkl.0);
            out.extend_from_slice(&pkr.0);
        }
        assert_eq!(out.len(), 68 + pks.len() * 64);
        for r in rs {
            out.extend_from_slice(&r.as_ref());
        }

        SecretKey {
            depth: depth,
            data: out,
        }
    }

    // Get the latest seed and drop it from the buffer
    pub fn rs_pop(&mut self) -> Option<Seed> {
        let seed_offset = Self::seed_offset(self.depth);
        if self.data.len() - seed_offset > 0 {
            // grab the last seed
            let last = self.data.len() - Seed::SIZE;
            let seed = Seed::from_slice(&self.data[last..]);
            // clear the seed memory in the secret key, then truncate
            self.data[last..].copy_from_slice(&[0u8; Seed::SIZE]);
            self.data.truncate(last);
            Some(seed)
        } else {
            None
        }
    }

    pub fn rs_extend<I>(&mut self, rs: I)
    where
        I: Iterator<Item = Seed>,
    {
        for r in rs {
            self.data.extend_from_slice(r.as_ref())
        }
    }

    pub fn depth(&self) -> Depth {
        self.depth
    }
    pub fn is_updatable(&self) -> bool {
        self.t() + 1 < self.depth.total()
    }

    pub fn from_bytes(depth: Depth, bytes: &[u8]) -> Result<Self, Error> {
        let minimum_size = Self::seed_offset(depth);
        // we need at least N bytes, anything under and it's invalid
        if bytes.len() < minimum_size {
            return Err(Error::InvalidSecretKeySize(bytes.len()));
        }

        // check if the remaining length is valid
        let rem = (bytes.len() - minimum_size) % 32;
        if rem > 0 {
            return Err(Error::InvalidSignatureSize(bytes.len()));
        }

        // get T and make sure it's under the total
        let mut t_bytes = [0u8; PERIOD_SERIALIZE_SIZE];
        t_bytes.copy_from_slice(&bytes[0..PERIOD_SERIALIZE_SIZE]);
        let t = PeriodSerialized::from_le_bytes(t_bytes) as usize;
        if t >= depth.total() {
            return Err(Error::InvalidSignatureCount(t, depth));
        }

        let keypair_slice = &bytes[Self::KEYPAIR_OFFSET..Self::MERKLE_PKS_OFFSET];

        // verify sigma and pk format, no need to verify pks nor rs
        let _ = ed25519::Keypair::from_bytes(keypair_slice)?;

        let mut out = Vec::with_capacity(bytes.len());
        out.extend_from_slice(bytes);

        Ok(SecretKey {
            depth: depth,
            data: out,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PublicKey([u8; PUBLIC_KEY_SIZE]);

impl PublicKey {
    pub fn from_ed25519_publickey(public: &ed25519::PublicKey) -> Self {
        let mut out = [0u8; PUBLIC_KEY_SIZE];
        out.copy_from_slice(public.as_bytes());
        PublicKey(out)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() == PUBLIC_KEY_SIZE {
            let mut v = [0u8; PUBLIC_KEY_SIZE];
            v.copy_from_slice(bytes);
            Ok(PublicKey(v))
        } else {
            Err(Error::InvalidPublicKeySize(bytes.len()))
        }
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Signature using the repetitive MMM sum composition
///
/// Serialization:
/// * period
/// * sigma : ED25519 individual signature linked to period
/// * ED25519 public key of this period
/// * public keys : merkle tree path elements
#[derive(Debug, Clone)]
pub struct Signature(Vec<u8>);

pub struct MerkleSignaturePublicKeys<'a>(&'a [u8]);

impl<'a> Iterator for MerkleSignaturePublicKeys<'a> {
    type Item = PublicKey;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.len() == 0 {
            None
        } else {
            let mut dat = [0u8; PUBLIC_KEY_SIZE];
            dat.copy_from_slice(&self.0[0..PUBLIC_KEY_SIZE]);
            *self = MerkleSignaturePublicKeys(&self.0[PUBLIC_KEY_SIZE..]);
            Some(PublicKey(dat))
        }
    }
}

impl<'a> DoubleEndedIterator for MerkleSignaturePublicKeys<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.0.len() == 0 {
            None
        } else {
            let mut dat = [0u8; PUBLIC_KEY_SIZE];
            let last_offset = self.0.len() - PUBLIC_KEY_SIZE;
            dat.copy_from_slice(&self.0[last_offset..]);
            *self = MerkleSignaturePublicKeys(&self.0[0..last_offset]);
            Some(PublicKey(dat))
        }
    }
}

impl<'a> ExactSizeIterator for MerkleSignaturePublicKeys<'a> {
    fn len(&self) -> usize {
        self.0.len() / PUBLIC_KEY_SIZE
    }
}

pub const fn signature_size(depth: Depth) -> usize {
    PERIOD_SERIALIZE_SIZE + SIGMA_SIZE + INDIVIDUAL_PUBLIC_SIZE + depth.0 * PUBLIC_KEY_SIZE
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Signature {
    const T_OFFSET: usize = 0;
    const SIGMA_OFFSET: usize = Self::T_OFFSET + PERIOD_SERIALIZE_SIZE;
    const PK_OFFSET: usize = Self::SIGMA_OFFSET + SIGMA_SIZE;
    const MERKLE_PKS_OFFSET: usize = Self::PK_OFFSET + INDIVIDUAL_PUBLIC_SIZE;

    pub fn depth(&self) -> Depth {
        Depth(self.merkle_pks().len())
    }

    /// Compute the size in bytes of a signature
    /// currently this is : 100 + 32*depth()
    pub fn size_bytes(&self) -> usize {
        self.0.len()
    }

    // --------------------------------------
    // Getter Accessors -- expect valid data
    pub fn t(&self) -> usize {
        let mut t = [0u8; PERIOD_SERIALIZE_SIZE];
        t.copy_from_slice(&self.0[0..PERIOD_SERIALIZE_SIZE]);
        PeriodSerialized::from_le_bytes(t) as usize
    }

    fn sigma(&self) -> ed25519::Signature {
        let bytes = &self.0[Self::SIGMA_OFFSET..Self::PK_OFFSET];
        ed25519::Signature::from_bytes(bytes).expect("internal error: signature invalid")
    }

    fn pk(&self) -> ed25519::PublicKey {
        let bytes = &self.0[Self::PK_OFFSET..Self::MERKLE_PKS_OFFSET];
        ed25519::PublicKey::from_bytes(bytes).expect("internal error: pk invalid")
    }

    fn merkle_pks(&self) -> MerkleSignaturePublicKeys {
        let bytes = &self.0[Self::MERKLE_PKS_OFFSET..];
        MerkleSignaturePublicKeys(bytes)
    }

    // --------------------------------------

    fn create(
        t: usize,
        sigma: ed25519::Signature,
        pk: &ed25519::PublicKey,
        pks: &[PublicKey],
    ) -> Self {
        let mut out = Vec::with_capacity(96 + PERIOD_SERIALIZE_SIZE + PUBLIC_KEY_SIZE * pks.len());
        let t_bytes = PeriodSerialized::to_le_bytes(t as PeriodSerialized);
        out.extend_from_slice(&t_bytes);
        assert_eq!(out.len(), 4);
        out.extend_from_slice(&sigma.to_bytes());
        assert_eq!(out.len(), 68);
        out.extend_from_slice(pk.as_bytes());
        assert_eq!(out.len(), 100);
        for p in pks {
            out.extend_from_slice(&p.0);
        }
        Signature(out)
    }

    /// Get the bytes representation of the signature
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn from_bytes(depth: Depth, bytes: &[u8]) -> Result<Self, Error> {
        let minimum_size = 96 + PERIOD_SERIALIZE_SIZE;
        // we need at least N bytes, anything under and it's invalid
        if bytes.len() < minimum_size {
            return Err(Error::InvalidSignatureSize(bytes.len()));
        }

        // check if the length is valid, and get the depth by the number of public key
        let rem = (bytes.len() - minimum_size) % 32;
        if rem > 0 {
            return Err(Error::InvalidSignatureSize(bytes.len()));
        }
        let found_depth = (bytes.len() - minimum_size) / 32;
        if found_depth == depth.0 {
            return Err(Error::InvalidSignatureSize(bytes.len()));
        }

        // get T and make sure it's under the total
        let mut t_bytes = [0u8; PERIOD_SERIALIZE_SIZE];
        t_bytes.copy_from_slice(&bytes[0..PERIOD_SERIALIZE_SIZE]);
        let t = PeriodSerialized::from_le_bytes(t_bytes) as usize;
        if t >= depth.total() {
            return Err(Error::InvalidSignatureCount(t, depth));
        }

        let sigma_slice = &bytes[Self::SIGMA_OFFSET..Self::SIGMA_OFFSET + SIGMA_SIZE];
        let pk_slice = &bytes[Self::PK_OFFSET..Self::PK_OFFSET + INDIVIDUAL_PUBLIC_SIZE];

        // verify sigma and pk format, no need to verify pks
        let _ = ed25519::PublicKey::from_bytes(pk_slice)?;
        let _ = ed25519::Signature::from_bytes(sigma_slice)?;

        let mut out = Vec::with_capacity(bytes.len());
        out.extend_from_slice(bytes);

        Ok(Signature(out))
    }
}

pub fn hash(pk1: &PublicKey, pk2: &PublicKey) -> PublicKey {
    let mut out = [0u8; 32];
    let mut h = sha2::Sha256::default();
    h.input(&pk1.0);
    h.input(&pk2.0);

    let o = h.result();
    out.copy_from_slice(&o);
    PublicKey(out)
}

// Generate the leftmost parts of generators pushing the right branch
// and return the leftmost given key pair
fn generate_leftmost_rs(rs: &mut Vec<Seed>, log_depth: Depth, master: &Seed) -> ed25519::Keypair {
    let mut depth = log_depth;
    let mut r = master.clone();
    loop {
        let (r0, r1) = common::split_seed(&r);
        rs.push(r1);
        if depth.0 == 1 {
            return common::keygen_1(&r0);
        } else {
            r = r0;
        }
        depth = depth.decr();
    }
}

/// Generate the public key from a specific level and a given seed
///
/// the following assumption hold:
///     pkeygen(depth, master) == keygen(depth, master).1
///
/// This is faster than using keygen directly
pub fn pkeygen(log_depth: Depth, master: &Seed) -> PublicKey {
    if log_depth.0 == 0 {
        let pk = common::keygen_1(master).public;
        return PublicKey::from_ed25519_publickey(&pk);
    }
    // first r1 is the topmost
    let mut rs = Vec::new();

    // generate the leftmost sk, pk, and accumulate all r1
    let keypair0 = generate_leftmost_rs(&mut rs, log_depth, master);
    let pk0 = keypair0.public;

    let mut depth = Depth(0);
    let mut pk_left = PublicKey::from_ed25519_publickey(&pk0);
    // append to storage from leaf to root
    for r in rs.iter().rev() {
        let pk_right = if depth.0 == 0 {
            PublicKey::from_ed25519_publickey(&common::keygen_1(r).public)
        } else {
            pkeygen(depth, r)
        };
        depth = depth.incr();
        pk_left = hash(&pk_left, &pk_right);
    }
    pk_left
}

/// Generate a keypair using the seed as master seed for the tree of depth log_depth
///
/// After creation the secret key is updatable 2^log_depth, and contains
/// the 0th version of the secret key.
///
/// log_depth=3 => 8 signing keys
///
pub fn keygen(log_depth: Depth, master: &Seed) -> (SecretKey, PublicKey) {
    if log_depth.0 == 0 {
        let keypair = common::keygen_1(master);
        let pk = PublicKey::from_ed25519_publickey(&keypair.public);
        return (SecretKey::create(0, keypair, &[], &[]), pk);
    }

    // first r1 is the topmost
    let mut rs = Vec::new();

    // generate the leftmost sk, pk, and accumulate all r1
    let keypair0 = generate_leftmost_rs(&mut rs, log_depth, master);
    let mut pk_left = PublicKey::from_ed25519_publickey(&keypair0.public);
    let sk0 = keypair0;

    let mut depth = Depth(0);
    let mut pks = Vec::new();
    // append to storage from leaf to root
    for r in rs.iter().rev() {
        let pk_right = if depth.0 == 0 {
            PublicKey::from_ed25519_publickey(&common::keygen_1(r).public)
        } else {
            pkeygen(depth, r)
        };
        pks.push((pk_left.clone(), pk_right.clone()));
        depth = depth.incr();
        pk_left = hash(&pk_left, &pk_right);
    }
    // then store pk{left,right} from root to leaf
    pks.reverse();
    assert_eq!(log_depth.0, pks.len());

    (SecretKey::create(0, sk0, &pks, &rs), pk_left)
}

pub fn sign(secret: &SecretKey, m: &[u8]) -> Signature {
    let sk = secret.sk();
    let sigma = sk.sign(m);
    let mut pks = Vec::new();
    let mut t = secret.t();

    for (i, (pk0, pk1)) in secret.merkle_pks().enumerate() {
        let d = Depth(secret.depth().0 - i);
        if t >= d.half() {
            t = t - d.half();
            pks.push(pk0.clone());
        } else {
            pks.push(pk1.clone());
        }
    }

    // disabled extra debug check that we can reconstruct from the pks the public key
    if false {
        let scheme_pk = secret.compute_public();
        let mut got = PublicKey::from_ed25519_publickey(&sk.public);
        for (i, p) in pks.iter().rev().enumerate() {
            let right = (secret.t() & (1 << i)) != 0;
            if right {
                got = hash(&p, &got);
            } else {
                got = hash(&got, &p);
            }
        }
        assert_eq!(scheme_pk, got);
    }

    Signature::create(secret.t(), sigma, &sk.public, &pks)
}

pub fn verify(pk: &PublicKey, m: &[u8], sig: &Signature) -> bool {
    // verify the signature of the leaf
    if !sig.pk().verify(m, &sig.sigma()).is_ok() {
        return false;
    }

    let t = sig.t();

    // verify that we have the expected root public key afterall
    let mut got = PublicKey::from_ed25519_publickey(&sig.pk());
    for (i, pk_combi) in sig.merkle_pks().rev().enumerate() {
        let right = (t & (1 << i)) != 0;
        if right {
            got = hash(&pk_combi, &got);
        } else {
            got = hash(&got, &pk_combi);
        }
    }
    if &got != pk {
        return false;
    }
    true
}

pub fn update(secret: &mut SecretKey) -> Result<(), Error> {
    //assert!(secret.t() < secret.depth().total());
    let diff = usize::count_ones(secret.t() ^ (secret.t() + 1));
    assert!(diff >= 1);

    match secret.rs_pop() {
        None => Err(Error::KeyCannotBeUpdatedMore),
        Some(seed) => {
            if diff == 1 {
                let keypair = common::keygen_1(&seed);
                secret.set_sk(&keypair);
                secret.set_t(secret.t() + 1);
            } else {
                let (sec_child, pub_child) = keygen(Depth((diff - 1) as usize), &seed);
                assert_eq!(
                    secret.get_merkle_pks(secret.depth().0 - diff as usize).1,
                    pub_child
                );

                secret.rs_extend(sec_child.rs());
                let offset = secret.merkle_pks().len() - sec_child.merkle_pks().len();
                for (i, c) in sec_child.merkle_pks().enumerate() {
                    secret.set_merkle_pks(offset + i, &c)
                }
                secret.set_sk(&sec_child.sk());
                secret.set_t(secret.t() + 1);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::sumrec;
    use quickcheck::{Arbitrary, Gen};
    impl Arbitrary for Seed {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut b = [0u8; 32];
            for v in b.iter_mut() {
                *v = Arbitrary::arbitrary(g)
            }
            Seed::from_bytes(b)
        }
    }
    impl Arbitrary for Depth {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Depth(usize::arbitrary(g) % 8)
        }
    }

    pub fn exhaustive_signing(depth: Depth) {
        let s = Seed::zero();
        let (mut sk, pk) = keygen(depth, &s);
        let m = [1, 2, 3];

        let pk_public = pkeygen(depth, &s);
        assert_eq!(pk, pk_public);

        for i in 0..depth.total() {
            let sig = sign(&sk, &m);
            let v = verify(&pk, &m, &sig);
            assert_eq!(v, true, "key {} failed verification", i);
            if sk.is_updatable() {
                update(&mut sk).unwrap();
            }
        }
    }

    fn secretkey_identical(sk: &[u8], expected: &[u8]) {
        assert_eq!(sk, expected)
    }

    #[test]
    pub fn d1_testvect() {
        let s = Seed::zero();
        let (mut sk, pk) = keygen(Depth(1), &s);

        secretkey_identical(
            &sk.sk().to_bytes(),
            &[
                26, 125, 253, 234, 255, 238, 218, 196, 137, 40, 126, 133, 190, 94, 156, 4, 154, 47,
                246, 71, 15, 85, 207, 48, 38, 15, 85, 57, 90, 193, 177, 89, 78, 235, 10, 159, 73,
                188, 163, 72, 115, 121, 126, 90, 61, 168, 98, 180, 65, 232, 227, 153, 30, 37, 185,
                126, 176, 154, 229, 246, 71, 227, 121, 87,
            ],
        );
        update(&mut sk).is_ok();
        secretkey_identical(
            &sk.sk().to_bytes(),
            &[
                82, 59, 165, 167, 236, 147, 98, 219, 176, 128, 57, 163, 135, 146, 37, 146, 204,
                234, 61, 222, 99, 99, 68, 128, 205, 27, 5, 183, 189, 80, 162, 105, 218, 6, 10, 158,
                150, 121, 109, 154, 129, 208, 227, 82, 89, 185, 132, 57, 60, 25, 22, 161, 74, 85,
                58, 137, 78, 81, 131, 138, 253, 43, 125, 198,
            ],
        );

        assert_eq!(
            pk.as_ref(),
            &[
                190, 13, 111, 45, 153, 107, 149, 75, 135, 90, 183, 153, 136, 191, 230, 90, 37, 84,
                209, 51, 112, 139, 61, 199, 190, 165, 166, 68, 79, 124, 221, 165
            ]
        );
    }

    #[test]
    pub fn check_public_is_recomputable() {
        let (mut sk, pk) = keygen(Depth(4), &Seed::zero());

        assert_eq!(sk.compute_public(), pk);
        update(&mut sk).is_ok();
        assert_eq!(sk.compute_public(), pk);
        update(&mut sk).is_ok();
        assert_eq!(sk.compute_public(), pk);
    }

    #[test]
    pub fn working_depth1() {
        exhaustive_signing(Depth(1));
    }

    #[test]
    pub fn working_depth2_8() {
        for i in 2..8 {
            exhaustive_signing(Depth(i));
        }
    }

    #[quickcheck]
    fn check_public(depth: Depth, seed: Seed) -> bool {
        let (_, pk) = keygen(depth, &seed);
        let pk_pub = pkeygen(depth, &seed);
        pk == pk_pub
    }

    #[quickcheck]
    fn check_sig(depth: Depth, seed: Seed) -> bool {
        let (sk, pk) = keygen(depth, &seed);

        let m = b"Arbitrary message";

        let sig = sign(&sk, m);
        let v = verify(&pk, m, &sig);
        v
    }

    #[quickcheck]
    fn check_recver_equivalent(depth: Depth, seed: Seed) -> bool {
        let (_, pk) = keygen(depth, &seed);

        let (_, pkrec) = sumrec::keygen(depth, &seed);
        pk.as_bytes() == pkrec.as_bytes()
    }
}

#[cfg(test)]
#[cfg(feature = "with-bench")]
mod bench {
    use super::*;

    fn keygen_with_depth(depth: Depth, b: &mut test::Bencher) {
        let seed = Seed::zero();
        b.iter(|| {
            let _ = keygen(depth, &seed);
        })
    }

    fn update_with_depth(depth: Depth, nb_update: usize, b: &mut test::Bencher) {
        let seed = Seed::zero();
        let (sk_orig, _) = keygen(depth, &seed);
        b.iter(|| {
            let mut sk = sk_orig.clone();
            for _ in 0..(nb_update - 1) {
                update(&mut sk).unwrap()
            }
        })
    }

    fn update_with_depth_skip(depth: Depth, nb_update_to_skip: usize, b: &mut test::Bencher) {
        let seed = Seed::zero();
        let (mut sk_orig, _) = keygen(depth, &seed);
        for _ in 0..(nb_update_to_skip - 1) {
            update(&mut sk_orig).unwrap()
        }
        b.iter(|| {
            let mut sk = sk_orig.clone();
            update(&mut sk).unwrap()
        })
    }

    /*
        #[bench]
        fn keygen_depth1(b: &mut test::Bencher) {
            keygen_with_depth(Depth(1), b)
        }
        #[bench]
        fn keygen_depth2(b: &mut test::Bencher) {
            keygen_with_depth(Depth(2), b)
        }
        #[bench]
        fn keygen_depth3(b: &mut test::Bencher) {
            keygen_with_depth(Depth(3), b)
        }
        #[bench]
        fn keygen_depth4(b: &mut test::Bencher) {
            keygen_with_depth(Depth(4), b)
        }
        #[bench]
        fn keygen_depth8(b: &mut test::Bencher) {
            keygen_with_depth(Depth(8), b)
        }
        #[bench]
        fn keygen_depth9(b: &mut test::Bencher) {
            keygen_with_depth(Depth(9), b)
        }

    */

    #[bench]
    fn keygen_depth25(b: &mut test::Bencher) {
        keygen_with_depth(Depth(14), b)
    }

    #[bench]
    fn update2_depth2(b: &mut test::Bencher) {
        update_with_depth(Depth(2), 2, b)
    }
    #[bench]
    fn update4_depth4(b: &mut test::Bencher) {
        update_with_depth(Depth(4), 4, b)
    }

    #[bench]
    fn update16_depth8(b: &mut test::Bencher) {
        update_with_depth(Depth(8), 16, b)
    }
    #[bench]
    fn update128_depth16(b: &mut test::Bencher) {
        update_with_depth_skip(Depth(16), 2 ^ 16 - 1, b)
    }

}
