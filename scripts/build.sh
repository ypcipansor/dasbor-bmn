#!/usr/bin/env bash
# Bangun ulang aset dasbor dari sumber: WASM hidrasi + glue JS, biner SSR, dan CSS.
#
# Penting: hidrasi Leptos mencocokkan pohon view di WASM dengan HTML yang dikirim
# server. Bila WASM tertinggal dari sumber, penanda hidrasi bergeser dan peramban
# gagal total ("expected a marker node"), sehingga semua tombol mati. Karena itu
# WASM wajib dibangun ulang setiap kali `src/` berubah, dan harus dihasilkan lewat
# `wasm-bindgen` agar glue JS merujuk berkas `*_bg.wasm` yang benar.

set -eu

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUTPUT_NAME="${LEPTOS_OUTPUT_NAME:-dasbor-bmn}"
SITE_ROOT="${LEPTOS_SITE_ROOT:-target/site}"
PKG_DIR="${LEPTOS_SITE_PKG_DIR:-pkg}"
PKG="$SITE_ROOT/$PKG_DIR"

# Target WASM terpisah dari target host agar cache tidak saling mengganggu.
WASM_TARGET_DIR="target/front"
WASM_ARTIFACT="$WASM_TARGET_DIR/wasm32-unknown-unknown/debug/dasbor_bmn.wasm"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "Alat '$1' tidak ditemukan. Pasang lebih dulu." >&2
    exit 1
  }
}
need cargo
need wasm-bindgen

echo "==> 1/3 Bangun WASM hidrasi"
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
CARGO_TARGET_DIR="$WASM_TARGET_DIR" cargo build \
  --lib --target wasm32-unknown-unknown \
  --no-default-features --features hydrate

echo "==> 2/3 Hasilkan glue JS + WASM lewat wasm-bindgen"
mkdir -p "$PKG"
wasm-bindgen --target web --out-dir "$PKG" --out-name "$OUTPUT_NAME" "$WASM_ARTIFACT"

echo "==> 3/3 Bangun biner SSR"
cargo build --no-default-features --features ssr

if [ ! -f "$PKG/$OUTPUT_NAME.css" ]; then
  echo "PERINGATAN: $PKG/$OUTPUT_NAME.css belum ada." >&2
  echo "Bangun CSS Tailwind lebih dulu, mis." >&2
  echo "  npx --yes @tailwindcss/cli -i style/tailwind.css -o $PKG/$OUTPUT_NAME.css --minify" >&2
fi

echo "Selesai. Aset di $PKG, biner di target/debug/dasbor-bmn."
