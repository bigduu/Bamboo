use crate::MessageHandler;
use crate::message::{Message, MessageKind, MessagePayload};
use crate::router::{MessageBus, Topics};
use crate::{BambooError, Result};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use tracing::{debug, error, info, instrument, warn};

/// 系统处理器 - 处理系统级消息
pub struct SystemHandler {
    name: String,
    /// 系统操作注册表
    actions: HashMap<String, Box<dyn SystemAction>>,
}

impl SystemHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            name: "SystemHandler".to_string(),
            actions: HashMap::new(),
        };
        
        handler.register_builtin_actions();
        handler
    }

    /// 注册内置系统操作
    fn register_builtin_actions(&mut self) {
        // health - 健康检查
        self.register("health", |_params| {
            json!({
                "status": "healthy",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        });

        // metrics - 系统指标
        self.register("metrics", |_params| {
            json!({
                "memory": "calculating...",
                "cpu": "calculating...",
                "uptime": "calculating...",
            })
        });

        // config - 获取/设置配置
        self.register("config", |params| {
            if let Some(key) = params.get("key") {
                json!({ "key": key, "value": null })
            } else {
                json!({ "config": {} })
            }
        });

        // shutdown - 优雅关闭
        self.register("shutdown", |_params| {
            json!({ "status": "shutdown_initiated" })
        });

        // subscribe - 订阅主题
        self.register("subscribe", |params| {
            if let Some(topic) = params.get("topic").and_then(|v| v.as_str()) {
                json!({ "status": "subscribed", "topic": topic })
            } else {
                json!({ "error": "missing topic parameter" })
            }
        });

        // unsubscribe - 取消订阅
        self.register("unsubscribe", |params| {
            if let Some(topic) = params.get("topic").and_then(|v| v.as_str()) {
                json!({ "status": "unsubscribed", "topic": topic })
            } else {
                json!({ "error": "missing topic parameter" })
            }
        });
    }

    /// 注册自定义系统操作
    pub fn register<F>(&mut self, name: impl Into<String>, action: F)
    where
        F: Fn(&HashMap<String, serde_json::Value>) -> serde_json::Value + Send + Sync + 'static,
    {
        let name = name.into();
        debug!("Registering system action: {}", name);
        self.actions.insert(name, Box::new(action));
    }

    /// 执行系统操作
    async fn execute(
        &self,
        action: &str,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        match self.actions.get(action) {
            Some(executor) => {
                let result = executor.execute(params).await;
                Ok(result)
            }
            None => Err(BambooError::Handler(format!(
                "Unknown system action: {}",
                action
            ))),
        }
    }
}

impl Default for SystemHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 系统操作 trait
#[async_trait]
pub trait SystemAction: Send + Sync {
    async fn execute(&self, params: &HashMap<String, serde_json::Value>) -> serde_json::Value;
}

#[async_trait]
impl<F> SystemAction for F
where
    F: Fn(&HashMap<String, serde_json::Value>) -> serde_json::Value + Send + Sync,
{
    async fn execute(&self, params: &HashMap<String, serde_json::Value>) -> serde_json::Value {
        (self)(params)
    }
}

#[async_trait]
impl MessageHandler for SystemHandler {
    fn name(&self) -> &str {
        &self.name
    }

    #[instrument(skip(self, bus), fields(msg_id = %msg.metadata.id))]
    async fn handle(&self, msg: Message, bus: &MessageBus) -> Result<Option<Message>> {
        info!("Processing system message for session: {}", msg.session_id());

        // 解析系统操作
        let (action, params) = match &msg.payload {
            MessagePayload::System(sys) => (sys.action.clone(), sys.params.clone()),
            _ => {
                return Err(BambooError::Handler(
                    "Invalid system message payload".to_string(),
                ))
            }
        };

        debug!("Executing system action: {} with params: {:?}", action, params);

        // 执行系统操作
        match self.execute(&action, &params).await {
            Ok(result) => {
                let response_text = serde_json::to_string(&result).unwrap_or_default();
                let response = Message::response(&msg, response_text);
                bus.publish(Topics::system_output(), response.clone()).await?;
                Ok(Some(response))
            }
            Err(e) => {
                let error_msg = Message::error(&msg, format!("System action failed: {}", e));
                bus.publish(Topics::system_output(), error_msg.clone()).await?;
                Ok(Some(error_msg))
            }
        }
    }

    fn can_handle(&self, kind: &MessageKind) -> bool {
        matches!(kind, MessageKind::System)
    }
}

/// 会话管理器 - 处理会话相关操作
pub struct SessionManager {
    name: String,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            name: "SessionManager".to_string(),
        }
    }

    /// 创建新会话
    async fn create_session(&self, client_id: &str) -> String {
        let session_id = uuid::Uuid::new_v4().to_string();
        info!("Created new session {} for client {}", session_id, client_id);
        session_id
    }

    /// 结束会话
    async fn end_session(&self, session_id: &str) {
        info!("Ending session {}", session_id);
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageHandler for SessionManager {
    fn name(&self) -> &str {
        &self.name
    }

    async fn handle(&self, msg: Message, _bus: &MessageBus) -> Result<Option<Message>> {
        // 会话管理逻辑
        Ok(None)
    }

    fn can_handle(&self, kind: &MessageKind) -> bool {
        // 可以处理所有消息类型进行会话跟踪
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_action() {
        let handler = SystemHandler::new();
        let result = handler.execute("health", &HashMap::new()).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["status"], "healthy");
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let handler = SystemHandler::new();
        let result = handler.execute("unknown", &HashMap::new()).await;
        assert!(result.is_err());
    }
}
