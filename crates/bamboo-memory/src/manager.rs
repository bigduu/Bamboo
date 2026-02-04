use std::path::{Path, PathBuf};

use tokio::fs;

use crate::{MemoryError, MemoryResult, models::{Memory, SessionMemory}};

#[derive(Debug, Clone)]
pub struct MemoryManager {
    root: PathBuf,
}

impl MemoryManager {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn init(&self) -> MemoryResult<()> {
        fs::create_dir_all(&self.root).await?;
        Ok(())
    }

    pub fn path_for_session(&self, session_id: &str) -> PathBuf {
        self.root.join(format!("{}.json", session_id))
    }

    pub async fn list_memories(&self) -> MemoryResult<Vec<Memory>> {
        self.init().await?;
        let mut memories = Vec::new();
        let mut dir = fs::read_dir(&self.root).await?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let content = fs::read_to_string(&path).await?;
            let session: SessionMemory = serde_json::from_str(&content)?;
            memories.extend(session.memories);
        }

        Ok(memories)
    }

    pub async fn get_session_memory(&self, session_id: &str) -> MemoryResult<SessionMemory> {
        let path = self.path_for_session(session_id);
        match fs::read_to_string(&path).await {
            Ok(content) => Ok(serde_json::from_str(&content)?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(SessionMemory::new(session_id)),
            Err(err) => Err(MemoryError::Io(err)),
        }
    }

    pub async fn save_session_memory(&self, session_memory: &SessionMemory) -> MemoryResult<()> {
        self.init().await?;
        let path = self.path_for_session(&session_memory.session_id);
        let content = serde_json::to_string_pretty(session_memory)?;
        fs::write(path, content).await?;
        Ok(())
    }

    pub async fn append_memory(&self, session_id: &str, mut memory: Memory) -> MemoryResult<SessionMemory> {
        let mut session = self.get_session_memory(session_id).await?;
        memory.session_id = session_id.to_string();
        session.memories.push(memory);
        session.updated_at = chrono::Utc::now().to_rfc3339();
        self.save_session_memory(&session).await?;
        Ok(session)
    }
}
