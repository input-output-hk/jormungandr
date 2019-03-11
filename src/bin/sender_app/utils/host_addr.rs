use reqwest::Url;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct HostAddr {
    /// Host node address
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
