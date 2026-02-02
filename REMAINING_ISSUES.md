# Bamboo 项目剩余问题清单

**文档日期**: 2026-02-03  
**状态**: 待修复

---

## 🔴 严重问题（需立即修复）

### 1. unwrap/panic 在生产代码中
**位置**: `bamboo-core/src/chat/chunk.rs:118`, `bamboo-core/src/types/content.rs:104-140`

**问题代码**:
```rust
_ => panic!("Expected content chunk"),
_ => panic!("Expected text content"),
```

**修复方案**: 使用 `Result` 类型替代 panic，添加错误处理

**影响**: 生产环境崩溃风险

---

### 2. SSE 流解析缺陷
**位置**: `bamboo-llm/src/provider/base.rs`

**问题**: 不处理 SSE 事件跨 chunk 分割的情况

**修复方案**: 实现 SSE 缓冲区，处理跨 chunk 的事件

**影响**: 流式响应可能丢失数据

---

## 🟡 中等问题（本周修复）

### 3. 心跳机制未实现
**位置**: `bamboo-server/src/websocket/connection.rs`

**问题**: 心跳方法存在但从未被调用，发送的是 Pong 而不是 Ping

**修复方案**: 
- 在 WebSocket 连接建立时启动心跳任务
- 修复为发送 Ping 并等待 Pong

---

### 4. 重复排序问题
**位置**: `bamboo-session/src/jsonl_storage.rs`

**问题代码**:
```rust
self.by_time.sort_by(|a, b| b.1.cmp(&a.1)); // 每次添加都排序
```

**修复方案**: 使用 `BTreeMap` 或延迟排序

**影响**: 性能问题，O(n log n) 每次添加

---

### 5. 内存缓存无上限
**位置**: `bamboo-session/src/session_manager.rs`

**问题**: 内存缓存没有大小限制，可能导致内存泄漏

**修复方案**: 添加 LRU 缓存或大小限制

---

### 6. 静默错误处理
**位置**: `bamboo-core/src/storage/jsonl.rs`

**问题代码**:
```rust
serde_json::from_str(&line).ok() // 静默忽略解析错误
```

**修复方案**: 记录错误日志，不要静默忽略

---

## 📝 文档和测试（本月完成）

### 7. 文档不完整
**问题**:
- 根目录 README.md 过于简单
- 缺少架构设计文档
- API 文档不完整

**修复方案**:
- 更新 README，添加项目介绍、快速开始、架构图
- 添加 ARCHITECTURE.md
- 完善 rustdoc

---

### 8. 测试覆盖不足
**现状**:
- 部分 crate 有单元测试
- 缺乏集成测试
- 缺乏端到端测试

**修复方案**:
- 添加集成测试
- 添加端到端测试（使用 testcontainers）
- 提高单元测试覆盖率到 80%

---

## ✅ 已修复问题（供参考）

| 问题 | 修复日期 | Commit |
|------|---------|--------|
| 路径遍历漏洞 | 2026-02-03 | 1434eb2 |
| 命令注入漏洞 | 2026-02-03 | 1c101bd |
| 配置系统接入 | 2026-02-03 | ad90c95 |
| 文件锁修复 | 2026-02-03 | - |
| 重复事件修复 | 2026-02-03 | - |
| 消息系统简化 | 2026-02-03 | 47a738d |
| 删除未使用 crates | 2026-02-03 | 3bf113a |

---

## 优先级建议

### 立即（本周）
1. unwrap/panic 移除
2. SSE 流解析修复

### 本周
3. 心跳机制实现
4. 重复排序优化

### 本月
5. 内存缓存限制
6. 错误处理改进
7. 文档完善
8. 测试覆盖

---

*文档维护: Bamboo Team*
