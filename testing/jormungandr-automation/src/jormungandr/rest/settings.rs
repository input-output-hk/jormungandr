use reqwest::Certificate;

#[derive(Debug, Clone, Default)]
pub struct RestSettings {
    pub enable_debug: bool,
    pub use_https: bool,
    pub certificate: Option<Certificate>,
    pub cors: Option<String>,
}

impl RestSettings {
    pub fn new_use_https() -> Self {
        RestSettings {
            enable_debug: false,
            use_https: true,
            certificate: None,
            cors: None,
        }
    }
}
