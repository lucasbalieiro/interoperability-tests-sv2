# interoperability-tests-sv2

Compliance testing for Stratum V2 implementations. Connects to Sv2 endpoints (pools, Job Declarator Servers, Template Providers) and runs a growing suite of spec-conformance scenarios against them, reporting pass/fail per scenario. The endpoints to test are listed in a TOML config file.

Built on top of [`integration_tests_sv2`](https://crates.io/crates/integration_tests_sv2).

## CLI

```sh
cargo run --bin sv2-compliance -- --config targets.toml
```

`--config` defaults to `targets.toml` in the current directory. Addresses accept either an IP or a hostname followed by a port (e.g. `stratum.ckpool.org:3336`); hostnames are resolved before the run starts, and an unresolvable one is reported as a failed target without stopping the remaining sites. Exits `0` when no scenario fails and every target resolves, `1` otherwise; a scenario the endpoint legitimately did not let the client exercise (e.g. a standard-channel scenario against a server that set `REQUIRES_EXTENDED_CHANNELS`) is reported but does not fail the run. See `targets.example.toml` for the config format and `--help` for all options.

## Library

The scenario suite is a public API: third-party implementers can add this crate as a dev-dependency and self-certify in their own CI:

```rust
#[tokio::test]
async fn sv2_compliance() {
    let endpoint = interoperability_tests_sv2::endpoint::Endpoint {
        addr: "127.0.0.1:34254".parse().unwrap(),
        app_type: interoperability_tests_sv2::app_type::AppType::MiningServer,
        user_identity: "my_miner".into(),
    };
    let reports = interoperability_tests_sv2::runner::run_suite(&endpoint).await;
    assert!(!reports.iter().any(|r| r.failed()));
}
```

For TOML batches, `interoperability_tests_sv2::config::Config::from_path` parses the file and `resolve_targets().await` returns the resolved targets plus any hostnames that failed to resolve.

## Running the tests

`cargo test` spins up a local SRI Template Provider and Pool (binaries under `template-provider/` are downloaded on first run) and runs the suite against them.

Set `SV2_TEST_ENDPOINT=host:port` (optionally `SV2_TEST_APP_TYPE`, `SV2_TEST_USER_IDENTITY`) to also run `tests/remote_endpoint.rs` against a remote endpoint.

## Adding scenarios

One module per Sv2 message family under `src/scenarios/{common,mining,...}/`. Scenario IDs are the stable `Proto-MSG-N` references from `COVERAGE.md` (e.g. `"C-SCS-2 used-version-within-range"`). Each scenario is an async fn returning `ScenarioResult` with a `/// Covers: <ID>` doc comment.

Register it in the appropriate `entries()` fn in its submodule, then wire the submodule into the suite fn(s) in `src/scenarios/mod.rs`. Scenario fns are registered only once; suites compose them by inclusion, not duplication.
