use std::time::Duration;

use async_channel::Sender;
use integration_tests_sv2::{
    interceptor::MessageDirection,
    mock_roles::{MockDownstream, WithSetup},
    sniffer::Sniffer,
    start_sniffer,
};
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
/// Bit 1 of `SetupConnection.Success.flags` (§5.3.1): the server will not
/// open a standard channel on this connection.
const REQUIRES_EXTENDED_CHANNELS: u32 = 1 << 1;

/// A test client connected to an endpoint under test.
///
/// Wires `MockDownstream -> Sniffer -> endpoint`: messages are driven through
/// [`TestClient::send`] and everything the endpoint sends back is observed via
/// the sniffer (a `MockDownstream` only logs responses, it cannot read them).
///
/// Every wait is bounded by [`TestClient::next_from_server`]'s timeout: the
/// panicking `Sniffer::wait_for_message_type` primitive is deliberately never
/// used.
pub struct TestClient {
    sender: Sender<AnyMessageOwned>,
    sniffer: Sniffer<'static>,
}

impl TestClient {
    /// Connects and performs the `SetupConnection` handshake with the
    /// protocol and flags appropriate for `endpoint.app_type`.
    pub async fn connect(endpoint: &Endpoint) -> Result<Self, ScenarioError> {
        let setup = WithSetup::yes_with_defaults(
            endpoint.app_type.protocol(),
            endpoint.app_type.default_flags(),
        );
        Self::connect_with_setup(endpoint, setup).await
    }

    /// Connects and sends a custom `SetupConnection` (for negotiation edge cases).
    pub async fn connect_with_setup(
        endpoint: &Endpoint,
        setup: WithSetup,
    ) -> Result<Self, ScenarioError> {
        let (sniffer, sniffer_addr) =
            start_sniffer("test-client", endpoint.addr, false, vec![], None);
        let sender = MockDownstream::new(sniffer_addr, setup).start().await;
        Ok(Self { sender, sniffer })
    }

    /// Sends a message to the endpoint.
    pub async fn send(&self, msg: AnyMessageOwned) -> Result<(), ScenarioError> {
        self.sender
            .send(msg)
            .await
            .map_err(|e| format!("failed to send message: {e}").into())
    }

    /// Opens a standard channel. A refusal with `OpenMiningChannel.Error` is
    /// *not exercised* when the server set `REQUIRES_EXTENDED_CHANNELS` in
    /// `SetupConnection.Success.flags`, since it announced it would not open
    /// one (§5.3.1, C-SC-5). Without that flag the server MUST accept standard
    /// channels (§5.2.2), so the refusal is a failure (M-OSMC-4).
    pub async fn open_standard_channel(
        &self,
        open: OpenStandardMiningChannelOwned,
    ) -> Result<OpenStandardMiningChannelSuccessOwned, ScenarioError> {
        self.send(AnyMessageOwned::Mining(
            MiningOwned::OpenStandardMiningChannel(open),
        ))
        .await?;
        // Skip whatever is still queued from before the open, keeping the
        // `SetupConnection.Success` flags on the way; stop at the open's
        // response. A caller that consumed the `.Success` frame itself leaves
        // the flag unseen, and a refusal is then judged as a failure.
        let mut requires_extended_channels = false;
        loop {
            match self.next_from_server().await? {
                (_, AnyMessageOwned::Common(CommonMessagesOwned::SetupConnectionSuccess(m))) => {
                    requires_extended_channels = m.flags & REQUIRES_EXTENDED_CHANNELS != 0;
                }
                (_, AnyMessageOwned::Mining(MiningOwned::OpenStandardMiningChannelSuccess(m))) => {
                    return Ok(m)
                }
                (_, AnyMessageOwned::Mining(MiningOwned::OpenMiningChannelError(e))) => {
                    let code = e.error_code.as_utf8_or_hex();
                    return Err(if requires_extended_channels {
                        ScenarioError::not_exercised(format!(
                            "server refused a standard channel (error_code=\"{code}\")"
                        ))
                    } else {
                        ScenarioError::Failed(format!(
                            "server refused a standard channel without REQUIRES_EXTENDED_CHANNELS (M-OSMC-4): error_code=\"{code}\""
                        ))
                    });
                }
                _ => continue,
            }
        }
    }

    /// Opens an extended channel and returns the endpoint's answer: `Ok` on
    /// `.Success`, `Err` on `OpenMiningChannel.Error`. Unlike a standard
    /// channel, a refusal is handed back rather than mapped to an outcome,
    /// because scenarios disagree on what it means: §5.3.4 allows it as an
    /// answer to `min_extranonce_size`, while §5.2.2 makes extended-channel
    /// support mandatory (M-OEMC-1).
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

    /// Waits for the next message of any type from the endpoint, returning
    /// `None` if `timeout` elapses first.
    ///
    /// A caller that makes several reads under one budget passes the remaining
    /// time on each call, so a server that keeps sending unrelated messages
    /// cannot keep the caller waiting past its deadline.
    pub async fn next_from_server_within(
        &self,
        timeout: Duration,
    ) -> Option<(u8, AnyMessageOwned)> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if let Some((t, msg)) = self.sniffer.next_message_from_upstream() {
                return Some((t, msg));
            }
            let now = tokio::time::Instant::now();
            if now >= deadline {
                return None;
            }
            tokio::time::sleep(POLL_INTERVAL.min(deadline.saturating_duration_since(now))).await;
        }
    }

    /// Waits for the next message of any type from the endpoint.
    pub async fn next_from_server(&self) -> Result<(u8, AnyMessageOwned), ScenarioError> {
        self.next_from_server_within(DEFAULT_MSG_TIMEOUT)
            .await
            .ok_or_else(|| {
                ScenarioError::Failed("timeout waiting for any message from endpoint".into())
            })
    }

    /// Asserts the endpoint sends no message of `msg_type` within `duration`.
    pub async fn expect_silence(
        &self,
        msg_type: u8,
        duration: Duration,
    ) -> Result<(), ScenarioError> {
        if self
            .sniffer
            .assert_message_not_present(MessageDirection::ToDownstream, msg_type, duration)
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
