# DeepPrint Web

`apps/web` 是 DeepPrint Studio 的 Web 控制台，技术栈为 Vite + React + TanStack。

## 运行

```bash
bun run dev
```

默认开发地址：

- `http://localhost:3000`

## 构建

```bash
bun run build
```

## 测试

```bash
bun run test
```

当前前端以 `src/features/deepprint` 为业务主目录，围绕打印任务、模板、诊断与 CUPS 状态构建页面与数据流。

## 关键目录

```text
src/
├── features/deepprint/      # 业务页面、查询、API、状态逻辑
├── routes/                  # TanStack Router 路由入口
├── components/              # 通用 UI 组件
└── styles.css               # 全局样式
```
