use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for a boundary component
#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Boundary {
    /// The component name to replace with
    pub component: String,
    /// The package to import the component from
    pub from: String,
}

/// Static plugin configuration.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Whether the plugin is enabled
    #[serde(default = "default_enabled")]
    pub enabled: Option<bool>,
    /// Boundary configurations (e.g., [{ component: 'ErrorBoundary', from: 'my-package' }])
    #[serde(default)]
    pub boundaries: HashSet<Boundary>,
}

/// Default value for the enabled field (defaults to Some(true) if not specified).
fn default_enabled() -> Option<bool> {
    None
}

/// Additional context for the plugin.
#[derive(Debug)]
pub struct Context {
    /// The target environment (from `NODE_ENV`).
    pub env_name: Environment,
    /// The name of the current file.
    pub filename: String,
}

/// The target environment.
#[derive(Debug, PartialEq, Eq)]
pub enum Environment {
    /// Development mode where Suspense tracking is most useful for debugging
    Development,
    /// Test mode where Suspense tracking is typically disabled like in production
    Test,
    /// Production mode where Suspense tracking is typically disabled for performance
    Production,
}

impl TryFrom<&str> for Environment {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "development" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "production" => Ok(Self::Production),
            _ => Err(format!("{value} is not a valid environment")),
        }
    }
}
