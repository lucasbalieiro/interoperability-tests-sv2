mod common;

use integration_tests_sv2::{
    start_pool, start_template_provider, start_tracing, sv2_tp_config,
    template_provider::DifficultyLevel,
};
use interoperability_tests_sv2::{config::Config, runner::run_suite};

/// The TOML batch path used by the CLI, with IP and hostname targets.
#[tokio::test]
async fn config_driven_run_against_pool() {
    start_tracing();
    let (_tp, tp_addr) = start_template_provider(None, DifficultyLevel::Low);
    let (pool, pool_addr, _) = start_pool(sv2_tp_config(tp_addr), vec![], vec![], false).await;

    let raw = format!(
        r#"
[[site]]
name = "Local Pool"
mining_server_address = "{pool_addr}"

[[site]]
name = "Local Pool by hostname"
mining_server_address = "localhost:{}"
"#,
        pool_addr.port()
    );

    let config: Config = toml::from_str(&raw).expect("valid toml");
    let targets = config.resolve_targets().await;
    assert!(targets.unresolved.is_empty());
    assert_eq!(targets.resolved.len(), 2);

    for target in &targets.resolved {
        let reports = run_suite(&target.endpoint).await;
        common::assert_suite(&reports);
    }

    pool.shutdown().await;
}
