use reqwest::Url;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct HostAddr {
    /// node API address. Must always have `http://` or `https://` prefix.
    /// E.g. `-h http://127.0.0.1`, `--host https://node.com:8443/cardano/api`
    #[structopt(short, long)]
    host: Url,
}

impl HostAddr {
    pub fn with_segments(mut self, segments: &[&str]) -> Self {
        self.host
            .path_segments_mut()
            .expect("Host address can't be used as base")
            .extend(segments);
        self
    }

    pub fn into_url(self) -> Url {
        self.host
    }
}
