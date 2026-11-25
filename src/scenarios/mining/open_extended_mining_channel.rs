//! `OpenExtendedMiningChannel` and `.Success` (M-OEMC-*, M-OEMCS-*).

use stratum_apps::stratum_core::mining_sv2::OpenExtendedMiningChannelOwned;

use super::super::ScenarioResult;
use crate::{
    client::TestClient,
    endpoint::Endpoint,
    scenarios::{ScenarioError, ScenarioFn},
};

const REQUEST_ID: u32 = 1;
const MIN_EXTRANONCE_SIZE: u16 = 4;

// ---------------------------------------------------------------------------
// M-OEMCS-1 — `request_id` echo on `OpenExtendedMiningChannel.Success`
// ---------------------------------------------------------------------------

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
        min_extranonce_size: MIN_EXTRANONCE_SIZE,
    };
    let m = client.open_extended_channel(open).await?.map_err(|e| {
        ScenarioError::not_exercised(format!(
            "server refused an extended channel (error_code=\"{}\")",
            e.error_code.as_utf8_or_hex()
        ))
    })?;
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

/// The endpoint MUST satisfy `min_extranonce_size` or refuse the channel with
/// `OpenMiningChannel.Error` (§5.3.4, §5.3.5); both answers are compliant.
///
/// Covers: M-OEMCS-3
pub async fn extranonce_size_at_least_min(endpoint: Endpoint) -> ScenarioResult {
    let client = TestClient::connect(&endpoint).await?;

    let open = OpenExtendedMiningChannelOwned {
        request_id: REQUEST_ID,
        user_identity: endpoint.user_identity.clone().try_into().unwrap(),
        nominal_hash_rate: 1_000_000.0,
        max_target: [0xff_u8; 32].into(),
        min_extranonce_size: MIN_EXTRANONCE_SIZE,
    };
    match client.open_extended_channel(open).await? {
        Ok(m) => {
            if m.extranonce_size < MIN_EXTRANONCE_SIZE {
                return Err(format!(
                    "extranonce_size {} smaller than requested min_extranonce_size {MIN_EXTRANONCE_SIZE}",
                    m.extranonce_size
                )
                .into());
            }
            Ok(Some(format!(
                "channel_id={}, group_channel_id={}, extranonce_size={}",
                m.channel_id, m.group_channel_id, m.extranonce_size
            )))
        }
        Err(e) => Ok(Some(format!(
            "refused with error_code=\"{}\"",
            e.error_code.as_utf8_or_hex()
        ))),
    }
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
