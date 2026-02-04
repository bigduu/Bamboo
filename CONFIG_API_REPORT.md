# Bamboo 配置管理 HTTP API 实现报告

## 完成情况

已成功为 Bamboo 后端实现完整的配置管理 REST API。

## API 端点列表

| 方法 | 端点 | 描述 | 认证要求 |
|------|------|------|----------|
| GET | `/api/v1/config` | 获取完整配置（敏感信息脱敏） | 可选 |
| POST | `/api/v1/config` | 更新完整配置 | 需要 admin_token |
| GET | `/api/v1/config/{section}` | 获取特定章节配置 | 可选 |
| POST | `/api/v1/config/{section}` | 更新特定章节配置 | 需要 admin_token |
| POST | `/api/v1/config/reload` | 重新加载配置文件 | 需要 admin_token |
| GET | `/api/v1/config/schema` | 获取配置 schema（用于前端表单生成） | 无 |

支持的配置章节：`server`, `gateway`, `llm`, `skills`, `agent`, `storage`, `logging`, `version`

## 请求/响应示例

### 1. 获取完整配置
```bash
GET /api/v1/config
```

响应：
```json
{
  "version": "0.1.0",
  "server": {
    "port": 8081,
    "host": "127.0.0.1",
    "cors": true,
    "admin_token": "***MASKED***"
  },
  "gateway": {
    "enabled": true,
    "bind": "127.0.0.1:18790",
    "auth_token": "***MASKED***",
    "max_connections": 1000,
    "heartbeat_interval_secs": 30
  },
  "llm": {
    "default_provider": "copilot",
    "providers": {
      "copilot": {
        "enabled": true,
        "base_url": "https://api.githubcopilot.com",
        "model": "copilot-chat",
        "auth_type": "device_code",
        "auth_configured": true
      }
    }
  },
  "skills": {
    "enabled": true,
    "auto_reload": true,
    "directories": ["~/.bamboo/skills"]
  },
  "agent": {
    "max_rounds": 10,
    "timeout_seconds": 300
  },
  "storage": {
    "type": "Jsonl",
    "path": "~/.bamboo/sessions"
  },
  "logging": {
    "level": "Info",
    "file": "~/.bamboo/logs/bamboo.log",
    "max_size_mb": 100,
    "max_files": 5
  }
}
```

### 2. 更新特定章节
```bash
POST /api/v1/config/server
Content-Type: application/json
Authorization: Bearer your-admin-token

{
  "port": 8082,
  "host": "0.0.0.0",
  "cors": true
}
```

响应：
```json
{
  "success": true,
  "message": "Section 'server' updated successfully",
  "sections": ["server"]
}
```

### 3. 重新加载配置
```bash
POST /api/v1/config/reload
Authorization: Bearer your-admin-token
```

响应：
```json
{
  "success": true,
  "message": "Configuration reloaded successfully",
  "timestamp": "2025-01-01T00:00:00+00:00"
}
```

### 4. 获取配置 Schema
```bash
GET /api/v1/config/schema
```

响应包含完整的 JSON Schema，用于前端表单生成。

## 验证规则说明

### 端口验证
- 范围：1-65535
- 错误时返回：400 Bad Request

### 主机名验证
- 支持 IP 地址（IPv4/IPv6）
- 支持 `localhost`
- 支持标准主机名（符合 RFC 1123）
- 错误时返回：400 Bad Request

### 绑定地址验证
- 格式：`host:port` 或 `[IPv6]:port`
- 端口范围：1-65535
- 错误时返回：400 Bad Request

### 路径验证
- 不能为空
- 不能包含空字符（\0）
- 支持 `~` 展开为用户主目录
- 错误时返回：400 Bad Request

### 枚举值验证
- **日志级别**：`debug`, `info`, `warn`, `error`
- **存储类型**：`jsonl`, `sqlite`
- **认证类型**：`api_key`, `bearer`, `device_code`, `none`
- 错误时返回：400 Bad Request

## 敏感信息处理

### 脱敏字段
以下字段在响应中会被脱敏显示为 `***MASKED***`：
- 包含 `token` 的字段
- 包含 `api_key` 或 `apikey` 的字段
- 包含 `secret` 的字段
- 包含 `password` 的字段
- 包含 `credential` 的字段
- 包含 `auth_token`、`access_token`、`refresh_token`、`bearer` 的字段

### 部分更新保护
更新配置时，如果敏感字段值为空或 `***MASKED***`，将保留原值不被覆盖。

## 认证机制

### 配置 admin_token
在 `~/.bamboo/config.json` 中配置：
```json
{
  "server": {
    "admin_token": "your-secure-token"
  }
}
```

### 认证方式
在请求头中提供：
```
Authorization: Bearer your-admin-token
```

或直接提供 token：
```
Authorization: your-admin-token
```

### 开发模式
如果未配置 `admin_token`，所有请求都允许访问（不强制认证）。

## 只读配置保护

以下字段为只读，尝试修改将返回 403 Forbidden：
- `version` - 版本号由系统自动管理

## 配置热重载

### 自动通知
配置更新后，系统会通过 EventBus 发布 `ConfigUpdated` 事件，通知其他组件配置已变更。

### 事件格式
```json
{
  "event_type": "config_updated",
  "sections": ["server", "gateway"]
}
```

### 监听配置变更
其他组件可以订阅 EventBus 接收配置变更通知：
```rust
let mut rx = state.event_bus.subscribe();
while let Ok(event) = rx.recv().await {
    match event {
        Event::ConfigUpdated { sections } => {
            // 处理配置变更
        }
        _ => {}
    }
}
```

## 文件修改清单

1. `crates/bamboo-config/src/config.rs` - 添加 `admin_token` 字段到 `ServerConfig`
2. `crates/bamboo-config/src/manager.rs` - 增强验证函数，添加端口、主机名、路径验证
3. `crates/bamboo-server/src/event_bus.rs` - 添加 `ConfigUpdated` 事件
4. `crates/bamboo-server/src/state.rs` - 添加 `notify_config_updated` 方法
5. `crates/bamboo-server/src/handlers/config.rs` - 完整的配置管理 API 实现
6. `crates/bamboo-server/src/server.rs` - 添加新的 API 路由

## 测试建议

1. **获取配置测试**：
   ```bash
   curl http://localhost:8081/api/v1/config
   ```

2. **更新配置测试**：
   ```bash
   curl -X POST http://localhost:8081/api/v1/config/server \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer your-token" \
     -d '{"port": 8082, "host": "0.0.0.0", "cors": true}'
   ```

3. **获取 Schema 测试**：
   ```bash
   curl http://localhost:8081/api/v1/config/schema
   ```

4. **重载配置测试**：
   ```bash
   curl -X POST http://localhost:8081/api/v1/config/reload \
     -H "Authorization: Bearer your-token"
   ```
