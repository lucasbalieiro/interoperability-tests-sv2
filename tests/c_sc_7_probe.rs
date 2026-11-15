//! C-SC-7 against mock endpoints with different protocol deployments.

use integration_tests_sv2::utils::{create_downstream, get_available_address, message_from_frame};
use interoperability_tests_sv2::{
    app_type::AppType,
    endpoint::Endpoint,
    scenarios::{
        common::setup_connection::unsupported_protocol_rejected, ScenarioError, ScenarioResult,
    },
};
use stratum_apps::{
    stratum_core::{
        common_messages_sv2::{
            SetupConnectionErrorOwned, SetupConnectionSuccessOwned,
            ERROR_CODE_SETUP_CONNECTION_UNSUPPORTED_PROTOCOL,
        },
        job_declaration_sv2::AllocateMiningJobTokenSuccessOwned,
        mining_sv2::OpenMiningChannelErrorOwned,
        parsers_sv2::{
            AnyMessageOwned, CommonMessagesOwned, JobDeclarationOwned, MiningOwned,
            TemplateDistributionOwned,
        },
        template_distribution_sv2::SetNewPrevHashOwned,
    },
    utils::types::OutboundFrame,
};
use tokio::net::{TcpListener, TcpStream};

#[derive(Clone, Copy)]
enum Reply {
    Serves,
    Silent,
    WrongFamily,
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
    let result = run(vec![1], Reply::Serves).await;
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
    let result = run(vec![1, 2], Reply::Serves).await;
    let detail = result
        .expect("multi-protocol endpoint passes")
        .expect("pass detail");
    assert_eq!(detail.matches("also served").count(), 2, "{detail}");
}

#[tokio::test]
async fn job_declaration_silence_is_not_exercised() {
    match run(vec![1], Reply::Silent).await {
        Err(ScenarioError::NotExercised(reason)) => {
            assert!(reason.contains("protocol=1"), "{reason}");
            assert!(reason.contains("Job Declaration"), "{reason}");
            // The Template Distribution probe still rejects the endpoint.
            assert!(reason.contains("protocol=2"), "{reason}");
        }
        other => panic!("expected a not-exercised result, got {other:?}"),
    }
}

#[tokio::test]
async fn template_distribution_silence_is_a_failure() {
    match run(vec![2], Reply::Silent).await {
        Err(ScenarioError::Failed(reason)) => {
            assert!(reason.contains("protocol=2"), "{reason}");
            assert!(reason.contains("CoinbaseOutputConstraints"), "{reason}");
        }
        other => panic!("expected a failed result, got {other:?}"),
    }
}

#[tokio::test]
async fn wrong_protocol_family_is_a_failure() {
    match run(vec![1], Reply::WrongFamily).await {
        Err(ScenarioError::Failed(reason)) => {
            assert!(reason.contains("protocol=1"), "{reason}");
            assert!(reason.contains("Mining Protocol"), "{reason}");
        }
        other => panic!("expected a failed result, got {other:?}"),
    }
}

#[tokio::test]
async fn job_declaration_target_probes_mining_and_template_distribution() {
    let result = run_as(AppType::JobDeclarationServer, vec![0, 2], Reply::Serves).await;
    let detail = result
        .expect("multi-protocol endpoint passes")
        .expect("pass detail");
    assert_eq!(detail.matches("also served").count(), 2, "{detail}");
    assert!(detail.contains("protocol=0"), "{detail}");
    assert!(detail.contains("protocol=2"), "{detail}");
}

async fn run(accepted: Vec<u8>, reply: Reply) -> ScenarioResult {
    run_as(AppType::MiningServer, accepted, reply).await
}

async fn run_as(app_type: AppType, accepted: Vec<u8>, reply: Reply) -> ScenarioResult {
    let listener = TcpListener::bind(get_available_address())
        .await
        .expect("the mock endpoint's address is available");
    let addr = listener
        .local_addr()
        .expect("the mock endpoint has an address");
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let accepted = accepted.clone();
            tokio::spawn(serve(stream, accepted, reply));
        }
    });

    let endpoint = Endpoint {
        addr,
        app_type,
        user_identity: "interoperability_tests_sv2".to_string(),
    };
    unsupported_protocol_rejected(endpoint).await
}

