use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

use fs4::FileExt;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{CURRENT_VERSION, Release, file_error, protocol, release::HttpReleases, update_failed};
use crate::{command_result::InstallationMethod, error::AppError};

const MAX_BINARY: u64 = 256 * 1024 * 1024;
const MAX_METADATA: u64 = 1024 * 1024;

pub(crate) trait Installer {
    fn method(&self) -> InstallationMethod;
    async fn install(&self, release: &Release) -> Result<(), AppError>;
}

#[derive(Clone, Debug, PartialEq)]
enum Installation {
    Release {
        receipt_path: PathBuf,
        receipt: Value,
    },
    Cargo {
        root: PathBuf,
        settings: CargoSettings,
    },
    Unknown,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct CargoSettings {
    bins: Vec<String>,
    features: Vec<String>,
    all_features: bool,
    no_default_features: bool,
    profile: String,
    target: String,
}

pub(crate) struct SystemInstaller<H = HttpReleases> {
    executable: PathBuf,
    installation: Installation,
    http: H,
    receipts: Vec<PathBuf>,
}

impl SystemInstaller {
    pub fn production(http: HttpReleases) -> Result<Self, AppError> {
        let executable = std::env::current_exe()
            .and_then(fs::canonicalize)
            .map_err(|e| file_error("locate the running executable", e))?;
        let receipts = receipt_paths();
        let installation = detect(&executable, CURRENT_VERSION, &receipts);
        Ok(Self {
            executable,
            installation,
            http,
            receipts,
        })
    }
}

impl<H: super::release::AssetDownloader> SystemInstaller<H> {
    async fn install_release(
        &self,
        release: &Release,
        receipt_path: &Path,
        receipt: &Value,
    ) -> Result<(), AppError> {
        let target = release_target()
            .ok_or_else(|| update_failed("no release archive is available for this platform"))?;
        let extension = if cfg!(windows) { "zip" } else { "tar.xz" };
        let archive_name = format!("wekan-cli-{target}.{extension}");
        let archive_asset = release.asset(&archive_name)?;
        let checksum_asset = release.asset(&format!("{archive_name}.sha256"))?;
        let parent = self.executable.parent().expect("executable has a parent");
        let staging =
            tempfile::tempdir_in(parent).map_err(|e| file_error("stage CLI update", e))?;
        let mut archive = tempfile::tempfile_in(staging.path())
            .map_err(|e| file_error("stage release archive", e))?;
        let mut checksum = tempfile::tempfile_in(staging.path())
            .map_err(|e| file_error("stage release checksum", e))?;
        self.http
            .download(archive_asset, &mut archive, MAX_BINARY)
            .await?;
        self.http
            .download(checksum_asset, &mut checksum, 16 * 1024)
            .await?;
        verify_checksum(&mut archive, &mut checksum, &archive_name)?;
        let binary = staging.path().join(binary_name());
        let mut output =
            File::create(&binary).map_err(|e| file_error("stage replacement executable", e))?;
        extract(&mut archive, extension, &archive_name, &mut output)?;
        output
            .sync_all()
            .map_err(|e| file_error("save replacement executable", e))?;
        drop(output);
        fs::set_permissions(
            &binary,
            fs::metadata(&self.executable)
                .map_err(|e| file_error("inspect executable permissions", e))?
                .permissions(),
        )
        .map_err(|e| file_error("preserve executable permissions", e))?;

        // Prepare metadata before replacing the executable. Existing receipt
        // fields (layout, aliases, source, etc.) survive the update unchanged.
        let mut updated = receipt.clone();
        updated["version"] = Value::String(release.version.to_string());
        let mut receipt_staging =
            tempfile::NamedTempFile::new_in(receipt_path.parent().expect("receipt has a parent"))
                .map_err(|e| file_error("stage installation receipt", e))?;
        serde_json::to_writer(&mut receipt_staging, &updated)
            .map_err(|e| file_error("write installation receipt", e))?;
        fs::set_permissions(
            receipt_staging.path(),
            fs::metadata(receipt_path)
                .map_err(|e| file_error("inspect receipt permissions", e))?
                .permissions(),
        )
        .map_err(|e| file_error("preserve receipt permissions", e))?;
        receipt_staging
            .as_file()
            .sync_all()
            .map_err(|e| file_error("save installation receipt", e))?;
        self_replace::self_replace(&binary).map_err(|e| file_error("replace CLI executable", e))?;
        receipt_staging.persist(receipt_path).map_err(|e| update_failed(format!(
            "CLI was replaced, but its installation receipt could not be refreshed: {e}; rerun the original installer to repair metadata"
        )))?;
        Ok(())
    }
}

impl<H: super::release::AssetDownloader> Installer for SystemInstaller<H> {
    fn method(&self) -> InstallationMethod {
        match self.installation {
            Installation::Release { .. } => InstallationMethod::Release,
            Installation::Cargo { .. } => InstallationMethod::Cargo,
            Installation::Unknown => InstallationMethod::Unknown,
        }
    }

