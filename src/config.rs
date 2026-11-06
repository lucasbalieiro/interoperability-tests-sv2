//! TOML batch configuration for the `sv2-compliance` CLI.

use std::{
    fmt, fs,
    net::{IpAddr, SocketAddr},
    path::Path,
    str::FromStr,
    time::Duration,
};

use serde::Deserialize;

use crate::{app_type::AppType, endpoint::Endpoint};

/// How long to wait for the resolver for a single configured address.
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(10);

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

        for site in &self.site {
            let user_identity = site
                .user_identity
                .clone()
                .unwrap_or_else(|| "interoperability_tests_sv2".to_string());

            for (address, app_type) in [
                (site.mining_server_address.as_ref(), AppType::MiningServer),
                (
                    site.job_declaration_server_address.as_ref(),
                    AppType::JobDeclarationServer,
                ),
                (
                    site.template_provider_address.as_ref(),
                    AppType::TemplateProvider,
                ),
            ]
            .into_iter()
            .filter_map(|(address, app_type)| address.map(|address| (address, app_type)))
            {
                match address.resolve().await {
                    Ok(addr) => resolved.push(Target {
                        site_name: site.name.clone(),
                        address: address.clone(),
                        endpoint: Endpoint {
                            addr,
                            app_type,
                            user_identity: user_identity.clone(),
                        },
                    }),
                    Err(error) => unresolved.push(UnresolvedTarget {
                        site_name: site.name.clone(),
                        address: address.clone(),
                        app_type,
                        error,
                    }),
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

/// A single endpoint under test, derived from a [`Site`].
#[derive(Debug, Clone)]
pub struct Target {
    pub site_name: String,
    /// The address as configured, before hostname resolution.
    pub address: HostPort,
    pub endpoint: Endpoint,
}

/// A configured endpoint whose hostname could not be resolved.
#[derive(Debug, Clone)]
pub struct UnresolvedTarget {
    pub site_name: String,
    pub address: HostPort,
    pub app_type: AppType,
    pub error: String,
}

/// A named deployment of one or more Sv2 endpoints.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub name: String,
    pub mining_server_address: Option<HostPort>,
    pub job_declaration_server_address: Option<HostPort>,
    pub template_provider_address: Option<HostPort>,
    pub user_identity: Option<String>,
}

/// A `host:port` address from the TOML config.
///
/// The host may be a DNS name, an IPv4 address, or a bracketed IPv6 address.
#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(try_from = "String")]
pub struct HostPort {
    pub host: String,
    pub port: u16,
}

impl HostPort {
    /// Resolves this address to one [`SocketAddr`].
    ///
    /// IP literals resolve without DNS. When the resolver returns both address
    /// families, IPv4 is preferred: some Sv2 endpoints publish AAAA records
    /// while many CI runners have no IPv6 route.
    async fn resolve(&self) -> Result<SocketAddr, String> {
        if let Ok(ip) = self.host.parse::<IpAddr>() {
            return Ok(SocketAddr::new(ip, self.port));
        }

        let addrs: Vec<SocketAddr> = tokio::time::timeout(
            RESOLVE_TIMEOUT,
            tokio::net::lookup_host((self.host.as_str(), self.port)),
        )
        .await
        .map_err(|_| format!("timed out after {RESOLVE_TIMEOUT:?} resolving {self}"))?
        .map_err(|e| format!("failed to resolve {self}: {e}"))?
        .collect();

        pick_address(&addrs).ok_or_else(|| format!("resolver returned no addresses for {self}"))
    }
}

impl TryFrom<String> for HostPort {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl FromStr for HostPort {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Let `SocketAddr` handle IPv4 and bracketed IPv6 literals.
        if let Ok(addr) = s.parse::<SocketAddr>() {
            return Ok(Self {
                host: addr.ip().to_string(),
                port: addr.port(),
            });
        }

        if s.contains('/') {
            return Err(format!(
                "invalid address {s:?}: expected host:port, not a URL"
            ));
        }

        let (host, port) = s.rsplit_once(':').ok_or_else(|| {
            format!(
                "invalid address {s:?}: expected host:port \
                 (for example stratum.ckpool.org:3336)"
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

        Ok(Self {
            host: host.to_string(),
            port: port
                .parse()
                .map_err(|_| format!("invalid address {s:?}: invalid port"))?,
        })
    }
}

impl fmt::Display for HostPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.host.contains(':') {
            write!(f, "[{}]:{}", self.host, self.port)
        } else {
            write!(f, "{}:{}", self.host, self.port)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn parses_hostname_host_port() {
        let addr: HostPort = "stratum.ckpool.org:3336".parse().unwrap();
        assert_eq!(addr.host, "stratum.ckpool.org");
        assert_eq!(addr.port, 3336);
        assert_eq!(addr.to_string(), "stratum.ckpool.org:3336");
    }

    #[test]
    fn parses_ip_literals() {
        let ipv4: HostPort = "75.119.150.111:3333".parse().unwrap();
        assert_eq!(ipv4.host, "75.119.150.111");
        assert_eq!(ipv4.port, 3333);

        let ipv6: HostPort = "[::1]:3334".parse().unwrap();
        assert_eq!(ipv6.host, "::1");
        assert_eq!(ipv6.port, 3334);
        assert_eq!(ipv6.to_string(), "[::1]:3334");
    }

    #[test]
    fn rejects_invalid_addresses() {
        for address in [
            "sri.pool.org",                    // no port
            ":3333",                           // no host
            "::1:3333",                        // unbracketed IPv6
            "stratum+tcp://sri.pool.org:3333", // URL
            "sri.pool.org:not-a-port-number",  // invalid port
            "sri pool.org:42",                 // host with a space
            "sri\tpool.org:42",                // host with a tab
        ] {
            assert!(
                address.parse::<HostPort>().is_err(),
                "{address} should be rejected"
            );
        }
    }

    #[test]
    fn host_port_deserializes_from_toml() {
        #[derive(serde::Deserialize)]
        struct Wrapper {
            addr: HostPort,
        }
        let wrapper: Wrapper = toml::from_str("addr = \"localhost:3333\"").unwrap();
        assert_eq!(wrapper.addr.host, "localhost");
        assert_eq!(wrapper.addr.port, 3333);
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
            Some(HostPort {
                host: "stratum.ckpool.org".to_string(),
                port: 3336
            })
        );

        assert_eq!(
            site.job_declaration_server_address,
            Some(HostPort {
                host: "192.168.0.1".to_string(),
                port: 34256
            })
        );

        assert_eq!(
            site.template_provider_address,
            Some(HostPort {
                host: "::1".to_string(),
                port: 8442
            })
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
            HostPort {
                host: "not-a-valid-host.invalid".to_string(),
                port: 3333,
            }
        );
        assert!(
            targets.unresolved[0]
                .error
                .contains("not-a-valid-host.invalid:3333"),
            "error should name the configured address: {}",
            targets.unresolved[0].error
        )
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
