use std::collections::{BTreeMap, BTreeSet};

use clap::{Command, CommandFactory};
use wekan_cli::cli::Cli;

const COVERAGE: &str = include_str!("../../docs/api-coverage.md");
const METHODS: [&str; 7] = ["get", "put", "post", "delete", "options", "head", "patch"];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Operation {
    method: String,
    path: String,
    id: String,
}

#[derive(Clone)]
struct Row {
    operation: Operation,
    syntax: String,
    implemented: bool,
    excluded: bool,
}

// GFM requires pipes to be escaped, including inside inline code.
fn cells(line: &str) -> Vec<&str> {
    let mut start = 0;
    let mut escaped = false;
    let mut result = Vec::new();
    for (index, character) in line.char_indices() {
        if !escaped && character == '|' {
            result.push(line[start..index].trim());
            start = index + 1;
        }
        escaped = character == '\\' && !escaped;
    }
    result.push(line[start..].trim());
    result
}

fn rows(document: &str) -> Result<Vec<Row>, String> {
    let mut result = Vec::new();
    let mut operations = BTreeSet::new();
    let mut routes = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for line in document.lines().filter(|line| {
        line.strip_prefix("| ")
            .and_then(|rest| rest.chars().next())
            .is_some_and(|first| first.is_ascii_digit())
    }) {
        let fields = cells(line);
        if fields.len() != 10 || fields[1].parse::<usize>() != Ok(result.len() + 1) {
            return Err(format!("invalid coverage row or sequence: {line}"));
        }
        let operation = Operation {
            method: fields[3].trim_matches('`').into(),
            path: fields[4].trim_matches('`').into(),
            id: fields[5].trim_matches('`').into(),
        };
        if !METHODS.contains(&operation.method.to_ascii_lowercase().as_str())
            || !operation.path.starts_with('/')
            || operation.id.is_empty()
            || !operations.insert(operation.clone())
            || !routes.insert((operation.method.clone(), operation.path.clone()))
            || !ids.insert(operation.id.clone())
        {
            return Err(format!(
                "invalid or duplicate coverage operation: {operation:?}"
            ));
        }
        let (implemented, excluded) = match fields[8] {
            "✓" => (true, false),
            "—" => (false, false),
            "Excluded" => (false, true),
            status => return Err(format!("unknown coverage status: {status}")),
        };
        result.push(Row {
            operation,
            syntax: fields[7].into(),
            implemented,
            excluded,
        });
    }
    if result.is_empty() {
        return Err("the coverage table is empty".into());
    }
    let implemented = result.iter().filter(|row| row.implemented).count();
    let excluded = result.iter().filter(|row| row.excluded).count();
    let total = result.len() - excluded;
    if total == 0 {
        return Err("the coverage table has no in-scope operations".into());
    }
    let summary = format!(
        "**Coverage:** {implemented} of {total} in-scope operations implemented ({}%); {} remaining; {excluded} intentionally excluded.",
        implemented * 100 / total,
        total - implemented
    );
    if !document.lines().any(|line| line == summary) {
        return Err(format!("coverage summary must be: {summary}"));
    }
    Ok(result)
}

