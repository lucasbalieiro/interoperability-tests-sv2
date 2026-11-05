//! Guards `COVERAGE.md` against drift from the scenario registry.
//!
//! The [Suite Coverage] table is the public statement of which scenarios are
//! implemented, while the registry in `src/scenarios/**` is what actually runs.
//! These tests fail when the two disagree in either direction, when an ID has
//! no Scenario Catalog row, or when a scenario fn is written but never
//! registered.
//!
//! [Suite Coverage]: ../COVERAGE.md#suite-coverage

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use interoperability_tests_sv2::scenarios::{self, ScenarioFn};

const COVERAGE_MD: &str = include_str!("../COVERAGE.md");
const SUITE_COVERAGE_HEADING: &str = "## Suite Coverage";
const CATALOG_HEADER: [&str; 7] = [
    "Scenario IDs",
    "Compliance tier",
    "Testability tier",
    "Failure mode",
    "Spec basis",
    "Detection",
    "Involves",
];

type Registry = fn() -> Vec<(&'static str, ScenarioFn)>;

/// Maps each Suite Coverage column to the registry of the suite it describes.
///
/// A suite column added to `COVERAGE.md` must get an entry here;
/// [`suite_coverage_columns_are_wired`] enforces that.
fn suites() -> &'static [(&'static str, Registry)] {
    &[("Mining Protocol Server", scenarios::mining_server)]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    Covered,
    Uncovered,
}

// ---------------------------------------------------------------------------
// registry side
// ---------------------------------------------------------------------------

/// IDs registered in one suite, taken from the token before the title slug in
/// each `entries()` tuple (e.g. `"C-SCS-2 used-version-within-range"`).
fn registered_ids(suite: Registry) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for (entry, _) in suite() {
        let id = entry
            .split_whitespace()
            .next()
            .unwrap_or_else(|| panic!("registry entry {entry:?} has no scenario ID"));
        assert!(
            is_scenario_id(id),
            "malformed scenario ID in registry entry {entry:?}"
        );
        assert!(
            ids.insert(id.to_string()),
            "duplicate scenario ID in suite registry: {id}"
        );
    }
    ids
}

fn all_registered() -> BTreeSet<String> {
    suites()
        .iter()
        .flat_map(|(_, suite)| registered_ids(*suite))
        .collect()
}

/// IDs that have a `/// Covers: <ID>` annotation under `src/scenarios/`.
fn documented_ids() -> BTreeMap<String, PathBuf> {
    fn visit(dir: &Path, found: &mut BTreeMap<String, PathBuf>) {
        let entries =
            fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("readable directory entry").path();
            if path.is_dir() {
                visit(&path, found);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let text = fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
                for (line_number, line) in text.lines().enumerate() {
                    let Some(rest) = line.trim().strip_prefix("/// Covers:") else {
                        continue;
                    };
                    let id = rest.trim();
                    assert!(
                        is_scenario_id(id),
                        "{}:{}: malformed `Covers:` ID {id:?}",
                        path.display(),
                        line_number + 1
                    );
                    if let Some(previous) = found.insert(id.to_string(), path.clone()) {
                        panic!(
                            "duplicate `Covers: {id}` in {} and {}",
                            previous.display(),
                            path.display()
                        );
                    }
                }
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/scenarios");
    let mut found = BTreeMap::new();
    visit(&root, &mut found);
    assert!(
        !found.is_empty(),
        "no `/// Covers:` annotations found under {}",
        root.display()
    );
    found
}

// ---------------------------------------------------------------------------
// COVERAGE.md side
// ---------------------------------------------------------------------------

/// `## <heading>` body, up to the next level-2 heading.
fn section_lines<'a>(md: &'a str, heading: &str) -> Vec<&'a str> {
    let lines: Vec<&str> = md.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.trim() == heading)
        .unwrap_or_else(|| panic!("COVERAGE.md is missing the {heading:?} heading"));
    let end = lines[start + 1..]
        .iter()
        .position(|line| line.trim_start().starts_with("## "))
        .map_or(lines.len(), |offset| start + 1 + offset);
    lines[start + 1..end].to_vec()
}

