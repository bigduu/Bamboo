# Copilot as OpenAI Provider Variant - Implementation Summary

## Overview
This implementation unifies GitHub Copilot and OpenAI under a single `OpenAiProvider` implementation. Copilot is treated as an OpenAI API-compatible provider with different authentication (Device Code OAuth) and custom headers.

## Files Modified

### 1. bamboo-llm/src/auth/mod.rs
- **Added**: Unified `Authenticator` trait with methods:
  - `get_auth_header()` - Returns authentication header
  - `needs_refresh()` - Check if token needs refresh
  - `refresh()` - Refresh authentication
- **Added**: `ApiKeyAuth` struct for API key authentication
- **Added**: `BearerAuth` struct for bearer token authentication
- **Added**: `NoAuth` struct for no authentication
- **Exported**: DeviceCodeAuth, TokenCache, CopilotToken

### 2. bamboo-llm/src/auth/device_code.rs
- **Implemented**: `DeviceCodeAuth` struct implementing `Authenticator` trait
- **Features**:
  - Request device code from GitHub
  - Poll for access token
  - Exchange for Copilot token
  - Cache tokens to `~/.bamboo/copilot_token.json`
  - Automatic token refresh when near expiry
- **Functions**:
  - `new()` - Create with default GitHub settings
  - `with_config()` - Create with custom OAuth endpoints
  - `init()` - Load cached token
  - `authenticate()` - Perform full device code flow
  - `is_authenticated()` - Check if valid token exists
  - `logout()` - Clear cached token

### 3. bamboo-llm/src/auth/token.rs
- **Implemented**: Token polling and exchange functions
- **Types**: `AccessTokenResponse`, `CopilotToken`
- **Functions**:
  - `poll_access_token()` - Poll GitHub for access token
  - `get_copilot_token()` - Exchange access token for Copilot token

### 4. bamboo-llm/src/auth/cache.rs
- **Implemented**: `TokenCache` struct for persisting tokens
- **Path**: `~/.bamboo/copilot_token.json`
- **Features**:
  - Secure file permissions (0o600 on Unix)
  - Token validity checking (5 minute buffer before expiry)
  - Automatic expiry tracking

### 5. bamboo-llm/src/provider/config.rs
- **Added**: `AuthConfig` enum with variants:
  - `ApiKey { key: String }`
  - `Bearer { token: String }`
  - `DeviceCode { client_id, device_code_url, access_token_url, copilot_token_url }`
  - `None`
- **Enhanced**: `ProviderConfig` struct:
  - Added `auth: AuthConfig` field (flattened for serialization)
  - Added `headers: HashMap<String, String>` for custom headers
  - Builder methods: `with_api_key()`, `with_bearer_token()`, `with_device_code()`, `with_headers()`

### 6. bamboo-llm/src/provider/base.rs
- **Enhanced**: `BaseProvider` now uses `Authenticator` trait
- **Features**:
  - Dynamic authenticator creation based on `AuthConfig`
  - Automatic authentication refresh before requests
  - Custom header support
- **Methods**:
  - `new()` now async to support DeviceCode initialization
  - `with_authenticator()` for custom authenticator injection

### 7. bamboo-llm/src/provider/mod.rs
- **Exported**: `AuthConfig` alongside existing types

### 8. bamboo-llm/src/providers/openai.rs
- **Enhanced**: `OpenAiProvider` now supports all authentication methods
- **Constructors**:
  - `new(api_key)` - Standard OpenAI with API key
  - `with_config(config)` - Custom configuration (async)
  - `with_base_url(api_key, base_url)` - Custom endpoint
  - `copilot()` - GitHub Copilot with device code auth
  - `copilot_with_headers(headers)` - Copilot with custom headers
