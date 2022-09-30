mod config;
mod rest;
mod version;

use crate::testing::{decompress, CachedReleases, GitHubApiBuilder, Release};
use assert_fs::{fixture::PathChild, prelude::*};
pub use config::{
    LegacyConfigConverter, LegacyConfigConverterError, LegacyNodeConfigConverter,
    NodeConfig as LegacyNodeConfig,
};
pub use jormungandr_lib::interfaces::{
    Log, Mempool, NodeConfig, P2p, Policy, Rest, TopicsOfInterest, TrustedPeer,
};
use jortestkit::file;
pub use rest::BackwardCompatibleRest;
use std::path::PathBuf;
pub use version::*;

const GITHUB_TOKEN: &str = "GITHUB_TOKEN";

lazy_static::lazy_static! {
    static ref RELEASES: CachedReleases = {
        let api = GitHubApiBuilder::new().with_token(std::env::var(GITHUB_TOKEN).ok()).build();
        api.describe_releases().unwrap()
    };
}

pub fn download_last_n_releases(n: u32) -> Vec<Release> {
    RELEASES
        .into_iter()
        .cloned()
        .filter(|x| !x.version_str().starts_with("nightly"))
        .take(n as usize)
        .collect()
}

pub fn get_jormungandr_bin(release: &Release, temp_dir: &impl PathChild) -> PathBuf {
    let asset = RELEASES
        .get_asset_for_current_os_by_version(release.version_str())
        .unwrap()
        .unwrap();
    let asset_name = asset.name();
    let output = temp_dir.child(&asset_name);
    asset
        .download_to(output.path())
        .expect("cannot download file");
    let release_dir = temp_dir.child(format!("release-{}", release.version()));
    release_dir.create_dir_all().unwrap();
    decompress(output.path(), release_dir.path()).unwrap();
    file::find_file(release_dir.path(), "jormungandr")
        .unwrap()
        .unwrap()
}
