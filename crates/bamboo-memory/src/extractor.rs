use crate::models::Memory;

#[derive(Debug, Default, Clone)]
pub struct MemoryExtractor;

impl MemoryExtractor {
    pub fn extract_from_text(&self, session_id: &str, text: &str) -> Vec<Memory> {
        let mut memories = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let lower = trimmed.to_lowercase();
            let mut content = None;

            if let Some(pos) = trimmed.find("记住") {
                let value = trimmed[pos + "记住".len()..].trim();
                if !value.is_empty() {
                    content = Some(value.trim_matches([':', '：', '-', ' ']).to_string());
                }
            } else if let Some(pos) = lower.find("remember") {
                let value = trimmed[pos + "remember".len()..].trim();
                if !value.is_empty() {
                    content = Some(value.trim_matches([':', '-', ' ']).to_string());
                }
            } else if let Some(pos) = lower.find("memory:") {
                let value = trimmed[pos + "memory:".len()..].trim();
                if !value.is_empty() {
                    content = Some(value.to_string());
                }
            } else if let Some(pos) = trimmed.find("记忆:") {
                let value = trimmed[pos + "记忆:".len()..].trim();
                if !value.is_empty() {
                    content = Some(value.to_string());
                }
            }

            if let Some(content) = content {
                if !content.is_empty() {
                    memories.push(Memory::new(session_id, content));
                }
            }
        }

        memories
    }
}
