use super::*;
use crate::accounting::account::DelegationType;
use crate::leadership::genesis::GenesisPraosLeader;
use crate::rewards::TaxType;
use chain_crypto::{testing, Ed25519};
use chain_core::mempack::{Readable, ReadBuf};
use chain_time::DurationSeconds;
use quickcheck::{Arbitrary, Gen, TestResult};
use quickcheck_macros::quickcheck;

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
        let mut signatoree = u8::arbitrary(g) % 32;
        if signatoree == 0 {
            signatoree = 1;
        }
        
        let mut signatures = Vec::new();
        for i in 0..signatoree {
            let s = Arbitrary::arbitrary(g);
            signatures.push((i, s));
        }
        PoolOwnersSigned { signatures }
    }
}

impl Arbitrary for PoolSignature {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        if bool::arbitrary(g) {
            PoolSignature::Operator(Arbitrary::arbitrary(g))
        } else {
            PoolSignature::Owners(Arbitrary::arbitrary(g))
        }
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

        let nb_owners = usize::arbitrary(g) % 32;
        let nb_operators = usize::arbitrary(g) % 4;

        let mut owners = Vec::new();
        for _ in 0..nb_owners {
            let pk = testing::arbitrary_public_key::<Ed25519, G>(g);
            owners.push(pk)
        }

        let mut operators = Vec::new();
        for _ in 0..nb_operators {
            let pk = testing::arbitrary_public_key::<Ed25519, G>(g);
            operators.push(pk)
        }

        PoolRegistration {
            serial: Arbitrary::arbitrary(g),
            permissions: PoolPermissions::new(1),
            start_validity: start_validity.into(),
            owners: owners,
            operators: operators.into(),
            rewards: TaxType::zero(),
            reward_account: None,
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

#[quickcheck]
fn pool_reg_serialization_bijection(b: PoolRegistration) -> TestResult {
    let b_got = b.serialize();
    let mut buf = ReadBuf::from(b_got.as_ref());
    let result = PoolRegistration::read(&mut buf);
    let left = Ok(b);
    assert_eq!(left, result);
    assert_eq!(buf.get_slice_end(), &[]);
    TestResult::from_bool(left == result)
}
