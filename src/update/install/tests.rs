use super::*;
use std::io::Write;

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, Value) {
    let directory = tempfile::tempdir().unwrap();
    let bin = directory.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let executable = bin.join(binary_name());
    fs::write(&executable, b"original executable").unwrap();
    let executable = executable.canonicalize().unwrap();
    let receipt_path = directory.path().join("receipt.json");
    let receipt = serde_json::json!({
        "binaries": [binary_name()], "binary_aliases": {}, "cdylibs": [], "cstaticlibs": [],
        "install_layout": "cargo-home", "install_prefix": directory.path().canonicalize().unwrap(),
        "modify_path": false, "provider": {"source": "cargo-dist", "version": "0.32.0"},
        "source": {"app_name": "wekan-cli", "name": "wekan-cli", "owner": "AhmedLukman", "release_type": "github"}, "version": "2.0.0"
    });
    (directory, executable, receipt_path, receipt)
}

fn cargo_settings() -> CargoSettings {
    CargoSettings {
        bins: vec![binary_name().into()],
        features: vec!["example".into()],
        all_features: true,
        no_default_features: true,
        profile: "release".into(),
        target: "x86_64-pc-windows-msvc".into(),
    }
}

#[test]
fn receipts_require_matching_executable_version_source_and_layout() {
    let (_dir, executable, path, receipt) = fixture();
    fs::write(&path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    assert!(matches!(
        detect(&executable, "2.0.0", std::slice::from_ref(&path)),
        Installation::Release { .. }
    ));
    assert_eq!(
        detect(&executable, "2.0.1", std::slice::from_ref(&path)),
        Installation::Unknown
    );
    for field in ["source", "install_prefix", "binaries", "install_layout"] {
        let mut changed = receipt.clone();
        changed[field] = "invalid".into();
        fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
        assert_eq!(
            detect(&executable, "2.0.0", std::slice::from_ref(&path)),
            Installation::Unknown
        );
    }
}

#[test]
fn cargo_installs_are_found_at_custom_roots_and_conflicting_records_are_unknown() {
    let (dir, executable, receipt_path, receipt) = fixture();
    let document = serde_json::json!({"installs": {
        "wekan-cli 2.0.0 (registry+https://github.com/rust-lang/crates.io-index)": {
            "bins": [binary_name()], "features": [], "all_features": false, "no_default_features": false,
            "profile": "release", "target": "x86_64-pc-windows-msvc", "rustc": "ignored"
        }
    }});
    fs::write(
        dir.path().join(".crates2.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        detect(&executable, "2.0.0", &[]),
        Installation::Cargo { .. }
    ));
    assert_eq!(detect(&executable, "2.1.0", &[]), Installation::Unknown);
    fs::write(&receipt_path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    assert_eq!(
        detect(&executable, "2.0.0", &[receipt_path]),
        Installation::Unknown
    );
    let source = document.to_string().replace(
        "registry+https://github.com/rust-lang/crates.io-index",
        "path+file:///source",
    );
    fs::write(dir.path().join(".crates2.json"), source).unwrap();
    assert_eq!(detect(&executable, "2.0.0", &[]), Installation::Unknown);
}

#[test]
fn cargo_command_preserves_root_version_target_profile_and_features_without_a_shell() {
    let settings = cargo_settings();
    let command = cargo_command(Path::new("custom root"), &settings, "2.1.0");
    let args: Vec<_> = command.get_args().map(|v| v.to_str().unwrap()).collect();
    assert_eq!(
        args,
        [
            "install",
            "wekan-cli",
            "--locked",
            "--version",
            "2.1.0",
            "--registry",
            "crates-io",
            "--bin",
            "wekan",
            "--root",
            "custom root",
            "--target",
            "x86_64-pc-windows-msvc",
            "--profile",
            "release",
            "--all-features",
            "--no-default-features",
            "--features",
            "example"
        ]
    );
}

#[test]
fn installation_lock_excludes_other_updates_until_released() {
    let (_dir, executable, _, _) = fixture();
    let lock = install_lock(&executable).unwrap();
    assert_eq!(
        install_lock(&executable).err().unwrap().code(),
        crate::error::ErrorCode::UpdateFailed
    );
    drop(lock);
    assert!(install_lock(&executable).is_ok());
}

#[test]
fn checksum_mismatch_or_wrong_filename_cannot_replace_an_executable() {
    let mut archive = tempfile::tempfile().unwrap();
    archive.write_all(b"archive bytes").unwrap();
    let mut checksum = tempfile::tempfile().unwrap();
    writeln!(
        checksum,
        "{:x}  archive.zip",
        Sha256::digest(b"archive bytes")
    )
    .unwrap();
    verify_checksum(&mut archive, &mut checksum, "archive.zip").unwrap();
    assert!(verify_checksum(&mut archive, &mut checksum, "different.zip").is_err());
    archive.rewind().unwrap();
    archive.write_all(b"tampered bytes").unwrap();
    assert_eq!(
        verify_checksum(&mut archive, &mut checksum, "archive.zip")
            .unwrap_err()
            .code(),
        crate::error::ErrorCode::UpdateFailed
    );
}

fn zip_fixture(names: &[&str]) -> File {
    let file = tempfile::tempfile().unwrap();
    let mut zip = zip::ZipWriter::new(file);
    for name in names {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"replacement executable").unwrap();
    }
    zip.finish().unwrap()
}

#[test]
fn extraction_selects_only_expected_executable_and_rejects_unsafe_paths() {
    let name = "wekan-cli-test.zip";
    let expected = format!("wekan-cli-test/{}", binary_name());
    let mut archive = zip_fixture(&[&expected, "README.md"]);
    let mut output = tempfile::tempfile().unwrap();
    extract(&mut archive, "zip", name, &mut output).unwrap();
    output.rewind().unwrap();
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"replacement executable");
    for names in [
        vec!["../wekan"],
        vec!["unrelated/wekan"],
        vec![binary_name(), &expected],
    ] {
        let mut archive = zip_fixture(&names);
        assert!(
            extract(
                &mut archive,
                "zip",
                name,
                &mut tempfile::tempfile().unwrap()
            )
            .is_err()
        );
    }
}

