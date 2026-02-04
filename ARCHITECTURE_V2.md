# Bamboo 迭代架构设计

**版本**: v2.0  
**目标**: 打造具备完整 Agent 能力的 Bamboo 系统

---

## 核心功能模块

### 1. 多轮对话系统 (Multi-Turn Conversation)

**目标**: 实现真正的多轮对话，保持上下文连贯

**当前问题**:
- Agent Runner 调用 LLM 时没有传递历史消息
- 每轮对话都是独立的单轮请求

**设计方案**:
```rust
// 1. 扩展 Session 结构
pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,  // 完整对话历史
    pub context_window: usize,   // 上下文窗口大小
    pub summary: Option<String>, // 对话摘要（长对话时）
}

// 2. 构建 LLM 请求时包含历史
impl AgentRunner {
    async fn build_chat_request(&self, session: &Session) -> ChatRequest {
        let messages = self.prepare_context(session).await;
        ChatRequest { messages, ... }
    }
    
    // 3. 上下文准备（支持截断和摘要）
    async fn prepare_context(&self, session: &Session) -> Vec<Message> {
        // 如果消息太多，使用摘要 + 最近 N 条
        if session.messages.len() > self.config.max_context_messages {
            self.summarize_and_truncate(session).await
        } else {
            session.messages.clone()
        }
    }
}
```

**实现步骤**:
1. 修改 `Session` 结构，添加 `messages` 字段
2. 修改 `AgentRunner::handle_chat`，构建请求时包含历史
3. 添加上下文压缩逻辑（token 限制时）
4. 测试多轮对话连贯性

---

### 2. 工具调用系统 (Tool Calling)

**目标**: LLM 可以调用外部工具，并获取结果继续对话

**当前状态**:
- Skill 系统已存在，可以转为 Tools
- 但 LLM 没有真正调用工具的能力

**设计方案**:
```rust
// 1. Tool 定义
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: ToolParameters,
    pub handler: Box<dyn ToolHandler>,
}

// 2. Tool 调用流程
impl AgentRunner {
    async fn run_with_tools(&self, session: &mut Session) -> Result<()> {
        loop {
            // 发送请求给 LLM（包含可用 tools）
            let response = self.llm.chat_with_tools(
                &session.messages,
                &self.available_tools
            ).await?;
            
            match response {
                // LLM 返回普通消息
                LLMResponse::Message(content) => {
                    session.add_assistant_message(content);
                    break;
                }
                // LLM 要求调用工具
                LLMResponse::ToolCall(tool_call) => {
                    // 执行工具
                    let result = self.execute_tool(&tool_call).await?;
                    // 将结果添加到对话
                    session.add_tool_result(tool_call.id, result);
                }
            }
        }
    }
}
```

**实现步骤**:
1. 定义 `Tool` trait 和调用流程
2. 将现有 Skills 转换为 Tools
3. 修改 LLM provider 支持 `tools` 参数
4. 实现 Tool 调用循环
5. 测试工具调用功能

---

### 3. 上下文压缩 (Context Compression)

**目标**: 长对话时自动压缩历史，保持 token 在限制内

**设计方案**:
```rust
pub struct ContextCompressor {
    max_tokens: usize,
    summarizer: Box<dyn LLMProvider>, // 用于生成摘要
}

impl ContextCompressor {
    async fn compress(&self, messages: &[Message]) -> Vec<Message> {
        let token_count = self.estimate_tokens(messages);
        
        if token_count <= self.max_tokens {
            return messages.to_vec();
        }
        
        // 策略1: 截断最早的消息
        // 策略2: 生成摘要替代早期消息
        // 策略3: 使用 RAG 检索相关历史
        
        self.summarize_and_truncate(messages).await
    }
    
    async fn summarize_and_truncate(&self, messages: &[Message]) -> Vec<Message> {
        // 保留最近 N 条完整消息
        let recent = messages.split_at(messages.len() - 10).1;
        
        // 对早期消息生成摘要
        let early = &messages[..messages.len() - 10];
        let summary = self.summarizer
            .summarize(&format!("{:?}", early))
            .await;
        
        vec![
            Message::system(format!("Previous conversation summary: {}", summary)),
            ...recent
        ]
    }
}
```

**实现步骤**:
1. 实现 token 估算
2. 实现摘要生成
3. 集成到对话流程
4. 测试长对话场景

