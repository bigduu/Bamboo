# Bamboo Masking 模块设计分析

## 需求分析

### 功能需求
1. **敏感信息脱敏**: 在发送给 LLM 之前，对用户输入进行 masking 处理
2. **可配置规则**: 支持正则表达式和关键字匹配
3. **持久化存储**: 配置保存在 `~/.bamboo/masking.json`
4. **灵活配置**: 支持启用/禁用、自定义替换文本

### 使用场景
- 隐藏 API Keys、密码、令牌
- 隐藏个人信息（手机号、身份证号）
- 隐藏内部系统 URL、IP 地址
- 隐藏财务数据

## 设计方案

### 1. 数据结构

```rust
// masking.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingConfig {
    pub enabled: bool,
    pub rules: Vec<MaskingRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingRule {
    pub id: String,              // 规则ID
    pub name: String,            // 规则名称
    pub rule_type: RuleType,     // 规则类型
    pub pattern: String,         // 正则或关键字
    pub replacement: String,     // 替换文本（默认 [MASKED]）
    pub enabled: bool,           // 是否启用
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    Regex,      // 正则表达式
    Keyword,    // 关键字匹配
}
```

### 2. 配置文件位置

```
~/.bamboo/
├── config.json          # 主配置
├── masking.json         # Masking 配置（新增）
├── sessions/           # 会话存储
└── skills/             # Skill 目录
```

### 3. 默认规则示例

```json
{
  "enabled": true,
  "rules": [
    {
      "id": "api_key",
      "name": "API Key",
      "rule_type": "regex",
      "pattern": "[a-zA-Z0-9]{32,}",
      "replacement": "[API_KEY_MASKED]",
      "enabled": true
    },
    {
      "id": "password",
      "name": "Password",
      "rule_type": "regex",
      "pattern": "(?i)(password|pwd|passwd)\s*[:=]\s*\S+",
      "replacement": "[PASSWORD_MASKED]",
      "enabled": true
    },
    {
      "id": "email",
      "name": "Email",
      "rule_type": "regex",
      "pattern": "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
      "replacement": "[EMAIL_MASKED]",
      "enabled": false
    },
    {
      "id": "internal_url",
      "name": "Internal URL",
      "rule_type": "keyword",
      "pattern": "internal.company.com",
      "replacement": "[INTERNAL_URL]",
      "enabled": true
    }
  ]
}
```

### 4. 集成点

#### Agent Loop Hook
在 `bamboo-core/src/agent/engine.rs` 中，在发送给 LLM 之前调用 masking：

```rust
// 在 AgentLoop::run 中
for round in 0..self.config.max_rounds {
    // ...
    
    // 1. 构建消息
    let messages = session.build_messages();
    
    // 2. 【新增】应用 masking
    let masked_messages = if let Some(masking) = &self.masking_config {
        masking.apply_to_messages(messages)
    } else {
        messages
    };
    
    // 3. 发送给 LLM
    let response = llm_provider.chat(masked_messages).await?;
    
    // ...
}
```

#### 配置管理集成
在 `bamboo-config` 中添加 masking 配置管理：

```rust
// config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // ... 现有字段
    pub masking: MaskingConfig,  // 新增
}
```

#### API 端点
新增 HTTP API 用于管理 masking 规则：

```
GET  /api/v1/masking/config      # 获取 masking 配置
POST /api/v1/masking/config      # 更新 masking 配置
GET  /api/v1/masking/rules       # 获取所有规则
POST /api/v1/masking/rules       # 创建规则
PUT  /api/v1/masking/rules/{id}  # 更新规则
DELETE /api/v1/masking/rules/{id} # 删除规则
POST /api/v1/masking/test        # 测试 masking 效果
```

### 5. 实现步骤

1. **创建 masking crate** (bamboo-masking)
   - MaskingConfig 结构体
   - MaskingRule 结构体
   - apply() 方法实现
   - 配置文件读写

2. **更新 bamboo-config**
   - 添加 masking 配置支持
   - 配置文件热重载

3. **更新 bamboo-core**
   - AgentLoop 集成 masking hook
   - 在发送 LLM 前调用

4. **更新 bamboo-server**
   - 新增 masking API 端点
   - 前端配置界面支持

5. **前端 UI**
   - Masking 配置页面
   - 规则编辑器（正则测试）
   - 实时预览效果

### 6. 安全考虑

- Masking 只在发送给 LLM 前应用，原始消息保存在 session 中
- 支持 reversible masking（可选，用于特定场景）
- 日志中也需要应用 masking

### 7. 性能考虑

- 使用 regex crate 进行高效匹配
- 支持规则优先级（按顺序匹配）
- 缓存编译后的正则表达式
