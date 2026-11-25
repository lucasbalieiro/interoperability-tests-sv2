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

/// How a bounded wait for a message from the endpoint ended.
#[derive(Debug)]
pub enum ReadOutcome {
    /// A message arrived before the deadline.
    Message(u8, AnyMessageOwned),
    /// The deadline passed while the connection was still open and silent.
    Timeout,
    /// The endpoint closed the connection before a message arrived.
    ConnectionClosed,
}

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
        let client = Self { client };
        client
            .send(AnyMessageOwned::Common(
                CommonMessagesOwned::SetupConnection(setup),
            ))
            .await?;
        Ok(client)
    }

    /// Sends a message to the endpoint.
    ///
    /// A connection the endpoint already closed is reported as an error rather
    /// than written to, so a caller can classify it instead of panicking.
    pub async fn send(&self, msg: AnyMessageOwned) -> Result<(), ScenarioError> {
        if self.is_closed().await {
            return Err("connection is closed".into());
        }
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

    /// Waits for the next message of any type from the endpoint, telling a
    /// live but silent connection apart from one the endpoint closed.
    ///
    /// A caller that makes several reads under one budget passes the remaining
    /// time on each call, so a server that keeps sending unrelated messages
    /// cannot keep the caller waiting past its deadline.
    pub async fn next_from_server_or_closed_within(&self, timeout: Duration) -> ReadOutcome {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if let Some((t, msg)) = self.client.next_message() {
                return ReadOutcome::Message(t, msg);
            }
            if self.is_closed().await {
                return ReadOutcome::ConnectionClosed;
            }
            let now = tokio::time::Instant::now();
            if now >= deadline {
                return ReadOutcome::Timeout;
            }
            tokio::time::sleep(POLL_INTERVAL.min(deadline.saturating_duration_since(now))).await;
        }
    }

    /// Waits for the next message of any type from the endpoint.
    pub async fn next_from_server(&self) -> Result<(u8, AnyMessageOwned), ScenarioError> {
        match self
            .next_from_server_or_closed_within(DEFAULT_MSG_TIMEOUT)
            .await
        {
            ReadOutcome::Message(msg_type, msg) => Ok((msg_type, msg)),
            ReadOutcome::Timeout => Err("timeout waiting for any message from endpoint".into()),
            ReadOutcome::ConnectionClosed => {
                Err("connection closed while waiting for a message from endpoint".into())
            }
        }
    }

    /// Whether the endpoint closed this client's connection.
    ///
    /// Polls the mock client's close watcher with a zero timeout, so it answers
    /// without blocking. A connection that failed inside the mock panics here,
    /// as it does on the next read.
    pub async fn is_closed(&self) -> bool {
        tokio::time::timeout(Duration::ZERO, self.client.wait_for_close())
            .await
            .is_ok()
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
