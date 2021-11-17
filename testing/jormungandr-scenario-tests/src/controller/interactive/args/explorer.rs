use crate::controller::UserInteractionController;
use crate::test::Result;
use jortestkit::prelude::InteractiveCommandError;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Explorer {
    /// Sends transaction
    Tip(ExplorerTip),
}

impl Explorer {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        match self {
            Explorer::Tip(tip) => tip.exec(controller),
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct ExplorerTip {
    #[structopt(short = "a", long = "alias")]
    pub alias: String,
}

impl ExplorerTip {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        let node = controller
            .nodes()
            .iter()
            .find(|x| *x.alias() == self.alias)
            .ok_or_else(|| {
                InteractiveCommandError::UserError(format!("Node '{}' not found", self.alias))
            })?;
        println!("{:#?}", node.explorer().last_block()?);
        Ok(())
    }
}
