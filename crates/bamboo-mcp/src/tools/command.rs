use tokio::process::Command;
use tokio::time::{timeout, Duration};
use serde_json::json;

/// 最大输出大小（1MB）
const MAX_OUTPUT_SIZE: usize = 1024 * 1024;

/// 命令执行工具
pub struct CommandTool;

/// 命令执行结果
#[derive(Debug)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

/// 允许执行的安全命令白名单
const ALLOWED_COMMANDS: &[&str] = &[
    // 文件/目录操作
    "ls", "cat", "head", "tail", "less", "more",
    "pwd", "cd", "dirname", "basename",
    "find", "grep", "wc", "sort", "uniq", "diff",
    "file", "stat", "readlink", "realpath",
    // 文本处理
    "echo", "printf", "sed", "awk", "cut", "tr",
    "tee", "xargs",
    // 系统信息
    "uname", "hostname", "whoami", "id", "groups",
    "date", "cal", "uptime", "which", "whereis",
    "env", "printenv",
    // 进程管理（只读）
    "ps", "top", "htop", "pgrep", "pidof",
    // 网络工具
    "ping", "curl", "wget", "nc", "netstat", "ss",
    "host", "dig", "nslookup", "whois",
    // 版本控制
    "git", "svn",
    // 构建工具
    "cargo", "rustc", "make", "cmake", "npm", "yarn", "pnpm",
    // 压缩/解压
    "tar", "gzip", "gunzip", "zip", "unzip", "bzip2", "xz",
    // 其他常用工具
    "tree", "du", "df", "free", "man", "help",
    "clear", "history", "alias", "type",
    "jq", "yq", "rg", "fd", "fzf", "bat", "exa", "lsd",
    "md5sum", "sha256sum", "shasum", "base64",
    "uuidgen", "openssl",
    // 编辑器（只读模式或安全模式）
    "vim", "nvim", "emacs", "nano", "code",
    // 脚本解释器（谨慎使用）
    "python3", "python", "node", "deno", "bun",
    "ruby", "perl", "php", "lua",
    "sh", "bash", "zsh", "fish",
];

impl CommandTool {
    /// 检查命令是否在白名单中
    fn is_command_allowed(cmd: &str) -> bool {
        // 获取命令的基础名称（去除路径）
        let base_cmd = std::path::Path::new(cmd)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(cmd);
        
        ALLOWED_COMMANDS.contains(&base_cmd)
    }

    /// 截断输出到最大大小
    fn truncate_output(output: String) -> String {
        if output.len() > MAX_OUTPUT_SIZE {
            let truncated = &output[..MAX_OUTPUT_SIZE];
            format!("{}\n[输出已截断 - 超过 {} 字节限制]", truncated, MAX_OUTPUT_SIZE)
        } else {
            output
        }
    }

    /// 执行系统命令（带超时）
    pub async fn execute(
        cmd: &str, 
        args: Vec<String>, 
        cwd: Option<&str>,
        timeout_secs: u64,
    ) -> Result<CommandResult, String> {
        // 白名单检查
        if !Self::is_command_allowed(cmd) {
            return Err(format!(
                "Command '{}' is not in the allowed whitelist. Allowed commands: {}",
                cmd,
                ALLOWED_COMMANDS.join(", ")
            ));
        }
        
        // 验证参数：禁止 shell 元字符
        for arg in &args {
            if arg.contains(';') || arg.contains('|') || arg.contains('&') 
                || arg.contains('$') || arg.contains('`') || arg.contains('(')
                || arg.contains(')') || arg.contains('<') || arg.contains('>')
                || arg.contains('*') || arg.contains('?') || arg.contains('[')
                || arg.contains(']') || arg.contains('{') || arg.contains('}')
                || arg.contains('~') || arg.contains('#') || arg.contains('\\')
                || arg.contains('\n') || arg.contains('\r') || arg.contains('\t')
            {
                return Err(format!(
                    "Invalid argument contains shell metacharacters: {}",
                    arg
                ));
            }
        }
        
        let mut command = Command::new(cmd);
        command.args(&args);
        
        if let Some(dir) = cwd {
            // 安全检查：确保工作目录不包含 .. 或绝对路径遍历
            let path = std::path::Path::new(dir);
            if dir.contains("..") {
                return Err("Invalid working directory: contains '..'".to_string());
            }
            // 规范化路径
            let canonical = std::fs::canonicalize(path)
                .map_err(|e| format!("Invalid working directory: {}", e))?;
            command.current_dir(canonical);
        }
        
        // 执行命令并设置超时
        let output = timeout(
            Duration::from_secs(timeout_secs),
            command.output()
        ).await
        .map_err(|_| format!("Command timed out after {} seconds", timeout_secs))?
        .map_err(|e| format!("Failed to execute command '{}': {}", cmd, e))?;
        
        let stdout = Self::truncate_output(String::from_utf8_lossy(&output.stdout).to_string());
        let stderr = Self::truncate_output(String::from_utf8_lossy(&output.stderr).to_string());
        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();
        
        Ok(CommandResult {
            stdout,
            stderr,
            exit_code,
            success,
        })
    }
    
