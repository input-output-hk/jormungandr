use super::data::{
    settings::SettingsSettingsFees,
    transaction_by_id::{
        TransactionByIdTransaction, TransactionByIdTransactionCertificate,
        TransactionByIdTransactionCertificateOnOwnerStakeDelegation,
        TransactionByIdTransactionCertificateOnPoolRegistration,
        TransactionByIdTransactionCertificateOnPoolRetirement,
        TransactionByIdTransactionCertificateOnPoolUpdate,
        TransactionByIdTransactionCertificateOnStakeDelegation,
    },
};
use bech32::FromBase32;
use chain_addr::AddressReadable;
use chain_crypto::{PublicKey, Ed25519};
use std::num::NonZeroU64;

use chain_impl_mockchain::{
    account::DelegationType,
    certificate::{
        OwnerStakeDelegation, PoolRegistration, PoolRetirement, PoolUpdate, StakeDelegation,
    },
    fee::LinearFee,
    fragment::Fragment,
    transaction::{AccountIdentifier, InputEnum, Transaction},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerifierError {
    #[error("Not implemented")]
    Unimplemented,
    #[error("Invalid certificate, received: {received}")]
    InvalidCertificate { received: String },
}
pub struct ExplorerVerifier;

impl ExplorerVerifier {
    pub fn assert_transaction(
        fragment: Fragment,
        exp_transaction: TransactionByIdTransaction,
    ) -> Result<(), VerifierError> {
        if exp_transaction.certificate.is_none() {
            if let Fragment::Transaction(frag_transaction) = fragment {
                Self::assert_transaction_params(frag_transaction, exp_transaction).unwrap();
                Ok(())
            } else {
                Err(VerifierError::InvalidCertificate {
                    received: "Transaction".to_string(),
                })
            }
        } else {
            let exp_certificate = exp_transaction.certificate.as_ref().unwrap();

            match exp_certificate {
                TransactionByIdTransactionCertificate::StakeDelegation(exp_cert) => {
                    if let Fragment::StakeDelegation(frag_cert) = fragment {
                        Self::assert_transaction_params(frag_cert.clone(), exp_transaction.clone())
                            .unwrap();
                        Self::assert_stake_delegation(frag_cert, exp_cert.clone()).unwrap();
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "StakeDelegation".to_string(),
                        })
                    }
                }
                TransactionByIdTransactionCertificate::OwnerStakeDelegation(exp_cert) => {
                    if let Fragment::OwnerStakeDelegation(frag_cert) = fragment {
                        Self::assert_transaction_params(frag_cert.clone(), exp_transaction.clone())
                            .unwrap();
                        Self::assert_owner_delegation(frag_cert, exp_cert.clone()).unwrap();
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "OwnerStakeDelegation".to_string(),
                        })
                    }
                }
                TransactionByIdTransactionCertificate::PoolRegistration(exp_cert) => {
                    if let Fragment::PoolRegistration(frag_cert) = fragment {
                        Self::assert_transaction_params(frag_cert.clone(), exp_transaction.clone())
                            .unwrap();
                        Self::assert_pool_registration(frag_cert, exp_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "PoolRegistration".to_string(),
                        })
                    }
                }
                TransactionByIdTransactionCertificate::PoolRetirement(exp_cert) => {
                    if let Fragment::PoolRetirement(frag_cert) = fragment {
                        Self::assert_transaction_params(frag_cert.clone(), exp_transaction.clone())
                            .unwrap();
                        Self::assert_pool_retirement(frag_cert, exp_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "PoolRetirement".to_string(),
                        })
                    }
                }
                TransactionByIdTransactionCertificate::PoolUpdate(exp_cert) => {
                    if let Fragment::PoolUpdate(frag_cert) = fragment {
                        Self::assert_transaction_params(frag_cert.clone(), exp_transaction.clone())
                            .unwrap();
                        Self::assert_pool_update(frag_cert, exp_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "PoolUpdate".to_string(),
                        })
                    }
                }
                TransactionByIdTransactionCertificate::VotePlan(_) => todo!(),
                TransactionByIdTransactionCertificate::VoteCast(_) => todo!(),
                TransactionByIdTransactionCertificate::VoteTally(_) => todo!(),
                TransactionByIdTransactionCertificate::UpdateProposal(_) => todo!(),
                TransactionByIdTransactionCertificate::UpdateVote(_) => todo!(),
                TransactionByIdTransactionCertificate::MintToken(_) => todo!(),
                TransactionByIdTransactionCertificate::EvmMapping(_) => todo!(),
            }
        }
    }

    fn assert_transaction_params<P>(
        frag_transaction: Transaction<P>,
        exp_transaction: TransactionByIdTransaction,
    ) -> Result<(), VerifierError> {
        assert_eq!(
            frag_transaction.as_slice().nb_inputs(),
            exp_transaction.inputs.len() as u8
        );

        if frag_transaction.as_slice().nb_inputs() > 0 {
            let mut frag_accounts = vec![];

            for frag_input in frag_transaction.as_slice().inputs().iter() {
                match frag_input.to_enum() {
                    InputEnum::AccountInput(account_id, amount) => {
                        frag_accounts.push((
                            account_id.to_single_account().unwrap().to_string(),
                            amount.to_string(),
                        ));
                        Ok(())
                    }
                    InputEnum::UtxoInput(_) => Err(VerifierError::Unimplemented),
                }
                .unwrap()
            }

            let mut exp_accounts = vec![];

            for exp_inputs in exp_transaction.inputs.iter() {
                let adr = AddressReadable::from_string_anyprefix(&exp_inputs.address.id).unwrap();
                exp_accounts.push((
                    adr.to_address().public_key().unwrap().to_string(),
                    exp_inputs.amount.clone(),
                ));
            }

            let matching_inputs = frag_accounts
                .iter()
                .zip(exp_accounts.iter())
                .filter(|&(a, b)| a == b)
                .count();
            assert_eq!(matching_inputs, exp_transaction.inputs.len());
        };

        assert_eq!(
            frag_transaction.as_slice().nb_outputs(),
            exp_transaction.outputs.len() as u8
        );

        if frag_transaction.as_slice().nb_outputs() > 0 {
            let mut frag_accounts = vec![];

            for frag_output in frag_transaction.as_slice().outputs().iter() {
                frag_accounts.push((
                    frag_output.address.public_key().unwrap().to_string(),
                    frag_output.value.to_string(),
                ));
            }

            let mut exp_accounts = vec![];

            for exp_outputs in exp_transaction.outputs.iter() {
                let adr = AddressReadable::from_string_anyprefix(&exp_outputs.address.id).unwrap();
                exp_accounts.push((
                    adr.to_address().public_key().unwrap().to_string(),
                    exp_outputs.amount.clone(),
                ));
            }

            let matching_outputs = frag_accounts
                .iter()
                .zip(exp_accounts.iter())
                .filter(|&(a, b)| a == b)
                .count();
            assert_eq!(matching_outputs, exp_transaction.outputs.len());
        };
        Ok(())
    }

    fn assert_pool_registration(
        frag_cert: Transaction<PoolRegistration>,
        exp_cert: TransactionByIdTransactionCertificateOnPoolRegistration,
    ) {
        let pool_cert = frag_cert.as_slice().payload().into_payload();

        assert_eq!(pool_cert.to_id().to_string(), exp_cert.pool.id);
        assert_eq!(
            u64::from(pool_cert.start_validity),
            exp_cert.start_validity.parse::<u64>().unwrap()
        );
        if pool_cert.reward_account.is_some() {
            if let AccountIdentifier::Single(id) = pool_cert.reward_account.as_ref().unwrap() {
                assert_eq!(id.to_string(), exp_cert.reward_account.unwrap().id);
            }
        }

        assert_eq!(
            pool_cert.rewards.ratio.numerator,
            exp_cert.rewards.ratio.numerator.parse::<u64>().unwrap()
        );
        assert_eq!(
            pool_cert.rewards.ratio.denominator,
            exp_cert
                .rewards
                .ratio
                .denominator
                .parse::<NonZeroU64>()
                .unwrap()
        );
        if pool_cert.rewards.max_limit.is_some() {
            assert_eq!(
                pool_cert.rewards.max_limit.unwrap(),
                exp_cert
                    .rewards
                    .max_limit
                    .unwrap()
                    .parse::<NonZeroU64>()
                    .unwrap()
            );
        }

        assert_eq!(
            pool_cert.management_threshold(),
            exp_cert.management_threshold as u8
        );

        assert_eq!(pool_cert.owners.len(), exp_cert.owners.len());

        let owners_matching = pool_cert
            .owners
            .iter()
            .zip(exp_cert.owners.iter())
            .filter(|&(a, b)| *a == Self::decode_bech32_pk(b))
            .count();

        assert_eq!(pool_cert.owners.len(), owners_matching);

        assert_eq!(pool_cert.operators.len(), exp_cert.operators.len());

        let operators_matching = pool_cert
            .operators
            .iter()
            .zip(exp_cert.operators.iter())
            .filter(|&(a, b)| *a == Self::decode_bech32_pk(b))
            .count();

        assert_eq!(pool_cert.operators.len(), operators_matching);

        assert!(exp_cert.pool.retirement.is_none());
    }

    fn assert_stake_delegation(
        frag_cert: Transaction<StakeDelegation>,
        exp_cert: TransactionByIdTransactionCertificateOnStakeDelegation,
    ) -> Result<(), VerifierError> {
        let deleg_cert = frag_cert.as_slice().payload().into_payload();
        let adr = AddressReadable::from_string_anyprefix(&exp_cert.account.id).unwrap();
        assert_eq!(
            deleg_cert
                .account_id
                .to_single_account()
                .unwrap()
                .to_string(),
                adr.to_address().public_key().unwrap().to_string()
        );

        match deleg_cert.delegation {
            DelegationType::NonDelegated => Err(VerifierError::Unimplemented),
            DelegationType::Full(pool_id) => {
                assert_eq!(exp_cert.pools.len(), 1);
                assert_eq!(pool_id.to_string(), exp_cert.pools[0].id);
                Ok(())
            }
            DelegationType::Ratio(deleg) => {
                assert_eq!(exp_cert.pools.len(), deleg.pools().len());
                let pools_matching = exp_cert
                    .pools
                    .iter()
                    .zip(deleg.pools().iter())
                    .filter(|&(a, b)| a.id == b.0.to_string())
                    .count();
                assert_eq!(pools_matching, exp_cert.pools.len());
                Ok(())
            }
        }
    }
    fn assert_owner_delegation(
        frag_cert: Transaction<OwnerStakeDelegation>,
        exp_cert: TransactionByIdTransactionCertificateOnOwnerStakeDelegation,
    )-> Result<(),VerifierError> {
        let owner_cert = frag_cert.as_slice().payload().into_payload();

        match owner_cert.delegation {
            DelegationType::NonDelegated => Err(VerifierError::Unimplemented),
            DelegationType::Full(pool_id) => {
                assert_eq!(exp_cert.pools.len(), 1);
                assert_eq!(pool_id.to_string(), exp_cert.pools[0].id);
                Ok(())
            }
            DelegationType::Ratio(deleg) => {
                assert_eq!(exp_cert.pools.len(), deleg.pools().len());
                let pools_matching = exp_cert
                    .pools
                    .iter()
                    .zip(deleg.pools().iter())
                    .filter(|&(a, b)| a.id == b.0.to_string())
                    .count();
                assert_eq!(pools_matching, exp_cert.pools.len());
                Ok(())
            }
        }
    }

    fn assert_pool_retirement(
        frag_cert: Transaction<PoolRetirement>,
        exp_cert: TransactionByIdTransactionCertificateOnPoolRetirement,
    ) {
        let ret_cert = frag_cert.as_slice().payload().into_payload();
        assert_eq!(ret_cert.pool_id.to_string(), exp_cert.pool_id);
        assert_eq!(
            u64::from(ret_cert.retirement_time),
            exp_cert.retirement_time.parse::<u64>().unwrap()
        );
    }

    fn assert_pool_update(
        frag_cert: Transaction<PoolUpdate>,
        exp_cert: TransactionByIdTransactionCertificateOnPoolUpdate,
    ) {
        let update_cert = frag_cert.as_slice().payload().into_payload();
        assert_eq!(update_cert.pool_id.to_string(), exp_cert.pool_id);
        assert_eq!(
            u64::from(update_cert.new_pool_reg.start_validity),
            exp_cert.start_validity.parse::<u64>().unwrap()
        );
    }

    pub fn epoch_stability_depth(depth: u32, exp_depth: i64) {
        assert_eq!(depth as u64, exp_depth as u64);
    }

    pub fn assert_fees(fees: LinearFee, exp_fees: SettingsSettingsFees) {
        assert_eq!(exp_fees.certificate as u64, fees.certificate);
        assert_eq!(exp_fees.coefficient as u64, fees.coefficient);
        assert_eq!(exp_fees.constant as u64, fees.constant);
        assert_eq!(
            exp_fees
                .per_certificate_fees
                .certificate_owner_stake_delegation
                .unwrap() as u64,
            u64::from(
                fees.per_certificate_fees
                    .certificate_owner_stake_delegation
                    .unwrap()
            )
        );
        assert_eq!(
            exp_fees
                .per_certificate_fees
                .certificate_pool_registration
                .unwrap() as u64,
            u64::from(
                fees.per_certificate_fees
                    .certificate_pool_registration
                    .unwrap()
            )
        );
        assert_eq!(
            exp_fees
                .per_certificate_fees
                .certificate_stake_delegation
                .unwrap() as u64,
            u64::from(
                fees.per_certificate_fees
                    .certificate_stake_delegation
                    .unwrap()
            )
        );
        assert_eq!(
            exp_fees
                .per_vote_certificate_fees
                .certificate_vote_cast
                .unwrap() as u64,
            u64::from(
                fees.per_vote_certificate_fees
                    .certificate_vote_cast
                    .unwrap()
            )
        );
        assert_eq!(
            exp_fees
                .per_vote_certificate_fees
                .certificate_vote_plan
                .unwrap() as u64,
            u64::from(
                fees.per_vote_certificate_fees
                    .certificate_vote_plan
                    .unwrap()
            )
        );
    }

    fn decode_bech32_pk(bech32_public_key: &str) -> PublicKey<Ed25519> {
        let (_, data, _variant) = bech32::decode(bech32_public_key).unwrap();
        let dat = Vec::from_base32(&data).unwrap();
        let pk = PublicKey::<Ed25519>::from_binary(&dat).unwrap();
        pk
    }
}
