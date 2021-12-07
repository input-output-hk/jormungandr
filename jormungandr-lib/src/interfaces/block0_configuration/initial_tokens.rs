use crate::{
    crypto::account::Identifier,
    interfaces::{
        mint_token::{MintingPolicy, TokenIdentifier},
        Value,
    },
};
use chain_impl_mockchain::{
    certificate, fragment::Fragment, tokens::identifier, transaction::Transaction,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct InitialTokens {
    token_id: TokenIdentifier,
    // TODO add a serde implementation for the MintingPolicy when it will be well specified
    #[serde(skip)]
    policy: MintingPolicy,
    to: Vec<(Identifier, Value)>,
}

pub fn initial_tokens_from_messages(message: &Fragment) -> Option<InitialTokens> {
    match message {
        Fragment::MintToken(tx) => {
            let tx = tx.as_slice();
            let mint_token = tx.payload().into_payload();
            let token_id = identifier::TokenIdentifier {
                token_name: mint_token.name,
                policy_hash: mint_token.policy.hash(),
            };
            Some(InitialTokens {
                token_id: token_id.into(),
                policy: mint_token.policy.into(),
                to: vec![(mint_token.to.into(), mint_token.value.into())],
            })
        }
        _ => None,
    }
}

impl<'a> From<&'a InitialTokens> for Vec<Fragment> {
    fn from(initial: &'a InitialTokens) -> Vec<Fragment> {
        pack_in_fragments(&initial.token_id, &initial.policy, &initial.to)
    }
}

fn pack_in_fragments(
    token_id: &TokenIdentifier,
    policy: &MintingPolicy,
    to: &Vec<(Identifier, Value)>,
) -> Vec<Fragment> {
    let token_id: identifier::TokenIdentifier = token_id.clone().into();
    to.iter()
        .map(|(account, value)| {
            let mint_token = certificate::MintToken {
                name: token_id.token_name.clone(),
                policy: policy.clone().into(),
                to: (*account).to_inner(),
                value: (*value).into(),
            };
            Fragment::MintToken(Transaction::block0_payload(&mint_token, &()))
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::Arbitrary;

    impl Arbitrary for InitialTokens {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            Self {
                token_id: Arbitrary::arbitrary(g),
                policy: Arbitrary::arbitrary(g),
                to: Arbitrary::arbitrary(g),
            }
        }
    }
}
