use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    Regex { pattern: String },
    Keyword { keyword: String, #[serde(default = "default_case_insensitive")] case_insensitive: bool },
}

fn default_case_insensitive() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingRule {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(flatten)]
    pub rule_type: RuleType,
    #[serde(default = "default_replacement")]
    pub replacement: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_replacement() -> String { "***".to_string() }
fn default_enabled() -> bool { true }

impl MaskingRule {
    pub fn new_regex(name: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self { name: name.into(), description: String::new(), rule_type: RuleType::Regex { pattern: pattern.into() }, replacement: default_replacement(), enabled: true }
    }
    pub fn new_keyword(name: impl Into<String>, keyword: impl Into<String>) -> Self {
        Self { name: name.into(), description: String::new(), rule_type: RuleType::Keyword { keyword: keyword.into(), case_insensitive: true }, replacement: default_replacement(), enabled: true }
    }
    pub fn with_replacement(mut self, replacement: impl Into<String>) -> Self { self.replacement = replacement.into(); self }
    pub fn with_description(mut self, description: impl Into<String>) -> Self { self.description = description.into(); self }
    pub fn with_enabled(mut self, enabled: bool) -> Self { self.enabled = enabled; self }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub rules: Vec<MaskingRule>,
}

impl Default for MaskingConfig {
    fn default() -> Self { Self { enabled: true, rules: default_rules() } }
}

impl MaskingConfig {
    pub fn empty() -> Self { Self { enabled: true, rules: Vec::new() } }
    pub fn add_rule(&mut self, rule: MaskingRule) { self.rules.push(rule); }
    pub fn enabled_rules(&self) -> Vec<&MaskingRule> { self.rules.iter().filter(|r| r.enabled).collect() }
    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> crate::MaskingResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> crate::MaskingResult<()> {
        let content = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.as_ref().parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(path, content)?;
        Ok(())
    }
    pub fn default_config_path() -> Option<PathBuf> { dirs::home_dir().map(|home| home.join(".bamboo").join("masking.json")) }
    pub fn load() -> crate::MaskingResult<Self> {
        match Self::default_config_path() { Some(path) if path.exists() => Self::load_from_file(path), _ => Ok(Self::default()) }
    }
    pub fn save(&self) -> crate::MaskingResult<()> {
        match Self::default_config_path() { Some(path) => self.save_to_file(path), None => Err(crate::MaskingError::ConfigPathError) }
    }
}

pub fn default_rules() -> Vec<MaskingRule> {
    vec![
        MaskingRule::new_regex("api_key", r"api[_-]?key[:=\s]+\w+").with_replacement("api_key=***").with_description("API Key"),
        MaskingRule::new_regex("password", r"password[:=\s]+\S+").with_replacement("password=***").with_description("Password"),
        MaskingRule::new_regex("token", r"token[:=\s]+\w+").with_replacement("token=***").with_description("Token"),
        MaskingRule::new_regex("secret", r"secret[:=\s]+\w+").with_replacement("secret=***").with_description("Secret"),
        MaskingRule::new_regex("credit_card", r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b").with_replacement("****-****-****-****").with_description("Credit Card"),
        MaskingRule::new_regex("phone", r"\b1[3-9]\d{9}\b").with_replacement("1****").with_description("Phone"),
        MaskingRule::new_regex("email", r"\S+@\S+\.\S+").with_replacement("***@***.com").with_description("Email"),
        MaskingRule::new_keyword("authorization", "Authorization").with_replacement("Authorization: ***").with_description("Auth header"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_default_rules() { let rules = default_rules(); assert!(!rules.is_empty()); assert_eq!(rules[0].name, "api_key"); }
    #[test] fn test_builder() {
        let rule = MaskingRule::new_regex("test", r"\d+").with_replacement("[NUM]").with_description("Test").with_enabled(false);
        assert_eq!(rule.name, "test"); assert_eq!(rule.replacement, "[NUM]"); assert!(!rule.enabled);
    }
    #[test] fn test_save_load() {
        let path = std::env::temp_dir().join("test_masking.json");
        let mut config = MaskingConfig::empty();
        config.add_rule(MaskingRule::new_regex("test", r"secret"));
        config.save_to_file(&path).unwrap();
        let loaded = MaskingConfig::load_from_file(&path).unwrap();
        assert_eq!(loaded.rules.len(), 1);
        std::fs::remove_file(&path).unwrap();
    }
    #[test] fn test_enabled_rules() {
        let mut config = MaskingConfig::empty();
        config.add_rule(MaskingRule::new_regex("e", r"t").with_enabled(true));
        config.add_rule(MaskingRule::new_regex("d", r"t").with_enabled(false));
        assert_eq!(config.enabled_rules().len(), 1);
    }
}
