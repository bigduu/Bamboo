# Bamboo UI 测试文档

## 测试结构

```
bamboo-ui/
├── src/
│   ├── stores/__tests__/        # Store 单元测试
│   │   ├── sessionStore.test.ts
│   │   ├── chatStore.test.ts
│   │   ├── configStore.test.ts
│   │   └── themeStore.test.ts
│   ├── lib/__tests__/           # 工具函数测试
│   │   ├── api.test.ts
│   │   └── utils.test.ts
│   ├── components/
│   │   ├── chat/__tests__/      # 聊天组件测试
│   │   │   ├── MessageBubble.test.tsx
│   │   │   └── InputArea.test.tsx
│   │   ├── session/__tests__/   # 会话组件测试
│   │   │   └── SessionList.test.tsx
│   │   └── settings/__tests__/  # 设置组件测试
│   │       └── ServerConfigPanel.test.tsx
│   ├── __tests__/               # 集成测试
│   │   ├── chat-flow.test.tsx
│   │   ├── settings-flow.test.tsx
│   │   └── theme-switching.test.tsx
│   └── test-utils/              # 测试工具
│       ├── fixtures.ts
│       ├── mock-store.ts
│       ├── setup.ts
│       └── index.ts
├── e2e/                         # E2E 测试
│   ├── chat.spec.ts
│   ├── settings.spec.ts
│   └── masking.spec.ts
├── vitest.config.ts             # Vitest 配置
└── playwright.config.ts         # Playwright 配置
```

## 安装依赖

```bash
npm install
```

## 运行测试

### 单元测试和组件测试 (Vitest)

```bash
# 运行所有测试
npm run test

# 监视模式 (开发时使用)
npm run test:watch

# 生成覆盖率报告
npm run test:coverage
```

### E2E 测试 (Playwright)

```bash
# 运行所有 E2E 测试
npm run test:e2e

# 使用 UI 模式运行
npm run test:e2e:ui

# 调试模式
npm run test:e2e:debug

# 安装 Playwright 浏览器
npx playwright install
```

## 覆盖率要求

- **组件覆盖率**: > 80%
- **Store 覆盖率**: > 90%
- **关键流程 E2E 覆盖**: 完整覆盖

## 测试文件说明

### 单元测试

1. **sessionStore.test.ts** - 会话状态管理测试
   - 创建会话
   - 切换会话
   - 删除会话
   - 更新会话

2. **chatStore.test.ts** - 对话状态管理测试
   - 添加消息
   - 更新消息
   - 流式消息处理
   - 会话清理

3. **configStore.test.ts** - 配置状态管理测试
   - API URL 设置
   - WebSocket URL 设置
   - 模型配置
   - 连接测试

4. **themeStore.test.ts** - 主题状态管理测试
   - 主题切换
   - 系统主题检测
   - 持久化

5. **api.test.ts** - API 客户端测试
   - 错误处理
   - HTTP 请求方法
   - Masking API
   - Backend Config API

6. **utils.test.ts** - 工具函数测试
   - className 合并

### 组件测试

1. **MessageBubble.test.tsx** - 消息气泡组件
2. **InputArea.test.tsx** - 输入区域组件
3. **SessionList.test.tsx** - 会话列表组件
4. **ServerConfigPanel.test.tsx** - 服务器配置面板

### 集成测试

1. **chat-flow.test.tsx** - 完整对话流程
2. **settings-flow.test.tsx** - 设置页面流程
3. **theme-switching.test.tsx** - 主题切换流程

### E2E 测试

1. **chat.spec.ts** - 对话功能 E2E 测试
2. **settings.spec.ts** - 设置功能 E2E 测试
3. **masking.spec.ts** - Masking 配置 E2E 测试

## 编写新测试

### 单元测试示例

```typescript
import { describe, it, expect } from "vitest";
import { useStore } from "@/stores/myStore";

describe("myStore", () => {
  it("should do something", () => {
    const store = useStore.getState();
    // 测试代码
  });
});
```

### 组件测试示例

```typescript
import { describe, it, expect } from "vitest";
import { render, screen } from "@/test-utils";
import { MyComponent } from "../MyComponent";

describe("MyComponent", () => {
  it("should render correctly", () => {
    render(<MyComponent />);
    expect(screen.getByText("Expected Text")).toBeInTheDocument();
  });
});
```

### E2E 测试示例

```typescript
import { test, expect } from "@playwright/test";

test("should work", async ({ page }) => {
  await page.goto("http://localhost:3000");
  await expect(page.locator("h1")).toContainText("Expected");
});
```

## 注意事项

1. 测试文件应该放在 `__tests__` 目录中，或者使用 `.test.ts(x)` 后缀
2. 使用 `@/test-utils` 中的工具函数进行组件测试
3. E2E 测试需要开发服务器运行在 `http://localhost:3000`
4. 覆盖率报告生成在 `coverage/` 目录
