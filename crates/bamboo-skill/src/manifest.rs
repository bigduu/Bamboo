//! Skill manifest definition

use serde::{Deserialize, Serialize};
use bamboo_tool::ToolDef;

/// Manifest parsed from SKILL.md frontmatter
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default)]
    pub tools: Vec<ToolDef>,
}

impl SkillManifest {
    /// Create a new manifest
    pub fn new(name: impl Into<String>, version: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: description.into(),
            author: None,
            tools: Vec::new(),
        }
    }

    /// Set the author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Add a tool to the manifest
    pub fn with_tool(mut self, tool: ToolDef) -> Self {
        self.tools.push(tool);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_serialization() {
        let manifest = SkillManifest::new(
            "web-search",
            "0.1.0",
            "Search the web using DuckDuckGo"
        ).with_author("Bamboo Team");

        let yaml = serde_yaml::to_string(&manifest).unwrap();
        assert!(yaml.contains("name: web-search"));
        assert!(yaml.contains("version: 0.1.0"));
        assert!(yaml.contains("author: Bamboo Team"));

        let parsed: SkillManifest = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.name, "web-search");
        assert_eq!(parsed.author, Some("Bamboo Team".to_string()));
    }
}
