use crate::blockchain::BlockchainR;
use actix_web::error::{Error, ErrorBadRequest, ErrorNotFound};
use actix_web::{App, Json, Path, Responder, State};
use chain_crypto::PublicKey;
use chain_impl_mockchain::account::{AccountAlg, Identifier};
use chain_impl_mockchain::accounting::account::AccountState;
use std::str::FromStr;

pub fn create_handler(
    blockchain: BlockchainR,
) -> impl Fn(&str) -> App<BlockchainR> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        App::with_state(blockchain.clone())
            .prefix(format!("{}/v0/account", prefix))
            .resource("/{account_id}", |r| r.get().with(handle_request))
    }
}

fn handle_request(
    blockchain: State<BlockchainR>,
    account_id_hex: Path<String>,
) -> Result<impl Responder, Error> {
    let account_id = parse_account_id(&account_id_hex)?;
    let blockchain = blockchain.lock_read();
    let state = blockchain
        .multiverse
        .get(&blockchain.get_tip().unwrap())
        .unwrap()
        .accounts()
        .get_state(&account_id)
        .map_err(|e| ErrorNotFound(e))?;
    Ok(Json(AccountDto::from(state)))
}

fn parse_account_id(id_hex: &str) -> Result<Identifier, Error> {
    PublicKey::<AccountAlg>::from_str(id_hex)
        .map(Into::into)
        .map_err(|e| ErrorBadRequest(e))
}

#[derive(Serialize)]
struct AccountDto {
    value: u64,
    counter: u32,
}

impl<'a> From<&'a AccountState> for AccountDto {
    fn from(state: &'a AccountState) -> Self {
        AccountDto {
            value: state.get_value().as_ref().clone(),
            counter: state.get_counter().into(),
        }
    }
}
