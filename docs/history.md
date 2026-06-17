# DeepPrint Studio 历史记录

本文档压缩保留已经完成或过期的规划/交接信息。当前使用说明请看：

- [系统设计](./system-design.md)
- [开发、部署与运维](./development-and-operations.md)

## 1. Server-Only 改造

DeepPrint 已经从桌面端/Tauri 主线收敛到 server-only 主线：

- 客户端只需要浏览器
- Web 控制台位于 `apps/web`
- Rust 服务端位于 `apps/server`
- CUPS 作为 external CUPS 容器运行
- managed gateway runtime 和 `/v1/gateway/*` 不再是当前 API surface

当前标准组件：

- `deepprint-web`
- `deepprint-server`
- `cups`
- SQLite 数据库文件

PostgreSQL 仍是后续规划，不是当前可用能力。

## 2. Web App 迁移

Web 主线已经迁入 `apps/web`，技术栈为：

- Vite
- React
- TypeScript
- TanStack Router
- TanStack Query
- shadcn/ui
- Tailwind CSS
- CodeMirror / PDF.js 等业务组件

早期规划中的 gateway 页面和 `/v1/gateway/*` 调用已经不再作为主线存在。

## 3. Storage Refactor

服务端存储边界已经从早期的 `print_server.rs` 大文件逐步拆出：

- `apps/server/src/storage/mod.rs`
- `apps/server/src/storage/sqlite.rs`
- `apps/server/src/storage/sqlite/jobs/**`
- `apps/server/src/storage/sqlite/auth/**`
- `apps/server/src/storage/sqlite/printers.rs`
- `apps/server/src/storage/sqlite/render_cache.rs`

Jobs 相关读写、状态流转、事件、失败处理和 worker claim 已迁移到 storage/sqlite 子模块。

保留的现实边界：

- 测试仍会直接准备 SQLite fixture，这是集成测试的一部分
- `bootstrap.rs` 仍集中管理 schema、兼容迁移和初始模板 seed
- 继续拆分应以真实领域边界为准，不按文件行数机械拆

## 4. 当前验证口径

当前常用门禁：

```bash
bun run web:build
bun run server:check
bun run server:test
```

Open API / API Key 相关改动额外执行：

```bash
cargo test --manifest-path apps/server/Cargo.toml open_ --quiet
cargo test --manifest-path apps/server/Cargo.toml api_key --quiet
```

打印链路、任务状态机或恢复逻辑相关改动额外执行：

```bash
bun run loadtest
bun run soaktest
DEEPPRINT_RECOVERY_RENDER_ENGINE=text bun run recovery:test
```
