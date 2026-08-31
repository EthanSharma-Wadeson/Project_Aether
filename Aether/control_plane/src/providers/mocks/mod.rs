pub mod finance;
pub mod hostile;
pub mod research;

pub use finance::FinanceSimulationModel;
pub use hostile::HostileInjectionModel;
pub use research::ResearchAgentModel;

use std::sync::Arc;

use super::errors::ProviderAdapterError;
use super::external::anthropic;
use super::external::transport::LabHttpTransport;
use super::mapper::MockAllowlistProfile;
use super::traits::Provider;

/// Select mock or external lab provider. `transport` overrides live HTTP for tests.
pub async fn select_provider(
    profile: &str,
    transport: Option<Arc<dyn LabHttpTransport>>,
) -> Result<(MockAllowlistProfile, Box<dyn Provider>), ProviderAdapterError> {
    if let Some(ext) = anthropic::try_build_external(profile, transport)? {
        return Ok(ext);
    }
    let p = MockAllowlistProfile::parse(profile).ok_or_else(|| {
        ProviderAdapterError::UnknownMockProfile(profile.to_string())
    })?;
    // External profile names already handled; remaining are mocks only.
    let provider: Box<dyn Provider> = match p {
        MockAllowlistProfile::Research
            if matches!(
                profile.trim().to_ascii_lowercase().as_str(),
                "research" | "research_agent" | "agent_a"
            ) =>
        {
            Box::new(ResearchAgentModel)
        }
        MockAllowlistProfile::Finance
            if matches!(
                profile.trim().to_ascii_lowercase().as_str(),
                "finance" | "finance_simulation" | "agent_b"
            ) =>
        {
            Box::new(FinanceSimulationModel)
        }
        MockAllowlistProfile::Hostile => Box::new(HostileInjectionModel),
        MockAllowlistProfile::Research => Box::new(ResearchAgentModel),
        MockAllowlistProfile::Finance => Box::new(FinanceSimulationModel),
    };
    Ok((p, provider))
}

pub fn select_mock(
    profile: &str,
) -> Result<(MockAllowlistProfile, Box<dyn Provider>), ProviderAdapterError> {
    // Sync helper for unit tests — mocks only.
    let p = MockAllowlistProfile::parse(profile).ok_or_else(|| {
        ProviderAdapterError::UnknownMockProfile(profile.to_string())
    })?;
    if matches!(
        profile.trim().to_ascii_lowercase().as_str(),
        "anthropic" | "claude" | "external" | "external_research" | "anthropic_finance"
            | "claude_finance" | "external_finance"
    ) {
        return Err(ProviderAdapterError::UnknownMockProfile(
            "use select_provider for external lab profiles".into(),
        ));
    }
    let provider: Box<dyn Provider> = match p {
        MockAllowlistProfile::Research => Box::new(ResearchAgentModel),
        MockAllowlistProfile::Finance => Box::new(FinanceSimulationModel),
        MockAllowlistProfile::Hostile => Box::new(HostileInjectionModel),
    };
    Ok((p, provider))
}
