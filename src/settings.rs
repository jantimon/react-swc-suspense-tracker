use serde::{Deserialize, Serialize};

/// Static plugin configuration.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Whether the plugin is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Default value for the enabled field (defaults to true if not specified).
fn default_enabled() -> bool {
    true
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
