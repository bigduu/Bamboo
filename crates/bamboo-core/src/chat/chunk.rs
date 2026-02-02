/// Chat stream chunk
#[derive(Debug, Clone)]
pub enum ChatChunk {
    /// Stream start
    Start { model: String },
    /// Text content delta
    Content { text: String },
    /// Tool call started
    ToolCallStart { call_id: String, name: String },
    /// Tool call arguments delta
    ToolCallDelta { call_id: String, arguments_delta: String },
    /// Tool call ended
    ToolCallEnd { call_id: String },
    /// Usage information
    Usage { input_tokens: u32, output_tokens: u32 },
    /// Stream finished
    Finish { reason: FinishReason },
    /// Error occurred
    Error { message: String },
}

impl ChatChunk {
    /// Create a content chunk
    pub fn content(text: impl Into<String>) -> Self {
        Self::Content { text: text.into() }
    }

    /// Create an error chunk
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error { message: message.into() }
    }

    /// Create a finish chunk
    pub fn finish(reason: FinishReason) -> Self {
        Self::Finish { reason }
    }

    /// Create a start chunk
    pub fn start(model: impl Into<String>) -> Self {
        Self::Start { model: model.into() }
    }

    /// Check if this is an error chunk
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// Check if this is a finish chunk
    pub fn is_finish(&self) -> bool {
        matches!(self, Self::Finish { .. })
    }
}

/// Reason for finishing the generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    /// Completed naturally
    Stop,
    /// Hit token limit
    Length,
    /// Tool calls were made
    ToolCalls,
    /// Content was filtered
    ContentFilter,
    /// User cancelled
    Cancelled,
    /// Error occurred
    Error,
}

impl FinishReason {
    /// Convert from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "stop" => Self::Stop,
            "length" => Self::Length,
            "tool_calls" => Self::ToolCalls,
            "content_filter" => Self::ContentFilter,
            _ => Self::Stop,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Length => "length",
            Self::ToolCalls => "tool_calls",
            Self::ContentFilter => "content_filter",
            Self::Cancelled => "cancelled",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finish_reason_from_str() {
        assert_eq!(FinishReason::from_str("stop"), FinishReason::Stop);
        assert_eq!(FinishReason::from_str("length"), FinishReason::Length);
        assert_eq!(FinishReason::from_str("tool_calls"), FinishReason::ToolCalls);
    }

    #[test]
    fn test_chat_chunk_helpers() {
        let chunk = ChatChunk::content("Hello");
        match chunk {
            ChatChunk::Content { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected content chunk"),
        }
    }
}
