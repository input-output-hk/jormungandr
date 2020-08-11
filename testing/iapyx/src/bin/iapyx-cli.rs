use iapyx::cli::args::interactive::UserInteraction;

pub fn main() {
    let mut user_interaction = UserInteraction::default();
    user_interaction.interact().unwrap();
}
