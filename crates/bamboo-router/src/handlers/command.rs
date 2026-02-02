use crate::MessageHandler;
use crate::message::{Message, MessageKind, MessagePayload};
use crate::router::{MessageBus, Topics};
use crate::{BambooError, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, error, info, instrument, warn};

/// 命令处理器
pub struct CommandHandler {
    name: String,
    /// 命令注册表
    commands: HashMap<String, Box<dyn CommandExecutor>>,
}

impl CommandHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            name: "CommandHandler".to_string(),
            commands: HashMap::new(),
        };
        
        // 注册内置命令
        handler.register_builtin_commands();
        handler
    }

    /// 注册内置命令
    fn register_builtin_commands(&mut self) {
        // help 命令
        self.register("help", |args| {
            format!("Available commands: ping, status, help, echo")
        });

        // ping 命令
        self.register("ping", |_args| {
            "Pong!".to_string()
        });

        // status 命令
        self.register("status", |_args| {
            "Bamboo is running".to_string()
        });

        // echo 命令
        self.register("echo", |args| {
            args.join(" ")
        });
    }

    /// 注册自定义命令
    pub fn register<F>(&mut self, name: impl Into<String>, executor: F)
    where
        F: Fn(&Vec<String>) -> String + Send + Sync + 'static,
    {
        let name = name.into();
        debug!("Registering command: {}", name);
        self.commands.insert(name, Box::new(executor));
    }

    /// 执行命令
    async fn execute(&self, cmd: &str, args: &Vec<String>) -> Result<String> {
        match self.commands.get(cmd) {
            Some(executor) => {
                let result = executor.execute(args).await;
                Ok(result)
            }
            None => {
                Err(BambooError::Handler(format!("Unknown command: {}", cmd)))
            }
        }
    }

    /// 解析命令消息
    fn parse_command(&self, msg: &Message) -> Result<(String, Vec<String>)> {
        match &msg.payload {
            MessagePayload::Command(cmd) => {
                Ok((cmd.command.clone(), cmd.args.clone()))
            }
            _ => {
                // 尝试从聊天内容中解析命令（以 / 开头）
                if let MessagePayload::Chat(chat) = &msg.payload {
                    let content = chat.content.trim();
                    if content.starts_with('/') {
                        let parts: Vec<&str> = content[1..].split_whitespace().collect();
                        if !parts.is_empty() {
                            let cmd = parts[0].to_string();
                            let args = parts[1..].iter().map(|s| s.to_string()).collect();
                            return Ok((cmd, args));
                        }
                    }
                }
                Err(BambooError::Handler("Not a command message".to_string()))
            }
        }
    }
}

impl Default for CommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 命令执行器 trait
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn execute(&self, args: &Vec<String>) -> String;
}

#[async_trait]
impl<F> CommandExecutor for F
where
    F: Fn(&Vec<String>) -> String + Send + Sync,
{
    async fn execute(&self, args: &Vec<String>) -> String {
        (self)(args)
    }
}

#[async_trait]
impl MessageHandler for CommandHandler {
    fn name(&self) -> &str {
        &self.name
    }

    #[instrument(skip(self, bus), fields(msg_id = %msg.metadata.id))]
    async fn handle(&self, msg: Message, bus: &MessageBus) -> Result<Option<Message>> {
        info!("Processing command for session: {}", msg.session_id());

        // 解析命令
        let (cmd, args) = match self.parse_command(&msg) {
            Ok(result) => result,
            Err(e) => {
                let error_msg = Message::error(&msg, format!("Command parse error: {}", e));
                bus.publish(Topics::command_output(), error_msg.clone()).await?;
                return Ok(Some(error_msg));
            }
        };

        debug!("Executing command: {} with args: {:?}", cmd, args);

        // 执行命令
        match self.execute(&cmd, &args).await {
            Ok(result) => {
                let response = Message::response(&msg, result);
                bus.publish(Topics::command_output(), response.clone()).await?;
                Ok(Some(response))
            }
            Err(e) => {
                let error_msg = Message::error(&msg, format!("Command execution failed: {}", e));
                bus.publish(Topics::command_output(), error_msg.clone()).await?;
                Ok(Some(error_msg))
            }
        }
    }

    fn can_handle(&self, kind: &MessageKind) -> bool {
        // 处理显式命令和聊天中的 /command 格式
        matches!(kind, MessageKind::Command | MessageKind::Chat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ping_command() {
        let handler = CommandHandler::new();
        let result = handler.execute("ping", &vec![]).await;
        assert_eq!(result.unwrap(), "Pong!");
    }

    #[tokio::test]
    async fn test_echo_command() {
        let handler = CommandHandler::new();
        let result = handler.execute("echo", &vec!["Hello".to_string(), "World".to_string()]).await;
        assert_eq!(result.unwrap(), "Hello World");
    }

    #[tokio::test]
    async fn test_unknown_command() {
        let handler = CommandHandler::new();
        let result = handler.execute("unknown", &vec![]).await;
        assert!(result.is_err());
    }
}
