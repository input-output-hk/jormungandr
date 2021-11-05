//! Functionality for building invalid blocks and timing their transmission.
use chain_crypto::Ed25519;
use chain_impl_mockchain::{
    block::{builder, Block, BlockDate, BlockVersion, Contents, Header},
    chaintypes::ConsensusVersion,
    key::BftLeaderId,
};
use jormungandr_lib::crypto::key::KeyPair;

pub fn block_with_incorrect_signature(
    key_pair: &KeyPair<Ed25519>,
    parent_header: &Header,
    block_date: BlockDate,
    consensus_protocol: ConsensusVersion,
) -> Block {
    builder(
        BlockVersion::Ed25519Signed,
        Contents::empty(),
        |hdr_builder| {
            let builder = hdr_builder
                .set_parent(&parent_header.id(), parent_header.chain_length())
                .set_date(block_date);

            Ok::<_, ()>(match consensus_protocol {
                ConsensusVersion::Bft => builder
                    .into_bft_builder()
                    .unwrap()
                    .set_consensus_data(&BftLeaderId::from(key_pair.identifier().into_public_key()))
                    .set_signature(
                        key_pair
                            .signing_key()
                            .into_secret_key()
                            .sign_slice(&[42u8])
                            .into(),
                    )
                    .generalize(),
                ConsensusVersion::GenesisPraos => todo!(),
            })
        },
    )
    .unwrap()
}
