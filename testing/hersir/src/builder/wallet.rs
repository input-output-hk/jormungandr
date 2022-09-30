use crate::{
    builder::{settings::wallet::Error, Wallet},
    config::WalletTemplate,
};
use chain_impl_mockchain::tokens::minting_policy::MintingPolicy;
use jormungandr_lib::interfaces::{Destination, Initial, InitialToken, TokenIdentifier};
use std::collections::hash_map::Iter;

pub fn generate(wallet_templates: &[WalletTemplate]) -> Result<(Vec<Wallet>, Vec<Initial>), Error> {
    let mut wallets: Vec<Wallet> = Vec::new();
    let mut initials = Vec::new();

    for wallet_template in wallet_templates {
        // TODO: check the wallet does not already exist ?
        let wallet: Wallet = wallet_template.clone().into();

        // TODO add support for sharing fragment with multiple utxos
        let initial_fragment = Initial::Fund(vec![wallet.to_initial_fund()?]);

        initials.push(initial_fragment);
        initials.extend(populate_tokens(
            wallet.address()?,
            wallet.template().tokens().iter(),
        ));
        wallets.push(wallet);
    }

    Ok((wallets, initials))
}

fn populate_tokens(
    address: jormungandr_lib::interfaces::Address,
    iterator: Iter<TokenIdentifier, u64>,
) -> Vec<Initial> {
    iterator
        .map(|(id, value)| {
            Initial::Token(InitialToken {
                token_id: id.clone(),
                // TODO: there are no policies now, but this will need to be changed later
                policy: MintingPolicy::new().into(),
                to: vec![Destination {
                    address: address.clone(),
                    value: (*value).into(),
                }],
            })
        })
        .collect()
}
