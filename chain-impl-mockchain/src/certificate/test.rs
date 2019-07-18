use super::*;
use crate::leadership::genesis::GenesisPraosLeader;
use chain_crypto::{Curve25519_2HashDH, PublicKey, SecretKey, SumEd25519_12};
use lazy_static::lazy_static;
use quickcheck::{Arbitrary, Gen};

impl Arbitrary for Certificate {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let content = match g.next_u32() % 3 {
            0 => CertificateContent::StakeDelegation(Arbitrary::arbitrary(g)),
            1 => CertificateContent::StakePoolRegistration(Arbitrary::arbitrary(g)),
            _ => CertificateContent::StakePoolRetirement(Arbitrary::arbitrary(g)),
        };
        Certificate { content }
    }
}
impl Arbitrary for StakeDelegation {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        StakeDelegation {
            stake_key_id: Arbitrary::arbitrary(g),
            pool_id: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for StakePoolInfo {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        use rand_core::SeedableRng;
        let mut seed = [0; 32];
        for byte in seed.iter_mut() {
            *byte = Arbitrary::arbitrary(g);
        }
        lazy_static! {
            static ref PK_KES: PublicKey<SumEd25519_12> = {
                let sk: SecretKey<SumEd25519_12> =
                    SecretKey::generate(&mut rand_chacha::ChaChaRng::from_seed([0; 32]));
                sk.to_public()
            };
        }
        let mut rng = rand_chacha::ChaChaRng::from_seed(seed);
        let vrf_sk: SecretKey<Curve25519_2HashDH> = SecretKey::generate(&mut rng);
        StakePoolInfo {
            serial: Arbitrary::arbitrary(g),
            owners: vec![Arbitrary::arbitrary(g)],
            initial_key: GenesisPraosLeader {
                vrf_public_key: vrf_sk.to_public(),
                kes_public_key: PK_KES.clone(),
            },
        }
    }
}

impl Arbitrary for StakePoolRetirement {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        StakePoolRetirement {
            pool_id: Arbitrary::arbitrary(g),
            pool_info: Arbitrary::arbitrary(g),
        }
    }
}
