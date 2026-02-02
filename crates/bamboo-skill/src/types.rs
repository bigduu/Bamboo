//! Core types for skill system

use crate::manifest::SkillManifest;
use bamboo_tool::ToolDef;
use std::path::PathBuf;

/// A loaded skill with all its metadata and tools
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub manifest: SkillManifest,
    pub tools: Vec<ToolDef>,
    pub system_prompt: Option<String>,
    pub path: PathBuf,
}

impl Skill {
    /// Create a new skill
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        manifest: SkillManifest,
        path: PathBuf,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            manifest,
            tools: Vec::new(),
            system_prompt: None,
            path,
        }
    }

    /// Add a tool to this skill
    pub fn with_tool(mut self, tool: ToolDef) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add multiple tools to this skill
    pub fn with_tools(mut self, tools: Vec<ToolDef>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Get full tool name (skill.tool format)
    pub fn full_tool_name(&self, tool_name: &str) -> String {
        format!("{}.{}", self.name, tool_name)
    }

    /// Find a tool by name
    pub fn find_tool(&self, name: &str) -> Option<&ToolDef> {
        self.tools.iter().find(|t| t.name == name)
    }
}
