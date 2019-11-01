use chain_addr::Address;
use chain_impl_mockchain::{
    self as chain,
    certificate::{Certificate, SignedCertificate},
    fee::FeeAlgorithm,
    fragment::Fragment,
    transaction::{
        self, InputOutput, InputOutputBuilder, Output, Payload, PayloadSlice, SetAuthData, SetIOs,
        Transaction, TransactionSignDataHash, TxBuilder, TxBuilderState,
    },
};
use jcli_app::transaction::Error;
use jcli_app::certificate::{pool_owner_sign, stake_delegation_account_binding_sign};
use jcli_app::utils::error::CustomErrorFiller;
use jcli_app::utils::io;
use jormungandr_lib::interfaces;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub enum StagingKind {
    /// Settings inputs and outputs
    Balancing,
    /// Settings witnesses
    Finalizing,
    Sealed,
    Authed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Staging {
    kind: StagingKind,
    inputs: Vec<interfaces::TransactionInput>,
    outputs: Vec<interfaces::TransactionOutput>,
    witnesses: Vec<interfaces::TransactionWitness>,
    extra: Option<interfaces::Certificate>,
    extra_authed: Option<interfaces::SignedCertificate>,
}

impl std::fmt::Display for StagingKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StagingKind::Balancing => write!(f, "balancing"),
            StagingKind::Finalizing => write!(f, "finalizing"),
            StagingKind::Sealed => write!(f, "sealed"),
            StagingKind::Authed => write!(f, "authed"),
        }
    }
}

impl Staging {
    pub fn new() -> Self {
        Staging {
            kind: StagingKind::Balancing,
            inputs: Vec::new(),
            outputs: Vec::new(),
            witnesses: Vec::new(),
            extra: None,
            extra_authed: None,
        }
    }

    pub fn load<P: AsRef<Path>>(path: &Option<P>) -> Result<Self, Error> {
        let file = io::open_file_read(path).map_err(|source| Error::StagingFileOpenFailed {
            source,
            path: io::path_to_path_buf(path),
        })?;
        bincode::deserialize_from(file).map_err(|source| Error::StagingFileReadFailed {
            source: *source,
            path: io::path_to_path_buf(path),
        })
    }

    pub fn store<P: AsRef<Path>>(&self, path: &Option<P>) -> Result<(), Error> {
        let file = io::open_file_write(path).map_err(|source| Error::StagingFileOpenFailed {
            source,
            path: io::path_to_path_buf(path),
        })?;
        bincode::serialize_into(file, self).map_err(|source| Error::StagingFileWriteFailed {
            source: *source,
            path: io::path_to_path_buf(path),
        })
    }

    pub fn add_input(&mut self, input: interfaces::TransactionInput) -> Result<(), Error> {
        if self.kind != StagingKind::Balancing {
            return Err(Error::TxKindToAddInputInvalid { kind: self.kind });
        }

        Ok(self.inputs.push(input))
    }

    pub fn add_output(&mut self, output: Output<Address>) -> Result<(), Error> {
        if self.kind != StagingKind::Balancing {
            return Err(Error::TxKindToAddOutputInvalid { kind: self.kind });
        }

        Ok(self.outputs.push(output.into()))
    }

    pub fn add_witness(&mut self, witness: chain::transaction::Witness) -> Result<(), Error> {
        if self.kind != StagingKind::Finalizing {
            return Err(Error::TxKindToAddWitnessInvalid { kind: self.kind });
        }

        if self.inputs.len() <= self.witnesses.len() {
            return Err(Error::TooManyWitnessesToAddWitness {
                actual: self.witnesses.len(),
                max: self.inputs.len(),
            });
        }

        Ok(self.witnesses.push(witness.into()))
    }

    pub fn set_auth(&mut self, keys: &[String]) -> Result<(), Error> {
        if self.kind != StagingKind::Sealed {
            return Err(Error::TxKindToSealInvalid { kind: self.kind });
        }

        if !self.need_auth() {
            return Err(Error::TxDoesntNeedPayloadAuth)
        }

        match &self.extra {
            None => {},
            Some(c) => {
                match c.clone().into() {
                    Certificate::StakeDelegation(s) => {
                        let c = unimplemented!();
                        self.extra_authed = Some(SignedCertificate::StakeDelegation(s, c).into());
                    }
                    Certificate::PoolRegistration(s) => {
                        let c = unimplemented!();
                        self.extra_authed = Some(SignedCertificate::PoolRegistration(s, c).into())
                    }
                    _ => unimplemented!()
                }
            }
        };
        self.kind = StagingKind::Authed;
        Ok(())
    }

    pub fn set_extra(&mut self, extra: chain::certificate::Certificate) -> Result<(), Error> {
        match self.kind {
            StagingKind::Balancing => Ok(self.extra = Some(extra.into())),
            kind => Err(Error::TxKindToAddExtraInvalid { kind }),
        }
    }

