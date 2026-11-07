//! Regression tests for https://github.com/stratum-mining/interoperability-tests-sv2/issues/18.
//!
//! A pool that answers `OpenExtendedMiningChannel` with a `.Success` carrying
//! `extranonce_size` below the requested `min_extranonce_size` violates §5.3.4
//! and must be reported as non-compliant by M-OEMCS-3.

use std::net::SocketAddr;

use integration_tests_sv2::utils::{create_downstream, get_available_address, message_from_frame};
use interoperability_tests_sv2::{
    app_type::AppType,
    endpoint::Endpoint,
    scenarios::{mining_server, ScenarioError},
};
use stratum_apps::stratum_core::{
    codec_sv2::MessageFrame,
    common_messages_sv2::SetupConnectionSuccessOwned,
    mining_sv2::OpenExtendedMiningChannelSuccessOwned,
    parsers_sv2::{AnyMessageOwned, CommonMessagesOwned, IsSv2Message, MiningOwned},
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

/// Starts a Mining Protocol Server that always grants exactly `cap` bytes of
/// extranonce space, regardless of the requested `min_extranonce_size`. This
/// mirrors a server with a fixed-size extranonce.
async fn start_capped_pool(address: SocketAddr, cap: u16) {
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("bind capped pool listener");

    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let Some((recv, send)) = create_downstream(stream).await else {
                    return;
                };

                // SetupConnection
                let mut frame = recv.recv().await.expect("SetupConnection frame");
                let _ = message_from_frame(&mut frame);
                let setup_success = AnyMessageOwned::Common(
                    CommonMessagesOwned::SetupConnectionSuccess(SetupConnectionSuccessOwned {
                        used_version: 2,
                        flags: 0,
                    }),
                );
                let msg_type = setup_success.message_type();
                let frame = MessageFrame::from_message(setup_success, msg_type, 0, false)
                    .expect("frame SetupConnection.Success");

                send.send(frame.into())
                    .await
                    .expect("send SetupConnection.Success");

                // OpenExtendedMiningChannel
                let mut frame = recv.recv().await.expect("OpenExtendedMiningChannel frame");
                let (_, msg) = message_from_frame(&mut frame);
                let AnyMessageOwned::Mining(MiningOwned::OpenExtendedMiningChannel(open)) = msg
                else {
                    panic!("expected OpenExtendedMiningChannel, got {msg}");
                };

                let open_success =
                    AnyMessageOwned::Mining(MiningOwned::OpenExtendedMiningChannelSuccess(
                        OpenExtendedMiningChannelSuccessOwned {
                            request_id: open.request_id,
                            channel_id: 1,
                            target: [0xff_u8; 32].into(),
                            extranonce_prefix: vec![0_u8; 4].try_into().unwrap(),
                            group_channel_id: 0,
                            extranonce_size: cap,
                        },
                    ));
                let msg_type = open_success.message_type();
                let frame = MessageFrame::from_message(open_success, msg_type, 0, false)
                    .expect("frame OpenExtendedMiningChannel.Success");

                send.send(frame.into())
                    .await
                    .expect("send OpenExtendedMiningChannel.Success");

                // Keep the connection alive until the scenario drops the client.
                while recv.recv().await.is_ok() {}
            });
        }
    });
}
