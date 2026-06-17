#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source ./.env
  set +a
fi

server_port="${DEEPPRINT_AGENT_PORT:-${DEEPPRINT_SERVER_PORT:-17801}}"
cups_port="${DEEPPRINT_CUPS_PORT:-631}"

echo "[dev:local] stopping compose server/web if they are running"
docker compose stop server web >/dev/null 2>&1 || true

echo "[dev:local] starting CUPS container"
docker compose up -d cups

cat <<EOF

[dev:local] local development is ready

Open two terminals:

1. Start the Rust server
   bun run server:dev

2. Start the web dev server
   bun run web:dev

URLs:
- Web:    http://localhost:3000
- Server: http://localhost:${server_port}
- CUPS:   http://localhost:${cups_port}

Notes:
- CUPS can stay in Docker, but web/server do not need Docker during development.
- If a local .env exists, dev:local now loads it before printing these commands.
- server:dev reads the root .env, but pins database, Typst, log, and diagnostics paths to ./.deepprint-dev/.
- server:dev ignores DEEPPRINT_DATABASE_URL, so a copied Docker .env will not break host startup.
- web:dev follows DEEPPRINT_SERVER_PORT / DEEPPRINT_AGENT_PORT for its API proxy target.
- DEEPPRINT_CUPS_BASE_URL now acts as a first-run default and fallback only.
- If the server has already saved a CUPS address in the database, the saved value takes precedence.
- If DEEPPRINT_INITIAL_ADMIN_PASSWORD is empty, the login page will show bootstrap guidance instead of a usable login form.

EOF
