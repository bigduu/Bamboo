# Bamboo Config 系统设计

## 参考 OpenClaw 设计

OpenClaw 结构:
```
~/.openclaw/
├── openclaw.json      # 主配置 (JSON5)
├── agents/            # Agent 数据
├── skills/            # Skills
├── credentials/       # 认证凭证
└── logs/              # 日志
```

## Bamboo 配置结构

```
~/.bamboo/
├── config.json        # 主配置
├── skills/            # Skills 目录
├── sessions/          # Session 数据
├── credentials/       # LLM API keys
└── logs/              # 日志
```

## 配置内容 (config.json)

```json5
{
  "version": "0.1.0",
  "server": {
    "port": 8081,
    "host": "127.0.0.1",
    "cors": true
  },
  "llm": {
    "default_provider": "copilot",  // copilot | openai
    "providers": {
      "copilot": {
        "enabled": true,
        "base_url": "https://api.githubcopilot.com",
        "auth_method": "device_code"  // device_code | token
      },
      "openai": {
        "enabled": false,
        "base_url": "http://localhost:12123/v1",
        "api_key": "${BAMBOO_OPENAI_KEY}",  // 支持环境变量
        "model": "kimi-for-coding"
      }
    }
  },
  "skills": {
    "enabled": true,
    "auto_reload": true,
    "directories": [
      "~/.bamboo/skills"
    ]
  },
  "agent": {
    "max_rounds": 10,
    "system_prompt": "You are a helpful assistant",
    "timeout_seconds": 300
  },
  "storage": {
    "type": "jsonl",  // jsonl | sqlite (未来)
    "path": "~/.bamboo/sessions"
  },
  "logging": {
    "level": "info",  // debug | info | warn | error
    "file": "~/.bamboo/logs/bamboo.log",
    "max_size_mb": 100,
    "max_files": 5
  }
}
```

## Crate: bamboo-config

职责:
- 配置加载和解析
- 环境变量替换 (${VAR})
- 配置验证
- 热重载支持
- 默认配置生成

核心类型:
```rust
pub struct Config {
    pub version: String,
    pub server: ServerConfig,
    pub llm: LlmConfig,
    pub skills: SkillsConfig,
    pub agent: AgentConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
}

pub struct ConfigManager {
    path: PathBuf,
    config: Arc<RwLock<Config>>,
    watcher: Option<RecommendedWatcher>,
}

impl ConfigManager {
    pub async fn load(path: &Path) -> Result<Self>;
    pub async fn save(&self) -> Result<()>;
    pub fn get(&self) -> Arc<Config>;
    pub async fn reload(&self) -> Result<()>;
    pub fn watch(&mut self) -> Result<()>;  // 热重载
}
```

## 功能

1. **默认配置**: 首次启动自动生成
2. **环境变量**: `${ENV_VAR}` 语法支持
3. **验证**: 启动时验证配置合法性
4. **热重载**: 文件变化自动重载 (可选)
5. **分层**: 默认 < 文件 < 环境变量

## 集成

- `bamboo-server` 启动时加载配置
- `bamboo-cli` 支持 `bamboo config get/set` 命令
- 配置变化通知其他组件
