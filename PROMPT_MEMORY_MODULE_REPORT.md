# Bamboo 系统提示词管理和记忆增强模块 - 创建报告

## 创建的文件

### 后端部分

#### 1. bamboo-prompt crate
- `crates/bamboo-prompt/Cargo.toml` - Crate 配置
- `crates/bamboo-prompt/src/lib.rs` - 库入口
- `crates/bamboo-prompt/src/models.rs` - SystemPrompt 数据模型
- `crates/bamboo-prompt/src/manager.rs` - PromptManager
- `crates/bamboo-prompt/src/storage.rs` - 存储实现

#### 2. bamboo-memory crate
- `crates/bamboo-memory/Cargo.toml` - Crate 配置
- `crates/bamboo-memory/src/lib.rs` - 库入口
- `crates/bamboo-memory/src/models.rs` - Memory 和 SessionMemory 数据模型
- `crates/bamboo-memory/src/manager.rs` - MemoryManager
- `crates/bamboo-memory/src/extractor.rs` - 记忆提取器
- `crates/bamboo-memory/src/enhancer.rs` - 提示词增强

#### 3. 集成到 bamboo-server
- `crates/bamboo-server/src/handlers/prompts.rs` - Prompt HTTP API 处理器
- `crates/bamboo-server/src/handlers/memories.rs` - Memory HTTP API 处理器
- `crates/bamboo-server/src/handlers/mod.rs` - 更新模块导出
- `crates/bamboo-server/src/server.rs` - 添加 API 路由
- `crates/bamboo-server/src/state.rs` - 添加 PromptManager 和 MemoryManager
- `crates/bamboo-server/Cargo.toml` - 添加依赖
- `Cargo.toml` - 添加 workspace 成员

### 前端部分

#### 1. 页面
- `bamboo-ui/src/app/settings/prompts/page.tsx` - 提示词管理页面
- `bamboo-ui/src/app/settings/memories/page.tsx` - 记忆管理页面

#### 2. 组件
- `bamboo-ui/src/components/prompts/PromptConfigPanel.tsx` - 提示词配置面板
- `bamboo-ui/src/components/memories/MemoryConfigPanel.tsx` - 记忆配置面板

#### 3. Store
- `bamboo-ui/src/stores/promptStore.ts` - 提示词状态管理
- `bamboo-ui/src/stores/memoryStore.ts` - 记忆状态管理

#### 4. API 和类型
- `bamboo-ui/src/lib/api.ts` - 添加 Prompt 和 Memory API 函数
- `bamboo-ui/src/types/index.ts` - 添加 TypeScript 类型定义

## 功能说明

### 系统提示词管理 (bamboo-prompt)

1. **数据模型 (SystemPrompt)**
   - id: 唯一标识
   - name: 提示词名称
   - content: 提示词内容
   - is_default: 是否为默认提示词
   - is_custom: 是否为自定义提示词
   - category: 分类

2. **存储 (PromptStorage)**
   - 使用 JSON 文件格式存储
   - 存储路径: ~/.bamboo/prompts/
   - 支持列出、加载、保存、删除提示词

3. **管理器 (PromptManager)**
   - 确保默认提示词存在
   - 创建、更新、删除提示词
   - 设置默认提示词
   - 自动排序（默认优先）

### 记忆增强 (bamboo-memory)

1. **数据模型**
   - Memory: 单个记忆（id, session_id, content, tags, created_at, updated_at）
   - SessionMemory: 会话记忆集合（session_id, memories, updated_at）

2. **存储 (MemoryManager)**
   - 使用 JSON 文件格式存储
   - 存储路径: ~/.bamboo/memories/
   - 按会话存储记忆

3. **记忆提取 (MemoryExtractor)**
   - 从文本中提取记忆
   - 支持关键词："记住", "remember", "memory:", "记忆:"

4. **提示词增强 (enhancer)**
   - 将记忆附加到系统提示词
   - 最多附加 50 条记忆

### HTTP API

#### 提示词 API
- `GET /api/v1/prompts` - 获取所有提示词
- `POST /api/v1/prompts` - 创建提示词
- `PUT /api/v1/prompts/{id}` - 更新提示词
- `DELETE /api/v1/prompts/{id}` - 删除提示词
- `POST /api/v1/prompts/{id}/default` - 设为默认

#### 记忆 API
- `GET /api/v1/memories` - 获取所有记忆
- `GET /api/v1/sessions/{id}/memory` - 获取会话记忆

### 前端界面

1. **提示词管理页面**
   - 列出所有提示词
   - 创建新提示词
   - 编辑提示词
   - 删除提示词
   - 设置默认提示词
   - 显示提示词预览

2. **记忆管理页面**
   - 查看所有记忆
   - 按会话查看记忆
   - 显示记忆创建时间

## 如何使用

### 后端使用

```rust
// 创建 PromptManager
let storage = PromptStorage::new(home_dir.join(".bamboo/prompts"));
let prompt_manager = PromptManager::new(storage);
prompt_manager.ensure_default_prompt().await?;

// 创建 MemoryManager
let memory_manager = MemoryManager::new(home_dir.join(".bamboo/memories"));

// 在 AppState 中使用
pub struct AppState {
    pub prompt_manager: Arc<PromptManager>,
    pub memory_manager: Arc<MemoryManager>,
    // ... 其他字段
}
```

### 前端使用

```typescript
// 使用 Prompt Store
const { prompts, fetchPrompts, addPrompt, editPrompt, removePrompt, setDefault } = usePromptStore();

// 使用 Memory Store
const { memories, sessionMemory, fetchMemories, fetchSessionMemory } = useMemoryStore();
```

### 访问设置页面

- 提示词管理: `/settings/prompts`
- 记忆管理: `/settings/memories`

## 后续集成建议

1. **Agent Loop 集成**
   - 在会话创建时选择系统提示词
   - 自动从对话中提取记忆
   - 在发送消息前增强系统提示词

2. **增强功能**
   - 提示词模板支持
   - 记忆搜索和过滤
   - 记忆重要性评分
   - 自动记忆清理

3. **UI 改进**
   - 提示词预览模态框
   - 记忆编辑功能
   - 批量操作
