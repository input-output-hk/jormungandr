use super::data::{
    settings::SettingsSettingsFees,
    transaction_by_id_certificates::{
        TransactionByIdCertificatesTransaction, TransactionByIdCertificatesTransactionCertificate,
        TransactionByIdCertificatesTransactionCertificateOnOwnerStakeDelegation,
        TransactionByIdCertificatesTransactionCertificateOnPoolRegistration,
        TransactionByIdCertificatesTransactionCertificateOnPoolRetirement,
        TransactionByIdCertificatesTransactionCertificateOnPoolUpdate,
        TransactionByIdCertificatesTransactionCertificateOnStakeDelegation,
    },
};
use bech32::FromBase32;
use chain_addr::AddressReadable;
use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::{
    account::DelegationType,
    certificate::{
        OwnerStakeDelegation, PoolRegistration, PoolRetirement, PoolUpdate, StakeDelegation,
    },
    fee::LinearFee,
    fragment::Fragment,
    transaction::{AccountIdentifier, InputEnum, Transaction},
};
use std::num::NonZeroU64;
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
    pub fn assert_transaction_certificates(
        fragment: Fragment,
        explorer_transaction: TransactionByIdCertificatesTransaction,
    ) -> Result<(), VerifierError> {
        if explorer_transaction.certificate.is_none() {
            if let Fragment::Transaction(fragment_transaction) = fragment {
                Self::assert_transaction_params(fragment_transaction, explorer_transaction)
                    .unwrap();
                Ok(())
            } else {
                Err(VerifierError::InvalidCertificate {
                    received: "Transaction".to_string(),
                })
            }
        } else {
            let explorer_certificate = explorer_transaction.certificate.as_ref().unwrap();
            match explorer_certificate {
                TransactionByIdCertificatesTransactionCertificate::StakeDelegation(
                    explorer_cert,
                ) => {
                    if let Fragment::StakeDelegation(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_stake_delegation(fragment_cert, explorer_cert.clone())
                            .unwrap();
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "StakeDelegation".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::OwnerStakeDelegation(
                    explorer_cert,
                ) => {
                    if let Fragment::OwnerStakeDelegation(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_owner_delegation(fragment_cert, explorer_cert.clone())
                            .unwrap();
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "OwnerStakeDelegation".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::PoolRegistration(
                    explorer_cert,
                ) => {
                    if let Fragment::PoolRegistration(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_pool_registration(fragment_cert, explorer_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "PoolRegistration".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::PoolRetirement(
                    explorer_cert,
                ) => {
                    if let Fragment::PoolRetirement(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_pool_retirement(fragment_cert, explorer_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "PoolRetirement".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::PoolUpdate(explorer_cert) => {
                    if let Fragment::PoolUpdate(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_pool_update(fragment_cert, explorer_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "PoolUpdate".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::VotePlan(_) => todo!(),
                TransactionByIdCertificatesTransactionCertificate::VoteCast(_) => todo!(),
                TransactionByIdCertificatesTransactionCertificate::VoteTally(_) => todo!(),
                TransactionByIdCertificatesTransactionCertificate::UpdateProposal(_) => todo!(),
                TransactionByIdCertificatesTransactionCertificate::UpdateVote(_) => todo!(),
                TransactionByIdCertificatesTransactionCertificate::MintToken(_) => todo!(),
                TransactionByIdCertificatesTransactionCertificate::EvmMapping(_) => todo!(),
            }
        }
    }

    fn assert_transaction_params<P>(
        fragment_transaction: Transaction<P>,
        explorer_transaction: TransactionByIdCertificatesTransaction,
    ) -> Result<(), VerifierError> {
        assert_eq!(
            fragment_transaction.as_slice().nb_inputs(),
            explorer_transaction.inputs.len() as u8
        );

        if fragment_transaction.as_slice().nb_inputs() > 0 {
            let mut fragment_accounts = vec![];

            for fragment_input in fragment_transaction.as_slice().inputs().iter() {
                match fragment_input.to_enum() {
                    InputEnum::AccountInput(account_id, amount) => {
                        fragment_accounts.push((
                            account_id.to_single_account().unwrap().to_string(),
                            amount.to_string(),
                        ));
                        Ok(())
                    }
                    InputEnum::UtxoInput(_) => Err(VerifierError::Unimplemented),
                }
                .unwrap()
            }

            let mut explorer_accounts = vec![];

            for explorer_inputs in explorer_transaction.inputs.iter() {
                let adr =
                    AddressReadable::from_string_anyprefix(&explorer_inputs.address.id).unwrap();
                explorer_accounts.push((
                    adr.to_address().public_key().unwrap().to_string(),
                    explorer_inputs.amount.clone(),
                ));
            }

            let matching_inputs = fragment_accounts
                .iter()
                .zip(explorer_accounts.iter())
                .filter(|&(a, b)| a == b)
                .count();
            assert_eq!(matching_inputs, explorer_transaction.inputs.len());
        };

        assert_eq!(
            fragment_transaction.as_slice().nb_outputs(),
            explorer_transaction.outputs.len() as u8
        );

        if fragment_transaction.as_slice().nb_outputs() > 0 {
            let mut fragment_accounts = vec![];

            for fragment_output in fragment_transaction.as_slice().outputs().iter() {
                fragment_accounts.push((
                    fragment_output.address.public_key().unwrap().to_string(),
                    fragment_output.value.to_string(),
                ));
            }

            let mut explorer_accounts = vec![];

            for explorer_outputs in explorer_transaction.outputs.iter() {
                let adr =
                    AddressReadable::from_string_anyprefix(&explorer_outputs.address.id).unwrap();
                explorer_accounts.push((
                    adr.to_address().public_key().unwrap().to_string(),
                    explorer_outputs.amount.clone(),
                ));
            }

            let matching_outputs = fragment_accounts
                .iter()
                .zip(explorer_accounts.iter())
                .filter(|&(a, b)| a == b)
                .count();
            assert_eq!(matching_outputs, explorer_transaction.outputs.len());
        };
        Ok(())
    }

    fn assert_pool_registration(
        fragment_cert: Transaction<PoolRegistration>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnPoolRegistration,
    ) {
        let pool_cert = fragment_cert.as_slice().payload().into_payload();

        assert_eq!(pool_cert.to_id().to_string(), explorer_cert.pool.id);
        assert_eq!(
            u64::from(pool_cert.start_validity),
            explorer_cert.start_validity.parse::<u64>().unwrap()
        );
        if pool_cert.reward_account.is_some() {
            if let AccountIdentifier::Single(id) = pool_cert.reward_account.as_ref().unwrap() {
                assert_eq!(id.to_string(), explorer_cert.reward_account.unwrap().id);
            }
        }

        assert_eq!(
            pool_cert.rewards.ratio.numerator,
            explorer_cert
                .rewards
                .ratio
                .numerator
                .parse::<u64>()
                .unwrap()
        );
        assert_eq!(
            pool_cert.rewards.ratio.denominator,
            explorer_cert
                .rewards
                .ratio
                .denominator
                .parse::<NonZeroU64>()
                .unwrap()
        );
        if pool_cert.rewards.max_limit.is_some() {
            assert_eq!(
                pool_cert.rewards.max_limit.unwrap(),
                explorer_cert
                    .rewards
                    .max_limit
                    .unwrap()
                    .parse::<NonZeroU64>()
                    .unwrap()
            );
        }

        assert_eq!(
            pool_cert.management_threshold(),
            explorer_cert.management_threshold as u8
        );

        assert_eq!(pool_cert.owners.len(), explorer_cert.owners.len());

        let owners_matching = pool_cert
            .owners
            .iter()
            .zip(explorer_cert.owners.iter())
            .filter(|&(a, b)| *a == Self::decode_bech32_pk(b))
            .count();

        assert_eq!(pool_cert.owners.len(), owners_matching);

        assert_eq!(pool_cert.operators.len(), explorer_cert.operators.len());

        let operators_matching = pool_cert
            .operators
            .iter()
            .zip(explorer_cert.operators.iter())
            .filter(|&(a, b)| *a == Self::decode_bech32_pk(b))
            .count();

        assert_eq!(pool_cert.operators.len(), operators_matching);

        assert!(explorer_cert.pool.retirement.is_none());
    }

    fn assert_stake_delegation(
        fragment_cert: Transaction<StakeDelegation>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnStakeDelegation,
    ) -> Result<(), VerifierError> {
        let deleg_cert = fragment_cert.as_slice().payload().into_payload();
        let adr = AddressReadable::from_string_anyprefix(&explorer_cert.account.id).unwrap();
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
                assert_eq!(explorer_cert.pools.len(), 1);
                assert_eq!(pool_id.to_string(), explorer_cert.pools[0].id);
                Ok(())
            }
            DelegationType::Ratio(deleg) => {
                assert_eq!(explorer_cert.pools.len(), deleg.pools().len());
                let pools_matching = explorer_cert
                    .pools
                    .iter()
                    .zip(deleg.pools().iter())
                    .filter(|&(a, b)| a.id == b.0.to_string())
                    .count();
                assert_eq!(pools_matching, explorer_cert.pools.len());
                Ok(())
            }
        }
    }
    fn assert_owner_delegation(
        fragment_cert: Transaction<OwnerStakeDelegation>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnOwnerStakeDelegation,
    ) -> Result<(), VerifierError> {
        let owner_cert = fragment_cert.as_slice().payload().into_payload();

        match owner_cert.delegation {
            DelegationType::NonDelegated => Err(VerifierError::Unimplemented),
            DelegationType::Full(pool_id) => {
                assert_eq!(explorer_cert.pools.len(), 1);
                assert_eq!(pool_id.to_string(), explorer_cert.pools[0].id);
                Ok(())
            }
            DelegationType::Ratio(deleg) => {
                assert_eq!(explorer_cert.pools.len(), deleg.pools().len());
                let pools_matching = explorer_cert
                    .pools
                    .iter()
                    .zip(deleg.pools().iter())
                    .filter(|&(a, b)| a.id == b.0.to_string())
                    .count();
                assert_eq!(pools_matching, explorer_cert.pools.len());
                Ok(())
            }
        }
    }

    fn assert_pool_retirement(
        fragment_cert: Transaction<PoolRetirement>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnPoolRetirement,
    ) {
        let ret_cert = fragment_cert.as_slice().payload().into_payload();
        assert_eq!(ret_cert.pool_id.to_string(), explorer_cert.pool_id);
        assert_eq!(
            u64::from(ret_cert.retirement_time),
            explorer_cert.retirement_time.parse::<u64>().unwrap()
        );
    }

    fn assert_pool_update(
        fragment_cert: Transaction<PoolUpdate>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnPoolUpdate,
    ) {
        let update_cert = fragment_cert.as_slice().payload().into_payload();
        assert_eq!(update_cert.pool_id.to_string(), explorer_cert.pool_id);
        assert_eq!(
            u64::from(update_cert.new_pool_reg.start_validity),
            explorer_cert.start_validity.parse::<u64>().unwrap()
        );
    }

    pub fn assert_epoch_stability_depth(depth: u32, explorer_depth: i64) {
        assert_eq!(depth as u64, explorer_depth as u64);
    }

    pub fn assert_fees(fees: LinearFee, explorer_fees: SettingsSettingsFees) {
        assert_eq!(explorer_fees.certificate as u64, fees.certificate);
        assert_eq!(explorer_fees.coefficient as u64, fees.coefficient);
        assert_eq!(explorer_fees.constant as u64, fees.constant);
        assert_eq!(
            explorer_fees
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
            explorer_fees
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
            explorer_fees
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
            explorer_fees
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
            explorer_fees
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
        PublicKey::<Ed25519>::from_binary(&dat).unwrap()
    }
}
