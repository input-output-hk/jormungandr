use super::*;
use crate::accounting::account::DelegationType;
use crate::leadership::genesis::GenesisPraosLeader;
use crate::rewards::TaxType;
use chain_crypto::{testing, Ed25519};
use chain_time::DurationSeconds;
use quickcheck::{Arbitrary, Gen};

impl Arbitrary for PoolRetirement {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let retirement_time = DurationSeconds::from(u64::arbitrary(g)).into();
        PoolRetirement {
            pool_id: Arbitrary::arbitrary(g),
            retirement_time,
        }
    }
}

impl Arbitrary for PoolUpdate {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let pool_id = Arbitrary::arbitrary(g);
        let start_validity = DurationSeconds::from(u64::arbitrary(g)).into();
        let prev = GenesisPraosLeader::arbitrary(g);
        let updated_keys = GenesisPraosLeader::arbitrary(g);
        let previous_keys = prev.digest();

        PoolUpdate {
            pool_id,
            start_validity,
            previous_keys,
            updated_keys,
        }
    }
}

impl Arbitrary for PoolOwnersSigned {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let signatoree = Arbitrary::arbitrary(g);
        let mut signatures = Vec::new();
        for i in 0..signatoree {
            let s = Arbitrary::arbitrary(g);
            signatures.push((i, s));
        }
        PoolOwnersSigned { signatures }
    }
}

impl Arbitrary for DelegationType {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        DelegationType::Full(Arbitrary::arbitrary(g))
    }
}

impl Arbitrary for StakeDelegation {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        StakeDelegation {
            account_id: Arbitrary::arbitrary(g),
            delegation: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for OwnerStakeDelegation {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Self {
            delegation: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for PoolRegistration {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let start_validity: DurationSeconds = u64::arbitrary(g).into();
        let keys = Arbitrary::arbitrary(g);

        let pk = testing::arbitrary_public_key::<Ed25519, G>(g);
        PoolRegistration {
            serial: Arbitrary::arbitrary(g),
            management_threshold: 1,
            start_validity: start_validity.into(),
            owners: vec![pk],
            rewards: TaxType::zero(),
            keys,
        }
    }
}

impl Arbitrary for Certificate {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let option = u8::arbitrary(g) % 5;
        match option {
            0 => Certificate::StakeDelegation(Arbitrary::arbitrary(g)),
            1 => Certificate::OwnerStakeDelegation(Arbitrary::arbitrary(g)),
            2 => Certificate::PoolRegistration(Arbitrary::arbitrary(g)),
            3 => Certificate::PoolRetirement(Arbitrary::arbitrary(g)),
            4 => Certificate::PoolUpdate(Arbitrary::arbitrary(g)),
            _ => panic!("unimplemented"),
        }
    }
}
