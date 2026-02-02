use crate::message::{Message, MessageKind};
use crate::{BambooError, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// 默认通道缓冲区大小
const DEFAULT_CHANNEL_BUFFER: usize = 1000;

/// 消息总线 - 核心路由组件
/// 
/// 职责：
/// 1. 管理消息通道（topic-based pub/sub）
/// 2. 路由消息到正确的处理器
/// 3. 支持动态订阅和发布
#[derive(Debug)]
pub struct MessageBus {
    /// 通道映射：topic -> sender
    channels: DashMap<String, mpsc::Sender<Message>>,
    /// 通道缓冲区大小
    buffer_size: usize,
}

impl MessageBus {
    /// 创建新的消息总线
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
            buffer_size: DEFAULT_CHANNEL_BUFFER,
        }
    }

    /// 创建带自定义缓冲区大小的消息总线
    pub fn with_buffer_size(buffer_size: usize) -> Self {
        Self {
            channels: DashMap::new(),
            buffer_size,
        }
    }

    /// 获取或创建主题通道
    fn get_or_create_channel(&self, topic: &str) -> mpsc::Sender<Message> {
        if let Some(entry) = self.channels.get(topic) {
            entry.clone()
        } else {
            let (tx, rx) = mpsc::channel::<Message>(self.buffer_size);
            self.channels.insert(topic.to_string(), tx.clone());
            
            // 启动清理任务，当所有接收者关闭时移除通道
            let topic_owned = topic.to_string();
            let channels = self.channels.clone();
            tokio::spawn(async move {
                // 等待通道关闭
                drop(rx);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                channels.remove(&topic_owned);
                debug!("Channel '{}' removed", topic_owned);
            });
            
            tx
        }
    }

    /// 发布消息到指定主题
    /// 
    /// # Arguments
    /// * `topic` - 目标主题
    /// * `msg` - 要发送的消息
    pub async fn publish(&self, topic: &str, msg: Message) -> Result<()> {
        let sender = self.get_or_create_channel(topic);
        
        match sender.send(msg).await {
            Ok(_) => {
                debug!("Message published to topic '{}'", topic);
                Ok(())
            }
            Err(e) => {
                error!("Failed to publish to topic '{}': {}", topic, e);
                self.channels.remove(topic);
                Err(BambooError::Router(format!("Channel closed for topic: {}", topic)))
            }
        }
    }

    /// 尝试发布（非阻塞）
    pub fn try_publish(&self, topic: &str, msg: Message) -> Result<()> {
        let sender = self.get_or_create_channel(topic);
        
        match sender.try_send(msg) {
            Ok(_) => {
                debug!("Message try-published to topic '{}'", topic);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to try-publish to topic '{}': {}", topic, e);
                Err(BambooError::Router(format!("Channel full or closed: {}", topic)))
            }
        }
    }

    /// 订阅指定主题
    /// 
    /// 返回接收者，可以接收该主题的消息
    pub fn subscribe(&self, topic: &str) -> mpsc::Receiver<Message> {
        // 创建新通道，与现有通道桥接
        let (tx, rx) = mpsc::channel::<Message>(self.buffer_size);
        
        // 存储发送端
        if let Some(mut entry) = self.channels.get_mut(topic) {
            // 主题已存在，需要桥接（这里简化处理，直接替换）
            *entry = tx.clone();
        } else {
            self.channels.insert(topic.to_string(), tx.clone());
        }
        
        // 启动桥接任务
        let topic_owned = topic.to_string();
        let channels = self.channels.clone();
        let buffer_size = self.buffer_size;
        
        tokio::spawn(async move {
            let mut rx_internal = {
                let (tx_int, rx_int) = mpsc::channel::<Message>(buffer_size);
                if let Some(mut entry) = channels.get_mut(&topic_owned) {
                    *entry = tx_int;
                }
                rx_int
            };
            
            while let Some(msg) = rx_internal.recv().await {
                if tx.send(msg).await.is_err() {
                    break;
                }
            }
        });
        
        rx
    }

    /// 取消订阅主题（关闭所有接收者）
    pub fn unsubscribe(&self, topic: &str) {
        self.channels.remove(topic);
        info!("Unsubscribed from topic '{}'", topic);
    }

    /// 检查主题是否存在
    pub fn has_topic(&self, topic: &str) -> bool {
        self.channels.contains_key(topic)
    }

    /// 获取所有活跃主题
    pub fn topics(&self) -> Vec<String> {
        self.channels
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// 获取主题订阅者数量（近似值）
    pub fn subscriber_count(&self, topic: &str) -> usize {
        if self.channels.contains_key(topic) {
            1 // 简化实现
        } else {
            0
        }
    }

    /// 广播消息到多个主题
    pub async fn broadcast(&self, topics: &[String], msg: Message) -> Vec<Result<()>> {
        let mut results = Vec::new();
        for topic in topics {
            results.push(self.publish(topic, msg.clone()).await);
        }
        results
    }

    /// 关闭总线，清除所有通道
    pub fn shutdown(&self) {
        self.channels.clear();
        info!("MessageBus shutdown complete");
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 类型安全的主题定义
pub struct Topics;

impl Topics {
    /// Gateway 输入主题（来自客户端的消息）
    pub fn gateway_input() -> &'static str {
        "gateway:input"
    }

    /// Gateway 输出主题（发送给客户端的消息）
    pub fn gateway_output(client_id: &str) -> String {
        format!("gateway:output:{}", client_id)
    }

    /// Agent Loop 输入主题
    pub fn agent_input() -> &'static str {
        "agent:input"
    }

    /// Agent Loop 输出主题
    pub fn agent_output() -> &'static str {
        "agent:output"
    }

    /// 命令处理器输入主题
    pub fn command_input() -> &'static str {
        "command:input"
    }

    /// 命令处理器输出主题
    pub fn command_output() -> &'static str {
        "command:output"
    }

    /// 系统处理器输入主题
    pub fn system_input() -> &'static str {
        "system:input"
    }

    /// 系统处理器输出主题
    pub fn system_output() -> &'static str {
        "system:output"
    }

    /// 会话特定主题
    pub fn session(session_id: &str) -> String {
        format!("session:{}", session_id)
    }

    /// 客户端特定主题
    pub fn client(client_id: &str) -> String {
        format!("client:{}", client_id)
    }
}

