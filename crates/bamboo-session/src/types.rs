//! # Bamboo Session Types
//! 
//! 定义 Session 相关的核心类型，包括消息、会话元数据、事件等。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 消息角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// 工具调用定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Token 使用统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl Message {
    /// 创建用户消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: Role::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            created_at: Utc::now(),
            metadata: None,
        }
    }

    /// 创建助手消息
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: Role::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            created_at: Utc::now(),
            metadata: None,
        }
    }

    /// 创建系统消息
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: Role::System,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            created_at: Utc::now(),
            metadata: None,
        }
    }

    /// 创建工具消息
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: Role::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            created_at: Utc::now(),
            metadata: None,
        }
    }

    /// 创建带工具调用的助手消息
    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: Role::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            created_at: Utc::now(),
            metadata: None,
        }
    }
}

/// 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// 活跃状态（有连接）
    Active,
    /// 空闲状态（无连接但可恢复）
    Idle,
    /// 已断开（等待重连）
    Disconnected,
    /// 已结束
    Closed,
    /// 已过期
    Expired,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Active => write!(f, "active"),
            SessionState::Idle => write!(f, "idle"),
            SessionState::Disconnected => write!(f, "disconnected"),
            SessionState::Closed => write!(f, "closed"),
            SessionState::Expired => write!(f, "expired"),
        }
    }
}

/// 会话元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub user_id: Option<String>,
    pub title: Option<String>,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    /// 过期时间（None 表示永不过期）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// 消息数量
    pub message_count: u32,
    /// 额外元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

impl SessionMetadata {
    /// 创建新的会话元数据
    pub fn new(id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            user_id: None,
            title: None,
            state: SessionState::Active,
            created_at: now,
            updated_at: now,
            last_activity_at: now,
            expires_at: None,
            message_count: 0,
            extra: None,
        }
    }

    /// 设置用户ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// 设置标题
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// 设置过期时间（从创建时间开始计算）
    pub fn with_ttl_seconds(mut self, ttl: u64) -> Self {
        self.expires_at = Some(self.created_at + chrono::Duration::seconds(ttl as i64));
        self
    }

    /// 检查是否已过期
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires) => Utc::now() > expires,
            None => false,
        }
    }

    /// 更新活动时间
    pub fn touch(&mut self) {
        let now = Utc::now();
        self.last_activity_at = now;
        self.updated_at = now;
    }

    /// 更新消息数量
    pub fn update_message_count(&mut self, count: u32) {
        self.message_count = count;
        self.updated_at = Utc::now();
    }
}

/// 完整会话（包含元数据和消息历史）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub metadata: SessionMetadata,
    pub messages: Vec<Message>,
}

impl Session {
    /// 创建新会话
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            metadata: SessionMetadata::new(&id),
            messages: Vec::new(),
        }
    }

    /// 设置用户ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.metadata.user_id = Some(user_id.into());
        self
    }

    /// 添加消息
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.metadata.update_message_count(self.messages.len() as u32);
    }

    /// 获取消息历史
    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }

    /// 获取最后一条消息
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// 更新状态
    pub fn set_state(&mut self, state: SessionState) {
        self.metadata.state = state;
        self.metadata.updated_at = Utc::now();
    }
}

