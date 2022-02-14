use crate::cli::config::WalletState;
use jormungandr_automation::jormungandr::MemPoolCheck;
use jormungandr_lib::crypto::account::SigningKey;
use jormungandr_lib::interfaces::Address;
use crate::cli::config::ConfigManager;
use crate::cli::config::SecretKey;
use crate::FragmentSender;
use crate::FragmentVerifier;
use super::WalletController;
use super::Error;
use crate::Wallet;
use std::time::Duration;
use chain_impl_mockchain::accounting::account::spending::SpendingCounterIncreasing;
use jormungandr_automation::jormungandr::RemoteJormungandrBuilder;
use jormungandr_automation::jormungandr::JormungandrRest;
use chain_impl_mockchain::fragment::FragmentId;
use cocoon::Cocoon;
use jormungandr_lib::interfaces::AccountState;
use jormungandr_lib::interfaces::AccountVotes;
use jormungandr_lib::interfaces::FragmentLog;
use jormungandr_lib::interfaces::FragmentStatus;
use jormungandr_lib::interfaces::VotePlanId;
use std::collections::HashMap;

const SLOT_COUNT: u64 = 3;

pub struct CliController {
    pub(crate) wallets: WalletController,
    pub(crate) client: JormungandrRest,
}

impl CliController {
    pub fn new() -> Result<Self, Error> {
        let wallets = WalletController::new(env!("CARGO_PKG_NAME"))?;

        Ok(Self {
            client: wallets.connection().into(),
            wallets,
        })
    }

    pub fn wallets(&self) -> &WalletController {
        &self.wallets
    }

    pub fn wallets_mut(&mut self) -> &mut WalletController {
        &mut self.wallets
    }

    pub fn new_from_client(client: JormungandrRest,config_manager: ConfigManager)-> Result<Self, Error> {
        Ok(Self {
            client,
            wallets: WalletController::new_from_manager(config_manager)?,
        })
    }

    pub fn check_connection(&self) -> Result<(), Error> {
        self.client
            .settings()
            .map(|_| {
                println!("Connection succesfull.");
            })
            .map_err(|e| {
                eprintln!("Connection unsuccesfull.");
                Error::Connection(self.wallets.connection().address, e)
            })
    }

    pub fn refresh_state(&mut self) -> Result<(), Error> {
        let mut wallet = self.wallets.wallet_mut()?;
        let new_state = self.client.account_state_by_pk(&wallet.pk_bech32())?;
        wallet.spending_counters = new_state.counters();
        wallet.value = (*new_state.value()).into();
        Ok(())
    }

    pub fn account_state(&self) -> Result<AccountState, Error> {
        self.client
            .account_state_by_pk(&self.wallets.wallet()?.pk_bech32())
            .map_err(Into::into)
    }

    fn thor_wallet(&self, password: &str) -> Result<Wallet, Error> {
        let template = self.wallets.wallet()?;

        Ok(Wallet::Account(crate::wallet::account::Wallet::from_secret_key(
            SigningKey::from_bech32_str(&self.secret_key_for_wallet_state(password,&template)?.secret)?,
            SpendingCounterIncreasing::new_from_counters(template.spending_counters.iter().cloned().map(Into::into).collect()).ok_or(Error::SpendingCounter)?,
            template.discrimination()
        )))
    }

    pub fn secret_key(&self,password: &str) -> Result<SecretKey,Error> {
        let template = self.wallets.wallet()?;
        self.secret_key_for_wallet_state(password,&template)
    }

    pub fn secret_key_for_wallet_state(&self,password: &str, wallet_state: &WalletState) -> Result<SecretKey,Error> {
        let contents = std::fs::read(&wallet_state.secret_file)?;
        let cocoon = Cocoon::new(password.as_bytes());

        let unwrapped: Vec<u8> = cocoon.unwrap(&contents)?;
        bincode::deserialize(&unwrapped[..]).map_err(Into::into)
    }

    pub fn transaction(&mut self,password: &str, wait_for_transaction: bool, target: Address, ada: u64) -> Result<MemPoolCheck, Error> {
        let mut thor_wallet = self.thor_wallet(password)?;
        let settings = self.client.settings()?;
        let node = RemoteJormungandrBuilder::new("dummy".to_string()).with_rest(self.client.address().parse().unwrap()).build();
        let check = FragmentSender::from(&settings).send_transaction_to_address(&mut thor_wallet, target, &node, ada.into())?;
        if wait_for_transaction {
            FragmentVerifier::wait_fragment(Duration::from_secs(settings.slot_duration * SLOT_COUNT),
                check.clone(),
                Default::default(),
                &node
            )?;
            self.wallets.wallet_mut()?.spending_counters = thor_wallet.spending_counter().ok_or(Error::SpendingCounter)?.get_valid_counters().into_iter().map(|x| x.into()).collect();
        }
        Ok(check)
    }

    pub fn confirm_tx(&mut self) -> Result<(),Error> {
        self.wallets.confirm_txs(self.statuses()?)
    }

    pub fn clear_txs(&mut self) -> Result<(),Error> {
        self.wallets.clear_txs()
    }

    pub fn statuses(&self) -> Result<HashMap<FragmentId, FragmentStatus>, Error> {
        if self.wallets.wallet()?.pending_tx.is_empty() {
            return Ok(HashMap::new());
        }
        Ok(self.client
            .fragments_statuses(
                self.wallets
                    .wallet()?
                    .pending_tx
                    .into_iter()
                    .map(|x| x.to_string())
                    .collect(),
            )?.into_iter().map(|(id,status)| (id.parse().unwrap(),status)).collect())
    }

    pub fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, Error> {
        self.client.fragment_logs().map_err(Into::into)
    }

    pub fn vote_plan_history(&self, vote_plan_id: VotePlanId) -> Result<Option<Vec<u8>>, Error> {
        self.client
            .account_votes_with_plan_id(vote_plan_id,self.wallets.wallet()?.address()?.into())
            .map_err(Into::into)
    }

    pub fn votes_history(&self) -> Result<Option<Vec<AccountVotes>>, Error> {
        self.client
            .account_votes(self.wallets.wallet()?.address()?.into())
            .map_err(Into::into)
    }

    pub fn save_config(&self) -> Result<(), Error> {
        self.wallets.save_config().map_err(Into::into)
    }
}
