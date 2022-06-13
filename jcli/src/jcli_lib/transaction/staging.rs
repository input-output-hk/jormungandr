use crate::jcli_lib::{
    certificate::{
        self, committee_vote_plan_sign, committee_vote_tally_sign, evm_mapping_sign,
        pool_owner_sign, stake_delegation_account_binding_sign, update_proposal_sign,
        update_vote_sign,
    },
    transaction::Error,
    utils::io,
};
use chain_addr::{Address, Kind};
use chain_impl_mockchain::{
    self as chain,
    certificate::{Certificate, CertificatePayload, PoolSignature, SignedCertificate},
    fee::FeeAlgorithm,
    fragment::Fragment,
    transaction::{
        self, Balance, InputOutputBuilder, Output, Payload, SetAuthData, SetTtl, Transaction,
        TransactionSignDataHash, TxBuilder, TxBuilderState, UnspecifiedAccountIdentifier,
    },
    value::{Value, ValueError},
};
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
    valid_until: Option<interfaces::BlockDate>,
    witnesses: Vec<interfaces::TransactionWitness>,
    extra: Option<interfaces::Certificate>,
    extra_authed: Option<interfaces::SignedCertificate>,
    evm_transaction: Option<interfaces::EvmTransaction>,
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

impl Default for Staging {
    fn default() -> Self {
        Self::new()
    }
}

