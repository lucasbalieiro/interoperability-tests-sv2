use std::{net::SocketAddr, time::Duration};

use integration_tests_sv2::utils::handshake_as_client;
use tokio_util::sync::CancellationToken;

use crate::{app_type::AppType, scenarios::ScenarioError};

const CONNECT_BUDGET: Duration = Duration::from_secs(3);

/// An Sv2 endpoint under test.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub addr: SocketAddr,
    pub app_type: AppType,
    pub user_identity: String,
}

impl Endpoint {
    /// Fails fast when the endpoint is unreachable or refuses the Noise
    /// handshake, instead of stalling every scenario on a message timeout.
    ///
    /// TCP connect retries within the budget: a freshly started local service
    /// may not be listening yet, while a dead endpoint refuses every attempt
    /// instantly. The budget bounds the whole retry loop, since a single
    /// connect to an address that drops packets only returns once the kernel
    /// gives up on SYN retries, long after any reasonable wait. The handshake
    /// then gets its own budget on the open stream, as the responder may
    /// accept TCP and never answer.
    pub async fn preflight(&self) -> Result<(), ScenarioError> {
        let mut last_error = None;
        let stream = tokio::time::timeout(CONNECT_BUDGET, async {
            loop {
                match tokio::net::TcpStream::connect(self.addr).await {
                    Ok(stream) => return stream,
                    Err(e) => last_error = Some(e),
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        })
        .await
        .map_err(|_| match last_error {
            Some(e) => format!("endpoint unreachable: {e}"),
            None => format!("endpoint unreachable: connect timed out after {CONNECT_BUDGET:?}"),
        })?;

        let cancellation = CancellationToken::new();
        let outcome = tokio::time::timeout(
            CONNECT_BUDGET,
            handshake_as_client(stream, cancellation.clone()),
        )
        .await;
        cancellation.cancel();
        match outcome {
            Ok(Some(_)) => Ok(()),
            Ok(None) => Err("Noise handshake with endpoint failed".into()),
            Err(_) => Err(format!(
                "Noise handshake with endpoint timed out after {CONNECT_BUDGET:?}"
            )
            .into()),
        }
    }
}