#[test]
fn release_assets_must_belong_to_the_offered_tag_and_repository() {
    let mut release = super::super::tests::release("2.1.0");
    release.assets.push(super::super::release::Asset {
        name: "archive.zip".into(),
        browser_download_url: "https://other.example/archive.zip".into(),
    });
    assert!(release.asset("archive.zip").is_err());
    release.assets[0].browser_download_url =
        "https://github.com/AhmedLukman/wekan-cli/releases/download/v2.1.0/archive.zip".into();
    assert!(release.asset("archive.zip").is_ok());
    release.assets.push(release.assets[0].clone());
    assert!(release.asset("archive.zip").is_err());
}

struct LocalAssets {
    archive: Vec<u8>,
    checksum: Vec<u8>,
}

impl super::super::release::AssetDownloader for LocalAssets {
    async fn download(
        &self,
        asset: &super::super::release::Asset,
        file: &mut File,
        _: u64,
    ) -> Result<(), AppError> {
        file.write_all(if asset.name.ends_with(".sha256") {
            &self.checksum
        } else {
            &self.archive
        })
        .map_err(|e| file_error("write test asset", e))
    }
}

fn native_assets(corrupt_checksum: bool) -> (Release, LocalAssets) {
    let extension = if cfg!(windows) { "zip" } else { "tar.xz" };
    let name = format!("wekan-cli-{}.{extension}", release_target().unwrap());
    let binary = format!(
        "{}/{}",
        name.trim_end_matches(&format!(".{extension}")),
        binary_name()
    );
    let mut archive = Vec::new();
    if cfg!(windows) {
        let mut file = zip_fixture(&[&binary]);
        file.rewind().unwrap();
        file.read_to_end(&mut archive).unwrap();
    } else {
        let mut tar = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(b"replacement executable".len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, binary, b"replacement executable".as_slice())
            .unwrap();
        let tar_bytes = tar.into_inner().unwrap();
        let mut xz =
            lzma_rust2::XzWriter::new(Vec::new(), lzma_rust2::XzOptions::with_preset(0)).unwrap();
        xz.write_all(&tar_bytes).unwrap();
        archive = xz.finish().unwrap();
    }
    let checksum = format!(
        "{:x}  {name}\n",
        Sha256::digest(if corrupt_checksum {
            b"incorrect".as_slice()
        } else {
            &archive
        })
    )
    .into_bytes();
    let mut release = super::super::tests::release("2.1.0");
    for name in [name.clone(), format!("{name}.sha256")] {
        release.assets.push(super::super::release::Asset {
            browser_download_url: format!(
                "https://github.com/AhmedLukman/wekan-cli/releases/download/v2.1.0/{name}"
            ),
            name,
        });
    }
    (release, LocalAssets { archive, checksum })
}

