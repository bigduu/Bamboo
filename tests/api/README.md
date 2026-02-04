# Bamboo API 工具和技能测试

本目录包含 Bamboo API 的工具和技能系统测试脚本。

## 测试脚本

### test_tools.py

Python 测试脚本，包含以下测试场景：

#### Mock 测试（无需服务器）
- **工具发现**: 获取可用工具列表
- **单工具调用**: 计算器、文件读取、文本处理、时间获取
- **多工具链式调用**: 工具组合使用
- **工具错误处理**: 工具不存在、参数错误等
- **技能加载**: 技能发现、获取、列表
- **技能热重载**: 技能重载验证
- **执行结果验证**: 结果结构、成功/失败状态

#### 真实 API 测试（需要服务器）
- **健康检查**: `/api/v1/health`
- **聊天端点**: `/api/v1/chat`

## 运行方式

### 使用 shell 脚本（推荐）

```bash
# 运行 Mock 测试（默认）
./scripts/test_tools.sh

# 运行演示
./scripts/test_tools.sh --demo

# 运行所有测试（Mock + 真实 API）
./scripts/test_tools.sh --all

# 只运行真实 API 测试
./scripts/test_tools.sh --real-only

# 指定 API 地址
./scripts/test_tools.sh --all --url http://localhost:8081

# 显示帮助
./scripts/test_tools.sh --help
```

### 直接使用 Python

```bash
# 运行 Mock 测试
cd ~/workspace/bamboo
python3 tests/api/test_tools.py --mock-only

# 运行真实 API 测试
python3 tests/api/test_tools.py --real-only

# 运行演示
python3 tests/api/test_tools.py --demo

# 指定 API 地址
python3 tests/api/test_tools.py --all --url http://localhost:8081
```

## 环境变量

- `BAMBOO_API_URL`: API 基础 URL（默认: `http://localhost:8080`）

## 测试覆盖场景

### 1. 工具发现
- 列出所有可用工具
- 验证工具定义结构

### 2. 单工具调用
- 计算器工具（数学运算）
- 文件读取工具（Mock 文件内容）
- 文本处理工具（统计、格式化）
- 时间工具（获取当前时间）

### 3. 多工具链式调用
- 读取文件后处理文本
- 计算后格式化结果
- 错误恢复继续执行

### 4. 工具错误处理
- 工具不存在
- 缺少必需参数
- 无效参数类型
- 执行超时模拟

### 5. 技能加载
- 列出所有技能
- 获取特定技能
- 技能包含的工具
- 获取所有工具

### 6. 技能热重载
- 重新加载存在的技能
- 处理不存在的技能
- 重载后技能保持

### 7. 执行结果验证
- 结果结构完整性
- 成功结果无错误
- 失败结果有错误
- 调用历史追踪

## Mock 工具说明

测试脚本包含以下 Mock 工具，无需外部依赖：

| 工具名 | 描述 | 参数 |
|--------|------|------|
| `calculator` | 数学计算 | `expression`: 数学表达式 |
| `read_file` | 读取文件 | `path`: 文件路径, `limit`: 行数限制 |
| `text_processor` | 文本处理 | `text`: 输入文本, `operation`: 操作类型 |
| `get_time` | 获取时间 | `format`: 时间格式 |

## Mock 技能说明

| 技能名 | 描述 | 工具 |
|--------|------|------|
| `math` | 数学计算技能 | calculator |
| `file_ops` | 文件操作技能 | read_file |
| `text` | 文本处理技能 | text_processor |
| `utils` | 通用工具技能 | get_time, text_processor |

## 依赖

- Python 3.7+
- requests（可选，用于真实 API 测试）

## 注意事项

1. Mock 测试不依赖任何外部服务，可以独立运行
2. 真实 API 测试需要 Bamboo 服务器正在运行
3. 如果服务器未运行，真实 API 测试会被跳过
4. 所有 Mock 工具都是内存实现，不会产生副作用
