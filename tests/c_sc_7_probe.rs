//! C-SC-7 against mock endpoints with different protocol deployments.

use std::sync::Arc;

use integration_tests_sv2::{
    mocks::{ConnectionId, MockServer, ServerSetupConnection},
    utils::get_available_address,
};
use interoperability_tests_sv2::{
    app_type::AppType,
    endpoint::Endpoint,
    probe,
    scenarios::{
        common::setup_connection::unsupported_protocol_rejected, ScenarioError, ScenarioResult,
    },
};
use stratum_apps::stratum_core::{
    job_declaration_sv2::AllocateMiningJobTokenSuccessOwned,
    mining_sv2::OpenMiningChannelErrorOwned,
    parsers_sv2::{
        AnyMessageOwned, IsSv2Message, JobDeclarationOwned, MiningOwned, TemplateDistributionOwned,
    },
    template_distribution_sv2::SetNewPrevHashOwned,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Reply {
    /// Answer the probe with a message from the probed protocol's family.
    Serves,
    /// Leave the connection open and answer nothing.
    Silent,
    /// Answer the probe with a message from another protocol family.
    WrongFamily,
    /// Close the connection as soon as the endpoint accepts the probe's `SetupConnection`.
    CloseBeforeProbe,
    /// Close the connection after receiving the probe's opening request.
    CloseAfterProbe,
}

#[tokio::test]
async fn passes_when_other_protocols_are_rejected() {
    let result = run(vec![], Reply::Silent).await;
    let detail = result.expect("rejections pass").expect("pass detail");
    assert!(detail.contains("protocol=1"), "{detail}");
    assert!(detail.contains("protocol=2"), "{detail}");
    assert!(detail.contains("rejected"), "{detail}");
}

#[tokio::test]
async fn passes_when_endpoint_also_serves_job_declaration() {
    let result = run(vec![AppType::JobDeclarationServer], Reply::Serves).await;
    let detail = result
        .expect("multi-protocol endpoint passes")
        .expect("pass detail");
    assert!(detail.contains("protocol=1"), "{detail}");
    assert!(detail.contains("also served"), "{detail}");
    assert!(detail.contains("protocol=2"), "{detail}");
    assert!(detail.contains("rejected"), "{detail}");
}

#[tokio::test]
async fn passes_when_endpoint_serves_both_other_protocols() {
    let result = run(
        vec![AppType::JobDeclarationServer, AppType::TemplateProvider],
        Reply::Serves,
    )
    .await;
    let detail = result
        .expect("multi-protocol endpoint passes")
        .expect("pass detail");
    assert_eq!(detail.matches("also served").count(), 2, "{detail}");
}

#[tokio::test]
async fn job_declaration_silence_is_not_exercised() {
    match run(vec![AppType::JobDeclarationServer], Reply::Silent).await {
        Err(ScenarioError::NotExercised(reason)) => {
            assert!(reason.contains("protocol=1"), "{reason}");
            assert!(reason.contains("Job Declaration"), "{reason}");
            assert!(reason.contains("sent no"), "{reason}");
            assert!(!reason.contains("connection closed"), "{reason}");
            // The Template Distribution probe still rejects the endpoint.
            assert!(reason.contains("protocol=2"), "{reason}");
        }
        other => panic!("expected a not-exercised result, got {other:?}"),
    }
}

#[tokio::test]
async fn template_distribution_silence_is_a_failure() {
    match run(vec![AppType::TemplateProvider], Reply::Silent).await {
        Err(ScenarioError::Failed(reason)) => {
            assert!(reason.contains("protocol=2"), "{reason}");
            assert!(reason.contains("CoinbaseOutputConstraints"), "{reason}");
            assert!(reason.contains("did not answer"), "{reason}");
            assert!(!reason.contains("connection closed"), "{reason}");
        }
        other => panic!("expected a failed result, got {other:?}"),
    }
}

#[tokio::test]
async fn wrong_protocol_family_is_a_failure() {
    match run(vec![AppType::JobDeclarationServer], Reply::WrongFamily).await {
        Err(ScenarioError::Failed(reason)) => {
            assert!(reason.contains("protocol=1"), "{reason}");
            assert!(reason.contains("Mining Protocol"), "{reason}");
        }
        other => panic!("expected a failed result, got {other:?}"),
    }
}

#[tokio::test]
async fn job_declaration_target_probes_mining_and_template_distribution() {
    let result = run_as(
        AppType::JobDeclarationServer,
        vec![AppType::MiningServer, AppType::TemplateProvider],
        Reply::Serves,
    )
    .await;
    let detail = result
        .expect("multi-protocol endpoint passes")
        .expect("pass detail");
    assert_eq!(detail.matches("also served").count(), 2, "{detail}");
    assert!(detail.contains("protocol=0"), "{detail}");
    assert!(detail.contains("protocol=2"), "{detail}");
}

#[tokio::test]
async fn connection_closed_while_waiting_is_not_exercised() {
    match run(vec![AppType::TemplateProvider], Reply::CloseAfterProbe).await {
        Err(ScenarioError::NotExercised(reason)) => {
            assert!(reason.contains("protocol=2"), "{reason}");
            assert!(
                reason.contains("connection closed before a reply"),
                "{reason}"
            );
        }
        other => panic!("expected a not-exercised result, got {other:?}"),
    }
}

#[tokio::test]
async fn connection_closed_before_the_probe_is_not_exercised() {
    match run(vec![AppType::TemplateProvider], Reply::CloseBeforeProbe).await {
        Err(ScenarioError::NotExercised(reason)) => {
            assert!(reason.contains("protocol=2"), "{reason}");
            assert!(
                reason.contains("connection closed before a reply"),
                "{reason}"
            );
        }
        other => panic!("expected a not-exercised result, got {other:?}"),
    }
}

async fn run(accepted: Vec<AppType>, reply: Reply) -> ScenarioResult {
    run_as(AppType::MiningServer, accepted, reply).await
}

async fn run_as(app_type: AppType, accepted: Vec<AppType>, reply: Reply) -> ScenarioResult {
    let address = get_available_address();
    let setup = ServerSetupConnection::new(
        accepted
            .iter()
            .map(|app_type| (app_type.protocol(), app_type.default_flags())),
    );
    let server = Arc::new(MockServer::new(address, Some(setup)).await);
    let responder = tokio::spawn(respond(server.clone(), app_type, accepted, reply));

    let endpoint = Endpoint {
        addr: address,
        app_type,
        user_identity: "interoperability_tests_sv2".to_string(),
    };
    let result = unsupported_protocol_rejected(endpoint).await;
    responder.abort();
    drop(server);
    result
}

/// Answers the probes the endpoint's `SetupConnection` configuration accepted, in the order
/// C-SC-7 opens their connections.
async fn respond(server: Arc<MockServer>, target: AppType, accepted: Vec<AppType>, reply: Reply) {
    for (id, probed) in probe_order(target).into_iter().enumerate() {
        let id = id as ConnectionId;
        server.wait_for_connection(id).await;
        if !accepted.contains(&probed) {
            // The server already answered `SetupConnection.Error` and closed.
            continue;
        }
        if reply == Reply::CloseBeforeProbe {
            server.close(id).await;
            continue;
        }

        let opening = probe::opening_request(probed, "interoperability_tests_sv2")
            .expect("the probe's opening request is well formed");
        server
            .wait_for_message_type(id, opening.message_type())
            .await;

        match reply {
            Reply::Serves => server.send(id, serving_reply(probed)).await,
            Reply::WrongFamily => server.send(id, wrong_family_reply(probed)).await,
            Reply::Silent => {}
            Reply::CloseAfterProbe => server.close(id).await,
            Reply::CloseBeforeProbe => unreachable!("handled before the opening request"),
        }
    }
}

fn probe_order(target: AppType) -> Vec<AppType> {
    AppType::ALL
        .into_iter()
        .filter(|app_type| *app_type != target)
        .collect()
}

fn serving_reply(probed: AppType) -> AnyMessageOwned {
    match probed {
        AppType::MiningServer => mining_reply(),
        AppType::JobDeclarationServer => job_declaration_reply(),
        AppType::TemplateProvider => template_distribution_reply(),
    }
}

fn wrong_family_reply(probed: AppType) -> AnyMessageOwned {
    match probed {
        AppType::MiningServer => job_declaration_reply(),
        AppType::JobDeclarationServer | AppType::TemplateProvider => mining_reply(),
    }
}

fn mining_reply() -> AnyMessageOwned {
    AnyMessageOwned::Mining(MiningOwned::OpenMiningChannelError(
        OpenMiningChannelErrorOwned {
            request_id: 1,
            error_code: "probe"
                .to_string()
                .try_into()
                .expect("the error code fits in STR0_255"),
        },
    ))
}

fn job_declaration_reply() -> AnyMessageOwned {
    AnyMessageOwned::JobDeclaration(JobDeclarationOwned::AllocateMiningJobTokenSuccess(
        AllocateMiningJobTokenSuccessOwned {
            request_id: 1,
            mining_job_token: Vec::<u8>::new()
                .try_into()
                .expect("an empty token fits in B0_255"),
            coinbase_outputs: Vec::<u8>::new()
                .try_into()
                .expect("empty outputs fit in B0_64K"),
        },
    ))
}

fn template_distribution_reply() -> AnyMessageOwned {
    AnyMessageOwned::TemplateDistribution(TemplateDistributionOwned::SetNewPrevHash(
        SetNewPrevHashOwned {
            template_id: 0,
            prev_hash: [0_u8; 32].into(),
            ntime_start: 0,
            n_bits: 0,
            target: [0_u8; 32].into(),
        },
    ))
}