#[test]
fn native_archive_format_is_verified_and_extractable() {
    let (release, assets) = native_assets(false);
    let mut archive = tempfile::tempfile().unwrap();
    archive.write_all(&assets.archive).unwrap();
    let mut checksum = tempfile::tempfile().unwrap();
    checksum.write_all(&assets.checksum).unwrap();
    verify_checksum(&mut archive, &mut checksum, &release.assets[0].name).unwrap();
    extract(
        &mut archive,
        if cfg!(windows) { "zip" } else { "tar.xz" },
        &release.assets[0].name,
        &mut tempfile::tempfile().unwrap(),
    )
    .unwrap();
}

/// Run real replacement inside a copied test executable so the test runner and
/// installed CLI are never the target. The same tests run on all CI platforms.
#[test]
fn isolated_release_replacement_refreshes_receipt_and_checksum_failure_leaves_it_untouched() {
    for corrupt in [false, true] {
        let (dir, executable, receipt_path, receipt) = fixture();
        fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
        fs::write(&receipt_path, serde_json::to_vec(&receipt).unwrap()).unwrap();
        let permissions = fs::metadata(&executable).unwrap().permissions();
        let output = Command::new(&executable)
            .args([
                "--exact",
                "update::install::tests::release_replacement_worker",
                "--ignored",
                "--nocapture",
            ])
            .env("WEKAN_TEST_UPDATE_RECEIPT", &receipt_path)
            .env("WEKAN_TEST_UPDATE_CORRUPT", if corrupt { "1" } else { "0" })
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let saved = read_metadata(&receipt_path).unwrap();
        if corrupt {
            assert_eq!(saved, receipt);
            assert!(fs::metadata(&executable).unwrap().len() > 1024);
        } else {
            assert_eq!(fs::read(&executable).unwrap(), b"replacement executable");
            assert_eq!(saved["version"], "2.1.0");
            let mut expected = receipt.clone();
            expected["version"] = "2.1.0".into();
            assert_eq!(saved, expected);
            assert_eq!(
                fs::metadata(&executable).unwrap().permissions().readonly(),
                permissions.readonly()
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    fs::metadata(&executable).unwrap().permissions().mode(),
                    permissions.mode()
                );
            }
        }
        // Windows deletion helpers finish shortly after their parent exits.
        #[cfg(windows)]
        wait_for_cleanup(dir.path());
        #[cfg(not(windows))]
        drop(dir);
    }
}

#[test]
#[ignore = "subprocess worker for isolated replacement tests"]
fn release_replacement_worker() {
    let Some(receipt_path) = std::env::var_os("WEKAN_TEST_UPDATE_RECEIPT").map(PathBuf::from)
    else {
        return;
    };
    let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
    let receipts = vec![receipt_path];
    let installation = detect(&executable, CURRENT_VERSION, &receipts);
    assert!(matches!(installation, Installation::Release { .. }));
    let corrupt = std::env::var("WEKAN_TEST_UPDATE_CORRUPT").unwrap() == "1";
    let (release, http) = native_assets(corrupt);
    let installer = SystemInstaller {
        executable,
        installation,
        http,
        receipts,
    };
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(installer.install(&release));
    if corrupt {
        assert!(result.is_err())
    } else {
        result.unwrap()
    }
}

