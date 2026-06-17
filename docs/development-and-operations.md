# DeepPrint Studio 开发、部署与运维

本文档是 DeepPrint Studio 的操作手册，合并了本地开发、容器部署、CUPS-PDF 调试、验证门禁和发布 Runbook。

## 1. 两条启动路径

本文档把两类场景分开：

- 开发人员：改代码、联调前后端，推荐宿主机跑 `server` / `web`，只把 CUPS 放进 Docker
- 使用人员：不改代码，只想本地构建并启动完整系统，直接走 `docker compose up -d --build`

依赖要求：

- 开发人员：Bun 1.x、Rust stable、Docker 与 Docker Compose
- 使用人员：Docker 与 Docker Compose

开发前先安装依赖：

```bash
bun install
```

## 2. 本地开发

推荐开发模式是只让 CUPS 跑在 Docker 里，`server` 与 `web` 直接跑在宿主机。这样 Rust 和前端改动不需要反复重建镜像。

```bash
bun run dev:local
```

然后分别在两个终端启动：

```bash
bun run server:dev
```

```bash
bun run web:dev
```

本地开发入口：

- Web：`http://localhost:3000`
- Server：`http://localhost:17801`
- CUPS：`http://localhost:631`

说明：

- `bun run dev:local` 会停掉 compose 里的 `server`/`web`，避免端口冲突，并启动 `cups`
- `bun run server:dev` 默认使用 `./.deepprint-dev/deepprint.db`
- `bun run server:dev` 会读取根目录 `.env`，但会把数据库、Typst 包、字体、缓存、日志和诊断目录固定到 `./.deepprint-dev/`
- `bun run server:dev` 会忽略 `.env` 里的 `DEEPPRINT_DATABASE_URL`，避免把 Docker 用的 `/data/...` 路径带到宿主机
- `bun run web:dev` 会自动跟随 `DEEPPRINT_SERVER_PORT` 或 `DEEPPRINT_AGENT_PORT` 调整代理目标
- `DEEPPRINT_CUPS_BASE_URL` 现在主要是首次启动默认值和兜底值
- 如果后台设置里保存过 CUPS 地址，数据库中的值优先于环境变量
- 如果 `DEEPPRINT_INITIAL_ADMIN_PASSWORD` 为空，登录页会显示初始化引导，而不是可登录表单

也可以单独启动：

```bash
bun run server:dev
bun run web:dev
```

如果希望本机 server 连接 Docker CUPS 并共用开发数据库：

```bash
DEEPPRINT_CUPS_BASE_URL=http://localhost:631/ \
bun run server:dev:raw
```

不建议直接裸跑 `cargo run` 或 `vite dev`，因为那样会绕过本地开发用的路径隔离和代理保护。

## 3. 容器部署

完整容器栈包括：

- `hanxi/cups:latest`：CUPS，端口 `631`
- `deepprint-server`：Rust API，端口 `17801`
- `deepprint-web`：React 控制台，端口 `8080`
- SQLite：持久化到 Docker 命名卷 `deepprint-data`

启动：

```bash
cp .env.example .env
docker compose up -d --build
```

如果不想在本地构建镜像，也可以直接使用仓库发布到 GHCR 的预构建镜像：

```bash
cp .env.example .env
docker compose -f docker-compose.ghcr.yml pull
docker compose -f docker-compose.ghcr.yml up -d
```

`docker-compose.ghcr.yml` 默认使用：

- `ghcr.io/boe1900/deepprint-server:edge`
- `ghcr.io/boe1900/deepprint-web:edge`

如果需要固定到某次构建产物，可以覆盖：

```bash
DEEPPRINT_SERVER_IMAGE=ghcr.io/boe1900/deepprint-server:sha-<commit> \
DEEPPRINT_WEB_IMAGE=ghcr.io/boe1900/deepprint-web:sha-<commit> \
docker compose -f docker-compose.ghcr.yml up -d
```

入口：

- Web：`http://localhost:8080`
- Server：`http://localhost:17801`
- CUPS Admin：`http://localhost:631/admin`

默认 CUPS 账号来自 `.env`：

- 用户：`print`
- 密码：`print`

当前服务端只支持 SQLite。`DEEPPRINT_DATABASE_URL` 必须是 `sqlite://...`，如果填成 `postgres://...` 会启动失败；PostgreSQL 是后续规划，不是隐藏可用能力。

首个 Web 管理员可通过环境变量初始化：

```bash
DEEPPRINT_INITIAL_ADMIN_USERNAME=admin
DEEPPRINT_INITIAL_ADMIN_PASSWORD='change-me-before-first-login'
```

如果数据库中已经有用户，启动时不会重复创建。初始管理员第一次登录后必须修改密码。

如果 `DEEPPRINT_INITIAL_ADMIN_PASSWORD` 留空，Web 登录页会直接显示初始化引导；这通常说明 `.env` 还没配置好，或服务没有用更新后的 `.env` 重启。

说明：