/// Every row of the Suite Coverage table's given column, with its mark.
fn suite_coverage(md: &str, column: &str) -> BTreeMap<String, Mark> {
    let lines = section_lines(md, SUITE_COVERAGE_HEADING);
    let table = markdown_tables(&lines)
        .into_iter()
        .find(|table| table.header.contains(&column))
        .unwrap_or_else(|| panic!("Suite Coverage table has no {column:?} column"));
    let column = table
        .header
        .iter()
        .position(|header| *header == column)
        .expect("column position");
    assert!(
        !table.rows.is_empty(),
        "Suite Coverage table has no scenario rows"
    );

    let mut marks = BTreeMap::new();
    for row in &table.rows {
        assert_eq!(
            row.len(),
            table.header.len(),
            "malformed Suite Coverage row: {row:?}"
        );
        let id = row[0].split_whitespace().next().unwrap_or_default();
        assert!(
            is_scenario_id(id),
            "Suite Coverage row does not start with a scenario ID: {row:?}"
        );
        let mark = match row[column] {
            "✅" => Mark::Covered,
            "❌" => Mark::Uncovered,
            other => panic!("unexpected mark {other:?} for {id}; expected ✅ or ❌"),
        };
        assert!(
            marks.insert(id.to_string(), mark).is_none(),
            "duplicate Suite Coverage row for {id}"
        );
    }
    marks
}

/// IDs from every Scenario Catalog table in `COVERAGE.md`.
fn catalog_ids(md: &str) -> BTreeSet<String> {
    let lines: Vec<&str> = md.lines().collect();
    let mut ids = BTreeSet::new();
    let mut tables = 0;
    for table in markdown_tables(&lines) {
        if table.header != CATALOG_HEADER {
            continue;
        }
        tables += 1;
        for row in &table.rows {
            assert_eq!(
                row.len(),
                CATALOG_HEADER.len(),
                "malformed Scenario Catalog row: {row:?}"
            );
            let id = row[0];
            assert!(is_scenario_id(id), "malformed Scenario Catalog ID {id:?}");
            assert!(
                ids.insert(id.to_string()),
                "duplicate Scenario Catalog row for {id}"
            );
        }
    }
    assert!(
        tables > 0,
        "no Scenario Catalog tables found in COVERAGE.md"
    );
    ids
}

// ---------------------------------------------------------------------------
// checks
// ---------------------------------------------------------------------------

#[test]
fn suite_coverage_matches_registry() {
    for &(column_name, suite) in suites() {
        let registered = registered_ids(suite);
        let marks = suite_coverage(COVERAGE_MD, column_name);

        let missing_row: Vec<&str> = registered
            .iter()
            .filter(|id| !marks.contains_key(id.as_str()))
            .map(String::as_str)
            .collect();
        let uncovered: Vec<&str> = registered
            .iter()
            .filter(|id| marks.get(id.as_str()) == Some(&Mark::Uncovered))
            .map(String::as_str)
            .collect();
        let stale: Vec<&str> = marks
            .iter()
            .filter(|(id, mark)| **mark == Mark::Covered && !registered.contains(id.as_str()))
            .map(|(id, _)| id.as_str())
            .collect();

        assert!(
            missing_row.is_empty() && uncovered.is_empty() && stale.is_empty(),
            "COVERAGE.md Suite Coverage drift for {column_name:?}:\n  \
             implemented but missing a Suite Coverage row: {missing_row:?}\n  \
             implemented but still marked ❌: {uncovered:?}\n  \
             marked ✅ but not implemented: {stale:?}\n\
             Update COVERAGE.md or the scenario registry so they agree."
        );
    }
}

