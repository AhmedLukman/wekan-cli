use std::{io::Write, time::Duration};

use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};

use super::{file_error, protocol};
use crate::{
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
};

pub(super) const REPOSITORY: &str = "https://github.com/AhmedLukman/wekan-cli";
const LATEST: &str = "https://api.github.com/repos/AhmedLukman/wekan-cli/releases/latest";

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Release {
    pub tag: String,
    #[serde(with = "version_string")]
    pub version: Version,
    pub assets: Vec<Asset>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

// GitHub is independent of the strict, pinned Wekan response contract. Its
// public release responses contain many fields unrelated to update discovery.
#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

impl Release {
    pub fn url(&self) -> String {
        format!("{REPOSITORY}/releases/tag/{}", self.tag)
    }

    pub fn asset(&self, name: &str) -> Result<&Asset, AppError> {
        let expected = format!("{REPOSITORY}/releases/download/{}/{name}", self.tag);
        let mut matches = self.assets.iter().filter(|a| a.name == name);
        let asset = matches
            .next()
            .ok_or_else(|| protocol(format!("release is missing asset {name}")))?;
        if matches.next().is_some() || asset.browser_download_url != expected {
            return Err(protocol(format!(
                "release has an invalid or duplicate asset {name}"
            )));
        }
        Ok(asset)
    }
}

pub(crate) trait ReleaseProvider {
    async fn latest(&self, timeout: Duration) -> Result<Release, AppError>;
}

pub(crate) trait AssetDownloader {
    async fn download(
        &self,
        asset: &Asset,
        file: &mut std::fs::File,
        limit: u64,
    ) -> Result<(), AppError>;
}

#[derive(Clone)]
pub(crate) struct HttpReleases {
    pub(super) client: Client,
    pub(super) endpoint: String,
}

impl HttpReleases {
    pub fn production() -> Result<Self, AppError> {
        let client = Client::builder()
            .https_only(true)
            .user_agent(concat!("wekan-cli/", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(transport)?;
        Ok(Self {
            client,
            endpoint: LATEST.to_owned(),
        })
    }
}

impl AssetDownloader for HttpReleases {
    async fn download(
        &self,
        asset: &Asset,
        file: &mut std::fs::File,
        limit: u64,
    ) -> Result<(), AppError> {
        let mut response = self
            .client
            .get(&asset.browser_download_url)
            .timeout(Duration::from_secs(300))
            .send()
            .await
            .map_err(transport)?
            .error_for_status()
            .map_err(transport)?;
        let mut total = 0;
        while let Some(chunk) = response.chunk().await.map_err(transport)? {
            total += chunk.len() as u64;
            if total > limit {
                return Err(protocol("release asset exceeds its download limit"));
            }
            file.write_all(&chunk)
                .map_err(|e| file_error("download release asset", e))?;
        }
        file.sync_all()
            .map_err(|e| file_error("save release asset", e))
    }
}

impl ReleaseProvider for HttpReleases {
    async fn latest(&self, timeout: Duration) -> Result<Release, AppError> {
        let mut response = self
            .client
            .get(&self.endpoint)
            .timeout(timeout)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(transport)?
            .error_for_status()
            .map_err(transport)?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport)? {
            if bytes.len() + chunk.len() > 1024 * 1024 {
                return Err(protocol("release metadata exceeds 1 MiB"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let raw: GithubRelease = serde_json::from_slice(&bytes)
            .map_err(|_| protocol("GitHub returned invalid release metadata"))?;
        let version = Version::parse(raw.tag_name.strip_prefix('v').unwrap_or(&raw.tag_name))
            .map_err(|_| protocol("GitHub release tag is not a semantic version"))?;
        if raw.draft || raw.prerelease || !version.pre.is_empty() {
            return Err(protocol("GitHub did not return a stable published release"));
        }
        Ok(Release {
            tag: raw.tag_name,
            version,
            assets: raw.assets,
        })
    }
}

fn transport(error: reqwest::Error) -> AppError {
    // Never echo proxy credentials, query strings, or response bodies.
    let message = if error.is_timeout() {
        "release request timed out".to_owned()
    } else if let Some(status) = error.status() {
        format!("release request failed with HTTP {}", status.as_u16())
    } else {
        "release request failed; check connectivity and proxy settings".to_owned()
    };
    AppError::new(
        ErrorCode::TransportError,
        message,
        StableExitCode::Transport,
    )
}

mod version_string {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        value: &Version,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Version, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Version::parse(&raw).map_err(serde::de::Error::custom)
    }
}
