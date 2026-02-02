pub mod transformer;
pub mod provider;
pub mod providers;
pub mod error;
pub mod auth;

// Re-export core types
pub use error::{LLMError, AuthError, ConversionError, Result};
pub use transformer::{SchemaTransformer, LLMStream};
pub use provider::{LLMProvider, BaseProvider, ProviderConfig, AuthConfig, ProviderMetadata, ProviderCapabilities};
pub use auth::{Authenticator, ApiKeyAuth, BearerAuth, DeviceCodeAuth};
