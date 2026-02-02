//! Skill manager for loading, caching and hot-reloading skills

use crate::error::Result;
use crate::parser::SkillParser;
use crate::types::Skill;
use crate::watcher::{FileSystemWatcher, SkillWatcher, WatchEvent};
use async_trait::async_trait;
use bamboo_tool::{ToolDef, ToolRegistry};
use bamboo_tool::registry::InMemoryToolRegistry;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Trait for loading skills
#[async_trait]
pub trait SkillLoader: Send + Sync {
    /// Load a skill from a path
    async fn load(&self, path: &Path) -> Result<Skill>;

    /// Unload a skill by name
    async fn unload(&self, name: &str) -> Result<()>;
}

/// Manager for skills with hot-reload support
pub struct SkillManager {
    skills_dir: PathBuf,
    skills: Arc<DashMap<String, Skill>>,
    parser: SkillParser,
    watcher: Arc<RwLock<FileSystemWatcher>>,
    tool_registry: Arc<dyn ToolRegistry>,
    loaded: Arc<RwLock<bool>>,
}

impl SkillManager {
    /// Create a new skill manager
    pub fn new(skills_dir: impl AsRef<Path>) -> Self {
        let skills_dir = skills_dir.as_ref().to_path_buf();
        let watcher = FileSystemWatcher::new(&skills_dir);

        Self {
            skills_dir,
            skills: Arc::new(DashMap::new()),
            parser: SkillParser::new(),
            watcher: Arc::new(RwLock::new(watcher)),
            tool_registry: Arc::new(InMemoryToolRegistry::new()),
            loaded: Arc::new(RwLock::new(false)),
        }
    }