    pub fn witness_count(&self) -> usize {
        self.witnesses.len()
    }

    pub fn staging_kind_name(&self) -> String {
        self.kind.to_string()
    }

    fn get_inputs_outputs(&self) -> InputOutputBuilder {
        let inputs: Vec<_> = self.inputs.iter().map(|i| i.clone().into()).collect();
        let outputs: Vec<_> = self.outputs.iter().map(|o| o.clone().into()).collect();
        InputOutputBuilder::new(inputs.iter(), outputs.iter()).unwrap() // TODO better error than unwrap
    }

    fn finalize_payload<'a, P, FA>(
        &mut self,
        payload: &P,
        fee_algorithm: &FA,
        output_policy: chain::transaction::OutputPolicy,
    ) -> Result<chain::transaction::Balance, Error>
    where
        FA: FeeAlgorithm,
        P: Payload,
    {
        let ios = self.get_inputs_outputs();
        let pdata = payload.payload_data();
        let (balance, added_outputs, _) =
            ios.seal_with_output_policy(pdata.borrow(), fee_algorithm, output_policy)?;

        for o in added_outputs {
            self.add_output(o.clone().into())?;
        }

        self.kind = StagingKind::Finalizing;

        Ok(balance)
    }

    pub fn balance_inputs_outputs<FA>(
        &mut self,
        fee_algorithm: &FA,
        output_policy: chain::transaction::OutputPolicy,
    ) -> Result<chain::transaction::Balance, Error>
    where
        FA: FeeAlgorithm,
    {
        if self.kind != StagingKind::Balancing {
            return Err(Error::TxKindToFinalizeInvalid { kind: self.kind });
        }

        match &self.extra {
            None => {
                self.finalize_payload(&chain::transaction::NoExtra, fee_algorithm, output_policy)
            }
            Some(ref c) => match c.clone().into() {
                Certificate::PoolRegistration(c) => {
                    self.finalize_payload(&c, fee_algorithm, output_policy)
                }
                Certificate::PoolUpdate(c) => {
                    self.finalize_payload(&c, fee_algorithm, output_policy)
                }
                Certificate::PoolRetirement(c) => {
                    self.finalize_payload(&c, fee_algorithm, output_policy)
                }
                Certificate::StakeDelegation(c) => {
                    self.finalize_payload(&c, fee_algorithm, output_policy)
                }
                Certificate::OwnerStakeDelegation(c) => {
                    self.finalize_payload(&c, fee_algorithm, output_policy)
                }
            },
        }
    }

    pub fn seal(&mut self) -> Result<(), Error> {
        if self.kind != StagingKind::Finalizing {
            return Err(Error::TxKindToSealInvalid { kind: self.kind });
        }

        if self.inputs.len() != self.witnesses.len() {
            return Err(Error::WitnessCountToSealInvalid {
                actual: self.witnesses.len(),
                expected: self.inputs.len(),
            });
        }

        Ok(self.kind = StagingKind::Sealed)
    }

    pub fn need_auth(&self) -> bool {
        match &self.extra {
            None => false,
            Some(ref c) => {
                let x: Certificate = c.clone().into();
                x.need_auth()
            }
        }
    }

    fn builder_after_witness<P: Payload>(
        &self,
        builder: TxBuilderState<SetIOs<P>>,
    ) -> Result<TxBuilderState<SetAuthData<P>>, Error> {
        if self.witnesses.len() != self.inputs.len() {
            return Err(Error::TxKindToFinalizeInvalid { kind: self.kind });
        }

        let ios = self.get_inputs_outputs().build();
        let witnesses: Vec<_> = self.witnesses.iter().map(|w| w.clone().into()).collect();
        Ok(builder
            .set_ios(&ios.inputs, &ios.outputs)
            .set_witnesses(&witnesses))
    }

    fn make_fragment<P: Payload, F>(
        &self,
        payload: &P,
        auth: &P::Auth,
        to_fragment: F,
    ) -> Result<Fragment, Error>
    where
        F: FnOnce(Transaction<P>) -> Fragment,
    {
        let tx = self
            .builder_after_witness(TxBuilder::new().set_payload(payload))?
            .set_payload_auth(auth);
        Ok(to_fragment(tx))
    }

    pub fn fragment(&self) -> Result<Fragment, Error> {
        match &self.extra_authed {
            None => {
                if self.kind != StagingKind::Sealed {
                    Err(Error::TxKindToGetMessageInvalid { kind: self.kind })?
                }
                assert!(self.extra.is_none());
                self.make_fragment(&chain::transaction::NoExtra, &(), Fragment::Transaction)
            }
            Some(signed_cert) => {
                if self.kind != StagingKind::Authed {
                    Err(Error::TxKindToGetMessageInvalid { kind: self.kind })?
                }
                match signed_cert.clone().into() {
                    SignedCertificate::PoolRegistration(c, a) => {
                        self.make_fragment(&c, &a, Fragment::PoolRegistration)
                    }
                    SignedCertificate::PoolUpdate(c, a) => {
                        self.make_fragment(&c, &a, Fragment::PoolUpdate)
                    }
                    SignedCertificate::PoolRetirement(c, a) => {
                        self.make_fragment(&c, &a, Fragment::PoolRetirement)
                    }
                    SignedCertificate::StakeDelegation(c, a) => {
                        self.make_fragment(&c, &a, Fragment::StakeDelegation)
                    }
                    SignedCertificate::OwnerStakeDelegation(c, a) => {
                        self.make_fragment(&c, &a, Fragment::OwnerStakeDelegation)
                    }
                }
            }
        }
    }

    fn transaction_sign_data_hash_on<P>(
        &self,
        builder: TxBuilderState<SetIOs<P>>,
    ) -> TransactionSignDataHash {
        let inputs: Vec<transaction::Input> =
            self.inputs.iter().map(|i| i.clone().into()).collect();
        let outputs: Vec<_> = self.outputs.iter().map(|o| o.clone().into()).collect();
        builder
            .set_ios(&inputs, &outputs)
            .get_auth_data_for_witness()
            .hash()
    }

    pub fn transaction_sign_data_hash(&self) -> TransactionSignDataHash {
        match &self.extra {
            None => self.transaction_sign_data_hash_on(TxBuilder::new().set_nopayload()),
            Some(ref c) => match c.clone().into() {
                Certificate::PoolRegistration(c) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&c))
                }
                Certificate::PoolUpdate(c) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&c))
                }
                Certificate::PoolRetirement(c) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&c))
                }
                Certificate::StakeDelegation(c) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&c))
                }
                Certificate::OwnerStakeDelegation(c) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&c))
                }
            },
        }
    }

    /*
    pub fn transaction<P>(
        &self,
    ) -> chain::transaction::Transaction<Address, Option<chain::certificate::Certificate>> {
        chain::transaction::Transaction {
            inputs: self.inputs(),
            outputs: self.outputs(),
            extra: self.extra.clone().map(|c| c.0),
        }
    }

    pub fn fees<FA>(&self, fee_algorithm: FA) -> Result<Value, Error>
    where
        FA: FeeAlgorithm<Transaction<Address, Option<chain::certificate::Certificate>>>,
    {
        let v = fee_algorithm
            .calculate(&self.transaction())
            .ok_or(Error::FeeCalculationFailed)?;
        Ok(v)
    }

    pub fn balance<FA>(&self, fee_algorithm: FA) -> Result<chain::transaction::Balance, Error>
    where
        FA: FeeAlgorithm<Transaction<Address, Option<chain::certificate::Certificate>>>,
    {
        let fees = self.fees(fee_algorithm)?;
        let transaction = self.transaction();
        let balance = transaction.balance(fees)?;
        Ok(balance)
    }

    pub fn finalizer(&self) -> Result<chain::txbuilder::TransactionFinalizer, Error> {
        let mut finalizer = chain::txbuilder::TransactionFinalizer::new(self.transaction());

        for (index, witness) in self.witnesses.iter().enumerate() {
            finalizer
                .set_witness(index, witness.clone().into())
                .map_err(|source| Error::AddingWitnessToFinalizedTxFailed {
                    source,
                    filler: CustomErrorFiller,
                })?;
        }

        Ok(finalizer)
    }
    */

    pub fn inputs(&self) -> &[interfaces::TransactionInput] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[interfaces::TransactionOutput] {
        &self.outputs
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use chain_impl_mockchain as chain;
    use chain_impl_mockchain::{key::Hash, transaction::Input, value::Value};
    use std::str::FromStr;

    #[test]
    pub fn test_initial_stage_is_balancing() {
        let staging = Staging::new();
        let expected_kind = StagingKind::Balancing;
        assert_eq!(
            staging.kind, expected_kind,
            "'initial staging kind should be {}",
            expected_kind
        );
    }

    #[test]
    pub fn test_cannot_add_input_when_stage_is_finalizing() {
        let hash =
            Hash::from_str("c355a02d3b5337ad0e5f5940582675229f25bc03e7feebc3aa929738e1fec35e")
                .unwrap();
        let incorrect_stage = StagingKind::Finalizing;

        let mut staging = Staging::new();
        staging.kind = incorrect_stage.clone();

        let mut input_ptr = [0u8; chain::transaction::INPUT_PTR_SIZE];
        input_ptr.clone_from_slice(hash.as_ref());

        let result = staging.add_input(Input::new(0, Value(200), input_ptr).into());

        assert!(
            result.is_err(),
            "add_input message should throw exception when adding inputs while in {:?} stage",
            &incorrect_stage
        );
    }
}
