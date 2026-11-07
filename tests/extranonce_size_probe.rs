//! Regression tests for https://github.com/stratum-mining/interoperability-tests-sv2/issues/18.
//!
//! A pool that answers `OpenExtendedMiningChannel` with a `.Success` carrying
//! `extranonce_size` below the requested `min_extranonce_size` violates §5.3.4
//! and must be reported as non-compliant by M-OEMCS-3.

use std::net::SocketAddr;

use integration_tests_sv2::{
    mock_roles::{channel_open_success, setup_connection_success, MockUpstream, WithSetup},
    utils::get_available_address,
};
use interoperability_tests_sv2::{
    app_type::AppType,
    endpoint::Endpoint,
    scenarios::{mining_server, ScenarioError},
};
use stratum_apps::{
    stratum_core::{
        mining_sv2::OpenExtendedMiningChannelSuccessOwned,
        parsers_sv2::{AnyMessageOwned, CommonMessagesOwned, MiningOwned},
    },
    utils::types::OutboundFrame,
};

#[tokio::test]
async fn rejects_pool_that_caps_extranonce_size_at_thirty_one() {
    assert_scenario_rejects_cap(31, 32).await;
}

#[tokio::test]
async fn rejects_pool_that_caps_extranonce_size_at_six() {
    assert_scenario_rejects_cap(6, 10).await;
}

async fn assert_scenario_rejects_cap(cap: u16, expected_min: u16) {
    let addr = get_available_address();
    start_capped_pool(addr, cap).await;

    let endpoint = Endpoint {
        addr,
        app_type: AppType::MiningServer,
        user_identity: "interoperability_tests_sv2".to_string(),
    };

    let (_, scenario) = mining_server()
        .into_iter()
        .find(|(id, _)| id.starts_with("M-OEMCS-3"))
        .expect("M-OEMCS-3 is registered in the mining server suite");

    let error: ScenarioError = scenario(endpoint)
        .await
        .expect_err("a pool that silently caps extranonce_size must fail M-OEMCS-3");

    let error = error.to_string();

    assert!(
        error.contains(&format!("min_extranonce_size={expected_min}"))
            && error.contains(&format!("extranonce_size={cap}")),
        "unexpected failure message: {error}"
    );
}

/// Starts a Mining Protocol Server that always grants exactly `cap` bytes of extranonce space,
/// regardless of the requested `min_extranonce_size`. This mirrors a server with a fixed-size
/// extranonce, as in issue #18.
async fn start_capped_pool(address: SocketAddr, cap: u16) {
    MockUpstream::new(address, WithSetup::no())
        .with_handler(move |message| match message {
            AnyMessageOwned::Common(CommonMessagesOwned::SetupConnection(_)) => {
                vec![setup_connection_success()]
            }
            AnyMessageOwned::Mining(MiningOwned::OpenExtendedMiningChannel(open)) => {
                vec![OutboundFrame::from_message(AnyMessageOwned::Mining(
                    MiningOwned::OpenExtendedMiningChannelSuccess(
                        OpenExtendedMiningChannelSuccessOwned {
                            extranonce_size: cap,
                            ..channel_open_success(&open)
                        },
                    ),
                ))
                .expect("OpenExtendedMiningChannel.Success can be framed")]
            }
            _ => vec![],
        })
        .start()
        .await;
}
