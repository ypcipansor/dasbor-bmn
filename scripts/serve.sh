#!/usr/bin/env bash
# Menjaga server dasbor tetap hidup: nyalakan, lalu nyalakan ulang bila mati.
# Dipakai di lingkungan pengembangan agar proxy publik tidak menampilkan bad gateway.

set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${DASBOR_BIN:-$ROOT/target/debug/dasbor-bmn}"
PORT="${DASBOR_PORT:-12000}"
LOG="${DASBOR_LOG:-/tmp/dasbor-bmn.log}"
PIDFILE="${DASBOR_PIDFILE:-/tmp/dasbor-bmn.pid}"

export LEPTOS_SITE_ROOT="${LEPTOS_SITE_ROOT:-target/site}"
export LEPTOS_SITE_ADDR="0.0.0.0:$PORT"
export LEPTOS_SITE_PKG_DIR="${LEPTOS_SITE_PKG_DIR:-pkg}"
# Wajib diisi saat biner dijalankan langsung (tanpa `cargo leptos`). Tanpa ini,
# `get_configuration` menghasilkan output-name kosong sehingga HTML memuat
# "/pkg/.js" (404), hidrasi gagal, dan seluruh tombol tidak merespons.
export LEPTOS_OUTPUT_NAME="${LEPTOS_OUTPUT_NAME:-dasbor-bmn}"

cd "$ROOT" || exit 1

if [ ! -x "$BIN" ]; then
  echo "Biner tidak ditemukan: $BIN" >&2
  exit 1
fi

echo "Penjaga dasbor aktif — port $PORT, log $LOG"
echo "$$" > "$PIDFILE"

while true; do
  "$BIN" >> "$LOG" 2>&1
  echo "$(date -Is) server berhenti, menyalakan ulang dalam 2 detik" >> "$LOG"
  sleep 2
done
