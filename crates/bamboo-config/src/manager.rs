use crate::config::{Config, ConfigError, ConfigResult};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 配置管理器
#[derive(Clone)]
pub struct ConfigManager {
    path: PathBuf,
    config: Arc<RwLock<Config>>,
    #[cfg(feature = "hot-reload")]
    watcher: Option<Arc<RwLock<notify::RecommendedWatcher>>>,
}

impl ConfigManager {
    /// 加载配置文件
    pub async fn load(path: &Path) -> ConfigResult<Self> {
        let config = if path.exists() {
            info!("Loading config from {:?}", path);
            let content = tokio::fs::read_to_string(path).await?;
            let content = Self::expand_env_vars(&content)?;
            serde_json::from_str(&content)?
        } else {
            info!("Config file not found, creating default config at {:?}", path);
            let default_config = Config::default();
            // 确保父目录存在
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
            #[cfg(feature = "hot-reload")]
            watcher: None,
        })
    }

    /// 从默认位置加载配置
    pub async fn load_default() -> ConfigResult<Self> {
        let config_path = Self::default_config_path()?;
        Self::load(&config_path).await
    }

    /// 获取默认配置路径 (~/.bamboo/config.json)
    pub fn default_config_path() -> ConfigResult<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| ConfigError::InvalidPath("Could not find home directory".to_string()))?;
        Ok(home.join(".bamboo").join("config.json"))
    }

    /// 创建一个新的配置管理器（用于测试）
    pub fn new(config: Config, path: PathBuf) -> Self {
        Self {
            path,
            config: Arc::new(RwLock::new(config)),
            #[cfg(feature = "hot-reload")]
            watcher: None,
        }
    }

    /// 获取配置的只读引用
    pub fn get(&self) -> Arc<RwLock<Config>> {
        Arc::clone(&self.config)
    }

    /// 保存配置到文件
    pub async fn save(&self) -> ConfigResult<()> {
        let config = self.config.read().await;
        let content = serde_json::to_string_pretty(&*config)?;
        drop(config);
        
        // 确保父目录存在
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::write(&self.path, content).await?;
        info!("Config saved to {:?}", self.path);
        Ok(())
    }

    /// 保存配置到指定路径
    pub async fn save_to(&self, path: &Path) -> ConfigResult<()> {
        let config = self.config.read().await;
        let content = serde_json::to_string_pretty(&*config)?;
        drop(config);
        
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// 重新加载配置
    pub async fn reload(&self) -> ConfigResult<()> {
        if !self.path.exists() {
            return Err(ConfigError::InvalidPath(format!(
                "Config file not found: {:?}",
                self.path
            )));
        }

        let content = tokio::fs::read_to_string(&self.path).await?;
        let content = Self::expand_env_vars(&content)?;
        let new_config: Config = serde_json::from_str(&content)?;
        
        // 验证新配置
        Self::validate(&new_config)?;
        
        let mut config = self.config.write().await;
        *config = new_config;
        drop(config);
        
        info!("Config reloaded from {:?}", self.path);
        Ok(())
    }

    /// 更新配置
    pub async fn update<F>(&self, f: F) -> ConfigResult<()>
    where
        F: FnOnce(&mut Config),
    {
        let mut config = self.config.write().await;
        f(&mut config);
        drop(config);
        self.save().await
    }

    /// 验证配置
    pub fn validate(config: &Config) -> ConfigResult<()> {
        // 验证服务器端口
        if config.server.port == 0 {
            return Err(ConfigError::Validation(
                "Server port cannot be 0".to_string(),
            ));
        }

        // 验证 agent 配置
        if config.agent.max_rounds == 0 {
            return Err(ConfigError::Validation(
                "Agent max_rounds must be greater than 0".to_string(),
            ));
        }

        if config.agent.timeout_seconds == 0 {
            return Err(ConfigError::Validation(
                "Agent timeout_seconds must be greater than 0".to_string(),
            ));
        }

        // 验证默认 provider 是否存在
        if !config.llm.providers.contains_key(&config.llm.default_provider) {
            return Err(ConfigError::Validation(format!(
                "Default LLM provider '{}' not found in providers list",
                config.llm.default_provider
            )));
        }

        Ok(())
    }

    /// 展开环境变量 ${VAR} 或 ${VAR:-default}
    fn expand_env_vars(content: &str) -> ConfigResult<String> {
        let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
        let mut result = content.to_string();
        
        for cap in re.captures_iter(content) {
            let full_match = cap.get(0).unwrap().as_str();
            let var_expr = cap.get(1).unwrap().as_str();
            
            // 处理 ${VAR:-default} 语法
            let (var_name, default_value) = if let Some(pos) = var_expr.find(":-") {
                let (name, rest) = var_expr.split_at(pos);
                (name, Some(&rest[2..]))
            } else {
                (var_expr, None)
            };
            
            let replacement = match std::env::var(var_name) {
                Ok(val) => val,
                Err(_) => {
                    if let Some(default) = default_value {
                        default.to_string()
                    } else {
                        return Err(ConfigError::EnvVarNotFound(var_name.to_string()));
                    }
                }
            };
            
            result = result.replace(full_match, &replacement);
        }
        
        Ok(result)
    }

    /// 获取配置文件路径
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(feature = "hot-reload")]
impl ConfigManager {
    /// 启动热重载监听
    pub fn watch<F>(&mut self, callback: F) -> ConfigResult<()>
    where
        F: Fn() + Send + 'static,
    {
        use notify::{Config as NotifyConfig, Event, RecommendedWatcher, RecursiveMode, Watcher, Result as NotifyResult};
        use std::sync::mpsc::channel;
        use std::thread;
        
        let path = self.path.clone();
        let config = Arc::clone(&self.config);
        
        let (tx, rx) = channel();
        
        let mut watcher = RecommendedWatcher::new(
            move |res: NotifyResult<Event>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() {
                        let _ = tx.send(());
                    }
                }
            },
            NotifyConfig::default(),
        )?;
        
        watcher.watch(&path, RecursiveMode::NonRecursive)?;
        self.watcher = Some(Arc::new(RwLock::new(watcher)));
        
        // 在后台线程中监听文件变化
        thread::spawn(move || {
            while rx.recv().is_ok() {
                debug!("Config file changed, reloading...");
                
                // 使用 tokio runtime 来执行异步重载
                let rt = tokio::runtime::Runtime::new().unwrap();
                let reload_result = rt.block_on(async {
                    if !path.exists() {
                        return Err(ConfigError::InvalidPath(format!(
                            "Config file not found: {:?}",
                            path
                        )));
                    }

                    let content = tokio::fs::read_to_string(&path).await?;
                    let content = ConfigManager::expand_env_vars(&content)?;
                    let new_config: Config = serde_json::from_str(&content)?;
                    
                    // 验证新配置
                    ConfigManager::validate(&new_config)?;
                    
                    let mut cfg = config.write().await;
                    *cfg = new_config;
                    Ok::<(), ConfigError>(())
                });
                
                match reload_result {
                    Ok(()) => {
                        info!("Config hot-reloaded successfully");
                        callback();
                    }
                    Err(e) => {
                        warn!("Failed to hot-reload config: {}", e);
                    }
                }
            }
        });
        
        info!("Started watching config file: {:?}", self.path);
        Ok(())
    }

    /// 停止热重载监听
    pub fn unwatch(&mut self) -> ConfigResult<()> {
        if let Some(ref watcher) = self.watcher {
            use notify::Watcher;
            // 获取写锁并停止监听
            if let Ok(mut w) = watcher.try_write() {
                w.unwatch(&self.path)?;
            }
            self.watcher = None;
            info!("Stopped watching config file");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        
        let manager = ConfigManager::load(&config_path).await.unwrap();
        let config = manager.get().read().await.clone();
        
        assert_eq!(config.server.port, 8081);
        assert_eq!(config.server.host, "127.0.0.1");
        assert!(config.skills.enabled);
    }

    #[tokio::test]
    async fn test_env_var_expansion() {
        std::env::set_var("TEST_VAR", "test_value");
        
        let content = r#"{"key": "${TEST_VAR}"}"#;
        let expanded = ConfigManager::expand_env_vars(content).unwrap();
        
        assert!(expanded.contains("test_value"));
    }

    #[tokio::test]
    async fn test_config_validation() {
        let mut config = Config::default();
        config.server.port = 0;
        
        assert!(ConfigManager::validate(&config).is_err());
        
        config.server.port = 8080;
        assert!(ConfigManager::validate(&config).is_ok());
    }
}
