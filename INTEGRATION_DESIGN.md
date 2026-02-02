# Bamboo Skill/Tool 集成到 Server 设计文档

## 目标
将 bamboo-skill 和 bamboo-tool 集成到 bamboo-server，替换现有的 skill_loader 和 bamboo-mcp 工具系统。

## 当前架构

### 现有依赖 (Cargo.toml)
```toml
bamboo-core = { path = "../bamboo-core" }
bamboo-llm = { path = "../bamboo-llm" }
bamboo-mcp = { path = "../bamboo-mcp" }  # ← 需要替换
```

### 现有 State (state.rs)
```rust
pub struct AppState {
    pub tools: Arc<dyn ToolExecutor>,  // ← 当前是 bamboo_mcp::McpClient
    pub skill_loader: SkillLoader,     // ← 当前是自定义实现
    pub loaded_skills: Vec<SkillDefinition>,
}
```

## 集成方案

### 1. Cargo.toml 修改
添加依赖：
```toml
bamboo-core = { path = "../bamboo-core" }
bamboo-llm = { path = "../bamboo-llm" }
bamboo-skill = { path = "../bamboo-skill" }  # ← 新增
bamboo-tool = { path = "../bamboo-tool" }    # ← 新增
# 移除 bamboo-mcp (可选保留但不使用)
```

### 2. state.rs 重构

替换 AppState：
```rust
use bamboo_skill::{SkillManager, Skill};
use bamboo_tool::{ToolRegistry, InMemoryToolRegistry};

pub struct AppState {
    pub sessions: Arc<RwLock<HashMap<String, Session>>>,
    pub storage: JsonlStorage,
    pub llm: Arc<dyn bamboo_llm::LLMProvider>,
    pub tool_registry: Arc<InMemoryToolRegistry>,  // ← 替换 tools
    pub skill_manager: Arc<SkillManager>,           // ← 替换 skill_loader
    pub cancel_tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
}

impl AppState {
    pub async fn new_with_config(...) -> Self {
        // ... 现有 LLM 初始化代码 ...
        
        // 初始化 Tool Registry
        let tool_registry = Arc::new(InMemoryToolRegistry::new());
        
        // 初始化 Skill Manager
        let skills_dir = dirs::home_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join(".bamboo/skills");
        
        let skill_manager = Arc::new(
            SkillManager::new(skills_dir, tool_registry.clone())
                .expect("Failed to create skill manager")
        );
        
        // 启动 skill 监听
        let mut manager = skill_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = manager.start_watching().await {
                log::error!("Skill watcher error: {}", e);
            }
        });
        
        // 加载初始 skills
        if let Err(e) = skill_manager.load_all_skills().await {
            log::error!("Failed to load skills: {}", e);
        }
        
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            storage,
            llm,
            tool_registry,
            skill_manager,
            cancel_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 获取所有 tool schemas (从 registry)
    pub fn get_all_tool_schemas(&self) -> Vec<bamboo_core::tools::ToolSchema> {
        // 从 tool_registry 获取所有工具并转换格式
        self.tool_registry.list()
            .into_iter()
            .map(|tool_def| convert_to_tool_schema(tool_def))
            .collect()
    }
    
    /// Build system prompt with skills context
    pub fn build_system_prompt(&self, base_prompt: &str) -> String {
        // 从 skill_manager 获取所有 skills 的 system_prompt
        let skills = self.skill_manager.list_skills();
        let mut prompt = base_prompt.to_string();
        
        for skill in skills {
            if let Some(sp) = &skill.system_prompt {
                prompt.push_str("\n\n");
                prompt.push_str(sp);
            }
        }
        
        prompt
    }
}
```

### 3. ToolExecutor 适配器

创建适配器让 bamboo-tool 兼容 bamboo-core 的 ToolExecutor trait：

```rust
// 在 state.rs 或单独模块
pub struct ToolRegistryExecutor {
    registry: Arc<InMemoryToolRegistry>,
}

#[async_trait]
impl ToolExecutor for ToolRegistryExecutor {
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult> {
        // 从 registry 获取 tool_def
        let tool_def = self.registry.get(&call.name)
            .ok_or_else(|| AgentError::Tool(format!("Tool not found: {}", call.name)))?;
        
        // 转换参数
        let args = call.arguments.clone();
        
        // 使用 bamboo_tool::ToolExecutor 执行
        let executor = bamboo_tool::ToolExecutor::new();
        let result = executor.execute(&tool_def, args).await
            .map_err(|e| AgentError::Tool(e.to_string()))?;
        
        Ok(ToolResult {
            success: result.success,
            output: result.output,
        })
    }
    
    fn list_tools(&self) -> Vec<ToolSchema> {
        self.registry.list()
            .into_iter()
            .map(convert_to_tool_schema)
            .collect()
    }
}
```

### 4. agent_runner.rs 修改

修改调用以使用新的 tool 系统：

```rust
// 在 run_agent_loop_with_config 中
// 当前: tools: Arc<dyn ToolExecutor>
// 改为使用从 state 传入的 tool executor

// 获取工具 schemas
let tool_schemas = state.get_all_tool_schemas();

// 执行工具时使用 ToolRegistryExecutor
```

### 5. 技能目录初始化

在 server 启动时创建默认技能目录：

```rust
// 在 AppState::new 中
let skills_dir = dirs::home_dir()
    .unwrap_or_else(|| std::env::temp_dir())
    .join(".bamboo/skills");

// 创建目录（如果不存在）
tokio::fs::create_dir_all(&skills_dir).await
    .expect("Failed to create skills directory");

// 创建示例 skill（如果不存在）
let example_skill_dir = skills_dir.join("example");
if !example_skill_dir.exists() {
    create_example_skill(&example_skill_dir).await;
}
```

### 6. 移除旧代码

- 移除 `mod skill_loader;` (保留文件但不再使用)
- 移除 `bamboo-mcp` 依赖 (可选)
- 更新所有使用旧 skill_loader 的代码

## 文件修改清单

| 文件 | 修改类型 | 说明 |
|------|----------|------|
| `Cargo.toml` | 修改 | 添加 bamboo-skill, bamboo-tool 依赖 |
| `src/state.rs` | 重写 | 使用 SkillManager 和 ToolRegistry |
| `src/agent_runner.rs` | 修改 | 适配新的 tool 系统 |
| `src/handlers/*.rs` | 可能需要修改 | 使用新的 state 方法 |

## 验证步骤

1. `cargo check` 编译通过
2. `cargo run -p bamboo-server` 启动正常
3. 检查 `~/.bamboo/skills/` 目录创建
4. 创建测试 skill，验证热重载
5. 发送请求验证工具执行

## 注意事项

- 保持向后兼容：如果 skill 目录不存在，优雅降级
- 错误处理：skill 加载失败不影响 server 启动
- 性能：ToolRegistry 使用 DashMap，并发安全
- 热重载：文件变化自动重新加载 skills
