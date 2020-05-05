mod release;

use jormungandr_lib::time::SystemTime;
use os_info::Type as OsType;
pub use release::{AssetDto, ReleaseDto};
use reqwest;
use reqwest::header::USER_AGENT;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitHubApiError {
    #[error("could not deserialize response")]
    CannotDeserialize(#[from] serde_json::Error),
    #[error("could not send reqeuest")]
    RequestError(#[from] reqwest::Error),
}

#[derive(Clone, Debug)]
pub struct Release {
    version: String,
    released_date: SystemTime,
    releases_per_os: HashMap<OsType, AssetDto>,
}

impl Release {
    pub fn get_release_for_os(&self, os_type: &OsType) -> Option<AssetDto> {
        let compacted_os_type = self.compact_os_types(*os_type);
        self.releases_per_os()
            .get(&compacted_os_type)
            .map(|x| x.clone())
    }

    /// narrow linux distribution to linux type
    fn compact_os_types(&self, os_type: OsType) -> OsType {
        match os_type {
            OsType::Emscripten => OsType::Linux,
            OsType::Redhat => OsType::Linux,
            OsType::RedHatEnterprise => OsType::Linux,
            OsType::Ubuntu => OsType::Linux,
            OsType::Debian => OsType::Linux,
            OsType::Arch => OsType::Linux,
            OsType::Centos => OsType::Linux,
            OsType::Fedora => OsType::Linux,
            OsType::Amazon => OsType::Linux,
            OsType::SUSE => OsType::Linux,
            OsType::openSUSE => OsType::Linux,
            OsType::Alpine => OsType::Linux,
            OsType::OracleLinux => OsType::Linux,
            _ => os_type,
        }
    }

    pub fn releases_per_os(&self) -> &HashMap<OsType, AssetDto> {
        &self.releases_per_os
    }

    pub fn version(&self) -> String {
        self.version.clone()
    }
}

pub struct GitHubApi {
    base_url: String,
}

impl GitHubApi {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.github.com/repos/input-output-hk/jormungandr".to_string(),
        }
    }

    fn get(&self, path: &str) -> Result<reqwest::blocking::Response, GitHubApiError> {
        let client = reqwest::blocking::Client::new();
        client
            .get(&format!("{}/{}", self.base_url, path))
            .header(USER_AGENT, "request")
            .send()
            .map_err(|err| GitHubApiError::RequestError(err))
    }

    pub fn describe_releases(&self) -> Result<Vec<Release>, GitHubApiError> {
        let response_text = self.get("releases")?.text()?;
        let releases: Vec<ReleaseDto> = serde_json::from_str(&response_text)
            .map_err(|err| GitHubApiError::CannotDeserialize(err))?;
        Ok(releases
            .iter()
            .cloned()
            .map(|release| release.into())
            .collect())
    }

    pub fn get_asset_for_current_os_by_version(
        &self,
        version: String,
    ) -> Result<Option<AssetDto>, GitHubApiError> {
        let info = os_info::get();
        Ok(
            match self
                .describe_releases()?
                .iter()
                .cloned()
                .find(|x| x.version == version)
            {
                None => None,
                Some(release) => release.get_release_for_os(&info.os_type()),
            },
        )
    }
}
