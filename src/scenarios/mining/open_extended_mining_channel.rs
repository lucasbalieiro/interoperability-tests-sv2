//! `OpenExtendedMiningChannel` and `.Success` (M-OEMC-*, M-OEMCS-*).

use stratum_apps::stratum_core::mining_sv2::OpenExtendedMiningChannelOwned;

use super::super::ScenarioResult;
use crate::{client::TestClient, endpoint::Endpoint, scenarios::ScenarioFn};

const REQUEST_ID: u32 = 1;

// ---------------------------------------------------------------------------
// M-OEMCS-1 — `request_id` echo on `OpenExtendedMiningChannel.Success`
// ---------------------------------------------------------------------------

/// A low `min_extranonce_size` for M-OEMCS-1, which only needs a `.Success`
/// envelope to check the `request_id` echo.
const REQUEST_ID_ECHO_MIN_EXTRANONCE_SIZE: u16 = 4;

/// The endpoint MUST echo `request_id` in `OpenExtendedMiningChannel.Success`.
///
/// Covers: M-OEMCS-1
pub async fn request_id_echo(endpoint: Endpoint) -> ScenarioResult {
    let client = TestClient::connect(&endpoint).await?;

    let open = OpenExtendedMiningChannelOwned {
        request_id: REQUEST_ID,
        user_identity: endpoint.user_identity.clone().try_into().unwrap(),
        nominal_hash_rate: 1_000_000.0,
        max_target: [0xff_u8; 32].into(),
        min_extranonce_size: REQUEST_ID_ECHO_MIN_EXTRANONCE_SIZE,
    };
    let m = client
        .open_extended_channel(open)
        .await?
        .map_err(|e| format!("server refused an extended channel (M-OEMC-1): {e}"))?;
    if m.request_id != REQUEST_ID {
        return Err(format!("request_id must echo {REQUEST_ID}, got {}", m.request_id).into());
    }
    Ok(Some(format!(
        "channel_id={}, group_channel_id={}, extranonce_size={}",
        m.channel_id, m.group_channel_id, m.extranonce_size
    )))
}

// ---------------------------------------------------------------------------
// M-OEMCS-3 — `extranonce_size >= min_extranonce_size`
// ---------------------------------------------------------------------------

/// `min_extranonce_size` values probed by M-OEMCS-3: a common baseline (4),
/// a value above common silent caps such as 6 (10), and the protocol maximum
/// (32). A server may satisfy each request or answer with
/// `OpenMiningChannel.Error`; silently granting fewer bytes is the violation.
const MIN_EXTRANONCE_SIZE_PROBES: [u16; 3] = [4, 10, 32];

/// The endpoint MUST satisfy every requested `min_extranonce_size` or refuse
/// that channel with `OpenMiningChannel.Error` (§5.3.4, §5.3.5). A `.Success`
/// granting fewer bytes than requested is a violation; refusing is not.
///
/// Covers: M-OEMCS-3
pub async fn extranonce_size_at_least_min(endpoint: Endpoint) -> ScenarioResult {
    let mut outcomes = Vec::with_capacity(MIN_EXTRANONCE_SIZE_PROBES.len());

    for min_extranonce_size in MIN_EXTRANONCE_SIZE_PROBES {
        let client = TestClient::connect(&endpoint).await?;

        let open = OpenExtendedMiningChannelOwned {
            request_id: REQUEST_ID,
            user_identity: endpoint.user_identity.clone().try_into().unwrap(),
            nominal_hash_rate: 1_000_000.0,
            max_target: [0xff_u8; 32].into(),
            min_extranonce_size,
        };

        match client.open_extended_channel(open).await? {
            Ok(m) if m.extranonce_size < min_extranonce_size => {
                return Err(format!(
                    "min_extranonce_size={min_extranonce_size}: server granted extranonce_size={}, smaller than requested",
                    m.extranonce_size
                )
                .into());
            }
            Ok(m) => outcomes.push(format!(
                "min={min_extranonce_size} -> extranonce_size={}",
                m.extranonce_size
            )),
            Err(e) => outcomes.push(format!(
                "min={min_extranonce_size} -> error_code=\"{}\"",
                e.error_code.as_utf8_or_hex()
            )),
        };
    }

    Ok(Some(outcomes.join("; ")))
}

// ---------------------------------------------------------------------------
// registry entries for this module
// ---------------------------------------------------------------------------

pub fn entries() -> Vec<(&'static str, ScenarioFn)> {
    vec![
        ("M-OEMCS-1 request-id-echo", |e| {
            Box::pin(request_id_echo(e))
        }),
        ("M-OEMCS-3 extranonce-size-at-least-min", |e| {
            Box::pin(extranonce_size_at_least_min(e))
        }),
    ]
}
