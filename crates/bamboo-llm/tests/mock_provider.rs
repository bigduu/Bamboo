use async_trait::async_trait;
use std::pin::Pin;
use futures::Stream;
use bamboo_llm::{LLMProvider, LLMError, ProviderMetadata, ProviderCapabilities};
use bamboo_core::chat::{ChatRequest, ChatResponse, ChatChunk};

/// Mock LLM Provider for testing
pub struct MockLLMProvider {
    responses: Vec<ChatChunk>,
    current_index: std::sync::atomic::AtomicUsize,
}

impl MockLLMProvider {
    pub fn new(responses: Vec<ChatChunk>) -> Self {
        Self {
            responses,
            current_index: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Create a simple text response mock
    pub fn with_text_response(text: &str) -> Self {
        let chunks: Vec<ChatChunk> = text
            .chars()
            .map(|c| ChatChunk::content(c.to_string()))
            .collect();
        Self::new(chunks)
    }

    /// Create a mock that returns tool calls
    pub fn with_tool_calls(tool_calls: Vec<bamboo_core::types::ToolCall>) -> Self {
        let mut chunks = vec![];
        for tc in &tool_calls {
            chunks.push(ChatChunk::ToolCallStart {
                call_id: tc.id.clone(),
                name: tc.name.clone(),
            });
        }
        Self::new(chunks)
    }

    /// Create a mock that simulates a conversation flow
    pub fn with_conversation_flow() -> Self {
        let responses = vec![
            ChatChunk::content("I "),
            ChatChunk::content("will "),
            ChatChunk::content("help "),
            ChatChunk::content("you."),
        ];
        Self::new(responses)
    }
}

#[async_trait]
impl LLMProvider for MockLLMProvider {
    fn provider_id(&self) -> &str {
        "mock"
    }

    fn metadata(&self) -> &ProviderMetadata {
        use std::sync::OnceLock;
        static METADATA: OnceLock<ProviderMetadata> = OnceLock::new();
        METADATA.get_or_init(|| ProviderMetadata {
            id: "mock".to_string(),
            name: "Mock Provider".to_string(),
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calling: true,
                vision: false,
                json_mode: false,
            },
        })
    }

    async fn chat(&self,
        _request: ChatRequest,
    ) -> Result<ChatResponse, LLMError> {
        let message = bamboo_core::types::Message::assistant("Mock response", None);
        Ok(ChatResponse::new(
            "mock-123",
            "mock-model",
            message,
        ))
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LLMError>> + Send>>, LLMError> {
        let responses = self.responses.clone();
        let stream = futures::stream::iter(responses.into_iter().map(Ok));
        Ok(Box::pin(stream))
    }

    async fn validate(&self) -> Result<(), LLMError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_mock_provider_text_response() {
        let mock = MockLLMProvider::with_text_response("Hello");
        let request = ChatRequest::new("mock-model");

        let mut stream = mock.chat_stream(request).await.unwrap();
        
        let mut result = String::new();
        while let Some(chunk) = stream.next().await {
            match chunk.unwrap() {
                ChatChunk::Content { text } => result.push_str(&text),
                _ => {}
            }
        }

        assert_eq!(result, "Hello");
    }

    #[tokio::test]
    async fn test_mock_provider_conversation_flow() {
        let mock = MockLLMProvider::with_conversation_flow();
        let request = ChatRequest::new("mock-model");

        let mut stream = mock.chat_stream(request).await.unwrap();
        
        let mut tokens = vec![];
        while let Some(chunk) = stream.next().await {
            match chunk.unwrap() {
                ChatChunk::Content { text } => tokens.push(text),
                _ => {}
            }
        }

        assert_eq!(tokens, vec!["I ", "will ", "help ", "you."]);
    }

    #[tokio::test]
    async fn test_mock_provider_empty_response() {
        let mock = MockLLMProvider::new(vec![]);
        let request = ChatRequest::new("mock-model");

        let mut stream = mock.chat_stream(request).await.unwrap();
        
        let count = stream.count().await;
        assert_eq!(count, 0);
    }
}
