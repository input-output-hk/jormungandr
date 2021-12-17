use chain_crypto::Ed25519;
use chain_impl_mockchain::{
    block::{builder, Block, BlockDate, BlockVersion, Contents, Header},
    chaintypes::ConsensusVersion,
    header::HeaderBuilder,
    key::BftLeaderId,
    testing::{data::StakePool, TestGen},
};
use jormungandr_lib::crypto::key::SigningKey;
use jormungandr_testing_utils::testing::startup;

pub struct BlockBuilder {
    block_date: BlockDate,
    consensus_protocol: ConsensusVersion,
    contents: Option<Contents>,
    signing_key: Option<SigningKey<Ed25519>>,
    invalid_hash: bool,
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
            signing_key: None,
            invalid_hash: false,
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
            signing_key: None,
            invalid_hash: false,
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

    pub fn signing_key(self, signing_key: SigningKey<Ed25519>) -> Self {
        Self {
            signing_key: Some(signing_key),
            ..self
        }
    }

    pub fn invalid_hash(self) -> Self {
        Self {
            invalid_hash: true,
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
            signing_key,
            invalid_hash,
            invalid_signature,
            parent_block_header,
            stake_pool,
        } = self;

        let contents = contents.unwrap_or_else(Contents::empty);

        let (mut content_hash, content_size) = contents.compute_hash_size();

        if invalid_hash {
            content_hash = TestGen::hash();
        }

        builder(BlockVersion::Ed25519Signed, contents, |_| {
            let builder =
                HeaderBuilder::new_raw(BlockVersion::Ed25519Signed, &content_hash, content_size)
                    .set_parent(
                        &parent_block_header.id(),
                        parent_block_header.chain_length().increase(),
                    )
                    .set_date(block_date);

            let header = match consensus_protocol {
                ConsensusVersion::Bft => {
                    let signing_key =
                        signing_key.unwrap_or_else(|| startup::create_new_key_pair().signing_key());

                    let bft_builder = builder.into_bft_builder().unwrap();

                    if invalid_signature {
                        bft_builder
                            .set_consensus_data(&BftLeaderId::from(
                                signing_key.identifier().into_public_key(),
                            ))
                            .set_signature(signing_key.into_secret_key().sign_slice(&[42u8]).into())
                            .generalize()
                    } else {
                        bft_builder
                            .sign_using(&signing_key.into_secret_key())
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
