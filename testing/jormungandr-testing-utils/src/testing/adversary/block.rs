//! Functionality for building invalid blocks and timing their transmission.
use crate::testing::startup;
use chain_crypto::Ed25519;
use chain_impl_mockchain::{
    block::{builder, Block, BlockDate, BlockVersion, Contents, Header},
    chaintypes::ConsensusVersion,
    key::BftLeaderId,
    testing::{data::StakePool, TestGen},
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

pub struct BlockBuilder {
    block_date: BlockDate,
    consensus_protocol: ConsensusVersion,
    contents: Option<Contents>,
    key_pair: Option<KeyPair<Ed25519>>,
    invalid_signature: bool,
    parent_block_header: Header,
    stake_pool: Option<StakePool>,
}

impl BlockBuilder {
    pub fn bft(block_date: BlockDate, parent_block_header: Header) -> Self {
        Self {
            block_date,
            consensus_protocol: ConsensusVersion::Bft,
            contents: None,
            key_pair: None,
            invalid_signature: false,
            parent_block_header,
            stake_pool: None,
        }
    }

    pub fn genesis_praos(block_date: BlockDate, parent_block_header: Header) -> Self {
        Self {
            block_date,
            consensus_protocol: ConsensusVersion::GenesisPraos,
            contents: None,
            key_pair: None,
            invalid_signature: false,
            parent_block_header,
            stake_pool: None,
        }
    }

    pub fn contents(self, contents: Contents) -> Self {
        Self {
            contents: Some(contents),
            ..self
        }
    }

    pub fn key_pair(self, key_pair: KeyPair<Ed25519>) -> Self {
        Self {
            key_pair: Some(key_pair),
            ..self
        }
    }

    pub fn invalid_signature(self) -> Self {
        Self {
            invalid_signature: true,
            ..self
        }
    }

    pub fn stake_pool(self, stake_pool: StakePool) -> Self {
        Self {
            stake_pool: Some(stake_pool),
            ..self
        }
    }

    pub fn build(self) -> Block {
        let Self {
            block_date,
            consensus_protocol,
            contents,
            key_pair,
            invalid_signature,
            parent_block_header,
            stake_pool,
        } = self;

        let contents = contents.unwrap_or_else(Contents::empty);

        builder(BlockVersion::Ed25519Signed, contents, |hdr_builder| {
            let builder = hdr_builder
                .set_parent(
                    &parent_block_header.id(),
                    parent_block_header.chain_length(),
                )
                .set_date(block_date);

            let header = match consensus_protocol {
                ConsensusVersion::Bft => {
                    let key_pair = key_pair.unwrap_or_else(startup::create_new_key_pair);

                    let bft_builder = builder.into_bft_builder().unwrap();

                    if invalid_signature {
                        bft_builder
                            .set_consensus_data(&BftLeaderId::from(
                                key_pair.identifier().into_public_key(),
                            ))
                            .set_signature(
                                key_pair
                                    .signing_key()
                                    .into_secret_key()
                                    .sign_slice(&[42u8])
                                    .into(),
                            )
                            .generalize()
                    } else {
                        bft_builder
                            .sign_using(&key_pair.signing_key().into_secret_key())
                            .generalize()
                    }
                }
                ConsensusVersion::GenesisPraos => {
                    let stake_pool = stake_pool.unwrap_or_else(TestGen::stake_pool);

                    let gp_builder = builder
                        .into_genesis_praos_builder()
                        .unwrap()
                        .set_consensus_data(&stake_pool.id(), &TestGen::vrf_proof(&stake_pool));

                    if invalid_signature {
                        gp_builder
                            .set_signature(
                                stake_pool.kes().private_key().sign_slice(&[42_u8]).into(),
                            )
                            .generalize()
                    } else {
                        gp_builder
                            .sign_using(stake_pool.kes().private_key())
                            .generalize()
                    }
                }
            };

            Ok::<_, ()>(header)
        })
        .unwrap()
    }
}
