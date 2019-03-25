mod message;
mod node;
mod tip;
mod utxo;

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum V0 {
    /// Message sending
    Message(message::Message),
    /// Node information
    Node(node::Node),
    /// Blockchain tip information
    Tip(tip::Tip),
    /// UTXO information
    Utxo(utxo::Utxo),
}

impl V0 {
    pub fn exec(self) {
        match self {
            V0::Node(node) => node.exec(),
            V0::Utxo(utxo) => utxo.exec(),
            V0::Message(message) => message.exec(),
            V0::Tip(tip) => tip.exec(),
        }
    }
}
