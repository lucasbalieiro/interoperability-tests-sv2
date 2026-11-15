//! Probing whether an endpoint serves a defined Sv2 sub-protocol.
//!
//! C-SC-7 cannot assume that the protocol selected by the suite is the only
//! one an endpoint serves (§3), and a `SetupConnection.Success` on its own is
//! not evidence that the endpoint serves the accepted protocol. The probe
//! sends the protocol's first client message and classifies what comes back.

use stratum_apps::stratum_core::{
    binary_sv2::Str0255Owned,
    job_declaration_sv2::AllocateMiningJobTokenOwned,
    mining_sv2::OpenStandardMiningChannelOwned,
    parsers_sv2::{AnyMessageOwned, JobDeclarationOwned, MiningOwned, TemplateDistributionOwned},
    template_distribution_sv2::CoinbaseOutputConstraintsOwned,
};

use crate::{app_type::AppType, client::TestClient, scenarios::ScenarioError};

/// The `request_id` used by both the Mining and Job Declaration probes.
const PROBE_REQUEST_ID: u32 = 1;

/// What an endpoint did after accepting a probe's `SetupConnection`.
#[derive(Debug)]
pub enum ProbeOutcome {
    /// The endpoint answered the opening request with a message from the
    /// accepted protocol's family.
    Serves { msg_type: u8 },
    /// The endpoint answered the opening request with a message from another
    /// protocol's family.
    WrongProtocol {
        got: AppType,
        msg_type: u8,
        msg: AnyMessageOwned,
    },
    /// The opening request could not be sent, e.g. because the endpoint closed
    /// the connection after `SetupConnection.Success`.
    NotEstablished { reason: String },
    /// No message from any protocol family arrived before the probe deadline.
    NoResponse,
}

/// Sends the first client message of `app_type` and classifies the first
/// protocol-family message that follows it.
///
/// The probe assumes the caller already consumed a `SetupConnection.Success`.
/// Common and extension messages are ignored while waiting, and every read
/// shares one deadline so a chatty endpoint cannot keep the probe alive.
pub async fn probe_protocol(
    client: &TestClient,
    app_type: AppType,
    user_identity: &str,
) -> Result<ProbeOutcome, ScenarioError> {
    let request = opening_request(app_type, user_identity)?;
    if let Err(e) = client.send(request).await {
        return Ok(ProbeOutcome::NotEstablished {
            reason: e.to_string(),
        });
    }

    let deadline = tokio::time::Instant::now() + crate::client::DEFAULT_MSG_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(ProbeOutcome::NoResponse);
        }
        let Some((msg_type, msg)) = client.next_from_server_within(remaining).await else {
            return Ok(ProbeOutcome::NoResponse);
        };
        match app_type_of(&msg) {
            Some(observed) if observed == app_type => {
                return Ok(ProbeOutcome::Serves { msg_type });
            }
            Some(got) => {
                return Ok(ProbeOutcome::WrongProtocol { got, msg_type, msg });
            }
            None => continue,
        }
    }
}

/// Builds the first client message a well-behaved client sends for
/// `app_type` after `SetupConnection.Success`.
pub fn opening_request(
    app_type: AppType,
    user_identity: &str,
) -> Result<AnyMessageOwned, ScenarioError> {
    Ok(match app_type {
        AppType::MiningServer => AnyMessageOwned::Mining(MiningOwned::OpenStandardMiningChannel(
            OpenStandardMiningChannelOwned {
                request_id: PROBE_REQUEST_ID,
                user_identity: str0255(user_identity)?,
                nominal_hash_rate: 1_000_000.0,
                max_target: [0xff_u8; 32].into(),
            },
        )),
        AppType::JobDeclarationServer => AnyMessageOwned::JobDeclaration(
            JobDeclarationOwned::AllocateMiningJobToken(AllocateMiningJobTokenOwned {
                user_identifier: str0255(user_identity)?,
                request_id: PROBE_REQUEST_ID,
            }),
        ),
        AppType::TemplateProvider => AnyMessageOwned::TemplateDistribution(
            TemplateDistributionOwned::CoinbaseOutputConstraints(CoinbaseOutputConstraintsOwned {
                coinbase_output_max_additional_size: 0,
                coinbase_output_max_additional_sigops: 0,
            }),
        ),
    })
}

/// The message a probe sends for `app_type`.
pub fn opening_request_name(app_type: AppType) -> &'static str {
    match app_type {
        AppType::MiningServer => "OpenStandardMiningChannel",
        AppType::JobDeclarationServer => "AllocateMiningJobToken",
        AppType::TemplateProvider => "CoinbaseOutputConstraints",
    }
}

/// The spec section that requires the server to answer the first client
/// message of `app_type`, or `None` when the spec does not mandate a reply.
pub fn first_response_requirement(app_type: AppType) -> Option<&'static str> {
    match app_type {
        AppType::MiningServer => Some(
            "§5.3.2: the server MUST accept the target or respond with OpenMiningChannel.Error",
        ),
        AppType::JobDeclarationServer => None,
        AppType::TemplateProvider => {
            Some("§7: the server MUST immediately reply with a NewTemplate")
        }
    }
}

/// The app type whose protocol family `msg` belongs to, if it belongs to one.
pub fn app_type_of(msg: &AnyMessageOwned) -> Option<AppType> {
    match msg {
        AnyMessageOwned::Mining(_) => Some(AppType::MiningServer),
        AnyMessageOwned::JobDeclaration(_) => Some(AppType::JobDeclarationServer),
        AnyMessageOwned::TemplateDistribution(_) => Some(AppType::TemplateProvider),
        AnyMessageOwned::Common(_) | AnyMessageOwned::Extensions(_) => None,
    }
}

fn str0255(value: &str) -> Result<Str0255Owned, ScenarioError> {
    value.to_string().try_into().map_err(|_| {
        ScenarioError::Failed(format!(
            "user_identity ({} bytes) does not fit in a STR0_255 field",
            value.len()
        ))
    })
}
