use assert_cmd::cargo::cargo_bin_cmd;

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

pub(crate) fn no_target_cli(directory: &TestDirectory) -> assert_cmd::Command {
    let mut command = cargo_bin_cmd!("wekan");
    command
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .env_remove("WEKAN_URL")
        .env_remove("WEKAN_PROFILE");
    command
}

pub(crate) fn add_profile(directory: &TestDirectory, name: &str, server: &str) {
    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args(["profile", "add", name, server])
        .assert()
        .success();
}
