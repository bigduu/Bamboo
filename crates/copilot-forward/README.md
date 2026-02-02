# copilot-forward

GitHub Copilot forwarding client with configurable UI behavior.

## Features

- ✅ **Device Code Authentication** - Complete OAuth flow with GitHub
- ✅ **Token Caching** - Automatic token persistence to `~/.copilot-forward/`
- ✅ **HTTP Forwarding** - Direct API access to `api.githubcopilot.com`
- ✅ **Configurable UI** - GUI, headless, or silent modes

## Quick Start

### GUI Mode (Default)

```rust
use copilot_forward::{CopilotClient, UiConfig};

let client = CopilotClient::new(UiConfig::gui()).await?;
let response = client.chat_completions(&request).await?;
```

### Headless Mode

```rust
use copilot_forward::{CopilotClient, UiConfig};

let client = CopilotClient::new(UiConfig::headless()).await?;
let response = client.chat_completions(&request).await?;
```

## UI Configuration

### `UiConfig::gui()`
Full GUI support:
- ✅ Open browser automatically
- ✅ Copy device code to clipboard
- ✅ Show GUI dialogs
- ✅ Print console output

### `UiConfig::headless()`
Console output only:
- ❌ No browser opening
- ❌ No clipboard operations
- ❌ No GUI dialogs
- ✅ Print console output

### `UiConfig::silent()`
No output:
- ❌ No UI operations
- ❌ No console output
- For custom handling

### Custom Configuration

```rust
use copilot_forward::UiConfig;

let config = UiConfig::none()
    .with_console()
    .with_browser();
```

## Cargo Features

```toml
[dependencies]
copilot-forward = { version = "0.1", features = ["gui"] }
```

- `gui` (default) - Enable browser, clipboard, and GUI dialogs
- `headless` - Console output only
- `sse` - Enable SSE streaming support

## Authentication Flow

1. **First Run**: Device Code authentication
   ```
   🔐 GitHub Copilot Authorization Required
   
   1. Open: https://github.com/login/device
   2. Enter code: XXXX-XXXX
   3. Click 'Authorize'
   ```

2. **Token Caching**: Saved to `~/.copilot-forward/token.json`

3. **Subsequent Runs**: Automatic token reuse

## Examples

### GUI Mode

```bash
cargo run --example gui_mode --features gui
```

### Headless Mode

```bash
cargo run --example headless_mode --features headless
```

## API

### `CopilotClient`

```rust
// Create with UI config
let client = CopilotClient::new(UiConfig::gui()).await?;

// Create with existing token
let client = CopilotClient::with_token("ghu_xxx", UiConfig::silent());

// Check authentication
assert!(client.is_authenticated());

// Chat completions
let response = client.chat_completions(&request).await?;

// List models
let models = client.models().await?;

// Logout
client.logout()?;
```

## License

MIT OR Apache-2.0
