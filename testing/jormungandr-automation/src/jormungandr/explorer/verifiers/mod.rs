pub mod all_blocks_verifier;
pub mod block_by_id_verifier;
pub mod blocks_by_chain_length_verifier;
pub mod last_block_verifier;
pub mod transaction_by_id_verifier;

use crate::jormungandr::explorer::data::settings::SettingsSettingsFees;
use bech32::FromBase32;
use chain_core::{packer::Codec, property::Deserialize};
use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::{block::Block, fee::LinearFee};
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

    pub fn decode_bech32_pk(bech32_public_key: &str) -> PublicKey<Ed25519> {
        let (_, data, _variant) = bech32::decode(bech32_public_key).unwrap();
        let dat = Vec::from_base32(&data).unwrap();
        PublicKey::<Ed25519>::from_binary(&dat).unwrap()
    }

    pub fn decode_block(encoded_block: String) -> Block {
        let bytes_block = hex::decode(encoded_block.trim()).unwrap();
        let reader = std::io::Cursor::new(&bytes_block);
        Block::deserialize(&mut Codec::new(reader)).unwrap()
    }
}
