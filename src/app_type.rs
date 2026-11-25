use stratum_apps::stratum_core::common_messages_sv2::Protocol;

/// The type of Sv2 application under test.
///
/// This selects the suite to run. It is not a claim that the endpoint serves
/// only this protocol: §3 allows one deployment to serve more than one Sv2
/// sub-protocol on the same address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppType {
    MiningServer,
    JobDeclarationServer,
    TemplateProvider,
}

impl AppType {
    /// Every server role the suite knows how to test.
    pub const ALL: [AppType; 3] = [
        AppType::MiningServer,
        AppType::JobDeclarationServer,
        AppType::TemplateProvider,
    ];

    /// A human-readable name for reports.
    pub fn name(&self) -> &'static str {
        match self {
            AppType::MiningServer => "Mining Protocol",
            AppType::JobDeclarationServer => "Job Declaration",
            AppType::TemplateProvider => "Template Distribution",
        }
    }

    /// The sub-protocol a well-behaved client speaks to this app type.
    pub fn protocol(&self) -> Protocol {
        match self {
            AppType::MiningServer => Protocol::MiningProtocol,
            AppType::JobDeclarationServer => Protocol::JobDeclarationProtocol,
            AppType::TemplateProvider => Protocol::TemplateDistributionProtocol,
        }
    }

    /// The flags a well-behaved client sets on `SetupConnection` for this app type.
    pub fn default_flags(&self) -> u32 {
        match self {
            AppType::MiningServer => 0,
            // DECLARE_TX_DATA selects Full-Template mode.
            AppType::JobDeclarationServer => 0b0001,
            AppType::TemplateProvider => 0,
        }
    }
}
