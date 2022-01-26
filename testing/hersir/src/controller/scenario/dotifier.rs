use crate::style;
use jormungandr_testing_utils::testing::network::Settings;
use jormungandr_testing_utils::testing::network::{NodeSetting, WalletTemplate, WalletType};
use std::io::Write;

pub struct Dotifier;

impl Dotifier {
    pub(crate) fn dottify<W: Write>(&self, settings: &Settings, mut w: W) -> std::io::Result<()> {
        writeln!(&mut w, r"digraph protocol {{")?;

        writeln!(
            &mut w,
            r###"  subgraph nodes {{
    node [ style = filled; color = lightgrey ];
"###
        )?;
        for node in settings.nodes.values() {
            let label = self.dot_node_label(node);
            writeln!(&mut w, "    {}", &label)?;

            for trusted_peer in node.node_topology.trusted_peers.iter() {
                let trusted_peer = settings.nodes.get(trusted_peer).unwrap();
                writeln!(
                    &mut w,
                    "    {} -> {} [ label = \"trusts\" ; color = blue ]",
                    &label,
                    self.dot_node_label(trusted_peer)
                )?;
            }
        }
        writeln!(&mut w, "  }}")?;

        for wallet in settings.wallets.values() {
            let template = wallet.template();
            let label = self.dot_wallet_label(template);
            writeln!(&mut w, "  {}", &label)?;

            if let Some(node) = template.delegate() {
                let trusted_peer = settings.nodes.get(node).unwrap();
                writeln!(
                    &mut w,
                    "  {} -> {} [ label = \"delegates\"; style = dashed ; color = red ]",
                    &label,
                    self.dot_node_label(trusted_peer)
                )?;
            }
        }

        writeln!(&mut w, "}}")?;
        Ok(())
    }

    pub(crate) fn dot_wallet_label(&self, wallet: &WalletTemplate) -> String {
        let t: crate::style::icons::Icon = if *wallet.wallet_type() == WalletType::Account {
            *crate::style::icons::account
        } else {
            *crate::style::icons::wallet
        };

        format!("\"{}{}\\nfunds = {}\"", &wallet.alias(), t, wallet.value())
    }

    pub(crate) fn dot_node_label(&self, node_settings: &NodeSetting) -> String {
        let bft = if let Some(_bft) = &node_settings.secret.bft {
            "[b]"
        } else {
            ""
        };

        let genesis = if let Some(_genesis) = &node_settings.secret.genesis {
            "[g]"
        } else {
            ""
        };
        format!(
            "\"{}{}{}{}\"",
            &node_settings.alias,
            *style::icons::jormungandr,
            bft,
            genesis
        )
    }
}