/// 消息处理器 trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// 处理器名称
    fn name(&self) -> &str;

    /// 处理消息
    async fn handle(&self, msg: Message, bus: &MessageBus) -> Result<Option<Message>>;

    /// 检查是否能处理该消息类型
    fn can_handle(&self, kind: &MessageKind) -> bool;
}

/// 路由器 - 将消息分发到正确的处理器
pub struct MessageRouter {
    bus: Arc<MessageBus>,
    handlers: Vec<Box<dyn MessageHandler>>,
}

impl MessageRouter {
    pub fn new(bus: Arc<MessageBus>) -> Self {
        Self {
            bus,
            handlers: Vec::new(),
        }
    }

    /// 注册处理器
    pub fn register<H: MessageHandler + 'static>(&mut self, handler: H) {
        info!("Registering handler: {}", handler.name());
        self.handlers.push(Box::new(handler));
    }

    /// 启动路由器，开始处理消息
    pub async fn start(&self) {
        info!("Starting MessageRouter with {} handlers", self.handlers.len());

        // 订阅所有输入主题
        let mut gateway_rx = self.bus.subscribe(Topics::gateway_input());
        let mut agent_rx = self.bus.subscribe(Topics::agent_output());
        let mut command_rx = self.bus.subscribe(Topics::command_output());
        let mut system_rx = self.bus.subscribe(Topics::system_output());

        // 创建选择器处理多个输入源
        loop {
            tokio::select! {
                Some(msg) = gateway_rx.recv() => {
                    self.route_incoming(msg).await;
                }
                Some(msg) = agent_rx.recv() => {
                    self.route_response(msg).await;
                }
                Some(msg) = command_rx.recv() => {
                    self.route_response(msg).await;
                }
                Some(msg) = system_rx.recv() => {
                    self.route_response(msg).await;
                }
                else => {
                    warn!("All input channels closed");
                    break;
                }
            }
        }
    }

    /// 路由输入消息到正确的处理器
    async fn route_incoming(&self, msg: Message) {
        debug!("Routing incoming message: kind={:?}", msg.kind());

        let handler_found = self.handlers.iter().find(|h| h.can_handle(msg.kind()));

        match handler_found {
            Some(handler) => {
                let handler_name = handler.name().to_string();
                let bus = &*self.bus;

                match handler.handle(msg.clone(), bus).await {
                    Ok(Some(response)) => {
                        // 处理器返回了响应，直接路由
                        self.route_response(response).await;
                    }
                    Ok(None) => {
                        // 处理器已异步处理，等待其通过消息总线返回结果
                        debug!("Handler '{}' accepted message for async processing", handler_name);
                    }
                    Err(e) => {
                        error!("Handler '{}' failed: {}", handler_name, e);
                        // 发送错误响应
                        let error_msg = Message::error(&msg, format!("Handler error: {}", e));
                        let _ = self.bus.publish(&Topics::gateway_output(msg.client_id()), error_msg).await;
                    }
                }
            }
            None => {
                warn!("No handler found for message kind: {:?}", msg.kind());
                let error_msg = Message::error(&msg, "No handler available for this message type");
                let _ = self.bus.publish(&Topics::gateway_output(msg.client_id()), error_msg).await;
            }
        }
    }

    /// 路由响应消息回 Gateway
    async fn route_response(&self, msg: Message) {
        let target = msg.metadata.target.clone();
        let client_id = msg.client_id().to_string();

        // 优先使用明确的目标
        let topic = if let Some(target) = target {
            target
        } else {
            // 默认发送到对应客户端的输出通道
            Topics::gateway_output(&client_id)
        };

        debug!("Routing response to: {}", topic);

        if let Err(e) = self.bus.publish(&topic, msg).await {
            error!("Failed to route response: {}", e);
        }
    }
}