/// Agent 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// 流式 Token
    Token {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    /// 工具开始执行
    ToolStart {
        tool_call_id: String,
        tool_name: String,
        arguments: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
    /// 工具执行完成
    ToolComplete {
        tool_call_id: String,
        result: ToolResult,
        timestamp: DateTime<Utc>,
    },
    /// 工具执行错误
    ToolError {
        tool_call_id: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// 生成完成
    Complete {
        usage: TokenUsage,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    /// 错误
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    /// 会话状态变更
    StateChange {
        from: SessionState,
        to: SessionState,
        timestamp: DateTime<Utc>,
    },
}

impl AgentEvent {
    /// 创建 Token 事件
    pub fn token(content: impl Into<String>) -> Self {
        Self::Token {
            content: content.into(),
            timestamp: Some(Utc::now()),
        }
    }

    /// 创建工具开始事件
    pub fn tool_start(
        tool_call_id: impl Into<String>,
        tool_name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self::ToolStart {
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
            arguments,
            timestamp: Utc::now(),
        }
    }

    /// 创建工具完成事件
    pub fn tool_complete(tool_call_id: impl Into<String>, result: ToolResult) -> Self {
        Self::ToolComplete {
            tool_call_id: tool_call_id.into(),
            result,
            timestamp: Utc::now(),
        }
    }

    /// 创建工具错误事件
    pub fn tool_error(tool_call_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::ToolError {
            tool_call_id: tool_call_id.into(),
            error: error.into(),
            timestamp: Utc::now(),
        }
    }

    /// 创建完成事件
    pub fn complete(usage: TokenUsage) -> Self {
        Self::Complete {
            usage,
            timestamp: Some(Utc::now()),
        }
    }

    /// 创建错误事件
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
            code: None,
            timestamp: Some(Utc::now()),
        }
    }

    /// 创建状态变更事件
    pub fn state_change(from: SessionState, to: SessionState) -> Self {
        Self::StateChange {
            from,
            to,
            timestamp: Utc::now(),
        }
    }
}

/// 会话查询过滤器
#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    /// 按用户ID过滤
    pub user_id: Option<String>,
    /// 按状态过滤
    pub state: Option<SessionState>,
    /// 按创建时间范围过滤（开始）
    pub created_after: Option<DateTime<Utc>>,
    /// 按创建时间范围过滤（结束）
    pub created_before: Option<DateTime<Utc>>,
    /// 按最后活动时间范围过滤（开始）
    pub active_after: Option<DateTime<Utc>>,
    /// 搜索标题关键词
    pub title_contains: Option<String>,
    /// 最大返回数量
    pub limit: Option<usize>,
    /// 偏移量（分页）
    pub offset: Option<usize>,
    /// 排序字段
    pub sort_by: Option<SortField>,
    /// 排序方向
    pub sort_order: Option<SortOrder>,
}

/// 排序字段
#[derive(Debug, Clone, Copy)]
pub enum SortField {
    CreatedAt,
    UpdatedAt,
    LastActivityAt,
    MessageCount,
}

/// 排序方向
#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Desc
    }
}

impl SessionFilter {
    /// 创建空过滤器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置用户ID过滤
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// 设置状态过滤
    pub fn with_state(mut self, state: SessionState) -> Self {
        self.state = Some(state);
        self
    }

    /// 设置创建时间范围
    pub fn created_between(
        mut self,
        after: DateTime<Utc>,
        before: DateTime<Utc>,
    ) -> Self {
        self.created_after = Some(after);
        self.created_before = Some(before);
        self
    }

    /// 设置最后活动时间过滤
    pub fn active_after(mut self, time: DateTime<Utc>) -> Self {
        self.active_after = Some(time);
        self
    }

    /// 设置标题搜索
    pub fn title_contains(mut self, keyword: impl Into<String>) -> Self {
        self.title_contains = Some(keyword.into());
        self
    }

    /// 设置分页
    pub fn with_pagination(mut self, limit: usize, offset: usize) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }

    /// 设置排序
    pub fn with_sort(mut self, field: SortField, order: SortOrder) -> Self {
        self.sort_by = Some(field);
        self.sort_order = Some(order);
        self
    }
}

/// 会话列表结果
#[derive(Debug, Clone)]
pub struct SessionListResult {
    pub sessions: Vec<SessionMetadata>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

impl SessionListResult {
    /// 创建结果
    pub fn new(sessions: Vec<SessionMetadata>, total: usize, offset: usize, limit: usize) -> Self {
        Self {
            sessions,
            total,
            offset,
            limit,
        }
    }

    /// 是否有更多结果
    pub fn has_more(&self) -> bool {
        self.offset + self.sessions.len() < self.total
    }

    /// 获取下一页偏移量
    pub fn next_offset(&self) -> Option<usize> {
        if self.has_more() {
            Some(self.offset + self.limit)
        } else {
            None
        }
    }
}