async fn serve(stream: TcpStream, accepted: Vec<u8>, reply: Reply) {
    let Some((receiver, sender)) = create_downstream(stream).await else {
        return;
    };
    while let Ok(mut frame) = receiver.recv().await {
        let (_, msg) = message_from_frame(&mut frame);
        for response in respond(msg, &accepted, reply) {
            if sender.send(response).await.is_err() {
                return;
            }
        }
    }
}

fn respond(msg: AnyMessageOwned, accepted: &[u8], reply: Reply) -> Vec<OutboundFrame> {
    match msg {
        AnyMessageOwned::Common(CommonMessagesOwned::SetupConnection(m)) => {
            if accepted.contains(&(m.protocol as u8)) {
                vec![setup_success()]
            } else {
                vec![setup_error()]
            }
        }
        AnyMessageOwned::Mining(MiningOwned::OpenStandardMiningChannel(_)) => match reply {
            Reply::Serves => vec![mining_reply()],
            Reply::Silent => vec![],
            Reply::WrongFamily => vec![job_declaration_reply()],
        },
        AnyMessageOwned::JobDeclaration(JobDeclarationOwned::AllocateMiningJobToken(_)) => {
            match reply {
                Reply::Serves => vec![job_declaration_reply()],
                Reply::Silent => vec![],
                Reply::WrongFamily => vec![mining_reply()],
            }
        }
        AnyMessageOwned::TemplateDistribution(
            TemplateDistributionOwned::CoinbaseOutputConstraints(_),
        ) => match reply {
            Reply::Serves => vec![template_distribution_reply()],
            Reply::Silent => vec![],
            Reply::WrongFamily => vec![mining_reply()],
        },
        _ => vec![],
    }
}

fn setup_success() -> OutboundFrame {
    frame(AnyMessageOwned::Common(
        CommonMessagesOwned::SetupConnectionSuccess(SetupConnectionSuccessOwned {
            used_version: 2,
            flags: 0,
        }),
    ))
}

fn setup_error() -> OutboundFrame {
    frame(AnyMessageOwned::Common(
        CommonMessagesOwned::SetupConnectionError(SetupConnectionErrorOwned {
            flags: 0,
            error_code: ERROR_CODE_SETUP_CONNECTION_UNSUPPORTED_PROTOCOL
                .to_string()
                .try_into()
                .expect("the error code fits in STR0_255"),
        }),
    ))
}

fn mining_reply() -> OutboundFrame {
    frame(AnyMessageOwned::Mining(
        MiningOwned::OpenMiningChannelError(OpenMiningChannelErrorOwned {
            request_id: 1,
            error_code: "probe"
                .to_string()
                .try_into()
                .expect("the error code fits in STR0_255"),
        }),
    ))
}

fn job_declaration_reply() -> OutboundFrame {
    frame(AnyMessageOwned::JobDeclaration(
        JobDeclarationOwned::AllocateMiningJobTokenSuccess(AllocateMiningJobTokenSuccessOwned {
            request_id: 1,
            mining_job_token: Vec::<u8>::new()
                .try_into()
                .expect("an empty token fits in B0_255"),
            coinbase_outputs: Vec::<u8>::new()
                .try_into()
                .expect("empty outputs fit in B0_64K"),
        }),
    ))
}

fn template_distribution_reply() -> OutboundFrame {
    frame(AnyMessageOwned::TemplateDistribution(
        TemplateDistributionOwned::SetNewPrevHash(SetNewPrevHashOwned {
            template_id: 0,
            prev_hash: [0_u8; 32].into(),
            ntime_start: 0,
            n_bits: 0,
            target: [0_u8; 32].into(),
        }),
    ))
}

fn frame(message: AnyMessageOwned) -> OutboundFrame {
    OutboundFrame::from_message(message).expect("the message can be framed")
}
