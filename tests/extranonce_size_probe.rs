//! Regression tests for https://github.com/stratum-mining/interoperability-tests-sv2/issues/18.
//!
//! A pool that answers `OpenExtendedMiningChannel` with a `.Success` carrying
//! `extranonce_size` below the requested `min_extranonce_size` violates §5.3.4
//! and must be reported as non-compliant by M-OEMCS-3.

use integration_tests_sv2::{
    mocks::{MockServer, ServerSetupConnection},
    utils::get_available_address,
};
use interoperability_tests_sv2::{
    app_type::AppType,
    endpoint::Endpoint,
    scenarios::{mining_server, ScenarioError},
};
use stratum_apps::stratum_core::{
    common_messages_sv2::Protocol,
    mining_sv2::{
        OpenExtendedMiningChannelSuccessOwned, MESSAGE_TYPE_OPEN_EXTENDED_MINING_CHANNEL,
    },
    parsers_sv2::{AnyMessageOwned, MiningOwned},
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
    let pool = MockServer::new(
        addr,
        Some(ServerSetupConnection::new([(Protocol::MiningProtocol, 0)])),
    )
    .await;

    let endpoint = Endpoint {
        addr,
        app_type: AppType::MiningServer,
        user_identity: "interoperability_tests_sv2".to_string(),
    };

    let (_, scenario) = mining_server()
        .into_iter()
        .find(|(id, _)| id.starts_with("M-OEMCS-3"))
        .expect("M-OEMCS-3 is registered in the mining server suite");

    let error: ScenarioError = tokio::select! {
        () = serve_capped_extranonce(&pool, cap) => {
            panic!("the mock pool stopped before the scenario finished")
        }
        error = scenario(endpoint) => {
            error.expect_err("a pool that silently caps extranonce_size must fail M-OEMCS-3")
        }
    };

    let error = error.to_string();

    assert!(
        error.contains(&format!("min_extranonce_size={expected_min}"))
            && error.contains(&format!("extranonce_size={cap}")),
        "unexpected failure message: {error}"
    );
}

/// Answers every `OpenExtendedMiningChannel` with a `.Success` that grants
/// exactly `cap` bytes of extranonce space, regardless of the requested
/// `min_extranonce_size`. This mirrors a server with a fixed-size extranonce.
async fn serve_capped_extranonce(pool: &MockServer, cap: u16) {
    let mut connection = 0;
    loop {
        pool.wait_for_message_type(connection, MESSAGE_TYPE_OPEN_EXTENDED_MINING_CHANNEL)
            .await;

        let open = loop {
            if let Some((
                _,
                AnyMessageOwned::Mining(MiningOwned::OpenExtendedMiningChannel(open)),
            )) = pool.next_message(connection)
            {
                break open;
            }
        };

        let open_success = AnyMessageOwned::Mining(MiningOwned::OpenExtendedMiningChannelSuccess(
            OpenExtendedMiningChannelSuccessOwned {
                request_id: open.request_id,
                channel_id: 1,
                target: [0xff_u8; 32].into(),
                extranonce_prefix: vec![0_u8; 4].try_into().unwrap(),
                group_channel_id: 0,
                extranonce_size: cap,
            },
        ));
        pool.send(connection, open_success).await;

        connection += 1;
    }
}
