use jormungandr_testing_utils::testing::{
    decompress, download_file,
    github::{GitHubApi, Release},
};

mod configuration_builder;
mod node;
mod rest;

pub use configuration_builder::{
    LegacyConfigConverter, LegacyConfigConverterError, LegacyNodeConfigConverter, version_0_8_19
};
pub use jormungandr_testing_utils::legacy::{NodeConfig, Version};
pub use rest::BackwardCompatibleRest;

use crate::common::file_utils;
use assert_fs::fixture::PathChild;
use assert_fs::prelude::*;
use url::Url;

use std::path::PathBuf;

pub fn download_last_n_releases(n: u32) -> Vec<Release> {
    let github_api = GitHubApi::new();
    github_api
        .describe_releases()
        .unwrap()
        .iter()
        .cloned()
        .filter(|x| !x.prerelease())
        .take(n as usize)
        .collect()
}

pub fn get_jormungandr_bin(release: &Release, temp_dir: &impl PathChild) -> PathBuf {
    let github_api = GitHubApi::new();
    let asset = github_api
        .get_asset_for_current_os_by_version(release.version())
        .unwrap()
        .unwrap();
    let url = Url::parse(&asset.download_url()).expect("cannot parse url");
    let file_name = url
        .path_segments()
        .unwrap()
        .last()
        .expect("cannot get last element from path");

    let output = temp_dir.child(&file_name);
    download_file(asset.download_url(), output.path()).expect("cannot download file");
    let release_dir = temp_dir.child(format!("release-{}", release.version()));
    release_dir.create_dir_all().unwrap();
    decompress(output.path(), release_dir.path()).unwrap();
    file_utils::find_file(release_dir.path(), "jormungandr").unwrap()
}