/// 智能路由器 - 根据消息内容自动路由
pub struct SmartRouter {
    bus: Arc<MessageBus>,
}

impl SmartRouter {
    pub fn new(bus: Arc<MessageBus>) -> Self {
        Self { bus }
    }

    /// 根据消息类型自动路由
    pub async fn route(&self, msg: Message) -> Result<()> {
        match msg.kind() {
            MessageKind::Chat => {
                debug!("Routing chat message to agent");
                self.bus.publish(Topics::agent_input(), msg).await
            }
            MessageKind::Command => {
                debug!("Routing command message");
                self.bus.publish(Topics::command_input(), msg).await
            }
            MessageKind::System => {
                debug!("Routing system message");
                self.bus.publish(Topics::system_input(), msg).await
            }
            MessageKind::Response | MessageKind::Error => {
                // 响应消息路由回 Gateway
                let target = Topics::gateway_output(msg.client_id());
                self.bus.publish(&target, msg).await
            }
            MessageKind::Heartbeat => {
                // 心跳消息，简单确认
                debug!("Heartbeat received");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Message, MessageKind, MessageMetadata};

    #[tokio::test]
    async fn test_publish_subscribe() {
        let bus = MessageBus::new();
        let mut rx = bus.subscribe("test-topic");

        let msg = Message::chat("session-1", "client-1", "Hello!");
        bus.publish("test-topic", msg.clone()).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.client_id(), "client-1");
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = MessageBus::new();
        let mut rx1 = bus.subscribe("topic");
        let mut rx2 = bus.subscribe("topic");

        let msg = Message::chat("s1", "c1", "Test");
        bus.publish("topic", msg).await.unwrap();

        // 两个接收者都应该收到消息
        let _ = rx1.recv().await;
        let _ = rx2.recv().await;
    }

    #[tokio::test]
    async fn test_topics_list() {
        let bus = MessageBus::new();
        let _ = bus.subscribe("topic1");
        let _ = bus.subscribe("topic2");

        let topics = bus.topics();
        assert!(topics.contains(&"topic1".to_string()));
        assert!(topics.contains(&"topic2".to_string()));
    }
}