---

### 4. 记忆模式 (Memory System)

**目标**: 长期记忆，跨会话保持知识

**设计方案**:
```rust
pub struct MemorySystem {
    vector_store: Arc<dyn VectorStore>,  // 向量数据库
    extractor: Box<dyn EntityExtractor>, // 实体抽取
}

impl MemorySystem {
    // 从对话中提取记忆
    async fn extract_memories(&self, session: &Session) -> Vec<Memory> {
        let text = format!("{:?}", session.messages);
        
        // 提取关键信息
        let entities = self.extractor.extract(&text).await;
        let facts = self.extract_facts(&text).await;
        
        entities.into_iter()
            .chain(facts)
            .map(|e| Memory::from_entity(e))
            .collect()
    }
    
    // 检索相关记忆
    async fn retrieve_relevant(&self, query: &str) -> Vec<Memory> {
        let embedding = self.embed(query).await;
        self.vector_store.search(embedding, 5).await
    }
}

// 在对话开始时注入相关记忆
impl AgentRunner {
    async fn start_session(&self, user_id: &str) -> Session {
        let mut session = Session::new();
        
        // 检索用户相关记忆
        let memories = self.memory
            .retrieve_relevant(&format!("user:{}", user_id))
            .await;
        
        if !memories.is_empty() {
            session.add_system_message(
                format!("Relevant context: {:?}", memories)
            );
        }
        
        session
    }
}
```

**实现步骤**:
1. 集成向量数据库 (如 pgvector 或 chroma)
2. 实现实体抽取
3. 实现记忆检索
4. 在对话流程中注入记忆

---

### 5. 人格/灵魂系统 (Persona/Soul)

**目标**: 可配置的人格，让 Agent 有不同行为模式

**设计方案**:
```rust
pub struct Persona {
    pub name: String,
    pub system_prompt: String,
    pub voice: VoiceStyle,
    pub behaviors: Vec<BehaviorRule>,
    pub memory_preferences: MemoryConfig,
}

pub struct Soul {
    persona: Persona,
    emotional_state: EmotionalState,
    goals: Vec<Goal>,
}

impl Soul {
    // 根据人格生成系统提示词
    fn generate_system_prompt(&self) -> String {
        format!(
            "You are {}. {}\n\nCurrent emotional state: {:?}\nActive goals: {:?}",
            self.persona.name,
            self.persona.system_prompt,
            self.emotional_state,
            self.goals
        )
    }
    
    // 更新情感状态
    fn update_emotion(&mut self, event: &Event) {
        match event {
            Event::UserMessage { sentiment, .. } => {
                self.emotional_state.adjust(*sentiment);
            }
            Event::TaskCompleted { success, .. } => {
                if *success {
                    self.emotional_state.boost_confidence();
                }
            }
            _ => {}
        }
    }
}
```

**实现步骤**:
1. 设计 Persona 配置格式
2. 实现动态系统提示词生成
3. 添加情感状态跟踪
4. 测试不同人格的行为差异

---

## 实施优先级

| 优先级 | 模块 | 预计工作量 | 依赖 |
|--------|------|-----------|------|
| 🔴 P0 | 多轮对话 | 2-3 天 | 无 |
| 🔴 P0 | 工具调用 | 3-4 天 | 多轮对话 |
| 🟡 P1 | 上下文压缩 | 2 天 | 多轮对话 |
| 🟡 P1 | 记忆模式 | 4-5 天 | 上下文压缩 |
| 🟢 P2 | 人格/灵魂 | 3 天 | 记忆模式 |

---

## 技术选型

| 组件 | 选择 | 理由 |
|------|------|------|
| 向量数据库 | pgvector / chroma | 开源、易集成 |
| 摘要生成 | 使用 LLM 自身 | 无需额外模型 |
| 实体抽取 | 使用 LLM 或 spaCy | 简单场景 LLM 足够 |
| 情感分析 | 使用 LLM | 无需额外依赖 |

---

## 测试策略

每个模块完成后需要：
1. 单元测试
2. 集成测试
3. 端到端测试（使用真实 LLM）
4. 性能测试（长对话场景）

---

*设计文档版本: 2026-02-03*  
*下一步: 使用 Codex CLI 实现 P0 功能*
