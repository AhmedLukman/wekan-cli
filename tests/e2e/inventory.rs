use std::{collections::BTreeSet, process::Command};

use serde::Deserialize;

#[derive(Deserialize)]
struct Suite {
    key: String,
    variable: String,
    test: String,
}

fn check_inventory(listing: &str, suites: &[Suite]) -> Result<(), String> {
    let discovered: BTreeSet<_> = listing
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .collect();
    if discovered.is_empty() {
        return Err("the compiled E2E binary did not list any ignored live tests".into());
    }
    let mut selected = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for suite in suites {
        if !keys.insert(&suite.key) {
            return Err(format!("duplicate E2E matrix key: {}", suite.key));
        }
        if !suite.variable.starts_with("WEKAN_") || !suite.variable.ends_with("_URL") {
            return Err(format!("invalid live-server variable for {}", suite.test));
        }
        if !selected.insert(suite.test.as_str()) {
            return Err(format!("duplicate E2E matrix test: {}", suite.test));
        }
    }
    if discovered != selected {
        return Err(format!(
            "E2E matrix differs from compiled ignored tests; missing selections: {:?}; stale selections: {:?}",
            discovered.difference(&selected).collect::<Vec<_>>(),
            selected.difference(&discovered).collect::<Vec<_>>()
        ));
    }
    Ok(())
}

#[test]
fn ci_matrix_selects_every_compiled_live_test_exactly_once() {
    // Ask this executable to enumerate tests, without running any live test.
    // Source scanning would miss cfg exclusions and unregistered modules.
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--list", "--ignored"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let listing = String::from_utf8(output.stdout).unwrap();
    let workflow: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(include_str!("../../.github/workflows/e2e.yml")).unwrap();
    let suites: Vec<Suite> = serde_yaml_ng::from_value(
        workflow["jobs"]["live-contract"]["strategy"]["matrix"]["include"].clone(),
    )
    .expect("the live-contract job must have an explicit E2E matrix");
    check_inventory(&listing, &suites).unwrap();
}

#[test]
fn inventory_rejects_empty_missing_stale_and_duplicate_selections() {
    let suite = |key: &str, test: &str| Suite {
        key: key.into(),
        variable: "WEKAN_E2E_URL".into(),
        test: test.into(),
    };
    let listing = "auth::live: test\nboards::live: test\n\n2 tests, 0 benchmarks\n";
    assert!(
        check_inventory(
            listing,
            &[suite("auth", "auth::live"), suite("boards", "boards::live")]
        )
        .is_ok()
    );
    for (listing, suites, diagnostic) in [
        ("0 tests, 0 benchmarks\n", vec![], "did not list"),
        (
            listing,
            vec![suite("auth", "auth::live")],
            "missing selections: [\"boards::live\"]",
        ),
        (
            listing,
            vec![suite("auth", "auth::live"), suite("boards", "boards::typo")],
            "stale selections: [\"boards::typo\"]",
        ),
        (
            listing,
            vec![suite("auth", "auth::live"), suite("other", "auth::live")],
            "duplicate E2E matrix test",
        ),
        (
            listing,
            vec![suite("same", "auth::live"), suite("same", "boards::live")],
            "duplicate E2E matrix key",
        ),
    ] {
        assert!(
            check_inventory(listing, &suites)
                .unwrap_err()
                .contains(diagnostic)
        );
    }
}
