use serde::{Deserialize, Serialize};
use std::collections::HashMap; // Added for HashMap

/// Settings for a custom boundary component.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CustomBoundarySetting {
    pub component: String,
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
    /// Configuration for custom boundaries
    #[serde(default)] // Add default for Option<HashMap<...>>
    pub custom_boundaries: Option<HashMap<String, CustomBoundarySetting>>,
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
