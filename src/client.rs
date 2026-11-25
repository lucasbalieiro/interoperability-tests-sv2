use std::time::Duration;

use integration_tests_sv2::mocks::{ClientSetupConnection, MockClient};
use stratum_apps::stratum_core::{
    common_messages_sv2::SetupConnectionOwned,
    mining_sv2::{
        OpenExtendedMiningChannelOwned, OpenExtendedMiningChannelSuccessOwned,
        OpenMiningChannelErrorOwned, OpenStandardMiningChannelOwned,
        OpenStandardMiningChannelSuccessOwned,
    },
    parsers_sv2::{AnyMessageOwned, CommonMessagesOwned, MiningOwned},
};

use crate::{endpoint::Endpoint, scenarios::ScenarioError};

/// How long a scenario waits for a message from the endpoint before failing.
pub const DEFAULT_MSG_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// A test client connected to an endpoint under test.
///
/// Messages are driven through [`TestClient::send`], and the endpoint's
/// replies are read from the [`MockClient`] that owns the connection.
///
/// Every wait is bounded: [`TestClient::next_from_server`] applies
/// [`DEFAULT_MSG_TIMEOUT`], and [`TestClient::expect_silence`] takes its own
/// deadline. A connection that fails inside the mock panics on the next read;
/// `runner::run_suite` reports that panic as a failed scenario.
pub struct TestClient {
    client: MockClient,
}

impl TestClient {
    /// Connects and performs the `SetupConnection` handshake with the
    /// protocol and flags appropriate for `endpoint.app_type`.
    pub async fn connect(endpoint: &Endpoint) -> Result<Self, ScenarioError> {
        let setup = ClientSetupConnection {
            protocol: endpoint.app_type.protocol(),
            min_version: 2,
            max_version: 2,
            flags: endpoint.app_type.default_flags(),
        };
        let client = MockClient::new(endpoint.addr, Some(setup)).await;
        Ok(Self { client })
    }

    /// Connects and sends the caller's full `SetupConnection`, for
    /// negotiation edge cases where the message itself is under test.
    pub async fn connect_with_setup(
        endpoint: &Endpoint,
        setup: SetupConnectionOwned,
    ) -> Result<Self, ScenarioError> {
        let client = MockClient::new(endpoint.addr, None).await;

        client
            .send(AnyMessageOwned::Common(
                CommonMessagesOwned::SetupConnection(setup),
            ))
            .await;
        Ok(Self { client })
    }

    /// Sends a message to the endpoint.
    pub async fn send(&self, msg: AnyMessageOwned) -> Result<(), ScenarioError> {
        self.client.send(msg).await;
        Ok(())
    }

    /// Opens a standard channel. A refusal with `OpenMiningChannel.Error` is
    /// *not exercised* (§5.3.2, §5.3.6): the spec allows a server to refuse an
    /// open (e.g. restrictions on `user_identity` or `max_target`), so the
    /// channel was not opened and subsequent assertions cannot be checked.
    pub async fn open_standard_channel(
        &self,
        open: OpenStandardMiningChannelOwned,
    ) -> Result<OpenStandardMiningChannelSuccessOwned, ScenarioError> {
        self.send(AnyMessageOwned::Mining(
            MiningOwned::OpenStandardMiningChannel(open),
        ))
        .await?;
        // Skip whatever is still queued from before the open (the
        // `SetupConnection.Success` frame); stop at the open's response.
        loop {
            match self.next_from_server().await? {
                (_, AnyMessageOwned::Mining(MiningOwned::OpenStandardMiningChannelSuccess(m))) => {
                    return Ok(m)
                }
                (_, AnyMessageOwned::Mining(MiningOwned::OpenMiningChannelError(e))) => {
                    let code = e.error_code.as_utf8_or_hex();
                    return Err(ScenarioError::not_exercised(format!(
                        "server refused a standard channel (error_code=\"{code}\")"
                    )));
                }
                _ => continue,
            }
        }
    }

    /// Opens an extended channel and returns the endpoint's answer: `Ok` on
    /// `.Success`, `Err` on `OpenMiningChannel.Error`. Unlike a standard
    /// channel, a refusal is handed back rather than mapped to an outcome,
    /// so calling scenarios can decide whether to assert against the error
    /// (e.g. M-OEMCS-3) or report downstream assertions as not exercised.
    pub async fn open_extended_channel(
        &self,
        open: OpenExtendedMiningChannelOwned,
    ) -> Result<
        Result<OpenExtendedMiningChannelSuccessOwned, OpenMiningChannelErrorOwned>,
        ScenarioError,
    > {
        self.send(AnyMessageOwned::Mining(
            MiningOwned::OpenExtendedMiningChannel(open),
        ))
        .await?;
        // Skip whatever is still queued from before the open (the
        // `SetupConnection.Success` frame); stop at the open's response.
        loop {
            match self.next_from_server().await? {
                (_, AnyMessageOwned::Mining(MiningOwned::OpenExtendedMiningChannelSuccess(m))) => {
                    return Ok(Ok(m))
                }
                (_, AnyMessageOwned::Mining(MiningOwned::OpenMiningChannelError(e))) => {
                    return Ok(Err(e))
                }
                _ => continue,
            }
        }
    }

    /// Waits for the next message of any type from the endpoint.
    pub async fn next_from_server(&self) -> Result<(u8, AnyMessageOwned), ScenarioError> {
        tokio::time::timeout(DEFAULT_MSG_TIMEOUT, async {
            loop {
                if let Some((t, msg)) = self.client.next_message() {
                    return (t, msg);
                }
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        })
        .await
        .map_err(|_| ScenarioError::Failed("timeout waiting for any message from endpoint".into()))
    }

    /// Asserts the endpoint sends no message of `msg_type` within `duration`.
    pub async fn expect_silence(
        &self,
        msg_type: u8,
        duration: Duration,
    ) -> Result<(), ScenarioError> {
        if self
            .client
            .assert_message_not_present(msg_type, duration)
            .await
        {
            Ok(())
        } else {
            Err(format!("expected no message of type 0x{msg_type:02x}, but one arrived").into())
        }
    }
}

/// Builds a `SetupConnection` with custom version range and flags, for
/// negotiation edge cases.
pub fn setup_connection(
    protocol: stratum_apps::stratum_core::common_messages_sv2::Protocol,
    min_version: u16,
    max_version: u16,
    flags: u32,
) -> SetupConnectionOwned {
    SetupConnectionOwned {
        protocol,
        min_version,
        max_version,
        flags,
        endpoint_host: "0.0.0.0".try_into().unwrap(),
        endpoint_port: 0,
        vendor: "interoperability-tests-sv2".try_into().unwrap(),
        hardware_version: "".try_into().unwrap(),
        firmware: "".try_into().unwrap(),
        device_id: "".try_into().unwrap(),
    }
}
