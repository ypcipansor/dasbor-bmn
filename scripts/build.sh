#!/usr/bin/env bash
# Bangun ulang aset dasbor dari sumber: CSS Tailwind, WASM hidrasi + glue JS, dan
# biner SSR. Hasilnya cukup untuk menjalankan `scripts/serve.sh`.
#
# Penting: hidrasi Leptos mencocokkan pohon view di WASM dengan HTML yang dikirim
# server. Bila WASM tertinggal dari sumber, penanda hidrasi bergeser dan peramban
# gagal total ("expected a marker node"), sehingga semua tombol mati. Karena itu
# WASM wajib dibangun ulang setiap kali `src/` berubah, dan harus dihasilkan lewat
# `wasm-bindgen` agar glue JS merujuk berkas `*_bg.wasm` yang benar.
#
# Perkakas: cargo, wasm-bindgen, dan Tailwind CSS v4 (standalone CLI `tailwindcss`,
# atau `npx` sebagai cadangan). Skrip berhenti dengan galat bila salah satu
# langkah gagal, termasuk pembuatan CSS.

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
need rustup

# Tailwind CSS v4: pakai CLI standalone bila ada, jika tidak jatuh ke npx.
TAILWIND=""
if command -v tailwindcss >/dev/null 2>&1; then
  TAILWIND="tailwindcss"
elif command -v npx >/dev/null 2>&1; then
  TAILWIND="npx --yes @tailwindcss/cli"
else
  echo "Alat Tailwind CSS tidak ditemukan (butuh 'tailwindcss' atau 'npx')." >&2
  echo "Pasang CLI standalone:" >&2
  echo "  curl -sL https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-linux-x64 -o ~/.local/bin/tailwindcss && chmod +x ~/.local/bin/tailwindcss" >&2
  exit 1
fi

mkdir -p "$PKG"

echo "==> 1/4 Bangun CSS Tailwind -> $PKG/$OUTPUT_NAME.css"
# Dihasilkan lebih dulu agar dasbor tidak pernah tersaji tanpa gaya, termasuk
# pada checkout bersih yang belum punya CSS sama sekali.
$TAILWIND -i style/tailwind.css -o "$PKG/$OUTPUT_NAME.css" --minify
if [ ! -s "$PKG/$OUTPUT_NAME.css" ]; then
  echo "GAGAL: CSS tidak terbentuk di $PKG/$OUTPUT_NAME.css" >&2
  exit 1
fi

echo "==> 2/4 Bangun WASM hidrasi"
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
CARGO_TARGET_DIR="$WASM_TARGET_DIR" cargo build \
  --lib --target wasm32-unknown-unknown \
  --no-default-features --features hydrate

echo "==> 3/4 Hasilkan glue JS + WASM lewat wasm-bindgen"
wasm-bindgen --target web --out-dir "$PKG" --out-name "$OUTPUT_NAME" "$WASM_ARTIFACT"

echo "==> 4/4 Bangun biner SSR"
cargo build --no-default-features --features ssr

echo "Selesai. Aset di $PKG, biner di target/debug/dasbor-bmn."
