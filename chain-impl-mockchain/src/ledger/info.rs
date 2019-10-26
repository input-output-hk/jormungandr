use super::ledger::Ledger;
use crate::value::Value;

impl Ledger {
    pub fn stats(&self) -> Vec<String> {
        let Ledger {
            utxos,
            oldutxos,
            accounts,
            settings: _,
            updates: _,
            multisig,
            delegation: _,
            static_params: _,
            date: _,
            chain_length: _,
            era: _,
            pots: _,
        } = self;

        vec![
            format!("utxos   : #{} Total={:?}", utxos.iter().count(), Value::sum(utxos.iter().map(|x| x.output.value))),
            format!("oldutxos: #{} Total={:?}", oldutxos.iter().count(), Value::sum(oldutxos.iter().map(|x| x.output.value))),
            format!("accounts: #{} Total={:?}", accounts.iter().count(), Value::sum(accounts.iter().map(|x| x.1.value))),
            format!("multisig: #{} Total={:?}", multisig.iter_accounts().count(), Value::sum(multisig.iter_accounts().map(|x| x.1.value))),
        ]
    }

    pub fn info_eq(&self, other: &Self) -> Vec<String> {
        let Ledger {
            utxos: utxos1,
            oldutxos: oldutxos1,
            accounts: accounts1,
            settings: settings1,
            updates: updates1,
            multisig: multisig1,
            delegation: delegation1,
            static_params: static_params1,
            date: date1,
            chain_length: chain_length1,
            era: era1,
            pots: pots1,
        } = self;

        let Ledger {
            utxos: utxos2,
            oldutxos: oldutxos2,
            accounts: accounts2,
            settings: settings2,
            updates: updates2,
            multisig: multisig2,
            delegation: delegation2,
            static_params: static_params2,
            date: date2,
            chain_length: chain_length2,
            era: era2,
            pots: pots2,
        } = other;

        vec![
            format!("utxos-same: {}", utxos1 == utxos2),
            format!("oldutxos-same: {}", oldutxos1 == oldutxos2),
            format!("accounts-same: {}", accounts1 == accounts2),
            format!("multisig-same: {}", multisig1 == multisig2),
            format!("settings-same: {}", settings1 == settings2),
            format!("delegation-same: {}", delegation1 == delegation2),
            format!("static_params-same: {}", static_params1 == static_params2),
            format!("updates-same: {}", updates1 == updates2),
            format!("chain-length: {}", chain_length1 == chain_length2),
            format!("date-same: {}", date1 == date2),
            format!("era-same: {}", era1 == era2),
            format!("pots-same: {}", pots1 == pots2),
        ]
    }
}
