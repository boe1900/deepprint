#!/usr/bin/env bash

set -euo pipefail

container_name="${DEEPPRINT_CUPS_CONTAINER_NAME:-}"
printer_name="${DEEPPRINT_CUPS_PDF_PRINTER_NAME:-CUPS-PDF}"
output_dir="${DEEPPRINT_CUPS_PDF_OUTPUT_DIR:-/var/spool/cups-pdf/ANONYMOUS}"
spool_dir="${DEEPPRINT_CUPS_PDF_SPOOL_DIR:-/var/spool/cups-pdf/SPOOL}"

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
echo "[setup-cups-pdf] spool_dir=${spool_dir}"

if ! docker inspect "${container_name}" >/dev/null 2>&1; then
  echo "[setup-cups-pdf] container not found: ${container_name}" >&2
  exit 1
fi

docker exec -i \
  -e DEEPPRINT_CUPS_PDF_PRINTER_NAME="${printer_name}" \
  -e DEEPPRINT_CUPS_PDF_OUTPUT_DIR="${output_dir}" \
  -e DEEPPRINT_CUPS_PDF_SPOOL_DIR="${spool_dir}" \
  "${container_name}" sh <<'CONTAINER_SH'
set -eu

printer_name="${DEEPPRINT_CUPS_PDF_PRINTER_NAME:-CUPS-PDF}"
output_dir="${DEEPPRINT_CUPS_PDF_OUTPUT_DIR:-/var/spool/cups-pdf/ANONYMOUS}"
spool_dir="${DEEPPRINT_CUPS_PDF_SPOOL_DIR:-/var/spool/cups-pdf/SPOOL}"

if ! command -v lpadmin >/dev/null 2>&1; then
  echo '[setup-cups-pdf] lpadmin not found in container' >&2
  exit 1
fi

model_name="$(lpinfo -m | awk 'tolower($0) ~ /pdf/ { print $1; exit }')"
if [ -z "${model_name}" ]; then
  echo '[setup-cups-pdf] could not locate a PDF printer model via lpinfo -m' >&2
  lpinfo -m | sed -n '1,40p' >&2 || true
  exit 1
fi

output_parent="$(dirname "${output_dir}")"
mkdir -p "${output_parent}" "${output_dir}" "${spool_dir}"
chmod 0777 "${output_parent}" "${output_dir}" "${spool_dir}"

if [ -f /etc/cups/cups-pdf.conf ]; then
  cp /etc/cups/cups-pdf.conf /etc/cups/cups-pdf.conf.deepprint-bak 2>/dev/null || true

  # Some CUPS-PDF images default to ${HOME}/PDF, which can report a
  # successful job without producing a visible file on bind-mounted NAS paths.
  awk \
    -v out="${output_dir}" \
    -v anon="${output_dir}" \
    -v spool="${spool_dir}" \
    '
      BEGIN { seen_out = seen_anon = seen_spool = seen_anon_umask = seen_user_umask = 0 }
      /^#?Out[[:space:]]/ { print "Out " out; seen_out = 1; next }
      /^#?AnonDirName[[:space:]]/ { print "AnonDirName " anon; seen_anon = 1; next }
      /^#?Spool[[:space:]]/ { print "Spool " spool; seen_spool = 1; next }
      /^#?AnonUMask[[:space:]]/ { print "AnonUMask 0000"; seen_anon_umask = 1; next }
      /^#?UserUMask[[:space:]]/ { print "UserUMask 0000"; seen_user_umask = 1; next }
      { print }
      END {
        if (!seen_out) print "Out " out
        if (!seen_anon) print "AnonDirName " anon
        if (!seen_spool) print "Spool " spool
        if (!seen_anon_umask) print "AnonUMask 0000"
        if (!seen_user_umask) print "UserUMask 0000"
      }
    ' /etc/cups/cups-pdf.conf > /tmp/cups-pdf.conf.deepprint
  cat /tmp/cups-pdf.conf.deepprint > /etc/cups/cups-pdf.conf
  rm -f /tmp/cups-pdf.conf.deepprint
  echo '[setup-cups-pdf] normalized /etc/cups/cups-pdf.conf output paths'
else
  echo '[setup-cups-pdf] /etc/cups/cups-pdf.conf not found; continuing with image defaults'
fi

if lpstat -p "${printer_name}" >/dev/null 2>&1; then
  echo '[setup-cups-pdf] printer already exists'
else
  lpadmin \
    -p "${printer_name}" \
    -E \
    -v 'cups-pdf:/' \
    -m "${model_name}"
  echo '[setup-cups-pdf] printer created'
fi

lpadmin -p "${printer_name}" -o printer-is-shared=true
cupsenable "${printer_name}"

if command -v cupsaccept >/dev/null 2>&1; then
  cupsaccept "${printer_name}"
elif command -v accept >/dev/null 2>&1; then
  accept "${printer_name}"
else
  echo '[setup-cups-pdf] accept/cupsaccept not found; continuing after cupsenable'
fi

echo '[setup-cups-pdf] final printer status:'
lpstat -l -p "${printer_name}"
CONTAINER_SH
