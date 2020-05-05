use jormungandr_testing_utils::testing::{
    decompress, download_file,
    github::{GitHubApi, Release},
};

use crate::common::file_utils;

use std::path::PathBuf;
use url::Url;

pub fn download_last_n_releases(n: usize) -> Vec<Release> {
    let github_api = GitHubApi::new();
    github_api
        .describe_releases()
        .unwrap()
        .iter()
        .cloned()
        .take(n)
        .collect()
}

pub fn get_jormungandr_bin(release: &Release) -> PathBuf {
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

    let version = release.version().replace(".", "_");
    let output = file_utils::get_path_in_temp(&file_name);
    download_file(asset.download_url(), &output).expect("cannot download file");
    let decompressed = file_utils::create_folder(&format!("unpacked_{}", version));
    decompress(&output, &decompressed).unwrap();
    file_utils::find_file(&decompressed, "jormungandr").unwrap()
}
