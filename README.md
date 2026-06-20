# DeepPrint Studio

English | [简体中文](./README.zh-CN.md)

DeepPrint Studio is a self-hosted web print center. It brings template rendering, printer management, the CUPS print pipeline, and external Open APIs into one browser-accessible service for orders, labels, receipts, PDF files, and similar workflows.

Users manage printing from the web console. The server connects to virtual PDF printers, LAN printers, or Linux USB printers through CUPS.

## Highlights

- Web console: print center, template management, printer management, print history, user management, and API key management.
- Template printing: render PDFs from Typst templates and JSON data, then submit them to a printer.
- Direct file printing: submit PDF and image files directly.
- CUPS integration: discover printers from the current CUPS instance, read printer capabilities, and submit print options based on those capabilities.
- CUPS-PDF debugging: validate the real IPP path and inspect output PDFs without a physical printer.
- Open API: external systems can call template, printer, preview, print, and job APIs with Bearer API keys.
- Local accounts: the web console uses session cookies and supports admins, operators, forced first-login password changes, and user management.
- Private deployment: SQLite by default, with Docker Compose for Web, Server, and CUPS.

## Roadmap

- [x] CUPS-based printer pipeline management
- [x] Typst modern typesetting engine integration
- [x] Standard external API
- [ ] AI-assisted workflow: generate and adjust print templates from natural-language prompts

## Screenshots

The screenshots below come from the default Docker Compose local setup and show the desktop console with the sidebar collapsed.

### Printer Management

<img src="./docs/images/readme-printers.png" alt="Printer management UI" width="900" />

### Print Center

<img src="./docs/images/readme-print-center.png" alt="Print center UI" width="900" />

### Template Management (Typst official playground example)

<img src="./docs/images/readme-templates.png" alt="Template management UI" width="900" />

## Use Cases

- Intranet order, label, and receipt print center.
- Connecting business-system printing to a unified CUPS backend.
- Generating printable PDFs from templates and JSON data.
- Validating the real print pipeline with CUPS-PDF before connecting physical printers.

## Quick Start

If you only want to run the whole system, you need:

- Docker
- Docker Compose

Option 1: build locally and start the full stack:

```bash
cp .env.example .env
docker compose up -d --build
```

Option 2: use prebuilt images without building locally:

```bash
cp .env.example .env
docker compose -f docker-compose.ghcr.yml pull
docker compose -f docker-compose.ghcr.yml up -d
```

`docker-compose.ghcr.yml` uses these images by default:

- `ghcr.io/boe1900/deepprint-server:edge`
- `ghcr.io/boe1900/deepprint-web:edge`

You can also pin a specific build:

```bash
DEEPPRINT_SERVER_IMAGE=ghcr.io/boe1900/deepprint-server:sha-<commit> \
DEEPPRINT_WEB_IMAGE=ghcr.io/boe1900/deepprint-web:sha-<commit> \
docker compose -f docker-compose.ghcr.yml up -d
```

Entrypoints:

- Web console: `http://localhost:8080`
- Server API: `http://localhost:17801`
- CUPS Admin: `http://localhost:631/admin`

Default CUPS credentials come from `.env.example`:

- Username: `print`
- Password: `print`

Before the first web-console login, set the initial admin password in `.env`:

```bash
DEEPPRINT_INITIAL_ADMIN_USERNAME=admin
DEEPPRINT_INITIAL_ADMIN_PASSWORD=change-me-min-8
```

If users already exist in the database, the server will not create the initial admin again. The initial admin must change the password after first login.

If `DEEPPRINT_INITIAL_ADMIN_PASSWORD` is not set, the login page shows the initialization guide instead of a form that cannot actually log in.

