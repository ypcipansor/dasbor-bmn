# Dasbor BMN — Kejaksaan RI

Dasbor analitik **Barang Milik Negara (BMN)** untuk Kejaksaan Republik Indonesia,
dibangun di atas **Web Service SLDK SIMAN v2** (DJKN, Kementerian Keuangan RI).

Dibangun dengan **Rust**: [Leptos 0.8](https://leptos.dev) (SSR + hidrasi) di
atas **Axum**, dengan **TypeScript-free** arsitektur ber-tipe penuh dari ujung ke
ujung dan gaya **Tailwind CSS v4**.

## Fitur

- **Ringkasan eksekutif** — total aset, nilai, akumulasi susut, sebaran wilayah,
  serta indikator aset idle, rusak, dan hilang.
- **Analitik** — komposisi nilai per kategori (donut), tren perolehan per tahun
  (area), peringkat provinsi dan satuan kerja, kondisi aset, dan sumber dana.
  Semua grafik berupa SVG yang dibuat sendiri: ringan, tajam, tanpa dependensi JS.
- **Kategori aset** — 15 kategori Master Aset BMN beserta volume sumber, jumlah
  unit, nilai, dan **struktur 106 kolom** sesuai dokumen SLDK.
- **Telusuri data** — pencarian, pengurutan, paginasi, dan ekspor CSV per kategori.
- **Sinkronisasi** — tarik data per tabel atau sekaligus, dengan catatan proses
  dan pemeriksaan jumlah baris pada sumber.
- **Pengaturan** — isi kredensial SLDK dari antarmuka, uji token SSO, dan lihat
  status koneksi. Rahasia tidak pernah dikirim balik ke peramban.
- **Responsif & aksesibel** — sidebar desktop, drawer mobile, mode terang/gelap,
  navigasi papan tuntas, dan atribut ARIA pada komponen interaktif.

## Mode data

Dasbor berjalan dalam dua mode:

| Mode | Kapan | Perilaku |
| --- | --- | --- |
| **Contoh** | Kredensial SLDK belum diisi | Data ilustratif deterministik agar dasbor tetap dapat dievaluasi |
| **Langsung** | BA_KEY + Client ID + Client Secret terisi | Menarik data nyata dari gateway SLDK |

Banner di setiap halaman selalu menandai mode yang sedang aktif.

## Menjalankan

```bash
# 1. Perkakas
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos          # sekali saja
# Tailwind CLI standalone (v4)
curl -sL https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-linux-x64 \
  -o ~/.local/bin/tailwindcss && chmod +x ~/.local/bin/tailwindcss

# 2. Kredensial (opsional — tanpa ini, dasbor memakai data contoh)
cp .env.example .env                # lalu isi nilainya

# 3. Jalankan
cargo leptos watch                  # pengembangan di http://127.0.0.1:3000
cargo leptos build --release        # build produksi ke target/site
```

### Konfigurasi

Nilai dibaca dari environment, dan dapat juga diisi lewat halaman **Pengaturan**
(disimpan ke `data/config.json`, direktori diatur oleh `SIMAN_DATA_DIR`).

| Variabel | Keterangan | Bawaan |
| --- | --- | --- |
| `SIMAN_BASE_URL` | Akar gateway KSB | `https://apigateway.kemenkeu.go.id` |
| `SIMAN_TOKEN_URL` | Endpoint token SSO | `https://sso.kemenkeu.go.id/connect/token` |
| `SIMAN_CLIENT_ID` | Client ID dari Kemenkeu | — |
| `SIMAN_CLIENT_SECRET` | Client Secret dari Kemenkeu | — |
| `SIMAN_GRANT_TYPE` | Tipe grant OAuth2 | `client_credentials` |
| `SIMAN_BA_KEY` | Kode BA unit Kejaksaan | — |
| `SIMAN_DATA_DIR` | Direktori cache & konfigurasi | `data` |
| `DASBOR_PASSWORD` | Kata sandi gerbang dasbor | dibuat otomatis |

Alias tanpa awalan `SIMAN_` (`BASE_URL`, `CLIENT_ID`, `CLIENT_SECRET`,
`GRANT_TYPE`, `BA_KEY`) juga dikenali.

## Arsitektur

```
src/
  main.rs        Server Axum: perutean Leptos + berkas statis
  app.rs         Shell HTML, hidrasi, dan perutean halaman
  lib.rs         Titik masuk hidrasi WASM
  model.rs       Model data bersama + pemformatan angka/rupiah
  catalog.rs     Katalog 15 tabel Master Aset BMN
  schema.rs      Metadata 106 kolom (dibangkitkan dari dokumen SLDK)
  config.rs      Konfigurasi runtime + berkas
  fs_aman.rs     Penulisan berkas dengan izin 0600
  auth.rs        Gerbang masuk: kata sandi, sesi cookie, middleware Axum
  sldk.rs        Klien Web Service SLDK (token SSO + gateway KSB)
  analytics.rs   Agregasi murni: bucket, tren, ringkasan
  store.rs       Cache lokal SQLite (baris, agregat, status)
  demo.rs        Pembangkit data contoh deterministik
  server_fns.rs  Server functions Leptos untuk seluruh alur data
  components/    Kerangka, komponen UI, dan grafik SVG
  pages/         Ringkasan, Kategori, Data, Sinkronisasi, Pengaturan
```

### Alur data

1. **Token** — `client_credentials` ke SSO Kemenkeu, disimpan di cache selama
   masa berlakunya.
2. **Hitung** — `getRowCount/{BA_KEY}/{TABEL}` pada gateway.
3. **Tarik** — resource aset (`getAsetTanah`, …) dengan `BA_KEY`, `ID_1`, `ID_2`
   secara bertahap.
4. **Agregat** — baris diringkas menjadi bucket dan tren, lalu disimpan di SQLite.
5. **Sajikan** — server functions mengirim ringkasan dan halaman data ke UI.

## Pengujian

```bash
cargo test --features ssr --lib
```

## Catatan keamanan

- Seluruh data dan aksi di balik gerbang masuk. Halaman meminta kata sandi
  (`DASBOR_PASSWORD`, atau kata sandi acak yang dibuat di `data/auth.json`),
  dan setiap panggilan `/api/*` tanpa sesi yang sah dijawab `401`.
- Sesi memakai cookie `HttpOnly` `SameSite=Lax`; token disimpan sebagai hash,
  kedaluwarsa, dan dibandingkan dengan waktu konstan.
- Rahasia hanya tersimpan di server (`data/config.json` atau environment) dan
  tidak pernah dikirim balik ke peramban.
- Berkas `data/config.json` dan `data/auth.json` ditulis dengan izin `0600`.
- Ekspor CSV menetralkan sel yang dimulai `= + - @` agar tidak menjadi formula.
- `data/` dan seluruh basis data SQLite dikecualikan dari Git.
- Data BMN bersifat **Terbatas** — gunakan sesuai kewenangan.

## Berkas referensi

- `docs/panduan-web-service-sldk.pdf` — panduan resmi Web Service SLDK.
- `docs/postman-collection.json` — koleksi Postman "API Siman v2.0" (variabel
  `token`, `base_url`, `ba_key` sengaja dikosongkan).