    /// Create a manager with a custom tool registry
    pub fn with_registry(
        skills_dir: impl AsRef<Path>,
        registry: Arc<dyn ToolRegistry>,
    ) -> Self {
        let skills_dir = skills_dir.as_ref().to_path_buf();
        let watcher = FileSystemWatcher::new(&skills_dir);

        Self {
            skills_dir,
            skills: Arc::new(DashMap::new()),
            parser: SkillParser::new(),
            watcher: Arc::new(RwLock::new(watcher)),
            tool_registry: registry,
            loaded: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize and load all skills
    pub async fn initialize(&self) -> Result<()> {
        // Ensure directory exists
        if !self.skills_dir.exists() {
            std::fs::create_dir_all(&self.skills_dir)?;
            info!("Created skills directory: {:?}", self.skills_dir);
        }

        // Load all existing skills
        self.load_all().await?;

        // Start file watcher
        let mut watcher = self.watcher.write().await;
        watcher.start().await?;

        *self.loaded.write().await = true;
        
        info!("SkillManager initialized with {} skills", self.skills.len());
        Ok(())
    }

    /// Load all skills from the directory
    async fn load_all(&self) -> Result<()> {
        let skills = self.parser.scan_directory(&self.skills_dir)?;
        
        for skill in skills {
            self.register_skill(skill).await?;
        }

        Ok(())
    }

    /// Register a skill and its tools
    async fn register_skill(&self, skill: Skill) -> Result<()> {
        let name = skill.name.clone();

        // Check for duplicate
        if self.skills.contains_key(&name) {
            warn!("Skill '{}' already exists, overwriting", name);
        }

        // Register tools
        for tool in &skill.tools {
            self.tool_registry.register(tool.clone()).await?;
        }

        // Store skill
        self.skills.insert(name.clone(), skill);
        info!("Registered skill: {}", name);

        Ok(())
    }

    /// Unregister a skill and its tools
    async fn unregister_skill(&self, name: &str) -> Result<()> {
        if let Some((_, skill)) = self.skills.remove(name) {
            // Unregister tools
            for tool in &skill.tools {
                let _ = self.tool_registry.unregister(&tool.name).await;
            }
            info!("Unregistered skill: {}", name);
        }

        Ok(())
    }

    /// Process file system events for hot-reload
    pub async fn process_events(&self) -> Result<()> {
        if !*self.loaded.read().await {
            return Ok(());
        }

        let mut watcher = self.watcher.write().await;
        
        while let Ok(Some(event)) = watcher.try_recv() {
            match event {
                WatchEvent::SkillModified(path) => {
                    self.handle_modified(path).await?;
                }
                WatchEvent::SkillRemoved(path) => {
                    self.handle_removed(path).await?;
                }
                WatchEvent::Error(e) => {
                    error!("Watch error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Handle a modified skill file
    async fn handle_modified(&self, path: PathBuf) -> Result<()> {
        // Find SKILL.md path
        let skill_md = if path.file_name() == Some("SKILL.md".as_ref()) {
            path
        } else {
            path.join("SKILL.md")
        };

        if skill_md.exists() {
            match self.parser.parse_file(&skill_md) {
                Ok(skill) => {
                    // Unregister old version if exists
                    self.unregister_skill(&skill.name).await?;
                    // Register new version
                    self.register_skill(skill).await?;
                    info!("Hot-reloaded skill from {:?}", skill_md);
                }
                Err(e) => {
                    warn!("Failed to reload skill at {:?}: {}", skill_md, e);
                }
            }
        }

        Ok(())
    }

    /// Handle a removed skill file
    async fn handle_removed(&self, path: PathBuf) -> Result<()> {
        // Try to find skill name from path
        if let Some(dir_name) = path.file_name() {
            let name = dir_name.to_string_lossy().to_string();
            if self.skills.contains_key(&name) {
                self.unregister_skill(&name).await?;
                info!("Removed skill: {}", name);
            }
        }

        Ok(())
    }

    /// Get a skill by name
    pub fn get_skill(&self, name: &str) -> Option<Skill> {
        self.skills.get(name).map(|s| s.clone())
    }

    /// List all loaded skills
    pub fn list_skills(&self) -> Vec<Skill> {
        self.skills.iter().map(|s| s.clone()).collect()
    }

    /// Get all tools from all skills
    pub fn get_all_tools(&self) -> Vec<ToolDef> {
        self.tool_registry.list()
    }

    /// Get a specific tool
    pub fn get_tool(&self, name: &str) -> Option<ToolDef> {
        self.tool_registry.get(name)
    }

    /// Check if a skill is loaded
    pub fn has_skill(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }

    /// Get the number of loaded skills
    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }

    /// Shutdown the manager
    pub async fn shutdown(&self) -> Result<()> {
        let mut watcher = self.watcher.write().await;
        watcher.stop().await?;
        
        self.skills.clear();
        self.tool_registry.clear();
        *self.loaded.write().await = false;

        info!("SkillManager shut down");
        Ok(())
    }
}

#[async_trait]
impl SkillLoader for SkillManager {
    async fn load(&self, path: &Path) -> Result<Skill> {
        let skill = self.parser.parse_file(path)?;
        self.register_skill(skill.clone()).await?;
        Ok(skill)
    }

    async fn unload(&self, name: &str) -> Result<()> {
        self.unregister_skill(name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_skill(dir: &Path, name: &str) -> PathBuf {
        let skill_dir = dir.join(name);
        std::fs::create_dir(&skill_dir).unwrap();

        let content = format!(r#"---
name: {}
version: 1.0.0
description: Test skill {}
tools:
  - name: test-tool
    description: A test tool
    command: tools/test.sh
---

# Test Skill
"#, name, name);

        let skill_md = skill_dir.join("SKILL.md");
        let mut file = std::fs::File::create(&skill_md).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        skill_md
    }

    #[tokio::test]
    async fn test_manager_initialize() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        create_test_skill(&skills_dir, "test-skill");

        let manager = SkillManager::new(&skills_dir);
        manager.initialize().await.unwrap();

        assert_eq!(manager.skill_count(), 1);
        assert!(manager.has_skill("test-skill"));

        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_manager_load_unload() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        let skill_md = create_test_skill(&skills_dir, "dynamic-skill");

        let manager = SkillManager::new(&skills_dir);
        
        // Load specific skill
        let skill = manager.load(&skill_md).await.unwrap();
        assert_eq!(skill.name, "dynamic-skill");
        assert!(manager.has_skill("dynamic-skill"));

        // Unload it
        manager.unload("dynamic-skill").await.unwrap();
        assert!(!manager.has_skill("dynamic-skill"));
    }

    #[tokio::test]
    async fn test_manager_tools() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        create_test_skill(&skills_dir, "skill1");
        create_test_skill(&skills_dir, "skill2");

        let manager = SkillManager::new(&skills_dir);
        manager.initialize().await.unwrap();

        // Both skills have a tool named "test-tool"
        // But tool registry uses tool name directly
        let tools = manager.get_all_tools();
        assert_eq!(tools.len(), 2); // Two tools with same name, last one wins

        manager.shutdown().await.unwrap();
    }
}
