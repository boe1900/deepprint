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

export DEEPPRINT_AGENT_BIND="${DEEPPRINT_AGENT_BIND:-127.0.0.1}"
export DEEPPRINT_AGENT_PORT="$server_port"
export DEEPPRINT_AGENT_DB_PATH="$PWD/.deepprint-dev/deepprint.db"
export DEEPPRINT_CUPS_BASE_URL="${DEEPPRINT_CUPS_BASE_URL:-http://127.0.0.1:${cups_port}/}"
export DEEPPRINT_TYPST_LOCAL_PACKAGES_ROOT="$PWD/.deepprint-dev/typst/packages"
export DEEPPRINT_TYPST_PREVIEW_CACHE_ROOT="$PWD/.deepprint-dev/cache/typst"
export DEEPPRINT_TYPST_FONTS_ROOT="$PWD/.deepprint-dev/typst/fonts"
export DEEPPRINT_LOG_DIR="$PWD/.deepprint-dev/logs"
export DEEPPRINT_DIAGNOSTICS_DIR="$PWD/.deepprint-dev/diagnostics"

unset DEEPPRINT_DATABASE_URL

cargo run --manifest-path apps/server/Cargo.toml --bin deepprint-server
