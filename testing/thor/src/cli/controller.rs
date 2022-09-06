use super::{Error, WalletController};
use crate::{
    cli::{
        config::{ConfigManager, WalletState},
        Alias, Connection,
    },
    wallet::committee::decrypt_tally_with_member_keys,
    FragmentSender, FragmentVerifier, Wallet,
};
use bech32::{u5, FromBase32};
use chain_crypto::{Ed25519Extended, SecretKey};
use chain_impl_mockchain::{
    accounting::account::spending::SpendingCounterIncreasing, fragment::FragmentId,
};
use chain_vote::committee::MemberSecretKey;
use cocoon::Cocoon;
use jormungandr_automation::jormungandr::{
    JormungandrRest, MemPoolCheck, RemoteJormungandr, RemoteJormungandrBuilder,
};
use jormungandr_lib::{
    crypto::account::SigningKey,
    interfaces::{
        AccountState, AccountVotes, Address, FragmentLog, FragmentStatus, SettingsDto, VotePlan,
        VotePlanId,
    },
};
use std::{collections::HashMap, path::Path, str::FromStr, time::Duration};

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

    pub fn new_from_client(
        client: JormungandrRest,
        config_manager: ConfigManager,
    ) -> Result<Self, Error> {
        Ok(Self {
            client,
            wallets: WalletController::new_from_manager(config_manager)?,
        })
    }

    pub fn update_connection(&mut self, connection: Connection) {
        self.client = connection.clone().into();
        self.wallets_mut().config_mut().connection = connection;
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

    pub fn client(&self) -> &JormungandrRest {
        &self.client
    }

    pub fn account_state(&self) -> Result<AccountState, Error> {
        self.client
            .account_state_by_pk(&self.wallets.wallet()?.pk_bech32())
            .map_err(Into::into)
    }

    fn thor_wallet(&self, password: &str) -> Result<Wallet, Error> {
        let template = self.wallets.wallet()?;

        Ok(Wallet::Account(
            crate::wallet::account::Wallet::from_secret_key(
                self.secret_key_for_wallet_state(password, &template)?,
                SpendingCounterIncreasing::new_from_counters(
                    template
                        .spending_counters
                        .iter()
                        .cloned()
                        .map(Into::into)
                        .collect(),
                )
                .ok_or(Error::SpendingCounter)?,
                template.discrimination(),
            ),
        ))
    }

    pub fn secret_key(&self, password: &str) -> Result<SigningKey, Error> {
        let template = self.wallets.wallet()?;
        self.secret_key_for_wallet_state(password, &template)
    }

    pub fn secret_key_for_wallet_state(
        &self,
        password: &str,
        wallet_state: &WalletState,
    ) -> Result<SigningKey, Error> {
        let unwrapped = self.secret_key_bytes(password, &wallet_state.secret_file)?;
        let data_u5: Vec<u5> = unwrapped
            .iter()
            .map(|x| bech32::u5::try_from_u8(*x).unwrap())
            .collect();
        let secret: SecretKey<Ed25519Extended> =
            SecretKey::from_binary(&Vec::<u8>::from_base32(&data_u5)?)?;
        Ok(secret.into())
    }

    pub fn secret_key_bytes<P: AsRef<Path>>(
        &self,
        password: &str,
        secret_key: P,
    ) -> Result<Vec<u8>, Error> {
        let contents = std::fs::read(secret_key)?;
        let cocoon = Cocoon::new(password.as_bytes());
        cocoon.unwrap(&contents).map_err(Into::into)
    }

    pub fn send_private_vote_tally(
        &mut self,
        password: &str,
        wait_for_transaction: bool,
        vote_plan_id: String,
        member_key_alias: Alias,
    ) -> Result<MemPoolCheck, Error> {
        let mut thor_wallet = self.thor_wallet(password)?;
        let settings = self.client.settings()?;
        let node = RemoteJormungandrBuilder::new("dummy".to_string())
            .with_rest_client(self.client.clone())
            .build();
        let wallet = self.wallets.wallet()?;
        let member_key_file = wallet
            .committee_members_key
            .get(&member_key_alias)
            .ok_or(Error::UnknownMemberKeyAlias(member_key_alias))?;

        let member_secret_key = self.secret_member_key(password, &member_key_file)?;

        let vote_plan_status = node
            .rest()
            .vote_plan_statuses()?
            .iter()
            .find(|x| x.id.to_string() == vote_plan_id)
            .ok_or(Error::CannotFindVoteplan)?
            .clone();

        let check = FragmentSender::from(&settings).send_private_vote_tally(
            &mut thor_wallet,
            VotePlanId::from_str(&vote_plan_id)
                .map_err(|_| Error::InvalidVoteplanId)?
                .into(),
            decrypt_tally_with_member_keys(vec![member_secret_key], &vote_plan_status.into())?,
            &node,
        )?;
        self.post_fragment_send(check, wait_for_transaction, settings, &node, thor_wallet)
    }

    pub fn send_public_vote_tally(
        &mut self,
        password: &str,
        wait_for_transaction: bool,
        vote_plan_id: String,
    ) -> Result<MemPoolCheck, Error> {
        let mut thor_wallet = self.thor_wallet(password)?;
        let settings = self.client.settings()?;
        let node = RemoteJormungandrBuilder::new("dummy".to_string())
            .with_rest_client(self.client.clone())
            .build();

        let check = FragmentSender::from(&settings).send_public_vote_tally(
            &mut thor_wallet,
            VotePlanId::from_str(&vote_plan_id)
                .map_err(|_| Error::InvalidVoteplanId)?
                .into(),
            &node,
        )?;
        self.post_fragment_send(check, wait_for_transaction, settings, &node, thor_wallet)
    }

    pub fn transaction(
        &mut self,
        password: &str,
        wait_for_transaction: bool,
        target: Address,
        ada: u64,
    ) -> Result<MemPoolCheck, Error> {
        let mut thor_wallet = self.thor_wallet(password)?;
        let settings = self.client.settings()?;
        let node = RemoteJormungandrBuilder::new("dummy".to_string())
            .with_rest_client(self.client.clone())
            .build();
        let check = FragmentSender::from(&settings).send_transaction_to_address(
            &mut thor_wallet,
            target,
            &node,
            ada.into(),
        )?;
        self.post_fragment_send(check, wait_for_transaction, settings, &node, thor_wallet)
    }

    pub fn send_vote_plan(
        &mut self,
        password: &str,
        wait_for_transaction: bool,
        vote_plan: VotePlan,
    ) -> Result<MemPoolCheck, Error> {
        let mut thor_wallet = self.thor_wallet(password)?;
        let settings = self.client.settings()?;
        let node = RemoteJormungandrBuilder::new("dummy".to_string())
            .with_rest_client(self.client.clone())
            .build();
        let check = FragmentSender::from(&settings).send_vote_plan(
            &mut thor_wallet,
            &vote_plan.into(),
            &node,
        )?;
        self.post_fragment_send(check, wait_for_transaction, settings, &node, thor_wallet)
    }

    fn post_fragment_send(
        &mut self,
        check: MemPoolCheck,
        wait_for_transaction: bool,
        settings: SettingsDto,
        node: &RemoteJormungandr,
        thor_wallet: Wallet,
    ) -> Result<MemPoolCheck, Error> {
        if wait_for_transaction {
            FragmentVerifier::wait_fragment(
                Duration::from_secs(settings.slot_duration * SLOT_COUNT),
                check.clone(),
                Default::default(),
                node,
            )?;
            self.wallets.wallet_mut()?.spending_counters = thor_wallet
                .spending_counter()
                .ok_or(Error::SpendingCounter)?
                .get_valid_counters()
                .into_iter()
                .map(|x| x.into())
                .collect();
        }
        Ok(check)
    }

    pub fn confirm_tx(&mut self) -> Result<(), Error> {
        self.wallets.confirm_txs(self.statuses()?)
    }

    pub fn clear_txs(&mut self) -> Result<(), Error> {
        self.wallets.clear_txs()
    }

    pub fn statuses(&self) -> Result<HashMap<FragmentId, FragmentStatus>, Error> {
        if self.wallets.wallet()?.pending_tx.is_empty() {
            return Ok(HashMap::new());
        }
        Ok(self
            .client
            .fragments_statuses(
                self.wallets
                    .wallet()?
                    .pending_tx
                    .into_iter()
                    .map(|x| x.to_string())
                    .collect(),
            )?
            .into_iter()
            .map(|(id, status)| (id.parse().unwrap(), status))
            .collect())
    }

    pub fn fragment_logs(&self) -> Result<HashMap<FragmentId, FragmentLog>, Error> {
        self.client.fragment_logs().map_err(Into::into)
    }

    pub fn vote_plan_history(&self, vote_plan_id: VotePlanId) -> Result<Option<Vec<u8>>, Error> {
        self.client
            .account_votes_with_plan_id(vote_plan_id, self.wallets.wallet()?.address()?.into())
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
    fn secret_member_key<P: AsRef<Path>>(
        &self,
        password: &str,
        secret_file: P,
    ) -> Result<MemberSecretKey, Error> {
        let data_u5: Vec<u5> = self
            .secret_key_bytes(password, &secret_file)
            .unwrap()
            .iter()
            .map(|x| bech32::u5::try_from_u8(*x).unwrap())
            .collect();
        MemberSecretKey::from_bytes(&Vec::<u8>::from_base32(&data_u5)?)
            .ok_or(Error::CannotReadSecretKey)
    }
}
