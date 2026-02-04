pub mod transformer;
pub mod provider;
pub mod providers;
pub mod error;
pub mod auth;
pub mod adapters;

// Re-export core types
pub use error::{LLMError, AuthError, ConversionError, Result};
pub use transformer::{SchemaTransformer, LLMStream};
pub use provider::{LLMProvider, BaseProvider, ProviderConfig, AuthConfig, ProviderMetadata, ProviderCapabilities};
pub use auth::{Authenticator, ApiKeyAuth, BearerAuth, DeviceCodeAuth};
pub use providers::{OpenAiProvider, AnthropicProvider, ForwardProvider, DirectProvider};
