use crate::client::{AgentClient, ChatResponse};
use chrono::Local;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<Local>,
    pub tool_calls: Vec<ToolCallInfo>,
    pub is_streaming: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub status: ToolStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolStatus {
    Running,
    Success,
    Error,
}

impl std::fmt::Display for ToolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolStatus::Running => write!(f, "⏳"),
            ToolStatus::Success => write!(f, "✅"),
            ToolStatus::Error => write!(f, "❌"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Connected => write!(f, "● Connected"),
            ConnectionStatus::Disconnected => write!(f, "○ Disconnected"),
            ConnectionStatus::Connecting => write!(f, "◐ Connecting"),
            ConnectionStatus::Error => write!(f, "✗ Error"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
}

pub struct App {
    pub client: AgentClient,
    pub messages: Vec<Message>,
    pub input: String,
    pub input_mode: InputMode,
    pub session_id: Option<String>,
    pub status: ConnectionStatus,
    pub scroll_offset: usize,
    pub event_rx: Option<tokio::sync::mpsc::Receiver<crate::client::AgentEvent>>,
    pub is_streaming: bool,
}

impl App {
    pub async fn new(server_url: &str) -> anyhow::Result<Self> {
        let client = AgentClient::new(server_url);

        Ok(Self {
            client,
            messages: Vec::new(),
            input: String::new(),
            input_mode: InputMode::Normal,
            session_id: None,
            status: ConnectionStatus::Disconnected,
            scroll_offset: 0,
            event_rx: None,
            is_streaming: false,
        })
    }

    pub fn input_mode(&self) -> InputMode {
        self.input_mode
    }

    pub async fn check_connection(&mut self) {
        self.status = ConnectionStatus::Connecting;
        if self.client.health_check().await {
            self.status = ConnectionStatus::Connected;
            self.add_system_message("Connected to Copilot Agent Server".to_string());
        } else {
            self.status = ConnectionStatus::Disconnected;
            self.add_system_message("Failed to connect to server. Make sure it's running on localhost:8081".to_string());
        }
    }

    pub async fn send_message(&mut self) -> anyhow::Result<()> {
        if self.input.is_empty() || self.is_streaming {
            return Ok(());
        }

        let content = self.input.clone();
        self.input.clear();

        // Add user message
        self.add_user_message(&content);

        // Send to server
        match self.client.send_message(&content, self.session_id.as_deref()).await {
            Ok(response) => {
                self.session_id = Some(response.session_id.clone());
                self.is_streaming = true;
                self.status = ConnectionStatus::Connected;

                // Start receiving events
                let (tx, rx) = tokio::sync::mpsc::channel(100);
                self.event_rx = Some(rx);

                // Add initial assistant message
                self.add_assistant_message("", true);

                // Spawn SSE listener
                let client = self.client.clone();
                let session_id = response.session_id;
                tokio::spawn(async move {
                    if let Err(e) = client.stream_events(&session_id, tx).await {
                        log::error!("SSE error: {}", e);
                    }
                });
            }
            Err(e) => {
                self.status = ConnectionStatus::Error;
                self.add_system_message(format!("Failed to send message: {}", e));
            }
        }

        Ok(())
    }

    pub async fn process_events(&mut self) {
        // Collect all pending events first to avoid borrow checker issues
        let mut events = Vec::new();
        let mut should_close_rx = false;
        
        if let Some(ref mut rx) = self.event_rx {
            while let Ok(event) = rx.try_recv() {
                // Check if this event signals stream completion
                match &event {
                    crate::client::AgentEvent::Complete { .. } | 
                    crate::client::AgentEvent::Error { .. } => {
                        should_close_rx = true;
                    }
                    _ => {}
                }
                events.push(event);
            }
        }
        
        // Close receiver if needed
        if should_close_rx {
            self.event_rx = None;
            self.is_streaming = false;
        }
        
        // Process collected events
        for event in events {
            match event {
                crate::client::AgentEvent::Token { content } => {
                    self.append_to_last_message(&content);
                }
                crate::client::AgentEvent::ToolStart { tool_call_id, tool_name, .. } => {
                    self.add_tool_call(&tool_call_id, &tool_name);
                }
                crate::client::AgentEvent::ToolComplete { tool_call_id, result } => {
                    self.update_tool_result(&tool_call_id, result.success, &result.result);
                }
                crate::client::AgentEvent::ToolError { tool_call_id, error } => {
                    self.update_tool_error(&tool_call_id, &error);
                }
                crate::client::AgentEvent::Complete { usage } => {
                    if let Some(msg) = self.messages.last_mut() {
                        msg.is_streaming = false;
                    }
                    log::info!("Stream completed. Usage: {:?}", usage);
                }
                crate::client::AgentEvent::Error { message } => {
                    if let Some(msg) = self.messages.last_mut() {
                        msg.is_streaming = false;
                    }
                    self.add_system_message(format!("Error: {}", message));
                }
            }
        }
    }

    pub fn on_tick(&mut self) {
        // Animation or periodic updates can go here
    }

    pub fn push_input(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn pop_input(&mut self) {
        self.input.pop();
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    pub fn scroll_page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
    }

    pub fn scroll_page_down(&mut self) {
        self.scroll_offset += 10;
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
    }

    pub async fn new_session(&mut self) -> anyhow::Result<()> {
        self.session_id = None;
        self.messages.clear();
        self.scroll_offset = 0;
        self.add_system_message("New session started".to_string());
        Ok(())
    }

    pub async fn stop_generation(&mut self) {
        if let Some(ref session_id) = self.session_id {
            if let Err(e) = self.client.stop_generation(session_id).await {
                log::error!("Failed to stop generation: {}", e);
            }
        }
        self.is_streaming = false;
        self.event_rx = None;
        if let Some(msg) = self.messages.last_mut() {
            msg.is_streaming = false;
        }
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.messages.push(Message {
            id: format!("user-{}", self.messages.len()),
            role: MessageRole::User,
            content: content.to_string(),
            timestamp: Local::now(),
            tool_calls: Vec::new(),
            is_streaming: false,
        });
        self.scroll_to_bottom();
    }

    pub fn add_assistant_message(&mut self, content: &str, is_streaming: bool) {
        self.messages.push(Message {
            id: format!("assistant-{}", self.messages.len()),
            role: MessageRole::Assistant,
            content: content.to_string(),
            timestamp: Local::now(),
            tool_calls: Vec::new(),
            is_streaming,
        });
        self.scroll_to_bottom();
    }

    pub fn add_system_message(&mut self, content: String) {
        self.messages.push(Message {
            id: format!("system-{}", self.messages.len()),
            role: MessageRole::System,
            content,
            timestamp: Local::now(),
            tool_calls: Vec::new(),
            is_streaming: false,
        });
        self.scroll_to_bottom();
    }

    pub fn append_to_last_message(&mut self, content: &str) {
        if let Some(msg) = self.messages.last_mut() {
            if msg.role == MessageRole::Assistant {
                msg.content.push_str(content);
            }
        }
    }

    pub fn add_tool_call(&mut self, id: &str, name: &str) {
        if let Some(msg) = self.messages.last_mut() {
            msg.tool_calls.push(ToolCallInfo {
                id: id.to_string(),
                name: name.to_string(),
                status: ToolStatus::Running,
                result: None,
            });
        }
    }

    pub fn update_tool_result(&mut self, id: &str, success: bool, result: &str) {
        if let Some(msg) = self.messages.last_mut() {
            if let Some(tool) = msg.tool_calls.iter_mut().find(|t| t.id == id) {
                tool.status = if success { ToolStatus::Success } else { ToolStatus::Error };
                tool.result = Some(result.to_string());
            }
        }
    }

    pub fn update_tool_error(&mut self, id: &str, error: &str) {
        if let Some(msg) = self.messages.last_mut() {
            if let Some(tool) = msg.tool_calls.iter_mut().find(|t| t.id == id) {
                tool.status = ToolStatus::Error;
                tool.result = Some(format!("Error: {}", error));
            }
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.messages.len().saturating_sub(1);
    }
}
