//! Common Protocol Messages — `SetupConnection` negotiation.
//!
//! Covers catalog rows C-SC-*, C-SCS-*, C-SCE-*.

use stratum_apps::stratum_core::parsers_sv2::{AnyMessageOwned, CommonMessagesOwned};

use crate::{
    client::{setup_connection, TestClient},
    endpoint::Endpoint,
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
        let client = TestClient::connect_with_setup(&endpoint, setup).await?;
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
// C-SC-7 — unsupported protocol rejected
// ---------------------------------------------------------------------------

/// The endpoint MUST reject a `SetupConnection` for a sub-protocol it does
/// not serve with a framed `SetupConnectionError`.
///
/// Covers: C-SC-7
pub async fn unsupported_protocol_rejected(endpoint: Endpoint) -> ScenarioResult {
    let setup = setup_connection(endpoint.app_type.wrong_protocol(), 2, 2, 0);
    let client = TestClient::connect_with_setup(&endpoint, setup).await?;

    // Take whatever the endpoint answers first: a `.Success` is the violation
    // and must fail right away, not after the message timeout.
    match client.next_from_server().await? {
        (_, AnyMessageOwned::Common(CommonMessagesOwned::SetupConnectionError(m))) => Ok(Some(
            format!("error_code=\"{}\"", m.error_code.as_utf8_or_hex()),
        )),
        (_, AnyMessageOwned::Common(CommonMessagesOwned::SetupConnectionSuccess(m))) => {
            Err(format!(
                "endpoint accepted the SetupConnection with used_version={}",
                m.used_version
            )
            .into())
        }
        (msg_type, other) => {
            Err(format!("expected SetupConnectionError, got type 0x{msg_type:02x}: {other}").into())
        }
    }
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