#[test]
fn isolated_cargo_success_and_failure_preserve_or_replace_the_executable() {
    for failure in [false, true] {
        let (dir, executable, _, _) = fixture();
        let runner = std::env::current_exe().unwrap();
        fs::copy(&runner, &executable).unwrap();
        let cargo = dir.path().join(if cfg!(windows) {
            "fake-cargo.exe"
        } else {
            "fake-cargo"
        });
        fs::copy(&runner, &cargo).unwrap();
        let original_size = fs::metadata(&executable).unwrap().len();
        let result = Command::new(&executable)
            .args([
                "--exact",
                "update::install::tests::cargo_replacement_worker",
                "--ignored",
                "--nocapture",
            ])
            .env("WEKAN_TEST_FAKE_CARGO", &cargo)
            .env("WEKAN_TEST_CARGO_FAIL", if failure { "1" } else { "0" })
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!String::from_utf8_lossy(&result.stdout).contains("captured installer output"));
        if failure {
            assert_eq!(fs::metadata(&executable).unwrap().len(), original_size);
            assert!(
                Command::new(&executable)
                    .arg("--list")
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        } else {
            assert_eq!(fs::read(&executable).unwrap(), b"Cargo replacement");
        }
        #[cfg(windows)]
        wait_for_cleanup(dir.path());
    }
}

#[test]
#[ignore = "subprocess worker for isolated Cargo tests"]
fn cargo_replacement_worker() {
    let Some(cargo) = std::env::var_os("WEKAN_TEST_FAKE_CARGO") else {
        return;
    };
    let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
    let mut command = Command::new(cargo);
    command
        .args([
            "--exact",
            "update::install::tests::fake_cargo_process",
            "--ignored",
            "--nocapture",
        ])
        .env("WEKAN_TEST_CARGO_DESTINATION", &executable)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let result = install_cargo_command(&executable, command);
    if std::env::var("WEKAN_TEST_CARGO_FAIL").unwrap() == "1" {
        assert!(result.is_err())
    } else {
        result.unwrap()
    }
}

#[test]
#[ignore = "fake Cargo subprocess for isolated tests"]
fn fake_cargo_process() {
    let Some(path) = std::env::var_os("WEKAN_TEST_CARGO_DESTINATION").map(PathBuf::from) else {
        return;
    };
    // Cargo can spend minutes downloading and compiling before it installs.
    // The current version must stay runnable even if that work is interrupted.
    assert_eq!(
        fs::metadata(&path).unwrap().len(),
        fs::metadata(std::env::current_exe().unwrap())
            .unwrap()
            .len()
    );
    println!("captured installer output");
    eprintln!("captured installer output");
    if std::env::var("WEKAN_TEST_CARGO_FAIL").unwrap() == "1" {
        std::process::exit(17);
    }
    // Unix updates replace a running image by rename, as Cargo itself does.
    let staged = path.with_extension("new");
    fs::write(&staged, b"Cargo replacement").unwrap();
    fs::rename(staged, path).unwrap();
}

#[cfg(windows)]
fn wait_for_cleanup(directory: &Path) {
    for _ in 0..100 {
        if fs::read_dir(directory.join("bin")).unwrap().all(|entry| {
            let name = entry.unwrap().file_name();
            name == binary_name() || name == ".wekan-update.lock"
        }) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("replacement helpers did not clean up");
}

#[tokio::test]
#[ignore = "downloads a published release archive; performs no installation"]
async fn published_release_archive_matches_the_updater_contract() {
    use super::super::release::{AssetDownloader, ReleaseProvider};
    let http = HttpReleases::production().unwrap();
    let release = http
        .latest(std::time::Duration::from_secs(15))
        .await
        .unwrap();
    let extension = if cfg!(windows) { "zip" } else { "tar.xz" };
    let name = format!("wekan-cli-{}.{extension}", release_target().unwrap());
    let mut archive = tempfile::tempfile().unwrap();
    let mut checksum = tempfile::tempfile().unwrap();
    http.download(release.asset(&name).unwrap(), &mut archive, MAX_BINARY)
        .await
        .unwrap();
    http.download(
        release.asset(&format!("{name}.sha256")).unwrap(),
        &mut checksum,
        16 * 1024,
    )
    .await
    .unwrap();
    verify_checksum(&mut archive, &mut checksum, &name).unwrap();
    let mut executable = tempfile::tempfile().unwrap();
    extract(&mut archive, extension, &name, &mut executable).unwrap();
    assert!(executable.metadata().unwrap().len() > 1024);
}
