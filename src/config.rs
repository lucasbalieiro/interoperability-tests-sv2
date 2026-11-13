//! TOML batch configuration for the `sv2-compliance` CLI.

use futures::future::join_all;
use serde::Deserialize;
use std::{fmt, fs, net::SocketAddr, path::Path, str::FromStr, time::Duration};

use crate::{app_type::AppType, endpoint::Endpoint};

/// How long to wait for the resolver for a single configured address.
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(10);

const DEFAULT_USER_IDENTITY: &str = "interoperability_tests_sv2";

/// A set of [`Site`]s to test, loaded from a TOML file on disk.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub site: Vec<Site>,
}

impl Config {
    /// Loads a [`Config`] from the TOML file at `path`.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        toml::from_str(&raw).map_err(|e| format!("failed to parse {}: {e}", path.display()))
    }

    /// Resolves every configured address into a [`Target`].
    ///
    /// `mining_server_address` is tested as a Mining Protocol Server,
    /// `job_declaration_server_address` as a Job Declarator Server and
    /// `template_provider_address` as a Template Provider.
    ///
    /// An address whose hostname fails to resolve is returned in
    /// [`Targets::unresolved`] instead of aborting, so the other sites still run.
    pub async fn resolve_targets(&self) -> Targets {
        let mut resolved = Vec::new();
        let mut unresolved = Vec::new();

        for site_results in join_all(self.site.iter().map(Site::resolve_addresses)).await {
            for result in site_results {
                match result {
                    Ok(target) => resolved.push(target),
                    Err(target) => unresolved.push(target),
                }
            }
        }

        Targets {
            resolved,
            unresolved,
        }
    }
}

/// Picks one address from a resolver result, preferring IPv4 when available.
fn pick_address(addrs: &[SocketAddr]) -> Option<SocketAddr> {
    addrs
        .iter()
        .find(|addr| addr.is_ipv4())
        .or_else(|| addrs.first())
        .copied()
}

/// The result of resolving a [`Config`]'s addresses.
pub struct Targets {
    /// Endpoints that resolved to a [`SocketAddr`].
    pub resolved: Vec<Target>,
    /// Endpoints whose hostname could not be resolved.
    pub unresolved: Vec<UnresolvedTarget>,
}

impl Targets {
    /// Returns `true` when the config contained no addresses at all.
    pub fn is_empty(&self) -> bool {
        self.resolved.is_empty() && self.unresolved.is_empty()
    }
}

/// A single endpoint under test, derived from a [`Site`].
#[derive(Debug, Clone)]
pub struct Target {
    pub site_name: String,
    /// The address as configured, before hostname resolution.
    pub address: TargetAddress,
    pub endpoint: Endpoint,
}

/// Why a configured address could not be resolved.
#[derive(Debug, Clone)]
pub enum ResolveError {
    /// The resolver did not answer within the configured timeout.
    Timeout(Duration),
    /// The OS resolver rejected the lookup; the string is its platform detail.
    Lookup(String),
    /// The resolver returned no addresses.
    NoAddresses,
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout(after) => write!(f, "timed out after {after:?}"),
            Self::Lookup(_) => write!(f, "DNS lookup failed"),
            Self::NoAddresses => write!(f, "resolver returned no addresses"),
        }
    }
}

impl ResolveError {
    /// The platform-specific resolver detail, when there is one.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Lookup(detail) => Some(detail),
            _ => None,
        }
    }
}

/// A configured endpoint whose hostname could not be resolved.
#[derive(Debug, Clone)]
pub struct UnresolvedTarget {
    pub site_name: String,
    pub address: TargetAddress,
    pub app_type: AppType,
    pub error: ResolveError,
}

/// A named deployment of one or more Sv2 endpoints.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub name: String,
    pub mining_server_address: Option<TargetAddress>,
    pub job_declaration_server_address: Option<TargetAddress>,
    pub template_provider_address: Option<TargetAddress>,
    pub user_identity: Option<String>,
}

