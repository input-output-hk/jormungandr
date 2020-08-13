use iapyx::cli::args::interactive::IapyxInteractiveCommandExec;
use iapyx::cli::args::interactive::{UserInteractionContoller, WalletState};
use jortestkit::console::UserInteraction;

pub fn main() {
    let user_interaction = UserInteraction::new(
        "iapyx".to_string(),
        "wallet interactive console".to_string(),
        "type command:".to_string(),
        "exit".to_string(),
        ">".to_string(),
        vec![
            "You can control each aspect of wallet:".to_string(),
            "- recover from mnemonic,".to_string(),
            "- vote,".to_string(),
            "- retrieve blockchain data,".to_string(),
            "- generate new wallet".to_string(),
        ],
    );

    user_interaction
        .interact(&mut IapyxInteractiveCommandExec {
            controller: UserInteractionContoller {
                state: WalletState::New,
                controller: None,
                backend_address: "127.0.0.1:8000".to_string(),
                settings: Default::default(),
            },
        })
        .unwrap();
}
