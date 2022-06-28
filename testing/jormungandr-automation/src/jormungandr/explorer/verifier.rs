use chain_impl_mockchain::fee::LinearFee;

use super::data::{
    settings::SettingsSettingsFees, transaction_by_id::TransactionByIdTransactionCertificate,
};

pub struct ExplorerVerifier;

impl ExplorerVerifier {
    pub fn assert_transaction(){

    }

    pub fn assert_transaction_certificate(){
        Self::assert_transaction();


    }

    pub fn epoch_stability_depth(depth: u32, exp_depth: i64){
        assert_eq!(depth as u64 ,exp_depth as u64);
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