    async fn install(&self, release: &Release) -> Result<(), AppError> {
        let _lease = install_lock(&self.executable)?;
        // Refuse stale decisions made while another invocation or installer
        // changed ownership. Never overwrite another installation method.
        if detect(&self.executable, CURRENT_VERSION, &self.receipts) != self.installation {
            return Err(update_failed(
                "installation metadata changed; rerun `wekan self update`",
            ));
        }
        match &self.installation {
            Installation::Release {
                receipt_path,
                receipt,
            } => self.install_release(release, receipt_path, receipt).await,
            Installation::Cargo { root, settings } => {
                let root = root.clone();
                let settings = settings.clone();
                let executable = self.executable.clone();
                let version = release.version.to_string();
                tokio::task::spawn_blocking(move || {
                    install_cargo(&executable, &root, &settings, &version)
                })
                .await
                .map_err(|_| update_failed("Cargo update worker failed"))?
            }
            Installation::Unknown => Err(update_failed("installation method is unrecognized")),
        }
    }
}

fn binary_name() -> &'static str {
    if cfg!(windows) { "wekan.exe" } else { "wekan" }
}

fn receipt_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(root) = std::env::var_os("XDG_CONFIG_HOME").filter(|s| !s.is_empty()) {
        paths.push(PathBuf::from(root).join("wekan-cli/wekan-cli-receipt.json"));
    }
    if cfg!(windows) {
        if let Some(root) = std::env::var_os("LOCALAPPDATA") {
            paths.push(PathBuf::from(root).join("wekan-cli/wekan-cli-receipt.json"));
        }
    } else if let Some(base) = directories::BaseDirs::new() {
        paths.push(
            base.home_dir()
                .join(".config/wekan-cli/wekan-cli-receipt.json"),
        );
    }
    paths.sort();
    paths.dedup();
    paths
}

