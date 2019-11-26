use crate::{
    fragment::Fragment,
    header::HeaderId,
    testing::{ledger::TestLedger, data::Wallet, builders::make_witness},
    transaction::{TxBuilder, Payload, SetIOs, TxBuilderState,SetAuthData,SingleAccountBindingSignature, AccountBindingSignature},
    value::Value,
    certificate::{Certificate,PoolSignature, PoolOwnersSigned},
    key::EitherEd25519SecretKey
};

pub struct TestTxCertBuilder<'a> {
    test_ledger: &'a TestLedger
}

impl<'a> TestTxCertBuilder<'a> {
    pub fn new(test_ledger: &'a TestLedger) -> Self {
        Self { test_ledger }
    }

    fn block0_hash(&self) -> &HeaderId {
        &self.test_ledger.block0_hash
    }

    fn fee(&self) -> Value {
        let linear_fee =  self.test_ledger.fee();
        Value(linear_fee.certificate + linear_fee.constant + linear_fee.coefficient)
    }

    fn set_initial_ios<P: Payload>(&self, builder: TxBuilderState<SetIOs<P>>, funder: &Wallet) -> TxBuilderState<SetAuthData<P>> {
        //utxo not supported yet
        let input = funder.make_input_with_value(&self.fee());
        let builder = builder.set_ios(&[input], &[]);
        let witness = make_witness(self.block0_hash(),&funder.as_account_data(),&builder.get_auth_data_for_witness().hash());
        builder.set_witnesses(&[witness])
    }

    fn fragment(&self, cert: &Certificate, keys: Vec<EitherEd25519SecretKey>, funder: &Wallet) -> Fragment {
        match cert {
                Certificate::StakeDelegation(s) => {
                    let builder = self.set_initial_ios(TxBuilder::new().set_payload(s),&funder);
                    let signature = AccountBindingSignature::new_single(&keys[0], &builder.get_auth_data());
                    let tx = builder.set_payload_auth(&signature);
                    Fragment::StakeDelegation(tx)
                }
                Certificate::PoolRegistration(s) => {
                    let builder = self.set_initial_ios(TxBuilder::new().set_payload(s),&funder);
                    let signature = pool_owner_sign(&keys, &builder);
                    let tx = builder.set_payload_auth(&signature);
                    Fragment::PoolRegistration(tx)
                }
                Certificate::PoolRetirement(s) => {
                    let builder = self.set_initial_ios(TxBuilder::new().set_payload(s),&funder);
                    let signature = pool_owner_sign(&keys, &builder);
                    let tx = builder.set_payload_auth(&signature);
                    Fragment::PoolRetirement(tx)
                }
                Certificate::PoolUpdate(s) => {
                    let builder = self.set_initial_ios(TxBuilder::new().set_payload(s),&funder);
                    let signature = pool_owner_sign(&keys, &builder);
                    let tx = builder.set_payload_auth(&signature);
                    Fragment::PoolUpdate(tx)
                }
                Certificate::OwnerStakeDelegation(_) => unreachable!(),
            }
    }

    pub fn make_transaction(self, signers: &[&Wallet], certificate: &Certificate) -> Fragment {
        let keys = signers.iter().cloned().map(|owner| owner.private_key()).collect();
        self.fragment(certificate,keys,&signers[0])
    }
}

pub fn pool_owner_sign<P: Payload>(
        keys: &[EitherEd25519SecretKey],
        builder: &TxBuilderState<SetAuthData<P>>
    ) -> PoolSignature {
        let auth_data = builder.get_auth_data();
        let mut sigs = Vec::new();
        for (i, key) in keys.iter().enumerate() {
            let sig = SingleAccountBindingSignature::new(key, &auth_data);
            sigs.push((i as u8, sig))
        }
        let pool_owner = PoolOwnersSigned { signatures: sigs };
        PoolSignature::Owners(pool_owner)
    }