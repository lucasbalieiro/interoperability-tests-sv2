//! Common Protocol Messages — `SetupConnection` negotiation.
//!
//! Covers catalog rows C-SC-*, C-SCS-*, C-SCE-*.

use integration_tests_sv2::mock_roles::WithSetup;
use stratum_apps::stratum_core::parsers_sv2::{
    message_type_to_name, AnyMessageOwned, CommonMessagesOwned,
};

use crate::{
    app_type::AppType,
    client::{setup_connection, TestClient, DEFAULT_MSG_TIMEOUT},
    endpoint::Endpoint,
    probe::{self, ProbeOutcome},
    scenarios::ScenarioError,
};

use super::super::ScenarioResult;

// ---------------------------------------------------------------------------
// C-SCS-1 — framed `SetupConnection.Success` response
// ---------------------------------------------------------------------------

/// The endpoint MUST respond to `SetupConnection` with a properly framed
/// `.Success` or `.Error` — not silence, a wrong message type, or a TCP close.
///
/// Covers: C-SCS-1
pub async fn framed_setup_connection_response(endpoint: Endpoint) -> ScenarioResult {
    let client = TestClient::connect(&endpoint).await?;

    let (msg_type, msg) = client.next_from_server().await?;
    match msg {
        AnyMessageOwned::Common(CommonMessagesOwned::SetupConnectionSuccess(m)) => {
            Ok(Some(format!("used_version={}", m.used_version)))
        }
        AnyMessageOwned::Common(CommonMessagesOwned::SetupConnectionError(m)) => Ok(Some(
            format!("error_code=\"{}\"", m.error_code.as_utf8_or_hex()),
        )),
        other => Err(format!(
            "first message must be SetupConnectionSuccess or SetupConnectionError, got type 0x{msg_type:02x}: {other}"
        )
        .into()),
    }
}

// ---------------------------------------------------------------------------
// C-SCS-2 — `used_version` within negotiated range
// ---------------------------------------------------------------------------

/// The endpoint MUST honour the version range: `[2,2]` → `used_version == 2`
/// (§3.6.2).
///
/// Covers: C-SCS-2
pub async fn used_version_within_range(endpoint: Endpoint) -> ScenarioResult {
    let client = TestClient::connect(&endpoint).await?;
    match client.next_from_server().await? {
        (_, AnyMessageOwned::Common(CommonMessagesOwned::SetupConnectionSuccess(m))) => {
            if m.used_version != 2 {
                return Err(format!(
                    "used_version must be 2 with [2,2] range, got {}",
                    m.used_version
                )
                .into());
            }
            Ok(Some(format!("used_version={}", m.used_version)))
        }
        (msg_type, other) => Err(format!(
            "expected SetupConnectionSuccess for [2,2], got type 0x{msg_type:02x}: {other}"
        )
        .into()),
    }
}

// ---------------------------------------------------------------------------
// C-SC-8 — version range with no supported version rejected
// ---------------------------------------------------------------------------

/// Version ranges with no overlap with 2 — tried as `(min_version, max_version)`.
const UNSUPPORTED_VERSION_RANGES: [(u16, u16); 3] = [(3, 3), (3, 2), (1, 1)];

/// The endpoint MUST reject a `SetupConnection` whose version range contains
/// no supported version with a framed `SetupConnectionError` (§3.6.1).
///
/// Covers: C-SC-8
pub async fn unsupported_version_rejected(endpoint: Endpoint) -> ScenarioResult {
    let protocol = endpoint.app_type.protocol();
    let flags = endpoint.app_type.default_flags();

    for (min, max) in UNSUPPORTED_VERSION_RANGES {
        let setup = setup_connection(protocol, min, max, flags);
        let client = TestClient::connect_with_setup(&endpoint, WithSetup::Yes(setup)).await?;
        let (msg_type, msg) = client.next_from_server().await.map_err(|e| {
            ScenarioError::Failed(format!("no response to version range ({min},{max}): {e}"))
        })?;
        match msg {
            AnyMessageOwned::Common(CommonMessagesOwned::SetupConnectionError(_)) => {}
            AnyMessageOwned::Common(CommonMessagesOwned::SetupConnectionSuccess(m)) => {
                return Err(format!(
                    "endpoint accepted unsatisfiable version range ({min},{max}) with used_version={}",
                    m.used_version
                )
                .into())
            }
            other => {
                return Err(format!(
                    "expected SetupConnectionError for ({min},{max}) range, got type 0x{msg_type:02x}: {other}"
                )
                .into())
            }
        }
    }

    Ok(Some("(3,3), (3,2), (1,1) rejected".into()))
}

// ---------------------------------------------------------------------------
// C-SC-7 — accepted protocol is actually served
// ---------------------------------------------------------------------------

