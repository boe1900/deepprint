# Contributing

Thanks for your interest in DeepPrint Studio.

This project currently welcomes:

- Bug fixes
- Documentation improvements
- Test coverage
- Small, verifiable feature improvements

## Before You Start

Please read:

- [README.md](./README.md)
- [docs/development-and-operations.md](./docs/development-and-operations.md)
- [docs/system-design.md](./docs/system-design.md)

Development requirements:

- Bun 1.x
- Rust stable
- Docker and Docker Compose

Install dependencies:

```bash
bun install
```

Recommended local development flow:

```bash
bun run dev:local
bun run server:dev
bun run web:dev
```

## Checks Before Submitting

Before opening a PR, run at least:

```bash
bun run web:build
bun run web:test
bun run server:check
bun run server:test
```

If your change touches the Open API, API keys, print pipeline, or recovery logic, also follow the targeted validation commands in [docs/development-and-operations.md](./docs/development-and-operations.md).

## Contribution Guidelines

- Prefer small, clear PRs. Do not mix unrelated changes.
- Update docs, screenshots, or tests when behavior changes.
- Do not commit `.env`, databases, logs, caches, build artifacts, or local runtime directories.
- Do not commit real passwords, API keys, internal addresses, or other sensitive data.
- For UI changes, include screenshots or recordings when possible, and mention desktop and mobile behavior.

## Issues and PRs

When opening an issue or PR, include as much of this as possible:

- Goal or background
- Reproduction steps
- Expected behavior and actual behavior
- Environment: local development / Docker Compose / Linux USB printing
- Affected area: Web, Server, CUPS, Typst templates, fonts, package management, etc.

## Security Issues

If you found a security vulnerability, please do not open a public issue. Read [SECURITY.md](./SECURITY.md) first.
