//! Tool registry for managing available tools

use crate::error::{Result, ToolError};
use crate::types::ToolDef;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

/// Trait for managing tool registry
#[async_trait]
pub trait ToolRegistry: Send + Sync {
    /// Register a new tool
    async fn register(&self, tool: ToolDef) -> Result<()>;

    /// Unregister a tool by name
    async fn unregister(&self, name: &str) -> Result<()>;

    /// List all registered tools
    fn list(&self) -> Vec<ToolDef>;

    /// Get a tool by name
    fn get(&self, name: &str) -> Option<ToolDef>;

    /// Check if a tool is registered
    fn contains(&self, name: &str) -> bool;

    /// Clear all registered tools
    fn clear(&self);
}

/// In-memory tool registry using DashMap for concurrent access
#[derive(Debug, Clone, Default)]
pub struct InMemoryToolRegistry {
    tools: Arc<DashMap<String, ToolDef>>,
}

impl InMemoryToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(DashMap::new()),
        }
    }

    /// Create a registry with initial tools
    pub fn with_tools(tools: Vec<ToolDef>) -> Self {
        let registry = Self::new();
        for tool in tools {
            registry.tools.insert(tool.name.clone(), tool);
        }
        registry
    }

    /// Get the number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

#[async_trait]
impl ToolRegistry for InMemoryToolRegistry {
    async fn register(&self, tool: ToolDef) -> Result<()> {
        self.tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    async fn unregister(&self, name: &str) -> Result<()> {
        if self.tools.remove(name).is_none() {
            return Err(ToolError::NotFound(name.to_string()));
        }
        Ok(())
    }

    fn list(&self) -> Vec<ToolDef> {
        self.tools
            .iter()
            .map(|entry: dashmap::mapref::multiple::RefMulti<'_, String, ToolDef>| entry.value().clone())
            .collect()
    }

    fn get(&self, name: &str) -> Option<ToolDef> {
        self.tools.get(name).map(|entry: dashmap::mapref::one::Ref<'_, String, ToolDef>| entry.clone())
    }

    fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    fn clear(&self) {
        self.tools.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tool(name: &str) -> ToolDef {
        ToolDef {
            name: name.to_string(),
            description: Some("Test tool".to_string()),
            command: "echo".to_string(),
            args: vec![],
        }
    }

    #[tokio::test]
    async fn test_register_and_get() {
        let registry = InMemoryToolRegistry::new();
        let tool = create_test_tool("test-tool");

        registry.register(tool.clone()).await.unwrap();

        assert!(registry.contains("test-tool"));
        let retrieved = registry.get("test-tool").unwrap();
        assert_eq!(retrieved.name, "test-tool");
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = InMemoryToolRegistry::new();
        let tool = create_test_tool("test-tool");

        registry.register(tool.clone()).await.unwrap();
        assert!(registry.contains("test-tool"));

        registry.unregister("test-tool").await.unwrap();
        assert!(!registry.contains("test-tool"));

        // Unregister non-existent should fail
        let result = registry.unregister("non-existent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list() {
        let registry = InMemoryToolRegistry::new();
        
        registry.register(create_test_tool("tool1")).await.unwrap();
        registry.register(create_test_tool("tool2")).await.unwrap();

        let list = registry.list();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_clear() {
        let registry = InMemoryToolRegistry::new();
        
        registry.register(create_test_tool("tool1")).await.unwrap();
        registry.register(create_test_tool("tool2")).await.unwrap();

        registry.clear();
        assert!(registry.is_empty());
    }
}