- 现在不需要先在宿主机执行 `bun run web:build`，前端构建已经放进 `docker/web.Dockerfile`
- 第一次 `docker compose up -d --build` 可能明显偏慢，因为 `server` 镜像会首次编译 Rust / Typst 依赖
- 之后再次构建通常会快很多，`docker/server.Dockerfile` 已开启 BuildKit cache mount
- compose 现在使用 Docker 命名卷 `deepprint-data` 持久化 `/data`，不再把 SQLite 数据库直接 bind mount 到宿主机目录；这能避免 macOS / Docker Desktop 下 SQLite 在 bind mount 上的稳定性问题

## 4. CUPS-PDF 调试

`hanxi/cups` 预装 `printer-driver-cups-pdf`，可以用虚拟 PDF 打印机验证真实 IPP 提交流程。

初始化共享的 `CUPS-PDF` 打印机：

```bash
bun run compose:setup-cups-pdf
```

脚本默认假设 CUPS 容器名是 `deepprint-cups-1`，需要时可以覆盖：

```bash
DEEPPRINT_CUPS_CONTAINER_NAME=my-cups-container \
bun run compose:setup-cups-pdf
```

生成的 PDF 通常在 CUPS 容器内：

```text
/var/spool/cups-pdf/ANONYMOUS
```

本仓库 compose 会把它挂到宿主机：

```text
./.deepprint-dev/cups-pdf/ANONYMOUS
```

也可以直接进容器查看：

```bash
docker exec deepprint-cups-1 ls -lah /var/spool/cups-pdf/ANONYMOUS
```

Linux USB 打印机场景使用：

```bash
bun run compose:up:usb
```

macOS 和 Windows Docker Desktop 的 USB 直通依赖宿主机能力，通常需要额外共享策略。

## 5. 验证门禁

常规门禁：

```bash
bun run web:build
bun run web:test
bun run server:check
bun run server:test
```

如果改动触及 Open API 或 API Key 权限：

```bash
cargo test --manifest-path apps/server/Cargo.toml open_ --quiet
cargo test --manifest-path apps/server/Cargo.toml api_key --quiet
```

如果改动触及打印链路、任务状态机或恢复逻辑，再补：

```bash
bun run loadtest
bun run soaktest
DEEPPRINT_RECOVERY_RENDER_ENGINE=text bun run recovery:test
```

前端现在已经补了基础回归测试，建议和上面的构建一起执行。

## 6. 运维脚本

### Compose Smoke

```bash
DEEPPRINT_SMOKE_ADMIN_PASSWORD='example-bootstrap-password-123!' \
DEEPPRINT_SMOKE_ADMIN_NEW_PASSWORD='example-rotated-password-123!' \
DEEPPRINT_SMOKE_PRINTER_NAME='CUPS-PDF' \
bun run compose:smoke
```

Smoke 会验证：

- Web 代理和 Server 健康检查
- 管理员登录与首登改密
- 已认证的 `/v1/health/deep`
- CUPS 发现与打印机注册
- 模板分组和模板创建
- Open API Key 创建与撤销
- Open 预览 PDF 渲染
- Open 打印任务创建和轮询到 `succeeded`

如果没有发现匹配 `DEEPPRINT_SMOKE_PRINTER_NAME` 的 CUPS 打印机，smoke 会失败，不会回退到 mock printer。

### Load / Soak / Recovery

```bash
bun run loadtest
bun run soaktest
DEEPPRINT_RECOVERY_RENDER_ENGINE=text bun run recovery:test
```

## 7. 环境变量

普通部署建议只关注 `.env.example` 里未注释的变量：

| 变量 | 默认值 | 是否建议修改 | 说明 |
| --- | --- | --- | --- |
| `DEEPPRINT_INITIAL_ADMIN_USERNAME` | `admin` | 可选 | 首个 Web 管理员用户名。仅数据库无用户时生效 |
| `DEEPPRINT_INITIAL_ADMIN_PASSWORD` | 空 | 必改 | 首个 Web 管理员密码，至少 8 位。数据库已有用户后不会再次生效 |
| `DEEPPRINT_INITIAL_ADMIN_EMAIL` | 空 | 可选 | 首个管理员邮箱 |
| `DEEPPRINT_INITIAL_ADMIN_DISPLAY_NAME` | 用户名 | 可选 | 首个管理员显示名 |
| `CUPSADMIN` | `print` | 可选 | CUPS Admin 登录用户名，由 `hanxi/cups` 容器使用 |
| `CUPSPASSWORD` | `print` | 建议生产修改 | CUPS Admin 登录密码，由 `hanxi/cups` 容器使用 |
| `DEEPPRINT_CUPS_PORT` | `631` | 端口冲突时修改 | 宿主机暴露的 CUPS 端口 |
| `DEEPPRINT_SERVER_PORT` | `17801` | 端口冲突时修改 | 宿主机暴露的 Server API 端口 |
| `DEEPPRINT_WEB_PORT` | `8080` | 端口冲突时修改 | 宿主机暴露的 Web 控制台端口 |
| `DEEPPRINT_DATABASE_URL` | `sqlite:///data/deepprint.db` | 通常不改 | 当前仅支持 `sqlite://...` |
| `DEEPPRINT_RENDER_ENGINE` | `typst` | 通常不改 | 模板打印渲染引擎。`typst` 是正式 PDF 渲染；`text` 仅用于调试/测试 |