    /// 执行简单命令（默认 30 秒超时）
    pub async fn execute_simple(cmd: &str, args: Vec<String>) -> Result<String, String> {
        let result = Self::execute(cmd, args, None, 30).await?;
        
        if result.success {
            Ok(result.stdout)
        } else {
            Err(format!(
                "Command failed with exit code {}: {}",
                result.exit_code,
                if result.stderr.is_empty() { &result.stdout } else { &result.stderr }
            ))
        }
    }
    
    /// 执行命令并返回详细结果
    pub async fn execute_detailed(
        cmd: &str, 
        args: Vec<String>,
        cwd: Option<&str>,
    ) -> Result<CommandResult, String> {
        Self::execute(cmd, args, cwd, 30).await
    }
    
    /// 获取当前工作目录
    pub async fn get_current_dir() -> Result<String, String> {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| format!("Failed to get current directory: {}", e))
    }
    
    /// 检查命令是否存在
    pub async fn command_exists(cmd: &str) -> bool {
        // 先检查白名单
        if !Self::is_command_allowed(cmd) {
            return false;
        }
        
        Command::new("which")
            .arg(cmd)
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    /// 获取工具 schema
    pub fn get_tool_schemas() -> Vec<serde_json::Value> {
        vec![
            json!({
                "type": "function",
                "function": {
                    "name": "execute_command",
                    "description": "执行系统命令。只允许白名单中的安全命令（如 ls, cat, pwd, echo, git, cargo 等）。命令会在 30 秒后超时，输出限制 1MB",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "命令名称，例如 'ls', 'cat', 'git'。必须是白名单中的命令"
                            },
                            "args": {
                                "type": "array",
                                "items": {
                                    "type": "string"
                                },
                                "description": "命令参数列表。禁止包含 shell 元字符"
                            },
                            "cwd": {
                                "type": "string",
                                "description": "工作目录（可选），默认为当前目录"
                            }
                        },
                        "required": ["command"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "get_current_dir",
                    "description": "获取当前工作目录的绝对路径",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            }),
        ]
    }
}

impl CommandResult {
    /// 格式化输出用于显示
    pub fn format_output(&self) -> String {
        let mut output = String::new();
        
        if !self.stdout.is_empty() {
            output.push_str("STDOUT:\n");
            output.push_str(&self.stdout);
            output.push('\n');
        }
        
        if !self.stderr.is_empty() {
            output.push_str("STDERR:\n");
            output.push_str(&self.stderr);
            output.push('\n');
        }
        
        output.push_str(&format!("Exit code: {}\n", self.exit_code));
        output.push_str(&format!("Success: {}\n", self.success));
        
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_echo() {
        let result = CommandTool::execute_simple("echo", vec!["Hello".to_string()]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Hello"));
    }

    #[tokio::test]
    async fn test_execute_ls() {
        let result = CommandTool::execute_simple("ls", vec!["/tmp".to_string()]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dangerous_command_blocked() {
        // rm 不在白名单中，应该被拒绝
        let result = CommandTool::execute_simple("rm", vec!["-rf".to_string(), "/".to_string()]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not in the allowed whitelist"));
    }

    #[tokio::test]
    async fn test_command_not_found() {
        let result = CommandTool::execute_simple("nonexistent_command_xyz", vec![]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_whitelist_blocks_unauthorized() {
        // 测试不在白名单中的命令被阻止
        let result = CommandTool::execute_simple("dd", vec!["if=/dev/zero".to_string()]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not in the allowed whitelist"));
    }

    #[tokio::test]
    async fn test_shell_metacharacters_blocked() {
        // 测试包含 shell 元字符的参数被阻止
        let result = CommandTool::execute_simple("echo", vec!["hello; rm -rf /".to_string()]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("shell metacharacters"));
    }

    #[tokio::test]
    async fn test_is_command_allowed() {
        assert!(CommandTool::is_command_allowed("ls"));
        assert!(CommandTool::is_command_allowed("/bin/ls"));
        assert!(!CommandTool::is_command_allowed("rm"));
        assert!(!CommandTool::is_command_allowed("/bin/rm"));
        assert!(!CommandTool::is_command_allowed("dd"));
    }
}
