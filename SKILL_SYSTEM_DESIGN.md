# Bamboo Skill & Tool System 设计文档

## 目标
实现类似 OpenClaw 的 SKILL.md 动态加载系统，支持运行时热重载。

## Crate 划分

### 1. bamboo-skill
**职责:** Skill 的发现、加载、管理和热重载

**核心类型:**
```rust
pub struct Skill {
    pub name: String,
    pub description: String,
    pub manifest: SkillManifest,
    pub tools: Vec<ToolDef>,
    pub system_prompt: Option<String>,
    pub path: PathBuf,
}

pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
}

pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub command: String,
    pub args: Vec<ArgDef>,
}

pub struct SkillManager {
    skills_dir: PathBuf,
    skills: Arc<RwLock<HashMap<String, Skill>>>,
    watcher: RecommendedWatcher,
}
```

**核心 Trait:**
```rust
#[async_trait]
pub trait SkillLoader: Send + Sync {
    async fn load(&self, path: &Path) -> Result<Skill>;
    async fn unload(&self, name: &str) -> Result<()>;
}

#[async_trait]
pub trait SkillWatcher: Send + Sync {
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
}
```

**功能:**
- 扫描 `~/.bamboo/skills/` 目录
- 解析 SKILL.md (YAML frontmatter + markdown)
- 文件变化监听 (notify crate)
- 热重载: 新增/修改/删除 skill 时自动更新
- 提供技能列表查询接口

### 2. bamboo-tool
**职责:** 工具脚本的执行和管理

**核心类型:**
```rust
pub struct ToolExecutor {
    timeout: Duration,
    allowed_commands: Vec<String>,
}

pub struct ToolRequest {
    pub name: String,
    pub arguments: HashMap<String, Value>,
}

pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
}

pub enum ToolType {
    Shell,
    Python,
    Node,
}
```

**核心 Trait:**
```rust
#[async_trait]
pub trait ToolRunner: Send + Sync {
    async fn execute(&self, tool_def: &ToolDef, args: HashMap<String, Value>) -> Result<ToolResult>;
    fn validate_args(&self, tool_def: &ToolDef, args: &HashMap<String, Value>) -> Result<()>;
}

#[async_trait]
pub trait ToolRegistry: Send + Sync {
    async fn register(&self, tool: ToolDef) -> Result<()>;
    async fn unregister(&self, name: &str) -> Result<()>;
    fn list(&self) -> Vec<ToolDef>;
    fn get(&self, name: &str) -> Option<ToolDef>;
}
```

**功能:**
- 根据 tool.command 后缀识别类型 (.sh/.py/.js)
- 执行脚本并捕获输出
- 参数注入: 通过环境变量 `$ARG_NAME`
- 超时控制 (默认 30s)
- 安全: 路径检查、危险命令过滤

## 目录结构

```
~/.bamboo/
├── skills/
│   ├── web-search/
│   │   ├── SKILL.md
│   │   └── tools/
│   │       └── search.sh
│   ├── file-operations/
│   │   ├── SKILL.md
│   │   └── tools/
│   │       ├── read.py
│   │       └── write.py
│   └── my-custom/
│       ├── SKILL.md
│       └── tools/
│           └── custom.js
```

## SKILL.md 格式

```markdown
---
name: web-search
version: 0.1.0
description: Search the web using DuckDuckGo
author: Bamboo Team
tools:
  - name: search
    description: Search for a query
    command: tools/search.sh
    args:
      - name: query
        type: string
        required: true
        description: The search query
      - name: limit
        type: number
        required: false
        default: 10
        description: Max results
---

# Web Search Skill

This skill provides web search capabilities.

## Usage

The assistant can search the web when it needs current information.
```

## 依赖

### bamboo-skill
- `notify` - 文件系统监听
- `yaml-rust` 或 `serde_yaml` - YAML 解析
- `markdown` 或 `pulldown-cmark` - Markdown 解析 (提取 frontmatter)
- `tokio` - 异步运行时

### bamboo-tool
- `tokio::process` - 异步进程执行
- `serde_json` - 参数/结果序列化

## 集成到现有系统

1. **bamboo-server** 依赖 bamboo-skill 和 bamboo-tool
2. **bamboo-core** 的 `ToolExecutor` trait 由 bamboo-tool 实现
3. **agent_runner** 从 SkillManager 获取 tools 注入 LLM

## 热重载流程

```
文件变化 (notify)
    ↓
SkillManager 检测到变化
    ↓
重新加载 SKILL.md
    ↓
更新 ToolRegistry (注册/注销 tools)
    ↓
通知 bamboo-server 刷新 tools 列表
    ↓
新请求使用更新后的 tools
```

## 实现步骤

1. 创建 `bamboo-skill` crate，实现:
   - Skill 结构定义
   - SKILL.md 解析器
   - 目录扫描器
   - 文件监听器 (notify)
   - SkillManager

2. 创建 `bamboo-tool` crate，实现:
   - Tool 结构定义
   - ToolRunner (脚本执行)
   - ToolRegistry
   - 参数验证

3. 在 `bamboo-server` 中集成:
   - 初始化 SkillManager
   - 从 skills 加载 tools
   - 注入到 AgentLoop

## 注意事项

- Skill 名称唯一性检查
- 工具名冲突处理 (后加载覆盖? 报错?)
- 脚本执行权限检查
- 资源清理 (停止监听时)
- 错误处理 (解析失败时的优雅降级)