impl Site {
    /// Resolves this site's configured addresses concurrently.
    async fn resolve_addresses(&self) -> Vec<Result<Target, UnresolvedTarget>> {
        let user_identity = self
            .user_identity
            .clone()
            .unwrap_or_else(|| DEFAULT_USER_IDENTITY.to_string());

        join_all(
            [
                (self.mining_server_address.as_ref(), AppType::MiningServer),
                (
                    self.job_declaration_server_address.as_ref(),
                    AppType::JobDeclarationServer,
                ),
                (
                    self.template_provider_address.as_ref(),
                    AppType::TemplateProvider,
                ),
            ]
            .into_iter()
            .filter_map(|(address, app_type)| {
                address.map(|address| {
                    let user_identity = user_identity.clone();
                    async move {
                        match address.resolve().await {
                            Ok(addr) => Ok(Target {
                                site_name: self.name.clone(),
                                address: address.clone(),
                                endpoint: Endpoint {
                                    addr,
                                    app_type,
                                    user_identity,
                                },
                            }),
                            Err(error) => Err(UnresolvedTarget {
                                site_name: self.name.clone(),
                                address: address.clone(),
                                app_type,
                                error,
                            }),
                        }
                    }
                })
            }),
        )
        .await
    }
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
#[serde(try_from = "String")]
pub enum TargetAddress {
    Socket(SocketAddr),
    Hostname { host: String, port: u16 },
}

impl TryFrom<String> for TargetAddress {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TargetAddress {
    async fn resolve(&self) -> Result<SocketAddr, ResolveError> {
        match self {
            Self::Socket(addr) => Ok(*addr),
            Self::Hostname { host, port } => {
                let addrs: Vec<SocketAddr> = tokio::time::timeout(
                    RESOLVE_TIMEOUT,
                    tokio::net::lookup_host((host.as_str(), *port)),
                )
                .await
                .map_err(|_| ResolveError::Timeout(RESOLVE_TIMEOUT))?
                .map_err(|e| ResolveError::Lookup(e.to_string()))?
                .collect();

                pick_address(&addrs).ok_or(ResolveError::NoAddresses)
            }
        }
    }
}

impl FromStr for TargetAddress {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(addr) = s.parse::<SocketAddr>() {
            return Ok(Self::Socket(addr));
        }

        if s.contains('/') {
            return Err(format!(
                "invalid address {s:?}: expected host:port, not a URL"
            ));
        }

        let (host, port) = s.rsplit_once(':').ok_or_else(|| {
            format!(
                "invalid address {s:?}: expected host:port \
                 (for example my.pool.org:3336)"
            )
        })?;

        if host.is_empty() {
            return Err(format!("invalid address {s:?}: host is empty"));
        }

        if host.contains(':') {
            return Err(format!(
                "invalid address {s:?}: IPv6 addresses must be bracketed, \
             for example [::1]:3333"
            ));
        }

        if host.chars().any(char::is_whitespace) {
            return Err(format!(
                "invalid address {s:?}: host must not contain whitespace"
            ));
        }

        if host.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Err(format!("invalid address {s:?}: invalid IPv4 address"));
        }

        Ok(Self::Hostname {
            host: host.to_string(),
            port: port
                .parse()
                .map_err(|_| format!("invalid address {s:?}: invalid port"))?,
        })
    }
}