impl Staging {
    pub fn new() -> Self {
        Staging {
            kind: StagingKind::Balancing,
            inputs: Vec::new(),
            outputs: Vec::new(),
            valid_until: None,
            witnesses: Vec::new(),
            extra: None,
            extra_authed: None,
            evm_transaction: None,
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

    pub fn set_expiry_date(&mut self, input: interfaces::BlockDate) -> Result<(), Error> {
        if self.kind != StagingKind::Balancing {
            return Err(Error::TxKindToSetValidityTimeInvalid { kind: self.kind });
        }

        self.valid_until = Some(input);

        Ok(())
    }

    pub fn add_input(&mut self, input: interfaces::TransactionInput) -> Result<(), Error> {
        if self.kind != StagingKind::Balancing {
            return Err(Error::TxKindToAddInputInvalid { kind: self.kind });
        }

        self.inputs.push(input);

        Ok(())
    }

    pub fn add_output(&mut self, output: Output<Address>) -> Result<(), Error> {
        if self.kind != StagingKind::Balancing {
            return Err(Error::TxKindToAddOutputInvalid { kind: self.kind });
        }

        self.outputs.push(output.into());

        Ok(())
    }

    pub fn add_account(
        &mut self,
        account: interfaces::Address,
        value: interfaces::Value,
    ) -> Result<(), Error> {
        let account_id = match Address::from(account).kind() {
            Kind::Account(key) => {
                UnspecifiedAccountIdentifier::from_single_account(key.clone().into())
            }
            Kind::Multisig(key) => UnspecifiedAccountIdentifier::from_multi_account((*key).into()),
            Kind::Single(_) => return Err(Error::AccountAddressSingle),
            Kind::Group(_, _) => return Err(Error::AccountAddressGroup),
            Kind::Script(_) => return Err(Error::AccountAddressScript),
        };

        self.add_input(interfaces::TransactionInput {
            input: interfaces::TransactionInputType::Account(account_id.into()),
            value,
        })?;
        Ok(())
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

        self.witnesses.push(witness.into());

        Ok(())
    }

    pub fn set_auth(&mut self, keys: &[String]) -> Result<(), Error> {
        if self.kind != StagingKind::Sealed {
            return Err(Error::TxKindToSealInvalid { kind: self.kind });
        }

        if !self.need_auth() {
            return Err(Error::TxDoesntNeedPayloadAuth);
        }

        match &self.extra {
            None => unreachable!(),
            Some(c) => match c.clone().into() {
                Certificate::StakeDelegation(s) => {
                    let builder = self.builder_after_witness(TxBuilder::new().set_payload(&s))?;
                    let sc = keys
                        .len()
                        .eq(&1)
                        .then(|| {
                            stake_delegation_account_binding_sign(s, &keys[0], builder)
                                .map_err(|e| Error::CertificateError { error: e })
                        })
                        .ok_or(certificate::Error::ExpectingOnlyOneSigningKey { got: keys.len() })
                        .map_err(|error| Error::CertificateError { error })??;
                    self.extra_authed = Some(sc.into());
                }
                Certificate::PoolRegistration(s) => {
                    let sclone = s.clone();
                    let pool_reg = Some(&sclone);
                    let builder = self.builder_after_witness(TxBuilder::new().set_payload(&s))?;
                    let sc = pool_owner_sign(s, pool_reg, keys, builder, |p, pos| {
                        SignedCertificate::PoolRegistration(p, PoolSignature::Owners(pos))
                    })
                    .map_err(|e| Error::CertificateError { error: e })?;
                    self.extra_authed = Some(sc.into())
                }
                Certificate::PoolRetirement(s) => {
                    let pool_reg = None; // TODO eventually ask for optional extra registration cert to do a better job
                    let builder = self.builder_after_witness(TxBuilder::new().set_payload(&s))?;
                    let sc = pool_owner_sign(s, pool_reg, keys, builder, |p, pos| {
                        SignedCertificate::PoolRetirement(p, PoolSignature::Owners(pos))
                    })
                    .map_err(|e| Error::CertificateError { error: e })?;
                    self.extra_authed = Some(sc.into())
                }
                Certificate::PoolUpdate(s) => {
                    let pool_reg = None; // TODO eventually ask for optional extra registration cert to do a better job
                    let builder = self.builder_after_witness(TxBuilder::new().set_payload(&s))?;
                    let sc = pool_owner_sign(s, pool_reg, keys, builder, |p, pos| {
                        SignedCertificate::PoolUpdate(p, PoolSignature::Owners(pos))
                    })
                    .map_err(|e| Error::CertificateError { error: e })?;
                    self.extra_authed = Some(sc.into())
                }
                Certificate::OwnerStakeDelegation(_) => unreachable!(),
                Certificate::VotePlan(vp) => {
                    let builder = self.builder_after_witness(TxBuilder::new().set_payload(&vp))?;
                    let sc = keys
                        .len()
                        .eq(&1)
                        .then(|| {
                            committee_vote_plan_sign(vp, &keys[0], builder)
                                .map_err(|e| Error::CertificateError { error: e })
                        })
                        .ok_or(certificate::Error::ExpectingOnlyOneSigningKey { got: keys.len() })
                        .map_err(|error| Error::CertificateError { error })??;
                    self.extra_authed = Some(sc.into())
                }
                Certificate::VoteCast(_) => unreachable!(),
                Certificate::VoteTally(vt) => {
                    let builder = self.builder_after_witness(TxBuilder::new().set_payload(&vt))?;
                    let sc = keys
                        .len()
                        .eq(&1)
                        .then(|| {
                            committee_vote_tally_sign(vt, &keys[0], builder)
                                .map_err(|e| Error::CertificateError { error: e })
                        })
                        .ok_or(certificate::Error::ExpectingOnlyOneSigningKey { got: keys.len() })
                        .map_err(|error| Error::CertificateError { error })??;
                    self.extra_authed = Some(sc.into())
                }
                Certificate::UpdateProposal(up) => {
                    let builder = self.builder_after_witness(TxBuilder::new().set_payload(&up))?;
                    let sc = keys
                        .len()
                        .eq(&1)
                        .then(|| {
                            update_proposal_sign(up, &keys[0], builder)
                                .map_err(|e| Error::CertificateError { error: e })
                        })
                        .ok_or(certificate::Error::ExpectingOnlyOneSigningKey { got: keys.len() })
                        .map_err(|error| Error::CertificateError { error })??;
                    self.extra_authed = Some(sc.into())
                }
                Certificate::UpdateVote(uv) => {
                    let builder = self.builder_after_witness(TxBuilder::new().set_payload(&uv))?;
                    let sc = keys
                        .len()
                        .eq(&1)
                        .then(|| {
                            update_vote_sign(uv, &keys[0], builder)
                                .map_err(|e| Error::CertificateError { error: e })
                        })
                        .ok_or(certificate::Error::ExpectingOnlyOneSigningKey { got: keys.len() })
                        .map_err(|error| Error::CertificateError { error })??;
                    self.extra_authed = Some(sc.into())
                }
                Certificate::EvmMapping(uv) => {
                    let builder = self.builder_after_witness(TxBuilder::new().set_payload(&uv))?;
                    let sc = keys
                        .len()
                        .eq(&1)
                        .then(|| {
                            evm_mapping_sign(uv, &keys[0], builder)
                                .map_err(|e| Error::CertificateError { error: e })
                        })
                        .ok_or(certificate::Error::ExpectingOnlyOneSigningKey { got: keys.len() })
                        .map_err(|error| Error::CertificateError { error })??;
                    self.extra_authed = Some(sc.into())
                }
                Certificate::MintToken(_) => unreachable!(),
            },
        };
        self.kind = StagingKind::Authed;
        Ok(())
    }

    pub fn set_extra(&mut self, extra: interfaces::Certificate) -> Result<(), Error> {
        match self.kind {
            StagingKind::Balancing => {
                self.evm_transaction = None;
                self.extra = Some(extra);
                Ok(())
            }
            kind => Err(Error::TxKindToAddExtraInvalid { kind }),
        }
    }

    pub fn set_evm_transaction(
        &mut self,
        evm_transaction: interfaces::EvmTransaction,
    ) -> Result<(), Error> {
        match self.kind {
            StagingKind::Balancing => {
                self.evm_transaction = Some(evm_transaction);
                self.extra = None;
                Ok(())
            }
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

    fn finalize_payload<P, FA>(
        &mut self,
        payload: &P,
        fee_algorithm: &FA,
        output_policy: chain::transaction::OutputPolicy,
    ) -> Result<Balance, Error>
    where
        FA: FeeAlgorithm,
        P: Payload,
    {
        if self.valid_until.is_none() {
            return Err(Error::CannotFinalizeWithoutValidUntil);
        }

        let ios = self.get_inputs_outputs();
        let pdata = payload.payload_data();
        let (balance, added_outputs, _) =
            ios.seal_with_output_policy(pdata.borrow(), fee_algorithm, output_policy)?;

        for o in added_outputs {
            self.add_output(o.clone())?;
        }

        self.kind = StagingKind::Finalizing;

        Ok(balance)
    }

    pub fn balance_inputs_outputs<FA>(
        &mut self,
        fee_algorithm: &FA,
        output_policy: chain::transaction::OutputPolicy,
    ) -> Result<Balance, Error>
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
            Some(c) => match c.clone().into() {
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
                Certificate::VotePlan(vp) => {
                    self.finalize_payload(&vp, fee_algorithm, output_policy)
                }
                Certificate::VoteCast(vp) => {
                    self.finalize_payload(&vp, fee_algorithm, output_policy)
                }
                Certificate::VoteTally(vt) => {
                    self.finalize_payload(&vt, fee_algorithm, output_policy)
                }
                Certificate::UpdateProposal(vt) => {
                    self.finalize_payload(&vt, fee_algorithm, output_policy)
                }
                Certificate::UpdateVote(vt) => {
                    self.finalize_payload(&vt, fee_algorithm, output_policy)
                }
                Certificate::MintToken(vt) => {
                    self.finalize_payload(&vt, fee_algorithm, output_policy)
                }
                Certificate::EvmMapping(vt) => {
                    self.finalize_payload(&vt, fee_algorithm, output_policy)
                }

                Certificate::OwnerStakeDelegation(c) => {
                    let balance = self.finalize_payload(&c, fee_algorithm, output_policy)?;
                    match self.inputs() {
                        [input] => match input.input {
                            interfaces::TransactionInputType::Account(_) => (),
                            interfaces::TransactionInputType::Utxo(_, _) => {
                                return Err(Error::TxWithOwnerStakeDelegationHasUtxoInput)
                            }
                        },
                        inputs => {
                            return Err(Error::TxWithOwnerStakeDelegationMultiInputs {
                                inputs: inputs.len(),
                            })
                        }
                    };
                    if !self.outputs().is_empty() {
                        return Err(Error::TxWithOwnerStakeDelegationHasOutputs);
                    }
                    Ok(balance)
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

        self.kind = StagingKind::Sealed;

        Ok(())
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
        builder: TxBuilderState<SetTtl<P>>,
    ) -> Result<TxBuilderState<SetAuthData<P>>, Error> {
        if self.witnesses.len() != self.inputs.len() {
            return Err(Error::TxKindToFinalizeInvalid { kind: self.kind });
        }

        let ios = self.get_inputs_outputs().build();
        let witnesses: Vec<_> = self.witnesses.iter().map(|w| w.clone().into()).collect();
        let valid_until = self
            .valid_until
            .expect("transaction validity time should be set at this point")
            .into();
        Ok(builder
            .set_expiry_date(valid_until)
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
                    return Err(Error::TxKindToGetMessageInvalid { kind: self.kind });
                }
                if self.need_auth() {
                    return Err(Error::TxNeedPayloadAuth);
                }
                match &self.extra {
                    None => {
                        self.make_fragment(&chain::transaction::NoExtra, &(), Fragment::Transaction)
                    }
                    Some(cert) => match cert.clone().into() {
                        Certificate::OwnerStakeDelegation(osd) => {
                            self.make_fragment(&osd, &(), Fragment::OwnerStakeDelegation)
                        }
                        Certificate::VoteCast(vote_cast) => {
                            self.make_fragment(&vote_cast, &(), Fragment::VoteCast)
                        }
                        Certificate::MintToken(mint_token) => {
                            self.make_fragment(&mint_token, &(), Fragment::MintToken)
                        }
                        _ => unreachable!(),
                    },
                }
            }
            Some(signed_cert) => {
                if self.kind != StagingKind::Authed {
                    return Err(Error::TxKindToGetMessageInvalid { kind: self.kind });
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
                    SignedCertificate::VotePlan(vp, a) => {
                        self.make_fragment(&vp, &a, Fragment::VotePlan)
                    }
                    SignedCertificate::VoteTally(vt, a) => {
                        self.make_fragment(&vt, &a, Fragment::VoteTally)
                    }
                    SignedCertificate::UpdateProposal(vt, a) => {
                        self.make_fragment(&vt, &a, Fragment::UpdateProposal)
                    }
                    SignedCertificate::UpdateVote(vt, a) => {
                        self.make_fragment(&vt, &a, Fragment::UpdateVote)
                    }
                    SignedCertificate::EvmMapping(vt, a) => {
                        self.make_fragment(&vt, &a, Fragment::EvmMapping)
                    }
                }
            }
        }
    }

    fn transaction_sign_data_hash_on<P>(
        &self,
        builder: TxBuilderState<SetTtl<P>>,
    ) -> TransactionSignDataHash {
        let inputs: Vec<transaction::Input> =
            self.inputs.iter().map(|i| i.clone().into()).collect();
        let outputs: Vec<_> = self.outputs.iter().map(|o| o.clone().into()).collect();
        let valid_until = self
            .valid_until
            .expect("transaction validity time should be set at this point")
            .into();
        builder
            .set_expiry_date(valid_until)
            .set_ios(&inputs, &outputs)
            .get_auth_data_for_witness()
            .hash()
    }

    pub fn transaction_sign_data_hash(&self) -> Result<TransactionSignDataHash, Error> {
        if self.kind != StagingKind::Finalizing {
            return Err(Error::TxKindToSignDataHashInvalid { kind: self.kind });
        }

        let res = match &self.extra {
            None => self.transaction_sign_data_hash_on(TxBuilder::new().set_nopayload()),
            Some(c) => match c.clone().into() {
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
                Certificate::VotePlan(cp) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&cp))
                }
                Certificate::VoteCast(cp) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&cp))
                }
                Certificate::VoteTally(vt) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&vt))
                }
                Certificate::UpdateProposal(vt) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&vt))
                }
                Certificate::UpdateVote(vt) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&vt))
                }
                Certificate::MintToken(vt) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&vt))
                }
                Certificate::EvmMapping(vt) => {
                    self.transaction_sign_data_hash_on(TxBuilder::new().set_payload(&vt))
                }
            },
        };

        Ok(res)
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

    pub fn finalizer(&self) -> Result<chain::txbuilder::TransactionFinalizer, Error> {
        let mut finalizer = chain::txbuilder::TransactionFinalizer::new(self.transaction());

        for (index, witness) in self.witnesses.iter().enumerate() {
            finalizer
                .set_witness(index, witness.clone().into())
                .map_err(|source| Error::AddingWitnessToFinalizedTxFailed {
                    source,
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

    pub fn total_input(&self) -> Result<Value, ValueError> {
        Value::sum(self.inputs().iter().map(|input| input.value.into()))
    }

    pub fn total_output(&self) -> Result<Value, ValueError> {
        Value::sum(self.outputs().iter().map(|output| *output.value().as_ref()))
    }

    pub fn fees(&self, fee_algorithm: &impl FeeAlgorithm) -> Value {
        let cert_extra = self.extra_authed.clone().map(|cert| cert.strip_auth());
        let cert_payload = cert_extra
            .as_ref()
            .or(self.extra.as_ref())
            .map(|cert| CertificatePayload::from(&cert.0));
        let cert_slice = cert_payload.as_ref().map(CertificatePayload::as_slice);
        let inputs_count = self.inputs().len() as u8;
        let outputs_count = self.outputs().len() as u8;
        fee_algorithm.calculate(cert_slice, inputs_count, outputs_count)
    }

    pub fn balance(&self, fee_algorithm: &impl FeeAlgorithm) -> Result<Balance, ValueError> {
        use std::cmp::Ordering::*;

        let fees = self.fees(fee_algorithm);
        let inputs = Value::sum(self.inputs().iter().map(|i| i.value.into()))?;
        let outputs = Value::sum(self.outputs().iter().map(|o| (*o.value()).into()))?;
        let z = (outputs + fees)?;
        match inputs.cmp(&z) {
            Greater => Ok(Balance::Positive((inputs - z)?)),
            Less => Ok(Balance::Negative((z - inputs)?)),
            Equal => Ok(Balance::Zero),
        }
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
        staging.kind = incorrect_stage;

        let mut input_ptr = [0u8; chain::transaction::INPUT_PTR_SIZE];
        input_ptr.clone_from_slice(hash.as_ref());

        let result = staging.add_input(Input::new(0, Value(200), input_ptr).into());

        assert!(
            result.is_err(),
            "add_input message should throw exception when adding inputs while in {:?} stage",
            incorrect_stage
        );
    }
}
