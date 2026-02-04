use crate::{MaskingConfig, MaskingRule, RuleType};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;

static REGEX_CACHE: Lazy<Mutex<HashMap<String, Regex>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self { role: role.into(), content: content.into() }
    }
}

#[derive(Debug, Clone)]
pub struct MaskingEngine {
    config: MaskingConfig,
}

impl MaskingEngine {
    pub fn new(config: MaskingConfig) -> Self { Self { config } }
    pub fn default() -> Self { Self::new(MaskingConfig::default()) }
    pub fn from_config_file(path: impl AsRef<std::path::Path>) -> crate::MaskingResult<Self> {
        let config = MaskingConfig::load_from_file(path)?;
        Ok(Self::new(config))
    }
    pub fn load() -> crate::MaskingResult<Self> {
        let config = MaskingConfig::load()?;
        Ok(Self::new(config))
    }
    pub fn config(&self) -> &MaskingConfig { &self.config }
    pub fn set_config(&mut self, config: MaskingConfig) { self.config = config; }

    pub fn apply(&self, text: &str) -> String {
        if !self.config.enabled { return text.to_string(); }
        let mut result = text.to_string();
        for rule in self.config.enabled_rules() {
            result = self.apply_rule(&result, rule);
        }
        result
    }

    pub fn apply_to_messages(&self, messages: &[Message]) -> Vec<Message> {
        messages.iter().map(|msg| Message { role: msg.role.clone(), content: self.apply(&msg.content) }).collect()
    }

    fn apply_rule(&self, text: &str, rule: &MaskingRule) -> String {
        match &rule.rule_type {
            RuleType::Regex { pattern } => self.apply_regex(text, pattern, &rule.replacement),
            RuleType::Keyword { keyword, case_insensitive } => self.apply_keyword(text, keyword, &rule.replacement, *case_insensitive),
        }
    }

    fn apply_regex(&self, text: &str, pattern: &str, replacement: &str) -> String {
        let regex = self.get_or_compile_regex(pattern);
        regex.replace_all(text, replacement).to_string()
    }

    fn apply_keyword(&self, text: &str, keyword: &str, replacement: &str, case_insensitive: bool) -> String {
        if case_insensitive {
            let pattern = regex::escape(keyword);
            let regex = self.get_or_compile_regex(&format!("(?i){}", pattern));
            regex.replace_all(text, replacement).to_string()
        } else {
            text.replace(keyword, replacement)
        }
    }

    fn get_or_compile_regex(&self, pattern: &str) -> Regex {
        {
            let cache = REGEX_CACHE.lock().unwrap();
            if let Some(regex) = cache.get(pattern) { return regex.clone(); }
        }
        let regex = Regex::new(pattern).unwrap_or_else(|_| Regex::new("").unwrap());
        let mut cache = REGEX_CACHE.lock().unwrap();
        cache.insert(pattern.to_string(), regex.clone());
        regex
    }

    pub fn contains_sensitive(&self, text: &str) -> bool {
        if !self.config.enabled { return false; }
        for rule in self.config.enabled_rules() {
            match &rule.rule_type {
                RuleType::Regex { pattern } => {
                    let regex = self.get_or_compile_regex(pattern);
                    if regex.is_match(text) { return true; }
                }
                RuleType::Keyword { keyword, case_insensitive } => {
                    if *case_insensitive {
                        let pattern = regex::escape(keyword);
                        let regex = self.get_or_compile_regex(&format!("(?i){}", pattern));
                        if regex.is_match(text) { return true; }
                    } else if text.contains(keyword) { return true; }
                }
            }
        }
        false
    }

    pub fn find_sensitive(&self, text: &str) -> Vec<SensitiveMatch> {
        let mut matches = Vec::new();
        if !self.config.enabled { return matches; }
        for rule in self.config.enabled_rules() {
            match &rule.rule_type {
                RuleType::Regex { pattern } => {
                    let regex = self.get_or_compile_regex(pattern);
                    for cap in regex.captures_iter(text) {
                        if let Some(m) = cap.get(0) {
                            matches.push(SensitiveMatch { rule_name: rule.name.clone(), start: m.start(), end: m.end(), matched: m.as_str().to_string(), replacement: rule.replacement.clone() });
                        }
                    }
                }
                RuleType::Keyword { keyword, case_insensitive } => {
                    let keyword_matches: Vec<(usize, usize)> = if *case_insensitive {
                        let pattern = regex::escape(keyword);
                        let regex = self.get_or_compile_regex(&format!("(?i){}", pattern));
                        regex.find_iter(text).map(|m| (m.start(), m.end())).collect()
                    } else {
                        text.match_indices(keyword).map(|(i, _)| (i, i + keyword.len())).collect()
                    };
                    for (start, end) in keyword_matches {
                        matches.push(SensitiveMatch { rule_name: rule.name.clone(), start, end, matched: text[start..end].to_string(), replacement: rule.replacement.clone() });
                    }
                }
            }
        }
        matches.sort_by_key(|m| m.start);
        matches
    }
}

