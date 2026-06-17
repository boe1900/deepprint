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

export DEEPPRINT_DEV_API_TARGET="${DEEPPRINT_DEV_API_TARGET:-http://127.0.0.1:${server_port}}"

cd apps/web
bun run dev
