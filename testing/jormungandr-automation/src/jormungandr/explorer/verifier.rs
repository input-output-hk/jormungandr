use super::data::{
    settings::SettingsSettingsFees,
    transaction_by_id::{
        TransactionByIdTransaction, TransactionByIdTransactionCertificate,
        TransactionByIdTransactionCertificateOnPoolRegistration,
    },
};
use chain_impl_mockchain::{
    certificate::PoolRegistration, fee::LinearFee, fragment::Fragment, transaction::Transaction,
};

pub struct ExplorerVerifier;

impl ExplorerVerifier {
    pub fn assert_transaction() {}

    pub fn assert_transaction_certificate(
        frag_transaction: Fragment,
        exp_transaction: TransactionByIdTransaction,
    ) {
        Self::assert_transaction();
        let exp_certificate = exp_transaction.certificate.unwrap();

        match exp_certificate {
            TransactionByIdTransactionCertificate::StakeDelegation(_) => todo!(),
            TransactionByIdTransactionCertificate::OwnerStakeDelegation(_) => todo!(),
            TransactionByIdTransactionCertificate::PoolRegistration(exp_cert) => {
                if let Fragment::PoolRegistration(frag_cert) = frag_transaction {
                    Self::assert_pool_registration(frag_cert, exp_cert);
                } else {
                    //TODO proper error
                    println!("ERROR: exp_fragment different from what has been sent");
                }
            }
            TransactionByIdTransactionCertificate::PoolRetirement(_) => todo!(),
            TransactionByIdTransactionCertificate::PoolUpdate(_) => todo!(),
            TransactionByIdTransactionCertificate::VotePlan(_) => todo!(),
            TransactionByIdTransactionCertificate::VoteCast => todo!(),
            TransactionByIdTransactionCertificate::VoteTally => todo!(),
            TransactionByIdTransactionCertificate::UpdateProposal => todo!(),
            TransactionByIdTransactionCertificate::UpdateVote => todo!(),
            TransactionByIdTransactionCertificate::MintToken => todo!(),
            TransactionByIdTransactionCertificate::EvmMapping => todo!(),
        }
    }

    fn assert_pool_registration(
        frag_cert: Transaction<PoolRegistration>,
        exp_cert: TransactionByIdTransactionCertificateOnPoolRegistration,
    ) {
        let pool_cert = frag_cert.as_slice().payload().into_payload();

        assert_eq!(pool_cert.to_id().to_string(), exp_cert.pool.id);
        assert!(exp_cert.pool.retirement.is_none());
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
}
