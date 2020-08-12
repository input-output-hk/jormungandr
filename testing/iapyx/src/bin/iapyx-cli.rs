use iapyx::cli::args::interactive::UserInteraction;

pub fn main() {
   
    let user_interaction = UserInteraction::new(
        "iapyx-cli".to_string(),
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
   
    user_integration.interact(&mut JormungandrInteractiveCommandExec{
        controller: UserInteractionController::new(&mut controller)
    }).unwrap();    
}
