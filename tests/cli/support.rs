use std::ops::{Deref, DerefMut};

use assert_cmd::cargo::cargo_bin_cmd;

/// Keep the isolated default configuration alive for the entire subprocess.
pub(crate) struct TestCommand {
    command: assert_cmd::Command,
    _directory: tempfile::TempDir,
}

impl Deref for TestCommand {
    type Target = assert_cmd::Command;

    fn deref(&self) -> &Self::Target {
        &self.command
    }
}

impl DerefMut for TestCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.command
    }
}

pub(crate) fn wekan() -> TestCommand {
    let directory = tempfile::tempdir().unwrap();
    let mut command = cargo_bin_cmd!("wekan");
    command
        .env("WEKAN_CONFIG_DIR", directory.path())
        .env_remove("WEKAN_URL")
        .env_remove("WEKAN_PROFILE");
    TestCommand {
        command,
        _directory: directory,
    }
}

pub(crate) struct TestDirectory(pub(crate) std::path::PathBuf);

impl TestDirectory {
    pub(crate) fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "wekan-cli-black-box-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn no_target_cli(directory: &TestDirectory) -> TestCommand {
    let mut command = wekan();
    command.env("WEKAN_CONFIG_DIR", &directory.0);
    command
}

pub(crate) fn add_profile(directory: &TestDirectory, name: &str, server: &str) {
    no_target_cli(directory)
        .args(["profile", "add", name, server])
        .assert()
        .success();
}
