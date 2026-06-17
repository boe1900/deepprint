#!/usr/bin/env bash

set -euo pipefail

container_name="${DEEPPRINT_CUPS_CONTAINER_NAME:-}"
printer_name="${DEEPPRINT_CUPS_PDF_PRINTER_NAME:-CUPS-PDF}"
output_dir="${DEEPPRINT_CUPS_PDF_OUTPUT_DIR:-/var/spool/cups-pdf/ANONYMOUS}"

if ! command -v docker >/dev/null 2>&1; then
  echo "[setup-cups-pdf] docker command not found" >&2
  exit 1
fi

if [[ -z "${container_name}" ]]; then
  container_id="$(docker compose ps -q cups 2>/dev/null || true)"
  if [[ -n "${container_id}" ]]; then
    container_name="$(docker inspect --format '{{.Name}}' "${container_id}" | sed 's#^/##')"
  else
    container_name="deepprint-cups-1"
  fi
fi

echo "[setup-cups-pdf] container=${container_name}"
echo "[setup-cups-pdf] printer=${printer_name}"
echo "[setup-cups-pdf] output_dir=${output_dir}"

if ! docker inspect "${container_name}" >/dev/null 2>&1; then
  echo "[setup-cups-pdf] container not found: ${container_name}" >&2
  exit 1
fi

docker exec "${container_name}" sh -lc "
set -eu

if ! command -v lpadmin >/dev/null 2>&1; then
  echo '[setup-cups-pdf] lpadmin not found in container' >&2
  exit 1
fi

model_name=\"\$(lpinfo -m | awk 'tolower(\$0) ~ /pdf/ { print \$1; exit }')\"
if [ -z \"\${model_name}\" ]; then
  echo '[setup-cups-pdf] could not locate a PDF printer model via lpinfo -m' >&2
  lpinfo -m | sed -n '1,40p' >&2 || true
  exit 1
fi

mkdir -p '${output_dir}'

if lpstat -p '${printer_name}' >/dev/null 2>&1; then
  echo '[setup-cups-pdf] printer already exists'
else
  lpadmin \
    -p '${printer_name}' \
    -E \
    -v 'cups-pdf:/' \
    -m \"\${model_name}\"
  echo '[setup-cups-pdf] printer created'
fi

lpadmin -p '${printer_name}' -o printer-is-shared=true
cupsenable '${printer_name}'

if command -v cupsaccept >/dev/null 2>&1; then
  cupsaccept '${printer_name}'
elif command -v accept >/dev/null 2>&1; then
  accept '${printer_name}'
else
  echo '[setup-cups-pdf] accept/cupsaccept not found; continuing after cupsenable'
fi

echo '[setup-cups-pdf] final printer status:'
lpstat -l -p '${printer_name}'
"
