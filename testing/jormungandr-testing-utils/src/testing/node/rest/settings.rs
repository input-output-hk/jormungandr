use reqwest::Certificate;

#[derive(Debug, Clone)]
pub struct RestSettings {
    pub enable_debug: bool,
    pub use_https_for_post: bool,
    pub certificate: Option<Certificate>,
}

impl RestSettings {
    pub fn new_use_https_for_post() -> Self {
        RestSettings {
            enable_debug: false,
            use_https_for_post: true,
            certificate: None,
        }
    }
}

impl Default for RestSettings {
    fn default() -> Self {
        RestSettings {
            enable_debug: false,
            use_https_for_post: false,
            certificate: None,
        }
    }
}
