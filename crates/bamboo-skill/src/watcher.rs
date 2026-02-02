//! File system watcher for skill hot-reload using notify crate

use crate::error::{Result, SkillError};
use async_trait::async_trait;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Events emitted by the file watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A skill was created or modified
    SkillModified(PathBuf),
    /// A skill was removed
    SkillRemoved(PathBuf),
    /// An error occurred
    Error(String),
}

/// Trait for watching skill files
#[async_trait]
pub trait SkillWatcher: Send + Sync {
    /// Start watching the skills directory
    async fn start(&mut self) -> Result<()>;

    /// Stop watching
    async fn stop(&mut self) -> Result<()>;

    /// Receive the next watch event (non-blocking)
    fn try_recv(&mut self) -> Result<Option<WatchEvent>>;

    /// Check if the watcher is running
    fn is_running(&self) -> bool;
}

/// File system watcher implementation using notify crate
pub struct FileSystemWatcher {
    skills_dir: PathBuf,
    watcher: Option<RecommendedWatcher>,
    event_rx: Receiver<WatchEvent>,
    event_tx: Sender<WatchEvent>,
    running: bool,
}

impl FileSystemWatcher {
    /// Create a new watcher for the given directory
    pub fn new(skills_dir: impl AsRef<Path>) -> Self {
        let (tx, rx) = channel(100);
        Self {
            skills_dir: skills_dir.as_ref().to_path_buf(),
            watcher: None,
            event_rx: rx,
            event_tx: tx,
            running: false,
        }
    }

    /// Get the skills directory
    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }

    /// Create the watcher instance
    fn create_watcher(&self) -> Result<RecommendedWatcher> {
        let tx = self.event_tx.clone();

        let watcher = RecommendedWatcher::new(
            move |res: std::result::Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        Self::handle_event(event, &tx);
                    }
                    Err(e) => {
                        let _ = tx.try_send(WatchEvent::Error(e.to_string()));
                    }
                }
            },
            Config::default(),
        )?;

        Ok(watcher)
    }

    /// Process a notify event and convert to WatchEvent
    fn handle_event(event: Event, tx: &Sender<WatchEvent>) {
        use notify::event::{CreateKind, ModifyKind, RemoveKind};

        for path in event.paths {
            // Only process SKILL.md files and directories
            if let Some(file_name) = path.file_name() {
                let is_skill_md = file_name == "SKILL.md";
                let is_skill_dir = path.is_dir() && path.extension().is_none();

                if !is_skill_md && !is_skill_dir {
                    continue;
                }
            }

            let watch_event = match event.kind {
                EventKind::Create(CreateKind::File) |
                EventKind::Create(CreateKind::Folder) |
                EventKind::Modify(ModifyKind::Data(_)) |
                EventKind::Modify(ModifyKind::Name(_)) => {
                    WatchEvent::SkillModified(path)
                }
                EventKind::Remove(RemoveKind::File) |
                EventKind::Remove(RemoveKind::Folder) => {
                    WatchEvent::SkillRemoved(path)
                }
                _ => continue,
            };

            let _ = tx.try_send(watch_event);
        }
    }
}

#[async_trait]
impl SkillWatcher for FileSystemWatcher {
    async fn start(&mut self) -> Result<()> {
        if self.running {
            return Ok(());
        }

        // Ensure directory exists
        if !self.skills_dir.exists() {
            std::fs::create_dir_all(&self.skills_dir)?;
        }

        let mut watcher = self.create_watcher()?;
        watcher.watch(&self.skills_dir, RecursiveMode::Recursive)?;

        self.watcher = Some(watcher);
        self.running = true;

        tracing::info!("Started watching skills directory: {:?}", self.skills_dir);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if let Some(watcher) = self.watcher.take() {
            drop(watcher);
        }
        self.running = false;
        tracing::info!("Stopped watching skills directory");
        Ok(())
    }

    fn try_recv(&mut self) -> Result<Option<WatchEvent>> {
        match self.event_rx.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                Err(SkillError::Watch("Channel disconnected".to_string()))
            }
        }
    }

    fn is_running(&self) -> bool {
        self.running
    }
}

impl Drop for FileSystemWatcher {
    fn drop(&mut self) {
        if self.running {
            // Best effort stop
            if let Some(watcher) = self.watcher.take() {
                drop(watcher);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_watcher_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = FileSystemWatcher::new(temp_dir.path());

        assert!(!watcher.is_running());
        
        watcher.start().await.unwrap();
        assert!(watcher.is_running());

        watcher.stop().await.unwrap();
        assert!(!watcher.is_running());
    }

    #[tokio::test]
    async fn test_watcher_detects_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        let mut watcher = FileSystemWatcher::new(&skills_dir);
        watcher.start().await.unwrap();

        // Create a skill directory with SKILL.md
        let skill_dir = skills_dir.join("test-skill");
        std::fs::create_dir(&skill_dir).unwrap();

        // Small delay for watcher to be ready
        tokio::time::sleep(Duration::from_millis(100)).await;

        let skill_md = r#"---
name: test-skill
version: 1.0.0
description: A test skill
---
"#;

        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        file.write_all(skill_md.as_bytes()).unwrap();

        // Wait for event
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Should have received an event
        let event = watcher.try_recv().unwrap();
        assert!(event.is_some());

        watcher.stop().await.unwrap();
    }
}