Compose 内部固定传入的变量：

| 变量 | 值 | 说明 |
| --- | --- | --- |
| `DEEPPRINT_AGENT_BIND` | `0.0.0.0` | 让 server 在容器内监听所有网卡，配合端口映射使用 |
| `DEEPPRINT_AGENT_PORT` | `17801` | server 容器内监听端口 |
| `DEEPPRINT_CUPS_BASE_URL` | `http://cups:631/` | server 容器访问 compose 内 CUPS 服务的地址。Web 后台保存的 CUPS 地址会优先生效 |
| `DEEPPRINT_TYPST_LOCAL_PACKAGES_ROOT` | `/data/typst/packages` | DeepPrint 管理的 Typst 包目录，挂在持久化 `/data` 卷下 |
| `DEEPPRINT_TYPST_PREVIEW_CACHE_ROOT` | `/data/cache/typst` | Typst 包与预览缓存目录，可清理，可按需持久化 |
| `DEEPPRINT_TYPST_FONTS_ROOT` | `/data/typst/fonts` | DeepPrint 唯一受管字体目录，挂在持久化 `/data` 卷下 |

高级调试变量：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `DEEPPRINT_AGENT_MOCK` | `false` | 使用内置 mock 打印后端。仅测试使用，真实 CUPS 链路不要开启 |
| `DEEPPRINT_RENDER_TIMEOUT_SEC` | `30` | 模板打印子进程渲染超时，最小 5 秒 |
| `DEEPPRINT_DIRECT_JOB_MAX_BYTES` | `26214400` | 文件直打最大请求体，默认 25 MiB |
| `DEEPPRINT_AUTH_SESSION_COOKIE_NAME` | `deepprint_session` | Web 登录 Cookie 名称 |
| `DEEPPRINT_AUTH_SESSION_TTL_SEC` | `604800` | Web 登录 Session 有效期，默认 7 天，最小 300 秒 |
| `DEEPPRINT_AUTH_COOKIE_SECURE` | `false` | HTTPS 部署时可设为 `true`，要求浏览器只通过 HTTPS 发送 Cookie |

目录约定建议：

- Docker / Compose 部署时，显式把 Typst 包和字体目录放到持久化卷里，避免依赖容器内部自动目录。
- 推荐直接沿用 compose 默认值：包目录 `/data/typst/packages`，字体目录 `/data/typst/fonts`，预览缓存 `/data/cache/typst`。
- 包目录和字体目录建议纳入备份；预览缓存不属于核心数据，丢失后会自动重建。
- 如果不是 Docker 部署，服务端会在未配置环境变量时退回到操作系统数据目录下的自动路径，并在启动时自动创建目录。
- 字体现在只从 `DEEPPRINT_TYPST_FONTS_ROOT` 读取。Docker 镜像会携带项目内 `apps/server/assets/fonts/` 的默认字体资源，服务首次启动时将它们初始化到这个目录，后续所有字体都在这里统一管理，不再单独扫描宿主机系统字体目录。
- 宿主机开发不要直接复用 `DEEPPRINT_DATABASE_URL=sqlite:///data/...` 这一类容器路径；推荐始终通过 `bun run server:dev` 启动本地服务。

## 8. 兼容性与发布

当前验证状态：

| 维度 | 状态 | 说明 |
| --- | --- | --- |
| macOS 开发环境 | 已验证基础链路 | Web、Rust server、external CUPS 容器 |
| Linux | 代码可运行，待重验 | external CUPS 更适合 Linux 容器部署 |
| SQLite | 已实现 | 当前默认持久化方案 |
| PostgreSQL | 规划中 | 尚未落地 |
| external CUPS | 已验证主线 | 当前推荐模式 |

发布前建议：

```bash
bun run web:build
bun run web:test
bun run server:check
bun run server:test
docker compose build
docker compose up -d
```

发布后至少确认：

1. Web 控制台可访问
2. `GET /v1/health` 正常
3. 已登录状态下 `GET /v1/health/deep` 正常
4. 可读取打印机列表
5. 可创建一个打印任务
6. 诊断导出正常

当前仓库已经补了 GHCR 预发布镜像工作流：

- `main` 分支会构建并发布 `ghcr.io/<owner>/deepprint-server:edge`
- `main` 分支会构建并发布 `ghcr.io/<owner>/deepprint-web:edge`
- 每次发布同时附带不可变的 `sha-<commit>` 标签
- 当前不会发布 `latest`

这样可以先让外部用户验证容器部署链路，同时明确区分“可试用的预发布镜像”和“经过真实打印机场景验证后的正式版镜像”。如果只是想部署试用而不想本地构建，优先使用 `docker-compose.ghcr.yml`。
