//! SKILL.md parser with YAML frontmatter extraction

use crate::error::{Result, SkillError};
use crate::manifest::SkillManifest;
use crate::types::Skill;
use bamboo_tool::ToolDef;
use std::path::Path;

/// Parser for SKILL.md files
#[derive(Debug, Clone)]
pub struct SkillParser;

impl SkillParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self
    }

    /// Parse a SKILL.md file from a path
    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<Skill> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        self.parse(&content, path)
    }

    /// Parse SKILL.md content
    /// 
    /// Format:
    /// ```markdown
    /// ---
    /// name: web-search
    /// version: 0.1.0
    /// description: Search the web
    /// tools:
    ///   - name: search
    ///     command: tools/search.sh
    /// ---
    /// 
    /// # Web Search
    /// 
    /// Description in markdown...
    /// ```
    pub fn parse(&self, content: &str, path: impl AsRef<Path>) -> Result<Skill> {
        let path = path.as_ref().to_path_buf();

        // Extract frontmatter between --- markers
        let (frontmatter, markdown) = self.extract_frontmatter(content)?;

        // Parse YAML frontmatter
        let manifest: SkillManifest = serde_yaml::from_str(&frontmatter)?;

        // Resolve tool command paths relative to skill directory
        let tools: Vec<ToolDef> = manifest
            .tools
            .iter()
            .map(|tool| {
                let mut tool = tool.clone();
                // If command is relative, resolve it relative to skill directory
                if !tool.command.starts_with('/') {
                    if let Some(parent) = path.parent() {
                        tool.command = parent.join(&tool.command).to_string_lossy().to_string();
                    }
                }
                tool
            })
            .collect();

        // Build skill
        let mut skill = Skill::new(
            manifest.name.clone(),
            manifest.description.clone(),
            manifest.clone(),
            path,
        )
        .with_tools(tools);

        // Use markdown content as system prompt if present
        let markdown = markdown.trim();
        if !markdown.is_empty() {
            skill = skill.with_system_prompt(markdown.to_string());
        }

        Ok(skill)
    }

    /// Extract YAML frontmatter and markdown content
    fn extract_frontmatter(&self, content: &str) -> Result<(String, String)> {
        let content = content.trim_start();
        
        // Check if starts with ---
        if !content.starts_with("---") {
            return Err(SkillError::ParseError(
                "SKILL.md must start with YAML frontmatter (---)".to_string()
            ));
        }

        // Find the closing ---
        let after_start = &content[3..];
        let Some(end_pos) = after_start.find("---") else {
            return Err(SkillError::ParseError(
                "YAML frontmatter not properly closed (missing ---)".to_string()
            ));
        };

        let frontmatter = after_start[..end_pos].trim();
        let markdown = &after_start[end_pos + 3..];

        Ok((frontmatter.to_string(), markdown.to_string()))
    }

    /// Scan a directory for all SKILL.md files
    pub fn scan_directory(&self, dir: impl AsRef<Path>) -> Result<Vec<Skill>> {
        let dir = dir.as_ref();
        let mut skills = Vec::new();

        if !dir.exists() {
            return Ok(skills);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Look for SKILL.md in subdirectory
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    match self.parse_file(&skill_md) {
                        Ok(skill) => skills.push(skill),
                        Err(e) => {
                            tracing::warn!("Failed to parse skill at {:?}: {}", skill_md, e);
                        }
                    }
                }
            }
        }

        Ok(skills)
    }
}

impl Default for SkillParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_parse_skill_md() {
        let content = r#"---
name: web-search
version: 0.1.0
description: Search the web using DuckDuckGo
author: Bamboo Team
tools:
  - name: search
    description: Search for a query
    command: tools/search.sh
    args:
      - name: query
        type: string
        required: true
        description: The search query
---

# Web Search Skill

This skill provides web search capabilities.

## Usage

The assistant can search the web when it needs current information.
"#;

        let parser = SkillParser::new();
        let skill = parser.parse(content, "/tmp/test/SKILL.md").unwrap();

        assert_eq!(skill.name, "web-search");
        assert_eq!(skill.manifest.version, "0.1.0");
        assert_eq!(skill.tools.len(), 1);
        assert!(skill.system_prompt.is_some());
        assert!(skill.system_prompt.unwrap().contains("Web Search Skill"));
    }

    #[test]
    fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        // Create a skill directory with SKILL.md
        let skill_dir = skills_dir.join("test-skill");
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md = r#"---
name: test-skill
version: 1.0.0
description: A test skill
---

# Test
"#;

        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        file.write_all(skill_md.as_bytes()).unwrap();

        let parser = SkillParser::new();
        let skills = parser.scan_directory(&skills_dir).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");
    }
}
