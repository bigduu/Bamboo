# Bamboo System Prompt 管理与记忆增强设计

## 需求分析

### 功能需求
1. **系统提示词管理**：
   - 默认系统提示词
   - 用户自定义提示词
   - 提示词模板库
   - 新会话时选择提示词

2. **记忆增强**：
   - 对话历史摘要
   - 关键信息提取
   - 长期记忆存储
   - 上下文压缩

## 设计方案

### 1. 数据模型

```rust
// System Prompt 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPrompt {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content: String,
    pub is_default: bool,
    pub is_custom: bool,  // 用户自定义 vs 系统预设
    pub category: PromptCategory,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub usage_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptCategory {
    General,      // 通用助手
    Coding,       // 编程助手
    Writing,      // 写作助手
    Analysis,     // 数据分析
    Custom,       // 用户自定义
}

// 记忆片段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub session_id: String,
    pub content: String,        // 记忆内容
    pub memory_type: MemoryType,
    pub importance: f32,        // 重要性分数 0-1
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Fact,         // 事实信息
    Preference,   // 用户偏好
    Context,      // 上下文信息
    Summary,      // 对话摘要
}

// 会话记忆上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMemory {
    pub session_id: String,
    pub summary: String,                    // 对话摘要
    pub key_facts: Vec<String>,             // 关键事实
    pub user_preferences: Vec<String>,      // 用户偏好
    pub related_memories: Vec<Memory>,      // 相关记忆
    pub token_count: usize,                 // 当前 Token 数
}
```

### 2. 存储设计

```
~/.bamboo/
├── prompts/
│   ├── default.json          # 默认提示词
│   ├── custom/               # 用户自定义提示词
│   │   ├── prompt_xxx.json
│   │   └── prompt_yyy.json
│   └── presets/              # 系统预设提示词
│       ├── coding.json
│       ├── writing.json
│       └── analysis.json
├── memories/
│   ├── memories.jsonl        # 所有记忆片段
│   ├── session_xxx.json      # 会话记忆
│   └── index/                # 向量索引（用于语义搜索）
└── context/
    └── summaries.jsonl       # 对话摘要
```

### 3. 核心功能

#### 3.1 提示词管理

```rust
pub struct PromptManager {
    storage: Arc<dyn PromptStorage>,
    default_prompt: SystemPrompt,
}

impl PromptManager {
    // 获取所有提示词
    pub async fn list_prompts(&self) -> Vec<SystemPrompt>;
    
    // 获取默认提示词
    pub async fn get_default(&self) -> SystemPrompt;
    
    // 创建自定义提示词
    pub async fn create_prompt(&self, name: &str, content: &str, category: PromptCategory) -> SystemPrompt;
    
    // 更新提示词
    pub async fn update_prompt(&self, id: &str, updates: PromptUpdates) -> Result<SystemPrompt>;
    
    // 删除自定义提示词
    pub async fn delete_prompt(&self, id: &str) -> Result<()>;
    
    // 设置默认提示词
    pub async fn set_default(&self, id: &str) -> Result<()>;
}
```

#### 3.2 记忆管理

```rust
pub struct MemoryManager {
    storage: Arc<dyn MemoryStorage>,
    llm: Arc<dyn LLMProvider>,
}

impl MemoryManager {
    // 提取记忆
    pub async fn extract_memories(&self, session: &Session) -> Vec<Memory>;
    
    // 生成对话摘要
    pub async fn generate_summary(&self, messages: &[Message]) -> String;
    
    // 检索相关记忆
    pub async fn retrieve_memories(&self, query: &str, limit: usize) -> Vec<Memory>;
    
    // 压缩上下文
    pub async fn compress_context(&self, context: &str, max_tokens: usize) -> String;
    
    // 增强系统提示词
    pub async fn enhance_prompt(&self, base_prompt: &str, session_memory: &SessionMemory) -> String;
}
```

#### 3.3 上下文增强流程

```
1. 获取基础系统提示词（用户选择或默认）
2. 检索相关记忆
   - 从当前会话提取关键信息
   - 从历史记忆检索相关内容
3. 生成增强提示词
   - 基础提示词
   + 对话摘要（如有）
   + 关键事实
   + 用户偏好
   + 相关记忆
4. 检查 Token 限制
   - 如果超出限制，进行压缩
   - 优先保留重要信息
5. 发送给 LLM
```

### 4. HTTP API

```
# 提示词管理
GET    /api/v1/prompts              # 获取所有提示词
GET    /api/v1/prompts/default      # 获取默认提示词
POST   /api/v1/prompts              # 创建自定义提示词
GET    /api/v1/prompts/{id}         # 获取特定提示词
PUT    /api/v1/prompts/{id}         # 更新提示词
DELETE /api/v1/prompts/{id}         # 删除提示词
POST   /api/v1/prompts/{id}/default # 设为默认

# 记忆管理
GET    /api/v1/memories             # 获取所有记忆
GET    /api/v1/memories/search?q=xxx # 搜索记忆
GET    /api/v1/sessions/{id}/memory # 获取会话记忆
POST   /api/v1/sessions/{id}/memory/summarize # 生成摘要
DELETE /api/v1/memories/{id}        # 删除记忆

# 上下文增强
POST   /api/v1/context/enhance      # 测试上下文增强
```

### 5. 前端界面

#### 5.1 提示词管理页面
```
/settings/prompts
├── 提示词列表（卡片展示）
│   ├── 系统预设（Coding、Writing、Analysis）
│   └── 用户自定义
├── 创建/编辑提示词
│   ├── 名称输入
│   ├── 描述输入
│   ├── 内容编辑（支持变量）
│   └── 预览
└── 设为默认按钮
```

#### 5.2 新会话选择提示词
```
/chat/new
├── 提示词选择器（下拉菜单）
│   ├── 默认提示词
│   ├── 最近使用
│   └── 所有提示词
├── 快速预览
└── 创建会话按钮
```

#### 5.3 记忆管理页面
```
/settings/memories
├── 记忆列表
│   ├── 搜索框
│   ├── 筛选（类型、重要性）
│   └── 记忆卡片（内容、来源、时间）
├── 会话记忆
│   └── 按会话查看
└── 统计信息
    ├── 记忆总数
    ├── 会话数
    └── 存储占用
```

### 6. 变量支持

提示词内容支持变量替换：

```markdown
{{current_date}} - 当前日期
{{user_name}} - 用户名
{{session_topic}} - 会话主题
{{memory_summary}} - 记忆摘要
{{preferences}} - 用户偏好
```

### 7. 实现步骤

1. **后端**
   - [ ] 创建 bamboo-memory crate
   - [ ] 实现 PromptManager
   - [ ] 实现 MemoryManager
   - [ ] 添加 HTTP API
   - [ ] 集成到 Agent Loop

2. **前端**
   - [ ] 创建提示词管理页面
   - [ ] 创建记忆管理页面
   - [ ] 新会话提示词选择
   - [ ] 提示词编辑器

3. **集成**
   - [ ] 会话创建时选择提示词
   - [ ] 自动记忆提取
   - [ ] 上下文增强
