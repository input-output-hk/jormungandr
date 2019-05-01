// this is only a test module that use only recursive functions calls
// to compute the trees, and its use to compare to the more imperative fnuctions
// is sum. this module should never be used for anything but testing.
use super::common::{self, Depth, Seed};
use ed25519_dalek as ed25519;
use ed25519_dalek::Digest;

pub enum SecretKey {
    Leaf(ed25519::Keypair),
    Node(usize, Depth, Box<SecretKey>, Seed, PublicKey, PublicKey),
}

#[derive(Clone)]
pub enum PublicKey {
    Leaf(ed25519::PublicKey),
    Node([u8; 32]),
}

impl PublicKey {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            PublicKey::Leaf(p) => p.as_bytes(),
            PublicKey::Node(h) => h,
        }
    }
}

pub enum Signature {
    Leaf(ed25519::Signature),
    Node(usize, Depth, Box<Signature>, PublicKey, PublicKey),
}

pub fn hash(pk1: &PublicKey, pk2: &PublicKey) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut h = sha2::Sha256::default();
    h.input(pk1.as_bytes());
    h.input(pk2.as_bytes());

    let o = h.result();
    out.copy_from_slice(&o);
    out
}

pub fn keygen(log_depth: Depth, r: &Seed) -> (SecretKey, PublicKey) {
    //println!("keygen: log_depth: {:?}", log_depth);
    assert!(log_depth.0 < 32);
    if log_depth.0 == 0 {
        let sk = common::keygen_1(r);
        let pk = sk.public.clone();
        (SecretKey::Leaf(sk), PublicKey::Leaf(pk))
    } else {
        let (r0, r1) = common::split_seed(r);
        let (sk0, pk0) = keygen(log_depth.decr(), &r0);
        let (_, pk1) = keygen(log_depth.decr(), &r1);
        let pk = hash(&pk0, &pk1);
        (
            SecretKey::Node(0, log_depth, Box::new(sk0), r1, pk0, pk1),
            PublicKey::Node(pk),
        )
    }
}

#[allow(dead_code)]
pub fn sign(enum_sk: &SecretKey, m: &[u8]) -> Signature {
    match enum_sk {
        SecretKey::Leaf(s) => {
            //println!("sign with leaf");
            let sigma = s.sign(m);
            Signature::Leaf(sigma)
        }
        SecretKey::Node(t, depth, sk, _, pk0, pk1) => {
            //println!("sign node : t={:?} T0={:?}", *t, depth.half());
            let sigma = if t < &depth.half() {
                sign(sk, m)
            } else {
                sign(sk, m)
            };
            Signature::Node(*t, *depth, Box::new(sigma), pk0.clone(), pk1.clone())
        }
    }
}

#[allow(dead_code)]
pub fn verify(pk: &PublicKey, m: &[u8], sig: &Signature) -> bool {
    match sig {
        Signature::Leaf(s) => match pk {
            PublicKey::Leaf(pk) => pk.verify(m, s).is_ok(),
            PublicKey::Node(_) => panic!("xxxxxxxx"),
        },
        Signature::Node(tsig, depth, sigma, pk0, pk1) => match pk {
            PublicKey::Leaf(_) => panic!("verify on leaf !"),
            PublicKey::Node(pk) => {
                //println!("verify: tsig={:?} T0:{:?}", *tsig, depth.half());
                if &hash(pk0, pk1) != pk {
                    return false;
                };
                if *tsig < depth.half() {
                    verify(pk0, m, sigma)
                } else {
                    verify(pk1, m, sigma)
                }
            }
        },
    }
}

#[allow(dead_code)]
pub fn update(sk: &mut SecretKey) {
    match sk {
        SecretKey::Leaf(_) => panic!("who you gonna call ?!"),
        SecretKey::Node(ref mut t, depth, ref mut skbox, ref mut r1, _, _) => {
            //println!("update called: t={:?} T0={:?}", *t, depth.half());
            let t0 = depth.half();
            if *t + 1 < t0 {
                update(skbox)
            } else {
                if *t + 1 == t0 {
                    let (newsk, _) = keygen(depth.decr(), &r1);
                    *skbox = Box::new(newsk);
                    *r1 = Seed::zero()
                } else {
                    update(skbox)
                }
            }
            *t = *t + 1
        }
    }
}