#[derive(Debug, Clone)]
pub struct SensitiveMatch {
    pub rule_name: String,
    pub start: usize,
    pub end: usize,
    pub matched: String,
    pub replacement: String,
}

pub fn clear_regex_cache() {
    let mut cache = REGEX_CACHE.lock().unwrap();
    cache.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MaskingConfig, MaskingRule};

    #[test] fn test_regex_masking() {
        let mut config = MaskingConfig::empty();
        config.add_rule(MaskingRule::new_regex("api_key", r"api_key[:=\s]+(\w+)").with_replacement("api_key:***"));
        let engine = MaskingEngine::new(config);
        let result = engine.apply("api_key: abc123xyz");
        assert_eq!(result, "api_key:***");
    }

    #[test] fn test_keyword_masking() {
        let mut config = MaskingConfig::empty();
        config.add_rule(MaskingRule::new_keyword("secret", "secret").with_replacement("[REDACTED]"));
        let engine = MaskingEngine::new(config);
        let result = engine.apply("my secret password");
        assert_eq!(result, "my [REDACTED] password");
    }

    #[test] fn test_case_insensitive_keyword() {
        let mut config = MaskingConfig::empty();
        config.add_rule(MaskingRule::new_keyword("password", "password").with_replacement("***"));
        let engine = MaskingEngine::new(config);
        let result = engine.apply("Password: 123, PASSWORD: 456");
        assert_eq!(result, "***: 123, ***: 456");
    }

    #[test] fn test_multiple_rules() {
        let mut config = MaskingConfig::empty();
        config.add_rule(MaskingRule::new_regex("api_key", r"api[_-]?key[:=\s]+\w+").with_replacement("API_KEY:***"));
        config.add_rule(MaskingRule::new_regex("password", r"password[:=\s]+\S+").with_replacement("PASSWORD:***"));
        let engine = MaskingEngine::new(config);
        let result = engine.apply("api_key: secret123, password: mypass");
        assert_eq!(result, "API_KEY:***, PASSWORD:***");
    }

    #[test] fn test_apply_to_messages() {
        let mut config = MaskingConfig::empty();
        config.add_rule(MaskingRule::new_regex("secret", r"secret_\d+").with_replacement("***"));
        let engine = MaskingEngine::new(config);
        let messages = vec![Message::new("user", "my secret_123 is here"), Message::new("assistant", "I see secret_456 in your message")];
        let masked = engine.apply_to_messages(&messages);
        assert_eq!(masked[0].content, "my *** is here");
        assert_eq!(masked[1].content, "I see *** in your message");
    }

    #[test] fn test_contains_sensitive() {
        let mut config = MaskingConfig::empty();
        config.add_rule(MaskingRule::new_regex("api_key", r"sk-\w+"));
        let engine = MaskingEngine::new(config);
        assert!(engine.contains_sensitive("my sk-abcdefghijklmnopqrstuvwx"));
        assert!(!engine.contains_sensitive("no secrets here"));
    }

    #[test] fn test_find_sensitive() {
        let mut config = MaskingConfig::empty();
        config.add_rule(MaskingRule::new_regex("api_key", r"\bsk-\w+\b").with_replacement("***"));
        let engine = MaskingEngine::new(config);
        let matches = engine.find_sensitive("keys: sk-abc and sk-xyz");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].matched, "sk-abc");
        assert_eq!(matches[1].matched, "sk-xyz");
    }

    #[test] fn test_disabled_config() {
        let mut config = MaskingConfig::empty();
        config.enabled = false;
        config.add_rule(MaskingRule::new_keyword("secret", "secret").with_replacement("***"));
        let engine = MaskingEngine::new(config);
        let result = engine.apply("my secret password");
        assert_eq!(result, "my secret password");
    }

    #[test] fn test_default_rules_masking() {
        let engine = MaskingEngine::default();
        let result = engine.apply("password: mySecret123");
        assert!(result.contains("***"));
        assert!(!result.contains("mySecret123"));
    }

    #[test] fn test_regex_cache() {
        clear_regex_cache();
        let config = MaskingConfig::default();
        let engine = MaskingEngine::new(config);
        engine.apply("test sk-abc123");
        engine.apply("test sk-xyz789");
        let cache = REGEX_CACHE.lock().unwrap();
        assert!(!cache.is_empty());
    }
}