- **Capabilities**: Adjusted based on provider ID (Copilot doesn't support vision/JSON mode)

### 9. bamboo-llm/src/providers/mod.rs
- **Removed**: `copilot` module export (now handled by OpenAiProvider)
- **Kept**: `openai` and `forward` modules

### 10. bamboo-llm/src/lib.rs
- **Exported**: New types from auth module
  - `Authenticator`, `ApiKeyAuth`, `BearerAuth`, `DeviceCodeAuth`
  - `AuthConfig` from provider module

### 11. bamboo-llm/src/error.rs
- **Added**: `AuthError` enum with variants:
  - `Failed(String)`
  - `TokenExpired`
  - `DeviceCodeExpired`
  - `AccessDenied`
  - `Network(String)`
  - `Io(std::io::Error)`
  - `Json(serde_json::Error)`
- **Added**: `From<AuthError>` impl for `LLMError`

### 12. bamboo-config/src/config.rs
- **Renamed**: `ProviderConfig` → `ProviderSettings` (to avoid conflict with bamboo-llm's ProviderConfig)
- **Enhanced**: `AuthSettings` enum with variants:
  - `ApiKey { env: String }` - Read from environment variable
  - `Bearer { env: String }` - Read from environment variable
  - `DeviceCode { client_id, device_code_url, access_token_url, copilot_token_url }`
  - `None`
- **Added**: Custom headers support in `ProviderSettings`
- **Updated**: Default providers configuration with proper auth settings

### 13. bamboo-config/src/lib.rs
- **Updated**: Exports to use new type names (`ProviderSettings`, `AuthSettings`)

### 14. bamboo-llm/src/providers/forward.rs
- **Updated**: Made `ForwardProvider` methods async to match `BaseProvider::new()`
- **Methods**: `new()`, `with_config()`, `with_api_key()` are now async

### 15. Deleted Files
- `bamboo-llm/src/providers/copilot.rs` - No longer needed, functionality merged into OpenAiProvider

## Configuration Examples

### TOML Configuration (bamboo.toml)

```toml
[[providers]]
id = "openai"
name = "OpenAI"
base_url = "https://api.openai.com/v1"
auth = { type = "api_key", env = "OPENAI_API_KEY" }
model = "gpt-4o-mini"

[[providers]]
id = "copilot"
name = "GitHub Copilot"
base_url = "https://api.githubcopilot.com"
auth = { type = "device_code", client_id = "Iv1.b507a08c87ecfe98" }
model = "copilot-chat"
headers = [
    { key = "editor-version", value = "vscode/1.99.2" },
    { key = "editor-plugin-version", value = "copilot-chat/0.20.3" },
]

[[providers]]
id = "moonshot"
name = "Moonshot"
base_url = "https://api.moonshot.cn/v1"
auth = { type = "api_key", env = "MOONSHOT_API_KEY" }
model = "moonshot-v1-8k"
```

### Rust Code Examples

```rust
use bamboo_llm::{OpenAiProvider, ProviderConfig, LLMProvider};

// OpenAI with API key
let openai = OpenAiProvider::new("sk-...")?;

// GitHub Copilot (async, triggers device code flow)
let copilot = OpenAiProvider::copilot().await?;

// Custom configuration
let config = ProviderConfig::new("custom", "https://api.example.com/v1")
    .with_bearer_token("token-...")
    .with_header("X-Custom-Header", "value");
let provider = OpenAiProvider::with_config(config).await?;
```

## Compilation

```bash
cd /Users/bigduu/workspace/bamboo
cargo check -p bamboo-llm
cargo check -p bamboo-config
```

Both crates compile successfully with no errors.

## Key Benefits

1. **Code Reuse**: Copilot and OpenAI share ~90% of code
2. **Easy Extension**: Adding new OpenAI-compatible providers (Moonshot, DeepSeek, etc.) requires only configuration
3. **Unified Interface**: All providers use the same `LLMProvider` trait
4. **Configuration-Driven**: New providers can be added without recompiling
5. **Flexible Authentication**: Support for API key, Bearer token, and Device Code OAuth

## Token Cache Location

Device Code authentication tokens are cached at:
- **Path**: `~/.bamboo/copilot_token.json`
- **Permissions**: 0o600 (read/write for owner only on Unix)
- **Expiry**: Tokens considered expired 5 minutes before actual expiry
