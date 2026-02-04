use std::path::{Path, PathBuf};

use tokio::fs;

use crate::{models::SystemPrompt, PromptError, PromptResult};

#[derive(Debug, Clone)]
pub struct PromptStorage {
    root: PathBuf,
}

impl PromptStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn init(&self) -> PromptResult<()> {
        fs::create_dir_all(&self.root).await?;
        Ok(())
    }

    pub fn path_for_id(&self, id: &str) -> PathBuf {
        self.root.join(format!("{}.json", id))
    }

    pub async fn list_prompts(&self) -> PromptResult<Vec<SystemPrompt>> {
        self.init().await?;
        let mut prompts = Vec::new();
        let mut dir = fs::read_dir(&self.root).await?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let content = fs::read_to_string(&path).await?;
            let prompt: SystemPrompt = serde_json::from_str(&content)?;
            prompts.push(prompt);
        }

        Ok(prompts)
    }

    pub async fn load_prompt(&self, id: &str) -> PromptResult<Option<SystemPrompt>> {
        let path = self.path_for_id(id);
        match fs::read_to_string(&path).await {
            Ok(content) => Ok(Some(serde_json::from_str(&content)?)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(PromptError::Io(err)),
        }
    }

    pub async fn save_prompt(&self, prompt: &SystemPrompt) -> PromptResult<()> {
        self.init().await?;
        let path = self.path_for_id(&prompt.id);
        let content = serde_json::to_string_pretty(prompt)?;
        fs::write(path, content).await?;
        Ok(())
    }

    pub async fn delete_prompt(&self, id: &str) -> PromptResult<()> {
        let path = self.path_for_id(id);
        match fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(PromptError::Io(err)),
        }
    }
}