Most deployments only need to change the initial admin password and exposed ports. For more environment variables, see [Development, Deployment, and Operations](./docs/development-and-operations.md#7-环境变量).

In container deployments, DeepPrint-managed Typst resources are stored in the persistent `/data` volume by default:

- Package directory: `/data/typst/packages`
- Font directory: `/data/typst/fonts`
- Preview cache: `/data/cache/typst`

`/data/typst/fonts` is the only managed font directory. On startup, the service syncs several bundled free fonts into it. Business fonts should also be uploaded there.

The default `docker-compose.yml` uses a Docker named volume, `deepprint-data`, to persist `/data`. It no longer bind-mounts the container database directly to the host. This is closer to real deployments and avoids SQLite stability issues on macOS / Docker Desktop bind mounts.

## Pre-release Images

GitHub Actions can build and publish GHCR pre-release images:

- `ghcr.io/<owner>/deepprint-server:edge`
- `ghcr.io/<owner>/deepprint-web:edge`
- `ghcr.io/<owner>/deepprint-server:sha-<commit>`
- `ghcr.io/<owner>/deepprint-web:sha-<commit>`

The current policy only publishes pre-release tags, not `latest`. If you do not want to build locally, use `edge` or pin a `sha-...` tag.

The current GHCR pre-release images support both `linux/amd64` and `linux/arm64`, and have been verified on an ARM NAS with `hanxi/cups:latest` and a CUPS-PDF rehearsal.

Current verification focuses on Docker Compose, CUPS-PDF, Typst rendering, and the web console. Physical printer paths still need more real-device testing. For external use, prefer `edge` or a pinned `sha-...` tag.

## Validate the Print Pipeline with CUPS-PDF

The `hanxi/cups` image includes `printer-driver-cups-pdf`, so you can validate the full pipeline with a virtual printer.

Initialize CUPS-PDF:

```bash
bun run compose:setup-cups-pdf
```

This script creates the printer, fixes the CUPS-PDF output path, and makes the PDF output directory writable inside the container. That avoids false success on NAS / bind-mount setups where the print job succeeds but no PDF file is written.

Then import the `CUPS-PDF` printer from the web console and submit a print job.

Default output directory:

```text
./.deepprint-dev/cups-pdf/ANONYMOUS
```

This path really goes through:

1. DeepPrint renders the PDF.
2. DeepPrint submits it to CUPS through IPP.
3. CUPS routes it to `CUPS-PDF`.
4. `CUPS-PDF` writes the final PDF file.

## Open API

External systems authenticate with API keys:

```text
Authorization: Bearer dp_...
```

Common endpoints:

| Endpoint | Scope | Description |
| --- | --- | --- |
| `GET /v1/open/me` | Valid API key | Current API key metadata |
| `GET /v1/open/templates` | `template:read` | List templates |
| `GET /v1/open/printers` | `printer:read` | List printers |
| `GET /v1/open/printers/{printer_id}` | `printer:read` | Read printer capabilities |
| `POST /v1/open/preview` | `preview:create` | Generate a PDF preview |
| `POST /v1/open/print` | `print:create` | Template printing |
| `POST /v1/open/print/direct` | `print:create` | Multipart direct file printing |
| `GET /v1/open/jobs/{job_id}` | `job:read` | Query a job |
| `GET /v1/open/jobs/by-request-id/{request_id}` | `job:read` | Query a job by business request ID |

Direct file printing example:

```bash
curl -X POST "http://localhost:17801/v1/open/print/direct" \
  -H "Authorization: Bearer $DEEPPRINT_API_KEY" \
  -F "request_id=erp-file-1001" \
  -F "printer_id=printer-xxx" \
  -F 'print_options={"copies":1,"media":"iso_a4_210x297mm"}' \
  -F "file=@invoice.pdf;type=application/pdf"
```

For more API and parameter details, see [System Design](./docs/system-design.md).

## Local Development

Developer mode requires:

- Bun 1.x
- Rust stable
- Docker and Docker Compose

Install frontend dependencies:

```bash
bun install
```

Recommended development mode: run CUPS in Docker, and run Server and Web on the host:

```bash
bun run dev:local
```

Then start the server and web app separately:

```bash
bun run server:dev
```

```bash
bun run web:dev
```

Local development entrypoints:

- Web console: `http://localhost:3000`
- Server API: `http://localhost:17801`
- CUPS Admin: `http://localhost:631/admin`

Notes:

- `server:dev` reads the root `.env`, but forces the database, Typst resources, logs, and diagnostics directories into `./.deepprint-dev/`, so container-only `/data/...` paths do not leak into host development.
- `server:dev` ignores `DEEPPRINT_DATABASE_URL` from `.env`, so you can reuse the same `.env` used for Docker deployment.
- `web:dev` follows `DEEPPRINT_SERVER_PORT` or `DEEPPRINT_AGENT_PORT` for the local proxy target instead of hard-coding `17801`.

Common commands:

```bash
bun run web:build
bun run server:check
bun run server:test
```

For Open API / API key changes, also run:

```bash
cargo test --manifest-path apps/server/Cargo.toml open_ --quiet
cargo test --manifest-path apps/server/Cargo.toml api_key --quiet
```

## Documentation

- [System Design](./docs/system-design.md)
- [Development, Deployment, and Operations](./docs/development-and-operations.md)
- [Third-party Notices and Licenses](./THIRD_PARTY_NOTICES.md)

## Community and Support

- [Contributing Guide](./CONTRIBUTING.md)
- [Security Policy](./SECURITY.md)
- [Support](./SUPPORT.md)

## Project Structure

```text
.
├── apps/
│   ├── server/              # Rust server
│   └── web/                 # Vite + React + TanStack app
├── docker/                  # Container images and startup resources
├── docs/                    # System design and operations docs
├── scripts/                 # Load / soak / recovery / smoke scripts
├── docker-compose.yml
└── docker-compose.usb.yml
```

## Project Status

- SQLite is the default database. PostgreSQL is planned.
- The recommended print backend is external CUPS.
- The web console supports local users, session cookies, and user management.
- The Open API supports Bearer API keys and scope authorization.

## Acknowledgements

DeepPrint Studio uses the [hanxi/cups](https://github.com/hanxi/cups-web) image for the CUPS development environment. It turns home USB printers into web-accessible network print services and includes `printer-driver-cups-pdf`, which is useful for testing real CUPS / IPP print paths.

Template rendering is built on the [Typst](https://typst.app/) ecosystem. Typst is a modern open-source typesetting system for generating high-quality PDFs with concise markup.

This project recognizes and thanks the [LINUX DO](https://linux.do/) community. Feedback and discussion are welcome there.

For more third-party projects and license details, see [Third-party Notices and Licenses](./THIRD_PARTY_NOTICES.md).

## License

DeepPrint Studio is licensed under the [Apache License 2.0](./LICENSE).
