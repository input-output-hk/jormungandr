mod node;

use self::node::Node;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum V0 {
    /// Node information
    Node(Node),
}

impl V0 {
    pub fn exec(self) {
        match self {
            V0::Node(node) => node.exec(),
        }
    }
}
