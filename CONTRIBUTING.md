# Contributing

感谢你关注 DeepPrint Studio。

这个项目目前优先接受以下类型的贡献：

- Bug 修复
- 文档改进
- 测试补充
- 小范围、可验证的功能增强

## 开始之前

请先阅读这些文档：

- [README.md](./README.md)
- [docs/development-and-operations.md](./docs/development-and-operations.md)
- [docs/system-design.md](./docs/system-design.md)

开发环境要求：

- Bun 1.x
- Rust stable
- Docker 与 Docker Compose

安装依赖：

```bash
bun install
```

推荐本地开发方式：

```bash
bun run dev:local
bun run server:dev
bun run web:dev
```

## 提交前检查

提交 PR 前请至少运行：

```bash
bun run web:build
bun run web:test
bun run server:check
bun run server:test
```

如果改动触及 Open API、API Key、打印链路或恢复逻辑，请再参考 [docs/development-and-operations.md](./docs/development-and-operations.md) 里的专项验证命令。

## 贡献约定

- 优先提交小而清晰的 PR，避免一次混入不相关改动。
- 行为变更请同步更新文档、截图或测试。
- 不要提交 `.env`、数据库、日志、缓存、构建产物或本地运行目录内容。
- 不要把真实密码、API Key、内部地址或其他敏感信息写进仓库。
- 如果改动涉及 UI，请尽量附上截图或录屏，说明桌面端和移动端表现。

## Issue 与 PR 建议

提交 Issue 或 PR 时，尽量带上这些信息：

- 改动目的或问题背景
- 复现步骤
- 预期行为与实际行为
- 运行环境：本地开发 / Docker Compose / Linux USB 打印
- 涉及的模块：Web、Server、CUPS、Typst 模板、字体、包管理等

## 安全问题

如果你发现的是安全漏洞，请不要公开提交 Issue。请先阅读 [SECURITY.md](./SECURITY.md)。
