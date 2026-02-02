//! Tool execution implementation

use crate::error::{Result, ToolError};
use crate::types::{ArgDef, ToolDef, ToolResult, ToolType};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

/// Trait for executing tools
#[async_trait]
pub trait ToolRunner: Send + Sync {
    /// Execute a tool with given arguments
    async fn execute(&self, tool_def: &ToolDef, args: HashMap<String, Value>) -> Result<ToolResult>;

    /// Validate arguments against tool definition
    fn validate_args(&self, tool_def: &ToolDef, args: &HashMap<String, Value>) -> Result<()>;
}

/// Default tool executor with configurable timeout and security settings
#[derive(Debug, Clone)]
pub struct ToolExecutor {
    timeout: Duration,
    allowed_commands: Vec<String>,
    dangerous_commands: Vec<String>,
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            allowed_commands: vec![],
            dangerous_commands: vec![
                "rm -rf /".to_string(),
                "mkfs".to_string(),
                "dd if=/dev/zero".to_string(),
                ":(){ :|:& };:".to_string(), // fork bomb
            ],
        }
    }
}

impl ToolExecutor {
    /// Create a new executor with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set execution timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout = Duration::from_secs(seconds);
        self
    }

    /// Set allowed commands (empty means allow all except dangerous)
    pub fn with_allowed_commands(mut self, commands: Vec<String>) -> Self {
        self.allowed_commands = commands;
        self
    }

    /// Check if command contains dangerous patterns
    fn is_dangerous(&self, command: &str) -> bool {
        let cmd_lower = command.to_lowercase();
        self.dangerous_commands
            .iter()
            .any(|dangerous| cmd_lower.contains(&dangerous.to_lowercase()))
    }

    /// Check if command is allowed
    fn is_allowed(&self, command: &str) -> bool {
        if self.allowed_commands.is_empty() {
            return true;
        }
        self.allowed_commands
            .iter()
            .any(|allowed| command.contains(allowed))
    }

    /// Build the command to execute based on tool type
    fn build_command(&self, tool_def: &ToolDef, args: &HashMap<String, Value>) -> Result<Command> {
        let cmd_path = Path::new(&tool_def.command);

        // Check if command is dangerous
        if self.is_dangerous(&tool_def.command) {
            return Err(ToolError::CommandNotAllowed(tool_def.command.clone()));
        }

        // Check if command is allowed
        if !self.is_allowed(&tool_def.command) {
            return Err(ToolError::CommandNotAllowed(tool_def.command.clone()));
        }

        // Determine tool type and build command
        let tool_type = ToolType::from_path(cmd_path);

        let mut command = match tool_type {
            Some(tool_type) => {
                let mut cmd = Command::new(tool_type.interpreter());
                cmd.arg(&tool_def.command);
                cmd
            }
            None => {
                // Direct execution for binaries
                Command::new(&tool_def.command)
            }
        };

        // Inject arguments as environment variables
        for (key, value) in args {
            let env_value = match value {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            command.env(format!("ARG_{}", key.to_uppercase()), env_value);
        }

        Ok(command)
    }
}

#[async_trait]
impl ToolRunner for ToolExecutor {
    fn validate_args(&self, tool_def: &ToolDef, args: &HashMap<String, Value>) -> Result<()> {
        for arg_def in &tool_def.args {
            let value = args.get(&arg_def.name);

            if arg_def.required && value.is_none() {
                return Err(ToolError::MissingArgument(arg_def.name.clone()));
            }

            if let Some(value) = value {
                if !arg_def.arg_type.matches(value) {
                    return Err(ToolError::TypeMismatch {
                        expected: arg_def.arg_type.to_string(),
                        actual: value.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    async fn execute(&self, tool_def: &ToolDef, args: HashMap<String, Value>) -> Result<ToolResult> {
        // Validate arguments
        self.validate_args(tool_def, &args)?;

        // Check if script exists and is executable
        let cmd_path = Path::new(&tool_def.command);
        if !cmd_path.exists() {
            return Err(ToolError::NotFound(format!(
                "Script not found: {}",
                tool_def.command
            )));
        }

        let start = Instant::now();

        // Build and execute command
        let mut command = self.build_command(tool_def, &args)?;

        let result = timeout(self.timeout, command.output()).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                if output.status.success() {
                    Ok(ToolResult::success(stdout, duration_ms))
                } else {
                    let error = if stderr.is_empty() {
                        format!("Exit code: {:?}", output.status.code())
                    } else {
                        stderr
                    };
                    Ok(ToolResult::failure(error, duration_ms))
                }
            }
            Ok(Err(e)) => Err(ToolError::ExecutionFailed(format!(
                "Failed to execute: {}",
                e
            ))),
            Err(_) => Err(ToolError::Timeout(self.timeout.as_millis() as u64)),
        }
    }
}
