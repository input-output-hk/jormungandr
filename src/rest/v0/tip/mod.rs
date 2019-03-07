use actix_web::{App, Responder, State};
use chain_core::property::Settings as SettingsTrait;
use chain_impl_mockchain::setting::Settings;
use std::sync::{Arc, RwLock};

type SettingsR = Arc<RwLock<Settings>>;

pub fn create_handler(
    settings: SettingsR,
) -> impl Fn(&str) -> App<SettingsR> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        App::with_state(settings.clone())
            .prefix(format!("{}/v0/tip", prefix))
            .resource("", |r| r.get().with(handle_request))
    }
}

fn handle_request(settings: State<SettingsR>) -> impl Responder {
    settings.read().unwrap().tip().to_string()
}
