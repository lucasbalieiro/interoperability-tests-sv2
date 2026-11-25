//! `NewExtendedMiningJob` (M-NEMJ-*).

use stratum_apps::stratum_core::{
    mining_sv2::{
        NewExtendedMiningJobOwned, OpenExtendedMiningChannelOwned,
        MESSAGE_TYPE_NEW_EXTENDED_MINING_JOB,
    },
    parsers_sv2::{AnyMessageOwned, MiningOwned},
};

use super::super::ScenarioResult;
use crate::{
    client::TestClient,
    endpoint::Endpoint,
    scenarios::{ScenarioError, ScenarioFn},
};

// ---------------------------------------------------------------------------
// M-NEMJ-1 — first post-open message is `NewExtendedMiningJob`
// ---------------------------------------------------------------------------

/// §5.3.16: the first message after an extended channel opens MUST be a
/// `NewExtendedMiningJob`.
///
/// Covers: M-NEMJ-1
pub async fn first_message_after_open_is_job(endpoint: Endpoint) -> ScenarioResult {
    first_job(&endpoint).await?;
    Ok(None)
}

// ---------------------------------------------------------------------------
// M-NEMJ-2 — first job is a future job
// ---------------------------------------------------------------------------

/// §5.3.16: the first `NewExtendedMiningJob` after an extended channel opens
/// MUST have `ntime_start` unset, since no `SetNewPrevHash` exists yet to
/// activate it.
///
/// Covers: M-NEMJ-2
pub async fn first_job_is_future_job(endpoint: Endpoint) -> ScenarioResult {
    let job = first_job(&endpoint).await?;
    match job.ntime_start.into_inner() {
        None => Ok(None),
        Some(ntime_start) => Err(format!(
            "first NewExtendedMiningJob must be a future job (ntime_start unset), got ntime_start={ntime_start}"
        )
        .into()),
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Opens an extended channel and returns the first message the endpoint sends
/// afterwards, failing unless it is a `NewExtendedMiningJob`.
async fn first_job(endpoint: &Endpoint) -> Result<NewExtendedMiningJobOwned, ScenarioError> {
    let client = TestClient::connect(endpoint).await?;

    let open = OpenExtendedMiningChannelOwned {
        request_id: 1,
        user_identity: endpoint.user_identity.clone().try_into().unwrap(),
        nominal_hash_rate: 1_000_000.0,
        max_target: [0xff_u8; 32].into(),
        min_extranonce_size: 4,
    };
    client.open_extended_channel(open).await?.map_err(|e| {
        ScenarioError::not_exercised(format!(
            "server refused an extended channel (error_code=\"{}\")",
            e.error_code.as_utf8_or_hex()
        ))
    })?;

    match client.next_from_server().await? {
        (_, AnyMessageOwned::Mining(MiningOwned::NewExtendedMiningJob(job))) => Ok(job),
        (msg_type, _) => Err(format!(
            "expected NewExtendedMiningJob (0x{MESSAGE_TYPE_NEW_EXTENDED_MINING_JOB:02x}) after extended channel open, got 0x{msg_type:02x}"
        )
        .into()),
    }
}

// ---------------------------------------------------------------------------
// registry entries for this module
// ---------------------------------------------------------------------------

pub fn entries() -> Vec<(&'static str, ScenarioFn)> {
    vec![
        ("M-NEMJ-1 first-message-after-open-is-job", |e| {
            Box::pin(first_message_after_open_is_job(e))
        }),
        ("M-NEMJ-2 first-job-is-future-job", |e| {
            Box::pin(first_job_is_future_job(e))
        }),
    ]
}
