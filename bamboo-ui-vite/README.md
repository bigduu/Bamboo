# Bamboo UI (Vite)

Bamboo 前端项目 - 使用 React + Vite 构建

## 迁移说明

本项目已从 Next.js 迁移到 React + Vite，保留了所有原有功能。

## 项目结构

```
bamboo-ui-vite/
├── src/
│   ├── components/     # UI 组件
│   │   ├── ui/        # shadcn/ui 组件
│   │   ├── chat/      # 聊天组件
│   │   ├── layout/    # 布局组件
│   │   ├── settings/  # 设置组件
│   │   ├── prompts/   # 提示词组件
│   │   └── memories/  # 记忆组件
│   ├── pages/         # 页面组件
│   │   ├── settings/  # 设置页面
│   │   ├── HomePage.tsx
│   │   └── StatsPage.tsx
│   ├── stores/        # Zustand 状态管理
│   ├── hooks/         # 自定义 Hooks
│   ├── lib/           # 工具函数和 API
│   ├── types/         # TypeScript 类型
│   └── test-utils/    # 测试工具
├── public/            # 静态资源
├── index.html         # HTML 入口
├── vite.config.ts     # Vite 配置
├── tsconfig.json      # TypeScript 配置
└── package.json       # 依赖配置
```

## 路由配置

| 路由 | 页面 | 说明 |
|------|------|------|
| `/` | HomePage | 首页（Tool Call Console） |
| `/stats` | StatsPage | 统计页面 |
| `/settings` | SettingsOverviewPage | 设置首页 |
| `/settings/server` | ServerSettingsPage | 服务器配置 |
| `/settings/backend` | BackendSettingsPage | 后端配置 |
| `/settings/prompts` | PromptsSettingsPage | 提示词管理 |
| `/settings/memories` | MemoriesSettingsPage | 记忆管理 |
| `/settings/masking` | MaskingSettingsPage | Masking 配置 |

## 环境变量

创建 `.env.local` 文件：

```env
VITE_API_URL=http://localhost:8081
VITE_WS_URL=ws://localhost:18790
```

注意：Vite 只暴露以 `VITE_` 开头的环境变量到客户端。

## 开发

```bash
# 安装依赖
npm install

# 启动开发服务器（端口 3000）
npm run dev

# 构建生产版本
npm run build

# 预览生产构建
npm run preview

# 运行测试
npm run test
```

## 主要变更

### 从 Next.js 迁移的改动

1. **路由**：使用 `react-router-dom` 替代 Next.js App Router
2. **环境变量**：`process.env.NEXT_PUBLIC_*` → `import.meta.env.VITE_*`
3. **图片**：`next/image` → 标准 `<img>` 标签
4. **链接**：`next/link` → `react-router-dom` 的 `Link`
5. **头部**：`next/head` → 标准 HTML `<head>`
6. **字体**：移除 next/font，使用系统字体

### 保留的功能

- ✅ shadcn/ui 组件
- ✅ Tailwind CSS 样式
- ✅ Zustand 状态管理
- ✅ 主题切换（深色/浅色模式）
- ✅ 所有设置页面
- ✅ API 客户端

## 依赖

- React 19
- React Router DOM 7
- Vite 7
- TypeScript 5
- Tailwind CSS 4
- shadcn/ui 组件库
- Zustand 状态管理
- Axios HTTP 客户端
