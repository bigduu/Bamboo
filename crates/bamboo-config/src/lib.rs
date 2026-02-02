pub mod config;
pub mod manager;

pub use config::{
    AgentConfig, AuthSettings, Config, ConfigError, ConfigResult, LlmConfig, LogLevel, LoggingConfig,
    ProviderSettings, ServerConfig, SkillsConfig, StorageConfig, StorageType,
};
pub use manager::ConfigManager;

use std::path::PathBuf;

/// 获取 Bamboo 配置目录路径
pub fn bamboo_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".bamboo"))
}

/// 获取默认配置文件路径
pub fn default_config_path() -> Option<PathBuf> {
    bamboo_dir().map(|dir| dir.join("config.json"))
}

/// 获取默认 skills 目录
pub fn default_skills_dir() -> Option<PathBuf> {
    bamboo_dir().map(|dir| dir.join("skills"))
}

/// 获取默认 sessions 目录
pub fn default_sessions_dir() -> Option<PathBuf> {
    bamboo_dir().map(|dir| dir.join("sessions"))
}

/// 获取默认日志文件路径
pub fn default_log_path() -> Option<PathBuf> {
    bamboo_dir().map(|dir| dir.join("logs").join("bamboo.log"))
}

/// 初始化 Bamboo 目录结构
pub async fn init_bamboo_dirs() -> ConfigResult<()> {
    if let Some(bamboo) = bamboo_dir() {
        tokio::fs::create_dir_all(&bamboo).await?;
        tokio::fs::create_dir_all(bamboo.join("skills")).await?;
        tokio::fs::create_dir_all(bamboo.join("sessions")).await?;
        tokio::fs::create_dir_all(bamboo.join("logs")).await?;
        tokio::fs::create_dir_all(bamboo.join("credentials")).await?;
    }
    Ok(())
}

/// 展开路径中的 ~ 为用户主目录
pub fn expand_tilde(path: &str) -> Option<PathBuf> {
    if path.starts_with("~/") {
        dirs::home_dir().map(|home| home.join(&path[2..]))
    } else {
        Some(PathBuf::from(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bamboo_dir() {
        let dir = bamboo_dir();
        assert!(dir.is_some());
        assert!(dir.unwrap().to_string_lossy().contains(".bamboo"));
    }

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~/.bamboo/config.json");
        assert!(expanded.is_some());
        assert!(!expanded.unwrap().to_string_lossy().starts_with("~"));
    }
}