impl fmt::Display for TargetAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Socket(addr) => write!(f, "{addr}"),
            Self::Hostname { host, port } => write!(f, "{host}:{port}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hostname_address() {
        let addr: TargetAddress = "stratum.ckpool.org:3336".parse().unwrap();
        assert_eq!(
            addr,
            TargetAddress::Hostname {
                host: "stratum.ckpool.org".to_string(),
                port: 3336,
            }
        );

        assert_eq!(addr.to_string(), "stratum.ckpool.org:3336");
    }

    #[test]
    fn parses_ip_literal() {
        let ipv4: TargetAddress = "75.119.150.111:3333".parse().unwrap();
        assert_eq!(
            ipv4,
            TargetAddress::Socket("75.119.150.111:3333".parse().unwrap())
        );

        let ipv6: TargetAddress = "[::1]:3334".parse().unwrap();
        assert_eq!(ipv6, TargetAddress::Socket("[::1]:3334".parse().unwrap()));
        assert_eq!(ipv6.to_string(), "[::1]:3334");
    }

    #[test]
    fn target_address_rejects_url_with_clear_error() {
        let error = "stratum+tcp://sri.pool.org:3333"
            .parse::<TargetAddress>()
            .unwrap_err();
        assert!(
            error.contains("not a URL"),
            "expected a URL-specific error, got {error}"
        );
    }

    #[test]
    fn target_address_rejects_invalid_addresses() {
        for address in [
            "sri.pool.org",
            ":3333",
            "::1:3333",
            "stratum+tcp://sri.pool.org:3333",
            "sri.pool.org:not-a-port-number",
            "sri pool.org:42",
            "sri\tpool.org:42",
            "51.161.49:34255",
            "1.2.3:80",
        ] {
            assert!(
                address.parse::<TargetAddress>().is_err(),
                "{address} should be rejected"
            );
        }
    }

    #[test]
    fn target_address_deserializes_from_toml() {
        #[derive(serde::Deserialize)]
        struct Wrapper {
            addr: TargetAddress,
        }

        let wrapper: Wrapper = toml::from_str("addr = \"localhost:3333\"").unwrap();
        assert_eq!(
            wrapper.addr,
            TargetAddress::Hostname {
                host: "localhost".to_string(),
                port: 3333
            }
        );
    }

    #[test]
    fn resolve_error_display_is_stable() {
        let lookup = ResolveError::Lookup("nodename nor servname provided".to_string());
        assert_eq!(lookup.to_string(), "DNS lookup failed");
        assert_eq!(lookup.detail(), Some("nodename nor servname provided"));

        let timeout = ResolveError::Timeout(Duration::from_secs(10));
        assert_eq!(timeout.to_string(), "timed out after 10s");
        assert_eq!(timeout.detail(), None);

        let no_addresses = ResolveError::NoAddresses;
        assert_eq!(no_addresses.to_string(), "resolver returned no addresses");
        assert_eq!(no_addresses.detail(), None);
    }

    #[tokio::test]
    async fn parses_sites_into_endpoints() {
        let raw = r#"
[[site]]
name = "SRI Pool"
mining_server_address = "51.161.49.55:34255"
job_declaration_server_address = "51.161.49.55:34256"
template_provider_address = "51.161.49.55:8442"

[[site]]
name = "Empty Site"
"#;
        let config: Config = toml::from_str(raw).expect("valid toml");
        let targets = config.resolve_targets().await;
        assert!(targets.unresolved.is_empty());
        assert_eq!(targets.resolved.len(), 3);
        assert_eq!(targets.resolved[0].endpoint.app_type, AppType::MiningServer);
        assert_eq!(
            targets.resolved[1].endpoint.app_type,
            AppType::JobDeclarationServer
        );
        assert_eq!(
            targets.resolved[2].endpoint.app_type,
            AppType::TemplateProvider
        );
    }

    /// A misspelled field must fail to parse rather than leave a site with
    /// nothing to test.
    #[test]
    fn rejects_unknown_fields() {
        let raw = r#"
[[site]]
name = "Typo"
mining_server_adress = "51.161.49.55:34255"
"#;
        assert!(toml::from_str::<Config>(raw).is_err());
    }

    #[test]
    fn parses_host_address_from_config() {
        let raw = r#"
        [[site]]
        name = "Mixed"
        mining_server_address = "stratum.ckpool.org:3336"
        job_declaration_server_address = "192.168.0.1:34256"
        template_provider_address = "[::1]:8442"
        "#;

        let config: Config = toml::from_str(raw).expect("expected a valid toml");
        let site = &config.site[0];

        assert_eq!(
            site.mining_server_address,
            Some(TargetAddress::Hostname {
                host: "stratum.ckpool.org".to_string(),
                port: 3336
            })
        );

        assert_eq!(
            site.job_declaration_server_address,
            Some(TargetAddress::Socket("192.168.0.1:34256".parse().unwrap()))
        );

        assert_eq!(
            site.template_provider_address,
            Some(TargetAddress::Socket("[::1]:8442".parse().unwrap()))
        );
    }

    #[tokio::test]
    async fn unresolved_hostname_does_not_drop_other_targets() {
        let raw = r#"
        [[site]]
        name = "Resolved"
        mining_server_address = "localhost:34255"

        [[site]]
        name = "Unresolved"
        mining_server_address = "not-a-valid-host.invalid:3333"
        "#;

        let config: Config = toml::from_str(raw).expect("expected a valid toml");
        let targets = config.resolve_targets().await;

        assert_eq!(targets.resolved.len(), 1);
        assert_eq!(targets.resolved[0].site_name, "Resolved");
        assert_eq!(targets.resolved[0].endpoint.addr.port(), 34255);

        assert_eq!(targets.unresolved.len(), 1);
        assert_eq!(targets.unresolved[0].site_name, "Unresolved");
        assert_eq!(
            targets.unresolved[0].address,
            TargetAddress::Hostname {
                host: "not-a-valid-host.invalid".to_string(),
                port: 3333,
            }
        );
    }

    #[test]
    fn targets_is_empty_only_when_both_lists_are_empty() {
        let empty = Targets {
            resolved: Vec::new(),
            unresolved: Vec::new(),
        };

        assert!(empty.is_empty());

        let unresolved = Targets {
            resolved: Vec::new(),
            unresolved: vec![UnresolvedTarget {
                site_name: "Unresolved".to_string(),
                address: TargetAddress::Hostname {
                    host: "does-not-exist.invalid".to_string(),
                    port: 3333,
                },
                app_type: AppType::MiningServer,
                error: ResolveError::NoAddresses,
            }],
        };
        assert!(!unresolved.is_empty());
    }

    #[test]
    fn prefers_ipv4_over_ipv6() {
        let addrs: [SocketAddr; 2] = [
            "[::1]:3333".parse().unwrap(),
            "127.0.0.1:3333".parse().unwrap(),
        ];

        assert_eq!(
            pick_address(&addrs),
            Some("127.0.0.1:3333".parse().unwrap())
        );
    }
}
