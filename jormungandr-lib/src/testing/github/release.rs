use crate::testing::github::Release;
use crate::time::SystemTime;
use os_info::Type as OsType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseDto {
    tag_name: String,
    published_at: SystemTime,
    assets: Vec<AssetDto>,
}

impl Into<Release> for ReleaseDto {
    fn into(self) -> Release {
        Release {
            version: self.tag_name.clone(),
            released_date: self.published_at.clone(),
            releases_per_os: self
                .assets
                .iter()
                .cloned()
                .map(|x| (x.os_type(), x))
                .collect(),
        }
    }
}

impl ReleaseDto {
    pub fn tag_name(self) -> String {
        self.tag_name
    }

    pub fn published_at(&self) -> &SystemTime {
        &self.published_at
    }

    pub fn assets(&self) -> &Vec<AssetDto> {
        &self.assets
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetDto {
    browser_download_url: String,
    name: String,
}

impl AssetDto {
    pub fn os_type(&self) -> OsType {
        if self.name.contains("windows") {
            return OsType::Windows;
        } else if self.name.contains("linux") {
            return OsType::Linux;
        } else if self.name.contains("apple") {
            return OsType::Macos;
        }
        OsType::Unknown
    }

    pub fn download_url(&self) -> String {
        self.browser_download_url.clone()
    }
}
