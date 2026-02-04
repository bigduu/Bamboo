use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Masking configuration and rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MaskingConfig {
    pub enabled: bool,
    pub rules: Vec<MaskingRule>,
}

impl Default for MaskingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: vec![
                MaskingRule {
                    id: "api_key".to_string(),
                    name: "API Key".to_string(),
                    rule_type: RuleType::Regex,
                    pattern: "[a-zA-Z0-9]{32,}".to_string(),
                    replacement: "[API_KEY_MASKED]".to_string(),
                    enabled: true,
                },
                MaskingRule {
                    id: "password".to_string(),
                    name: "Password".to_string(),
                    rule_type: RuleType::Regex,
                    pattern: "(?i)(password|pwd|passwd)\\s*[:=]\\s*\\S+".to_string(),
                    replacement: "[PASSWORD_MASKED]".to_string(),
                    enabled: true,
                },
                MaskingRule {
                    id: "email".to_string(),
                    name: "Email".to_string(),
                    rule_type: RuleType::Regex,
                    pattern: "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}".to_string(),
                    replacement: "[EMAIL_MASKED]".to_string(),
                    enabled: false,
                },
                MaskingRule {
                    id: "internal_url".to_string(),
                    name: "Internal URL".to_string(),
                    rule_type: RuleType::Keyword,
                    pattern: "internal.company.com".to_string(),
                    replacement: "[INTERNAL_URL]".to_string(),
                    enabled: true,
                },
            ],
        }
    }
}

impl MaskingConfig {
    /// Apply masking rules to text.
    pub fn apply_to_text(&self, input: &str) -> String {
        if !self.enabled {
            return input.to_string();
        }

        let mut output = input.to_string();
        for rule in self.rules.iter().filter(|r| r.enabled) {
            output = rule.apply_to_text(&output);
        }
        output
    }

    /// Validate masking rules (e.g. regex syntax).
    pub fn validate(&self) -> MaskingResult<()> {
        for rule in &self.rules {
            rule.validate()?;
        }
        Ok(())
    }
}

/// Single masking rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaskingRule {
    pub id: String,
    pub name: String,
    pub rule_type: RuleType,
    pub pattern: String,
    pub replacement: String,
    pub enabled: bool,
}

impl MaskingRule {
    /// Apply this rule to text.
    pub fn apply_to_text(&self, input: &str) -> String {
        if !self.enabled {
            return input.to_string();
        }

        match self.rule_type {
            RuleType::Regex => {
                if let Ok(regex) = Regex::new(&self.pattern) {
                    regex.replace_all(input, self.replacement.as_str()).to_string()
                } else {
                    input.to_string()
                }
            }
            RuleType::Keyword => input.replace(&self.pattern, &self.replacement),
        }
    }

    /// Validate rule pattern (regex syntax).
    pub fn validate(&self) -> MaskingResult<()> {
        if matches!(self.rule_type, RuleType::Regex) {
            Regex::new(&self.pattern)
                .map(|_| ())
                .map_err(|e| MaskingError::InvalidRegex {
                    rule_id: self.id.clone(),
                    error: e.to_string(),
                })?;
        }
        Ok(())
    }
}

/// Rule matching type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    Regex,
    Keyword,
}

/// Error type for masking config management.
#[derive(Error, Debug)]
pub enum MaskingError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid regex for rule '{rule_id}': {error}")]
    InvalidRegex { rule_id: String, error: String },

    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

pub type MaskingResult<T> = std::result::Result<T, MaskingError>;

/// Default masking config path: ~/.bamboo/masking.json
pub fn default_masking_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".bamboo").join("masking.json"))
}

/// Manage masking config persistence.
#[derive(Clone)]
pub struct MaskingConfigManager {
    path: PathBuf,
    config: Arc<RwLock<MaskingConfig>>,
}

impl MaskingConfigManager {
    /// Load masking config from path (creates defaults if missing).
    pub async fn load(path: &Path) -> MaskingResult<Self> {
        let config = if path.exists() {
            let content = tokio::fs::read_to_string(path).await?;
            let parsed: MaskingConfig = serde_json::from_str(&content)?;
            parsed.validate()?;
            parsed
        } else {
            let default_config = MaskingConfig::default();
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let content = serde_json::to_string_pretty(&default_config)?;
            tokio::fs::write(path, &content).await?;
            default_config
        };

        Ok(Self {
            path: path.to_path_buf(),
            config: Arc::new(RwLock::new(config)),
        })
    }

    /// Load masking config from default path.
    pub async fn load_default() -> MaskingResult<Self> {
        let path = Self::default_path()?;
        Self::load(&path).await
    }

    /// Default masking config path (~/.bamboo/masking.json).
    pub fn default_path() -> MaskingResult<PathBuf> {
        default_masking_path().ok_or_else(|| {
            MaskingError::InvalidPath("Could not find home directory".to_string())
        })
    }

    /// Get shared config reference.
    pub fn get(&self) -> Arc<RwLock<MaskingConfig>> {
        Arc::clone(&self.config)
    }

    /// Save config to disk.
    pub async fn save(&self) -> MaskingResult<()> {
        let config = self.config.read().await;
        let content = serde_json::to_string_pretty(&*config)?;
        drop(config);

        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&self.path, content).await?;
        Ok(())
    }

    /// Update config and persist to disk.
    pub async fn update<F>(&self, f: F) -> MaskingResult<()>
    where
        F: FnOnce(&mut MaskingConfig),
    {
        let mut config = self.config.write().await;
        f(&mut config);
        config.validate()?;
        drop(config);
        self.save().await
    }

    /// Config file path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_keyword_rule() {
        let config = MaskingConfig {
            enabled: true,
            rules: vec![MaskingRule {
                id: "keyword".to_string(),
                name: "Keyword".to_string(),
                rule_type: RuleType::Keyword,
                pattern: "secret".to_string(),
                replacement: "[MASKED]".to_string(),
                enabled: true,
            }],
        };

        let masked = config.apply_to_text("this is secret data");
        assert_eq!(masked, "this is [MASKED] data");
    }

    #[test]
    fn test_validate_regex_rule() {
        let rule = MaskingRule {
            id: "regex".to_string(),
            name: "Regex".to_string(),
            rule_type: RuleType::Regex,
            pattern: "[a-z]+".to_string(),
            replacement: "[MASKED]".to_string(),
            enabled: true,
        };

        assert!(rule.validate().is_ok());
    }
}