fn options(syntax: &str) -> BTreeSet<String> {
    syntax
        .match_indices("--")
        .filter_map(|(index, _)| {
            let name: String = syntax[index + 2..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

fn check_commands(rows: &[Row], root: &Command) -> Result<(), String> {
    // A single CLI command can select several endpoints (for example, export
    // formats). Check option completeness across all its documented variants.
    let mut documented: BTreeMap<String, (&Command, BTreeSet<String>)> = BTreeMap::new();
    for row in rows.iter().filter(|row| row.implemented) {
        let mut has_command = false;
        // An option which selects another endpoint may be documented in a
        // separate inline-code note, rather than the example for this route.
        let annotations: BTreeSet<_> = row
            .syntax
            .split('`')
            .skip(1)
            .step_by(2)
            .filter(|code| !code.starts_with("wekan "))
            .flat_map(options)
            .collect();
        for syntax in row
            .syntax
            .split('`')
            .skip(1)
            .step_by(2)
            .filter(|syntax| syntax.starts_with("wekan "))
        {
            has_command = true;
            let mut words = syntax.split_whitespace().skip(1);
            let mut leaf = root;
            let mut path = String::from("wekan");
            while leaf.has_subcommands() {
                let word = words
                    .next()
                    .ok_or_else(|| format!("incomplete command: {syntax}"))?;
                if let Some(flag) = word.strip_prefix("--") {
                    let arg = leaf
                        .get_arguments()
                        .find(|arg| arg.get_long() == Some(flag))
                        .ok_or_else(|| format!("unknown global option {word} in {syntax}"))?;
                    if arg.get_num_args().is_some_and(|arity| arity.takes_values()) {
                        words
                            .next()
                            .ok_or_else(|| format!("missing {word} value in {syntax}"))?;
                    }
                    continue;
                }
                leaf = leaf
                    .find_subcommand(word)
                    .ok_or_else(|| format!("unknown command `{word}` in {syntax}"))?;
                path.push(' ');
                path.push_str(word);
            }
            let literal_args: Vec<_> = words
                .take_while(|word| !word.starts_with(['[', '<', '-']))
                .collect();
            let literal_positionals_valid = !literal_args.is_empty()
                && root
                    .clone()
                    .try_get_matches_from(
                        path.split_whitespace().chain(literal_args.iter().copied()),
                    )
                    .is_ok();
            let allowed: BTreeSet<_> = leaf
                .get_arguments()
                .filter_map(|arg| arg.get_long().map(str::to_owned))
                .collect();
            let mut actual = options(syntax);
            let unknown: Vec<_> = actual.difference(&allowed).collect();
            if !unknown.is_empty() {
                return Err(format!(
                    "{} documents unsupported options in {path}: {unknown:?}",
                    row.operation.id
                ));
            }
            for arg in leaf.get_arguments().filter(|arg| {
                arg.get_num_args().is_some_and(|arity| arity.takes_values())
                    && arg.get_long().is_some()
            }) {
                let flag = format!("--{}", arg.get_long().unwrap());
                for (index, _) in syntax.match_indices(&flag) {
                    let after = &syntax[index + flag.len()..];
                    // A longer option with the same prefix is a different flag.
                    if after.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-') {
                        continue;
                    }
                    let value = after.split_whitespace().next().unwrap_or("");
                    if !value.starts_with('<') {
                        let literal = value.trim_end_matches(']');
                        let valid = !literal.is_empty() && !literal.starts_with(['[', '-', ']'])
                            || literal == "-";
                        let parser = Command::new("value").arg(
                            clap::Arg::new("value")
                                .allow_hyphen_values(true)
                                .required(true)
                                .value_parser(arg.get_value_parser().clone()),
                        );
                        if valid && parser.try_get_matches_from(["value", literal]).is_ok() {
                            continue;
                        }
                        return Err(format!(
                            "{} is missing a value placeholder or valid literal for {flag} in {path}",
                            row.operation.id
                        ));
                    }
                }
            }
            for arg in leaf.get_positionals() {
                // Some routes document a concrete positional value, such as
                // `auth preflight login`. Check it with the real parser.
                if literal_positionals_valid
                    && arg
                        .get_index()
                        .is_some_and(|index| index <= literal_args.len())
                {
                    continue;
                }
                let name = arg
                    .get_value_names()
                    .and_then(|names| names.first())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| arg.get_id().as_str().to_ascii_uppercase());
                let syntax = syntax.replace("\\|", "|");
                if !syntax.split('<').skip(1).any(|rest| {
                    rest.split('>')
                        .next()
                        .is_some_and(|value| value.split('|').any(|part| part == name))
                }) {
                    return Err(format!(
                        "{} is missing positional <{name}> in {path}",
                        row.operation.id
                    ));
                }
            }
            actual.extend(annotations.intersection(&allowed).cloned());
            documented
                .entry(path)
                .or_insert_with(|| (leaf, BTreeSet::new()))
                .1
                .extend(actual);
        }
        if !has_command {
            return Err(format!(
                "{} has no equivalent CLI command",
                row.operation.id
            ));
        }
    }
    for (path, (leaf, actual)) in documented {
        let expected: BTreeSet<_> = leaf
            .get_arguments()
            .filter_map(|arg| arg.get_long().map(str::to_owned))
            .collect();
        let missing: Vec<_> = expected.difference(&actual).collect();
        if !missing.is_empty() {
            return Err(format!("missing options in {path}: {missing:?}"));
        }
    }
    Ok(())
}

fn check_operations(rows: &[Row], expected: &BTreeSet<Operation>) -> Result<(), String> {
    let documented: BTreeSet<_> = rows.iter().map(|row| row.operation.clone()).collect();
    if &documented != expected {
        return Err(format!(
            "coverage differs from the corrected contract; undocumented: {:?}; stale: {:?}",
            expected.difference(&documented).collect::<Vec<_>>(),
            documented.difference(expected).collect::<Vec<_>>()
        ));
    }
    Ok(())
}

#[test]
fn every_documented_command_matches_the_parser_and_totals() {
    let mut root = Cli::command();
    root.build();
    check_commands(&rows(COVERAGE).unwrap(), &root).unwrap();
}

#[test]
fn coverage_rejects_unescaped_gfm_pipes_inside_inline_code() {
    assert!(rows(&COVERAGE.replacen(r"\|", "|", 1)).is_err());
}

#[test]
#[ignore = "requires WEKAN_CORRECTED_SPEC pointing to a freshly generated corrected contract"]
fn generated_contract_matches_documented_operations() {
    let path = std::env::var_os("WEKAN_CORRECTED_SPEC")
        .expect("generate the corrected specification and set WEKAN_CORRECTED_SPEC to its path");
    let file =
        std::fs::File::open(path).expect("the generated corrected contract must be readable");
    let spec: serde_yaml_ng::Value = serde_yaml_ng::from_reader(file).unwrap();
    let paths = spec["paths"]
        .as_mapping()
        .expect("the corrected contract must define paths");
    let mut expected = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut exclusions = BTreeSet::new();
    let documented = rows(COVERAGE).unwrap();
    for (path, item) in paths {
        let path = path.as_str().expect("API paths must be strings");
        for method in METHODS {
            if let Some(operation) = item.get(method) {
                let id = operation["operationId"]
                    .as_str()
                    .expect("every operation must have an ID");
                assert!(
                    !id.is_empty() && ids.insert(id.to_owned()),
                    "invalid or duplicate operation ID: {id}"
                );
                let excluded = operation["x-cli-excluded"].as_bool().unwrap_or(false);
                let row = documented
                    .iter()
                    .find(|row| row.operation.id == id)
                    .expect("every corrected operation must have a coverage row");
                assert_eq!(row.excluded, excluded, "exclusion status differs for {id}");
                if excluded {
                    assert!(!row.implemented);
                    assert!(
                        operation["x-cli-exclusion-reason"]
                            .as_str()
                            .is_some_and(|reason| !reason.is_empty())
                    );
                    exclusions.insert(id.to_owned());
                }
                expected.insert(Operation {
                    method: method.to_ascii_uppercase(),
                    path: path.into(),
                    id: id.into(),
                });
            }
        }
    }
    assert!(
        !expected.is_empty(),
        "the corrected contract has no operations"
    );
    assert_eq!(
        exclusions,
        BTreeSet::from(["submit_ses_notification".to_owned()])
    );
    check_operations(&documented, &expected).unwrap();
}

#[test]
fn coverage_checks_reject_inventory_summary_and_command_drift() {
    let original = rows(COVERAGE).unwrap();
    let expected = original.iter().map(|row| row.operation.clone()).collect();
    for change in ["method", "path", "id", "missing", "extra"] {
        let mut changed = original.clone();
        match change {
            "method" => changed[0].operation.method = "GET".into(),
            "path" => changed[0].operation.path.push_str("/stale"),
            "id" => changed[0].operation.id.push_str("_stale"),
            "missing" => {
                changed.pop();
            }
            "extra" => {
                let mut extra = changed[0].clone();
                extra.operation.id = "extra".into();
                changed.push(extra);
            }
            _ => unreachable!(),
        }
        assert!(
            check_operations(&changed, &expected).is_err(),
            "accepted {change} operation drift"
        );
    }
    assert!(rows(&COVERAGE.replacen("**Coverage:**", "**Stale coverage:**", 1)).is_err());
    assert!(rows(&COVERAGE.replacen("| Excluded |", "| ✓ |", 1)).is_err());
    let mut root = Cli::command();
    root.build();
    for (before, after) in [
        ("--username", "--typo"),
        ("[--help]", ""),
        ("<USERNAME>", ""),
    ] {
        let mut changed = original.clone();
        changed[0].syntax = changed[0].syntax.replace(before, after);
        assert!(
            check_commands(&changed, &root).is_err(),
            "accepted command drift: {before}"
        );
    }
}
