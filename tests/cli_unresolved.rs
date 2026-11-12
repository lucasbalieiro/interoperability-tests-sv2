use std::process::Command;

fn run_cli(config_name: &str, contents: &str) -> String {
    let config = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(config_name);
    std::fs::write(&config, contents).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sv2-compliance"))
        .args(["--config", config.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    stdout.into_owned()
}

#[test]
fn cli_reports_unresolved_target_and_exits_nonzero() {
    let stdout = run_cli(
        "unresolved.toml",
        r#"
[[site]]
name = "Unresolved"
mining_server_address = "unreachable-hostname.invalid:3333"
"#,
    );

    assert!(stdout.contains("Unresolved"), "stdout: {stdout}");
    assert!(
        stdout.contains("unreachable-hostname.invalid:3333"),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains("failed to lookup address information"),
        "default output should not expose resolver detail: {stdout}"
    );
}

#[test]
fn cli_shows_resolved_ip_for_hostname() {
    let stdout = run_cli(
        "resolved-hostname.toml",
        r#"
[[site]]
name = "Resolved"
mining_server_address = "localhost:9"
"#,
    );

    assert!(
        stdout.contains("Resolved: MiningServer at localhost:9 (resolved to 127.0.0.1:9)"),
        "stdout: {stdout}"
    );
}

#[test]
fn cli_reports_no_resolved_targets() {
    let stdout = run_cli(
        "no-resolved.toml",
        r#"
[[site]]
name = "Unresolved"
mining_server_address = "unreachable-hostname.invalid:3333"
"#,
    );

    assert!(
        stdout.contains("no targets resolved (1 unresolved)"),
        "stdout: {stdout}"
    );
    assert!(!stdout.contains("0/0 scenarios passed"), "stdout: {stdout}");
}
