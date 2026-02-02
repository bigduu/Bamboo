//! bamboo-skill - Skill discovery, loading and hot-reload system for Bamboo
//!
//! This crate provides:
//! - Skill definition and manifest parsing
//! - SKILL.md parsing (YAML frontmatter + markdown)
//! - Directory scanning and discovery
//! - File watching for hot-reload (using notify crate)
//! - Skill management

pub mod error;
pub mod manifest;
pub mod parser;
pub mod watcher;
pub mod manager;
pub mod types;

pub use error::{SkillError, Result};
pub use manifest::SkillManifest;
pub use parser::SkillParser;
pub use watcher::{SkillWatcher, FileSystemWatcher};
pub use manager::{SkillManager, SkillLoader};
pub use types::Skill;

/// Re-export types from bamboo-tool for convenience
pub use bamboo_tool::{ArgDef, ToolDef, ToolType};
