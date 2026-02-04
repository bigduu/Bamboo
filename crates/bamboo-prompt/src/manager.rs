use uuid::Uuid;

use crate::{models::SystemPrompt, PromptError, PromptResult, PromptStorage};

#[derive(Debug, Clone)]
pub struct PromptManager {
    storage: PromptStorage,
}

impl PromptManager {
    pub fn new(storage: PromptStorage) -> Self {
        Self { storage }
    }

    pub fn storage(&self) -> &PromptStorage {
        &self.storage
    }

    pub async fn ensure_default_prompt(&self) -> PromptResult<()> {
        let mut prompts = self.storage.list_prompts().await?;
        if prompts.is_empty() {
            let mut default_prompt = SystemPrompt::default_prompt();
            default_prompt.id = Uuid::new_v4().to_string();
            self.storage.save_prompt(&default_prompt).await?;
            return Ok(());
        }

        if !prompts.iter().any(|prompt| prompt.is_default) {
            if let Some(first) = prompts.first_mut() {
                first.is_default = true;
                self.storage.save_prompt(first).await?;
            }
        }

        Ok(())
    }

    pub async fn list_prompts(&self) -> PromptResult<Vec<SystemPrompt>> {
        let mut prompts = self.storage.list_prompts().await?;
        prompts.sort_by(|a, b| {
            b.is_default
                .cmp(&a.is_default)
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(prompts)
    }

    pub async fn get_prompt(&self, id: &str) -> PromptResult<Option<SystemPrompt>> {
        self.storage.load_prompt(id).await
    }

    pub async fn create_prompt(&self, mut prompt: SystemPrompt) -> PromptResult<SystemPrompt> {
        if prompt.id.trim().is_empty() {
            prompt.id = Uuid::new_v4().to_string();
        }
        if prompt.name.trim().is_empty() {
            prompt.name = "Untitled".to_string();
        }
        if prompt.category.trim().is_empty() {
            prompt.category = "general".to_string();
        }

        self.storage.save_prompt(&prompt).await?;
        if prompt.is_default {
            return self.set_default(&prompt.id).await;
        }
        Ok(prompt)
    }

    pub async fn update_prompt(&self, prompt: SystemPrompt) -> PromptResult<SystemPrompt> {
        let existing = self.storage.load_prompt(&prompt.id).await?;
        if existing.is_none() {
            return Err(PromptError::NotFound(prompt.id));
        }

        self.storage.save_prompt(&prompt).await?;
        if prompt.is_default {
            return self.set_default(&prompt.id).await;
        }
        Ok(prompt)
    }

    pub async fn delete_prompt(&self, id: &str) -> PromptResult<()> {
        let existing = self.storage.load_prompt(id).await?;
        if existing.is_none() {
            return Err(PromptError::NotFound(id.to_string()));
        }
        self.storage.delete_prompt(id).await?;
        Ok(())
    }

    pub async fn set_default(&self, id: &str) -> PromptResult<SystemPrompt> {
        let mut prompts = self.storage.list_prompts().await?;
        let mut target_index = None;

        for (index, prompt) in prompts.iter_mut().enumerate() {
            if prompt.id == id {
                prompt.is_default = true;
                target_index = Some(index);
            } else if prompt.is_default {
                prompt.is_default = false;
            }
        }

        let target_index = match target_index {
            Some(index) => index,
            None => return Err(PromptError::NotFound(id.to_string())),
        };

        for prompt in &prompts {
            self.storage.save_prompt(prompt).await?;
        }

        Ok(prompts[target_index].clone())
    }
}