fn read_metadata(path: &Path) -> Option<Value> {
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(MAX_METADATA + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_METADATA {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}

fn detect(executable: &Path, version: &str, receipts: &[PathBuf]) -> Installation {
    if executable.file_name() != Some(std::ffi::OsStr::new(binary_name())) {
        return Installation::Unknown;
    }
    let mut candidates = Vec::new();
    for path in receipts {
        let Some(receipt) = read_metadata(path) else {
            continue;
        };
        if receipt["version"] != version
            || receipt["provider"]["source"] != "cargo-dist"
            || receipt["source"]["owner"] != "AhmedLukman"
            || receipt["source"]["name"] != "wekan-cli"
            || receipt["source"]["app_name"] != "wekan-cli"
            || receipt["source"]["release_type"] != "github"
            || !receipt["binaries"]
                .as_array()
                .is_some_and(|bins| bins.iter().any(|bin| bin == binary_name()))
        {
            continue;
        }
        let Some(prefix) = receipt["install_prefix"]
            .as_str()
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
        else {
            continue;
        };
        let possible = match receipt["install_layout"].as_str() {
            Some("flat") => vec![prefix.join(binary_name())],
            Some("cargo-home" | "hierarchical") => vec![prefix.join("bin").join(binary_name())],
            None | Some("unspecified") => vec![
                prefix.join(binary_name()),
                prefix.join("bin").join(binary_name()),
            ],
            _ => continue,
        };
        if possible
            .iter()
            .any(|p| fs::canonicalize(p).ok().as_deref() == Some(executable))
        {
            candidates.push(Installation::Release {
                receipt_path: path.clone(),
                receipt,
            });
        }
    }
    if let Some(bin) = executable
        .parent()
        .filter(|p| p.file_name() == Some(std::ffi::OsStr::new("bin")))
    {
        if let Some(root) = bin.parent() {
            if let Some(document) = read_metadata(&root.join(".crates2.json")) {
                if let Some(installs) = document["installs"].as_object() {
                    for (package, value) in installs {
                        let supported = [
                            format!(
                                "wekan-cli {version} (registry+https://github.com/rust-lang/crates.io-index)"
                            ),
                            format!("wekan-cli {version} (sparse+https://index.crates.io/)"),
                        ];
                        if !supported.contains(package) {
                            continue;
                        }
                        let Ok(settings) = serde_json::from_value::<CargoSettings>(value.clone())
                        else {
                            continue;
                        };
                        if settings.bins == [binary_name()]
                            && !settings.target.is_empty()
                            && !settings.profile.is_empty()
                        {
                            candidates.push(Installation::Cargo {
                                root: root.to_owned(),
                                settings,
                            });
                        }
                    }
                }
            }
        }
    }
    if candidates.len() == 1 {
        candidates.remove(0)
    } else {
        Installation::Unknown
    }
}

fn install_lock(executable: &Path) -> Result<File, AppError> {
    let path = executable
        .parent()
        .expect("executable has a parent")
        .join(".wekan-update.lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| file_error("open installation lock", e))?;
    FileExt::try_lock(&file)
        .map_err(|_| update_failed("another CLI update is in progress; retry when it finishes"))?;
    Ok(file)
}

fn cargo_command(root: &Path, settings: &CargoSettings, version: &str) -> Command {
    let mut command = Command::new("cargo");
    command
        .args([
            "install",
            "wekan-cli",
            "--locked",
            "--version",
            version,
            "--registry",
            "crates-io",
            "--bin",
            "wekan",
            "--root",
        ])
        .arg(root)
        .args(["--target", &settings.target, "--profile", &settings.profile]);
    if settings.all_features {
        command.arg("--all-features");
    }
    if settings.no_default_features {
        command.arg("--no-default-features");
    }
    if !settings.features.is_empty() {
        command.arg("--features").arg(settings.features.join(","));
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

fn install_cargo(
    executable: &Path,
    root: &Path,
    settings: &CargoSettings,
    version: &str,
) -> Result<(), AppError> {
    install_cargo_command(executable, cargo_command(root, settings, version))
}

fn install_cargo_command(executable: &Path, mut command: Command) -> Result<(), AppError> {
    if cfg!(windows) {
        // Cargo cannot replace a running Windows image. Give the installed
        // path to an identical, unlocked copy before starting Cargo, so it
        // stays runnable even if the download or build is interrupted.
        let current =
            tempfile::NamedTempFile::new_in(executable.parent().expect("executable has a parent"))
                .map_err(|e| file_error("stage current executable", e))?;
        fs::copy(executable, current.path())
            .map_err(|e| file_error("copy current executable", e))?;
        self_replace::self_replace(current.path())
            .map_err(|e| file_error("release running executable for Cargo", e))?;
    }
    match command.output() {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(update_failed(format!(
            "Cargo update failed (exit {}); verify the requested version is published on crates.io and your Rust toolchain supports it, then retry `wekan self update`",
            output
                .status
                .code()
                .map_or_else(|| "unknown".to_owned(), |c| c.to_string())
        ))),
        Err(error) => Err(update_failed(format!(
            "could not start Cargo: {error}; install Cargo or update using your original installation method"
        ))),
    }
}

fn verify_checksum(
    archive: &mut File,
    checksum: &mut File,
    archive_name: &str,
) -> Result<(), AppError> {
    archive
        .rewind()
        .map_err(|e| file_error("read release archive", e))?;
    checksum
        .rewind()
        .map_err(|e| file_error("read release checksum", e))?;
    let mut text = String::new();
    checksum
        .read_to_string(&mut text)
        .map_err(|e| file_error("read release checksum", e))?;
    let fields: Vec<_> = text.split_whitespace().collect();
    if fields.len() != 2
        || fields[0].len() != 64
        || !fields[0].bytes().all(|b| b.is_ascii_hexdigit())
        || fields[1].trim_start_matches('*') != archive_name
    {
        return Err(protocol(
            "release checksum file has an unexpected format or filename",
        ));
    }
    let mut digest = Sha256::new();
    io::copy(archive, &mut digest).map_err(|e| file_error("hash release archive", e))?;
    if format!("{:x}", digest.finalize()) != fields[0].to_ascii_lowercase() {
        return Err(update_failed(
            "release archive SHA-256 checksum does not match; no executable was replaced",
        ));
    }
    archive
        .rewind()
        .map_err(|e| file_error("read release archive", e))?;
    Ok(())
}

fn expected_entry(path: &Path, archive_name: &str) -> bool {
    if path
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return false;
    }
    let stem = archive_name
        .strip_suffix(".tar.xz")
        .or_else(|| archive_name.strip_suffix(".zip"))
        .unwrap_or(archive_name);
    path == Path::new(binary_name()) || path == Path::new(stem).join(binary_name())
}

fn copy_binary(reader: impl Read, output: &mut File) -> Result<(), AppError> {
    let bytes = io::copy(&mut reader.take(MAX_BINARY + 1), output)
        .map_err(|e| file_error("extract replacement executable", e))?;
    if bytes == 0 || bytes > MAX_BINARY {
        return Err(protocol("replacement executable has an invalid size"));
    }
    Ok(())
}

fn extract(
    archive: &mut File,
    extension: &str,
    archive_name: &str,
    output: &mut File,
) -> Result<(), AppError> {
    archive
        .rewind()
        .map_err(|e| file_error("read release archive", e))?;
    let mut found = false;
    if extension == "zip" {
        let mut zip =
            zip::ZipArchive::new(archive).map_err(|_| protocol("invalid release ZIP archive"))?;
        for index in 0..zip.len() {
            let entry = zip
                .by_index(index)
                .map_err(|_| protocol("invalid release ZIP entry"))?;
            let Some(path) = entry.enclosed_name() else {
                return Err(protocol("unsafe path in release ZIP archive"));
            };
            if !expected_entry(&path, archive_name) {
                continue;
            }
            if !entry.is_file() || entry.is_symlink() || found {
                return Err(protocol("invalid or duplicate replacement executable"));
            }
            copy_binary(entry, output)?;
            found = true;
        }
    } else {
        let decoder = lzma_rust2::XzReader::new(archive, false);
        let mut tar = tar::Archive::new(decoder);
        for entry in tar
            .entries()
            .map_err(|_| protocol("invalid release tar archive"))?
        {
            let entry = entry.map_err(|_| protocol("invalid release tar entry"))?;
            let path = entry
                .path()
                .map_err(|_| protocol("invalid release tar path"))?;
            if path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(protocol("unsafe path in release tar archive"));
            }
            if !expected_entry(&path, archive_name) {
                continue;
            }
            if !entry.header().entry_type().is_file() || found {
                return Err(protocol("invalid or duplicate replacement executable"));
            }
            copy_binary(entry, output)?;
            found = true;
        }
    }
    if !found {
        return Err(protocol(
            "release archive does not contain the expected executable",
        ));
    }
    Ok(())
}

fn release_target() -> Option<&'static str> {
    if cfg!(all(
        target_os = "windows",
        target_arch = "x86_64",
        target_env = "msvc"
    )) {
        Some("x86_64-pc-windows-msvc")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "aarch64",
        target_env = "gnu"
    )) {
        Some("aarch64-unknown-linux-gnu")
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        Some("x86_64-unknown-linux-gnu")
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