/// The endpoint MUST reject a `SetupConnection` for a sub-protocol it does not
/// serve with a framed `SetupConnectionError` (§3.6.1). Because a `.Success`
/// alone does not show whether the endpoint serves the protocol, the scenario
/// sends that protocol's first client message and classifies the answer.
///
/// Covers: C-SC-7
pub async fn unsupported_protocol_rejected(endpoint: Endpoint) -> ScenarioResult {
    let mut details = Vec::new();
    let mut failed = false;
    let mut inconclusive = false;

    for probed in AppType::ALL
        .into_iter()
        .filter(|app_type| *app_type != endpoint.app_type)
    {
        match probe_endpoint(&endpoint, probed).await {
            ProbeReport::Pass(detail) => details.push(detail),
            ProbeReport::Fail(detail) => {
                failed = true;
                details.push(detail);
            }
            ProbeReport::Inconclusive(detail) => {
                inconclusive = true;
                details.push(detail);
            }
        }
    }

    let detail = details.join("; ");
    if failed {
        Err(ScenarioError::Failed(detail))
    } else if inconclusive {
        Err(ScenarioError::NotExercised(detail))
    } else {
        Ok(Some(detail))
    }
}

/// How one protocol probe contributed to the scenario's outcome.
enum ProbeReport {
    Pass(String),
    Fail(String),
    Inconclusive(String),
}

/// Opens a connection for `probed`, reads its `SetupConnection` response, and
/// probes the protocol when the endpoint accepted it.
async fn probe_endpoint(endpoint: &Endpoint, probed: AppType) -> ProbeReport {
    let label = protocol_label(probed);
    let setup = setup_connection(probed.protocol(), 2, 2, probed.default_flags());
    let client = match TestClient::connect_with_setup(endpoint, WithSetup::Yes(setup)).await {
        Ok(client) => client,
        Err(e) => {
            return ProbeReport::Inconclusive(format!("{label}: probe connection failed: {e}"))
        }
    };

    let (msg_type, msg) = match client.next_from_server().await {
        Ok(msg) => msg,
        Err(e) => {
            return ProbeReport::Inconclusive(format!("{label}: no SetupConnection response: {e}"))
        }
    };

    let used_version = match msg {
        AnyMessageOwned::Common(CommonMessagesOwned::SetupConnectionError(m)) => {
            return ProbeReport::Pass(format!(
                "{label} rejected (error_code=\"{}\")",
                m.error_code.as_utf8_or_hex()
            ))
        }
        AnyMessageOwned::Common(CommonMessagesOwned::SetupConnectionSuccess(m)) => m.used_version,
        other => {
            return ProbeReport::Inconclusive(format!(
                "{label}: expected SetupConnection.Success or SetupConnection.Error, got type 0x{msg_type:02x}: {other}"
            ))
        }
    };

    match probe::probe_protocol(&client, probed, &endpoint.user_identity).await {
        Ok(ProbeOutcome::Serves { msg_type }) => ProbeReport::Pass(format!(
            "{label} also served (used_version={used_version}, reply {})",
            message_type_to_name(msg_type)
        )),
        Ok(ProbeOutcome::WrongProtocol {
            got,
            msg_type,
            msg,
        }) => ProbeReport::Fail(format!(
            "{label} accepted but answered {} with a {} message (type 0x{msg_type:02x}: {msg})",
            probe::opening_request_name(probed),
            got.name()
        )),
        Ok(ProbeOutcome::NotEstablished { reason }) => ProbeReport::Inconclusive(format!(
            "{label} accepted but the probe could not be sent: {reason}"
        )),
        Ok(ProbeOutcome::NoResponse) => match probe::first_response_requirement(probed) {
            Some(requirement) => ProbeReport::Fail(format!(
                "{label} accepted but did not answer {} within {:?}; either it does not serve the protocol and should have sent SetupConnection.Error (§3.6.1), or it failed the requirement: {requirement}",
                probe::opening_request_name(probed),
                DEFAULT_MSG_TIMEOUT
            )),
            None => ProbeReport::Inconclusive(format!(
                "{label} accepted but sent no {} response to {} within {:?}; §6.4.2 rate-limits this request and leaves `user_identifier` restrictions to the server, so whether the endpoint serves {} could not be determined",
                probed.name(),
                probe::opening_request_name(probed),
                DEFAULT_MSG_TIMEOUT,
                probed.name()
            )),
        },
        Err(e) => ProbeReport::Inconclusive(format!(
            "{label} accepted but the probe could not run: {e}"
        )),
    }
}

fn protocol_label(app_type: AppType) -> String {
    format!(
        "protocol={} ({})",
        app_type.protocol() as u8,
        app_type.name()
    )
}

// ---------------------------------------------------------------------------
// registry entries for this module
// ---------------------------------------------------------------------------

use crate::scenarios::ScenarioFn;

pub fn entries() -> Vec<(&'static str, ScenarioFn)> {
    vec![
        ("C-SCS-1 framed-setup-connection-response", |e| {
            Box::pin(framed_setup_connection_response(e))
        }),
        ("C-SCS-2 used-version-within-range", |e| {
            Box::pin(used_version_within_range(e))
        }),
        ("C-SC-7 unsupported-protocol-rejected", |e| {
            Box::pin(unsupported_protocol_rejected(e))
        }),
        ("C-SC-8 unsupported-version-rejected", |e| {
            Box::pin(unsupported_version_rejected(e))
        }),
    ]
}