#[test]
fn suite_coverage_columns_are_wired() {
    let lines = section_lines(COVERAGE_MD, SUITE_COVERAGE_HEADING);
    let table = markdown_tables(&lines)
        .into_iter()
        .next()
        .expect("COVERAGE.md has a Suite Coverage table");
    let mapped: BTreeSet<&str> = suites().iter().map(|(name, _)| *name).collect();
    let unmapped: Vec<&str> = table
        .header
        .iter()
        .skip(1)
        .copied()
        .filter(|column| !mapped.contains(*column))
        .collect();
    assert!(
        unmapped.is_empty(),
        "Suite Coverage columns without a registry mapping in tests/coverage.rs: {unmapped:?}"
    );
}

#[test]
fn coverage_rows_have_catalog_rows() {
    let catalog = catalog_ids(COVERAGE_MD);
    let registered = all_registered();
    let mut coverage_ids = BTreeSet::new();
    for &(column_name, _) in suites() {
        coverage_ids.extend(suite_coverage(COVERAGE_MD, column_name).into_keys());
    }

    let missing_registered = missing_from(&registered, &catalog);
    let missing_coverage = missing_from(&coverage_ids, &catalog);

    assert!(
        missing_registered.is_empty() && missing_coverage.is_empty(),
        "COVERAGE.md Scenario Catalog is missing rows:\n  \
         registered scenarios: {missing_registered:?}\n  \
         Suite Coverage IDs: {missing_coverage:?}\n\
         Add a row for each ID to the matching section of the Scenario Catalog."
    );
}

#[test]
fn documented_scenarios_are_registered() {
    let documented = documented_ids();
    let documented_set: BTreeSet<String> = documented.keys().cloned().collect();
    let registered = all_registered();

    let unregistered: Vec<String> = documented
        .iter()
        .filter(|(id, _)| !registered.contains(*id))
        .map(|(id, path)| {
            let relative = path
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .unwrap_or(path.as_path());
            format!("{id} ({})", relative.display())
        })
        .collect();
    let undocumented = missing_from(&registered, &documented_set);

    assert!(
        unregistered.is_empty() && undocumented.is_empty(),
        "scenario registry drift:\n  \
         scenario fns written but never registered (add them to `entries()` and the suite): \
         {unregistered:?}\n  \
         registry entries with no `/// Covers:` doc comment: {undocumented:?}"
    );
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn missing_from<'a>(ids: &'a BTreeSet<String>, known: &'a BTreeSet<String>) -> Vec<&'a str> {
    ids.difference(known).map(String::as_str).collect()
}

fn is_scenario_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id.split('-').count() >= 2
        && id
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
        && id
            .rsplit('-')
            .next()
            .is_some_and(|number| number.chars().all(|c| c.is_ascii_digit()))
}

struct Table<'a> {
    header: Vec<&'a str>,
    rows: Vec<Vec<&'a str>>,
}

/// Every GitHub-flavored markdown table in `lines`, ignoring fenced code blocks.
fn markdown_tables<'a>(lines: &[&'a str]) -> Vec<Table<'a>> {
    let mut tables = Vec::new();
    let mut index = 0;
    let mut in_code = false;
    while index < lines.len() {
        let line = lines[index].trim();
        if line.starts_with("```") {
            in_code = !in_code;
            index += 1;
            continue;
        }
        if in_code || !is_table_row(line) || index + 1 >= lines.len() {
            index += 1;
            continue;
        }
        if !is_separator(lines[index + 1]) {
            index += 1;
            continue;
        }
        let header = parse_row(line);
        index += 2;
        let mut rows = Vec::new();
        while index < lines.len() && is_table_row(lines[index]) {
            rows.push(parse_row(lines[index]));
            index += 1;
        }
        tables.push(Table { header, rows });
    }
    tables
}

fn is_table_row(line: &str) -> bool {
    let line = line.trim();
    line.starts_with('|') && line.ends_with('|')
}

fn is_separator(line: &str) -> bool {
    if !is_table_row(line) {
        return false;
    }
    let cells = parse_row(line);
    !cells.is_empty()
        && cells
            .iter()
            .all(|cell| !cell.is_empty() && cell.chars().all(|c| c == '-' || c == ':'))
}

fn parse_row(line: &str) -> Vec<&str> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect()
}
