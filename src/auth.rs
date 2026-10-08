//! Gerbang otorisasi dasbor.
//!
//! Dasbor ini menyajikan data BMN berkategori **Terbatas**, jadi seluruh
//! server function (`/api/*`) hanya boleh dipanggil oleh operator yang sudah
//! masuk. Kata sandi diambil dari `DASBOR_PASSWORD`; bila tidak diset, kata
//! sandi acak dibuat sekali dan disimpan di `data/auth.json` (mode 0600).
//!
//! Sesi disimpan di memori proses dan diikat ke cookie `HttpOnly` +
//! `SameSite=Strict`, sehingga permintaan lintas situs tidak ikut membawa sesi.

#![cfg(feature = "ssr")]

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{ConnectInfo, Request};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::config;

/// Nama cookie sesi.
pub const COOKIE_NAME: &str = "dasbor_sesi";

/// Umur sesi (8 jam) — sepanjang satu hari kerja.
const SESSION_TTL: Duration = Duration::from_secs(8 * 60 * 60);

/// Jalan yang boleh diakses tanpa sesi.
const JALAN_BEBAS: &[&str] = &["/api/login", "/api/logout"];

static SESSIONS: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
static PASSWORD: OnceLock<String> = OnceLock::new();
static PERCOBAAN: OnceLock<Mutex<HashMap<String, (u32, Instant)>>> = OnceLock::new();

/// Jendela pembatasan percobaan masuk.
const JENDELA_PERCOBAAN: Duration = Duration::from_secs(5 * 60);
/// Percobaan gagal maksimum per alamat sebelum penundaan dinaikkan.
const MAKS_PERCOBAAN: u32 = 5;
/// Batas percobaan gagal seluruh klien sebelum penundaan dinaikkan lagi.
///
/// Header alamat klien diisi proxy dan karena itu tidak dapat dipercaya, jadi
/// penyerang dapat memalsukan alamat baru pada tiap percobaan. Batas global ini
/// menutup lubang itu: setelah jendela penuh, setiap percobaan salah ditunda
/// makin lama, berapa pun alamat yang diklaim (CWE-290/CWE-307).
const MAKS_GLOBAL: u32 = 30;
/// Penundaan dasar tiap percobaan salah.
const TUNDA_DASAR_MS: u64 = 300;
/// Batas atas penundaan satu percobaan salah (detik).
const TUNDA_MAKS_MS: u64 = 30_000;

fn sessions() -> &'static Mutex<HashMap<String, Instant>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn percobaan() -> &'static Mutex<HashMap<String, (u32, Instant)>> {
    PERCOBAAN.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Jumlah percobaan gagal seluruh klien dalam jendela berjalan.
static GLOBAL: OnceLock<Mutex<(u32, Instant)>> = OnceLock::new();

fn global() -> &'static Mutex<(u32, Instant)> {
    GLOBAL.get_or_init(|| Mutex::new((0, Instant::now())))
}

/// Apakah alamat berasal dari jaringan lokal (loopback atau privat).
fn alamat_lokal(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

/// Gerbang satu-slot untuk verifikasi kata sandi.
///
/// Pemeriksaan kata sandi memakan waktu, dan penundaan percobaan salah tidak
/// boleh menjadi satu-satunya pengaman: tanpa gerbang ini, banyak permintaan
/// paralel sama-sama memeriksa kata sandi sebelum ada yang tertunda, sehingga
/// laju tebak-tebakan lolos dari penundaan (CWE-307). Hanya satu permintaan
/// yang boleh **menunggu** di sini; percobaan yang salah menahan izin selama
/// penundaannya, sedangkan kata sandi yang benar tidak pernah menunggu.
static GERBANG_VERIFIKASI: OnceLock<tokio::sync::Semaphore> = OnceLock::new();

fn gerbang_verifikasi() -> &'static tokio::sync::Semaphore {
    GERBANG_VERIFIKASI.get_or_init(|| tokio::sync::Semaphore::new(1))
}

/// Hasil satu percobaan masuk.
enum Hasil {
    /// Kata sandi benar dan sesi baru dibuat.
    Sukses(String),
    /// Kata sandi salah; verifikasi sudah ditunda sesuai jumlah percobaan.
    Salah,
}

/// Verifikasi satu percobaan masuk.
///
/// Pemeriksaan kata sandi dilakukan lebih dulu **di luar** gerbang, sehingga
/// operator yang mengirim kata sandi benar tidak pernah menunggu di belakang
/// antrean percobaan salah (CWE-400). Hanya percobaan yang salah yang mengambil
/// izin gerbang, lalu menghitung percobaan dan menahan izin selama penundaan;
/// dengan begitu percobaan serentak tetap diserialkan dan tidak dapat
/// menyelinap sebelum penundaan berlaku (CWE-307).
async fn verifikasi(password: &str, kunci: &str) -> Hasil {
    if cocok(password) {
        hapus_percobaan(kunci);
        return Hasil::Sukses(buat_sesi());
    }
    let _izin = gerbang_verifikasi()
        .acquire()
        .await
        .expect("gerbang tertutup");
    let n = catat_gagal_global().max(catat_gagal(kunci));
    tokio::time::sleep(tunda_percobaan(n)).await;
    Hasil::Salah
}

/// Kunci pembatasan: alamat TCP peer yang sebenarnya.
///
/// Header seperti `X-Forwarded-For` **tidak** dipakai karena datang dari klien
/// dan mudah dipalsukan; memakainya membuat penyerang mendapat jatah percobaan
/// baru pada tiap permintaan (CWE-290). `ConnectInfo` diisi kernel, bukan header.
fn kunci_klien(req: &Request) -> String {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "tanpa-alamat".to_string())
}

/// Apakah permintaan datang dari jaringan lokal (untuk aturan cookie Secure).
fn permintaan_lokal(req: &Request) -> bool {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| alamat_lokal(&ci.0.ip()))
        .unwrap_or(false)
}

/// Hitung satu percobaan gagal per alamat; kembalikan jumlahnya di jendela ini.
fn catat_gagal(kunci: &str) -> u32 {
    let mut p = percobaan().lock().unwrap();
    let now = Instant::now();
    p.retain(|_, (_, until)| *until > now);
    let entri = p
        .entry(kunci.to_string())
        .or_insert((0, now + JENDELA_PERCOBAAN));
    entri.0 += 1;
    entri.1 = now + JENDELA_PERCOBAAN;
    entri.0
}

/// Hitung satu percobaan gagal global; kembalikan jumlahnya di jendela ini.
fn catat_gagal_global() -> u32 {
    let mut g = global().lock().unwrap();
    let now = Instant::now();
    if g.1 <= now {
        *g = (0, now + JENDELA_PERCOBAAN);
    }
    g.0 += 1;
    g.0
}

/// Lama penundaan untuk percobaan salah ke-`n`.
///
/// Menaikkan penundaan secara berlipat setelah ambang terlampaui, lalu berhenti
/// di `TUNDA_MAKS_MS`. Kata sandi yang benar tidak pernah ditunda, sehingga
/// operator tidak dapat terkunci oleh percobaan orang lain (CWE-400), sementara
/// laju tebak-tebakan tetap dibatasi ketat.
fn tunda_percobaan(n: u32) -> Duration {
    let ambang = MAKS_PERCOBAAN.min(MAKS_GLOBAL);
    if n <= ambang {
        return Duration::from_millis(TUNDA_DASAR_MS);
    }
    let langkah = (n - ambang).min(10);
    let ms = TUNDA_DASAR_MS
        .saturating_mul(1u64 << langkah)
        .min(TUNDA_MAKS_MS);
    Duration::from_millis(ms)
}

/// Hapus catatan percobaan setelah masuk berhasil.
fn hapus_percobaan(kunci: &str) {
    percobaan().lock().unwrap().remove(kunci);
    // Masuk yang sah mengosongkan penghitung global agar operator tidak ikut
    // terdampak oleh percobaan orang lain.
    let now = Instant::now();
    *global().lock().unwrap() = (0, now + JENDELA_PERCOBAAN);
}

/// Baca byte acak dari sistem operasi.
///
/// Gagal-tertutup: bila sumber acak sistem tidak tersedia, proses panik alih-alih
/// menurunkan kredensial dari waktu sistem yang dapat ditebak (CWE-338).
fn acak_bytes(n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    getrandom::fill(&mut buf).expect("sumber acak sistem operasi tidak tersedia");
    buf
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}

/// Bandingkan dua nilai tanpa membocorkan posisi perbedaan lewat waktu.
fn sama_konstan(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut beda = (a.len() ^ b.len()) as u8;
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = *a.get(i).unwrap_or(&0);
        let y = *b.get(i).unwrap_or(&0);
        beda |= x ^ y;
    }
    beda == 0
}

fn password_path() -> std::path::PathBuf {
    config::data_dir().join("auth.json")
}

/// Kata sandi aktif; dibuat sekali bila belum ada.
pub fn password() -> &'static str {
    PASSWORD.get_or_init(|| {
        if let Some(p) = std::env::var("DASBOR_PASSWORD")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
        {
            return p;
        }
        let path = password_path();
        if let Ok(teks) = std::fs::read_to_string(&path) {
            if let Some(p) = serde_json::from_str::<serde_json::Value>(&teks)
                .ok()
                .and_then(|v| {
                    v.get("password")
                        .and_then(|x| x.as_str())
                        .map(str::to_string)
                })
                .filter(|v| !v.is_empty())
            {
                return p;
            }
        }
        let baru = hex(&acak_bytes(16));
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let isi = serde_json::json!({ "password": baru }).to_string();
        if crate::fs_aman::tulis_privat(&path, isi.as_bytes()).is_ok() {
            eprintln!(
                "Kata sandi dasbor dibuat dan disimpan di {}. Gunakan kata sandi ini untuk masuk.",
                path.display()
            );
        } else {
            eprintln!("Kata sandi dasbor sesi ini: {baru} (gagal menulis berkas)");
        }
        baru
    })
}

/// Buat sesi baru dan kembalikan tokennya.
pub fn buat_sesi() -> String {
    let token = hex(&acak_bytes(32));
    let mut s = sessions().lock().unwrap();
    let now = Instant::now();
    s.retain(|_, exp| *exp > now);
    s.insert(token.clone(), now + SESSION_TTL);
    token
}

/// Hapus satu sesi.
pub fn hapus_sesi(token: &str) {
    sessions().lock().unwrap().remove(token);
}

/// Apakah token sesi masih berlaku.
pub fn sesi_valid(token: &str) -> bool {
    let mut s = sessions().lock().unwrap();
    let now = Instant::now();
    s.retain(|_, exp| *exp > now);
    s.get(token).is_some_and(|exp| *exp > now)
}

/// Periksa kata sandi yang dikirim pengguna.
pub fn cocok(kandidat: &str) -> bool {
    sama_konstan(kandidat.trim(), password())
}

fn ambil_cookie(req: &Request, nama: &str) -> Option<String> {
    let raw = req.headers().get(header::COOKIE)?.to_str().ok()?;
    for bagian in raw.split(';') {
        let bagian = bagian.trim();
        if let Some(v) = bagian.strip_prefix(nama).and_then(|r| r.strip_prefix('=')) {
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn permintaan_https(req: &Request) -> bool {
    req.headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("https")
        })
}

/// Apakah cookie sesi harus ditandai `Secure`.
///
/// Gagal-tertutup: `Secure` dipasang secara bawaan dan hanya dilepas untuk
/// pengembangan lokal. Dua syarat harus terpenuhi: koneksi TCP berasal dari
/// jaringan lokal, dan nama host persis host lokal. Pencocokan awalan tidak
/// dipakai karena host seperti `localhost.example.com` dimiliki pihak lain dan
/// akan menerima cookie sesi yang dapat dikirim lewat HTTP biasa (CWE-614).
fn pakai_secure(req: &Request) -> bool {
    if permintaan_https(req) {
        return true;
    }
    if !permintaan_lokal(req) {
        return true;
    }
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    // Ambil nama host tanpa port; IPv6 ditulis dalam kurung siku.
    let nama = if let Some(sisa) = host.strip_prefix('[') {
        sisa.split(']').next().unwrap_or("")
    } else {
        host.split(':').next().unwrap_or("")
    };
    let lokal = matches!(nama, "localhost" | "127.0.0.1" | "::1");
    !lokal
}

fn header_cookie(token: &str, https: bool) -> String {
    let mut c = format!(
        "{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        SESSION_TTL.as_secs()
    );
    if https {
        c.push_str("; Secure");
    }
    c
}

fn cookie_hapus(https: bool) -> String {
    let mut c = format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
    if https {
        c.push_str("; Secure");
    }
    c
}

/// Halaman masuk mandiri (tanpa aset eksternal) agar tetap ringan dan aman.
fn halaman_masuk(pesan: Option<&str>) -> String {
    let pesan_html = match pesan {
        Some(p) => format!(
            "<p class=\"galat\" role=\"alert\">{}</p>",
            p.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        ),
        None => String::new(),
    };
    format!(
        r#"<!DOCTYPE html>
<html lang="id">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<meta name="robots" content="noindex, nofollow"/>
<title>Masuk — Dasbor BMN</title>
<style>
  :root {{ color-scheme: light dark; }}
  * {{ box-sizing: border-box; }}
  body {{ margin:0; min-height:100dvh; display:grid; place-items:center; padding:1.5rem;
         font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
         background:#f4f5f9; color:#101322; }}
  @media (prefers-color-scheme: dark) {{ body {{ background:#0b0d16; color:#e8eaf2; }} }}
  .kotak {{ width:100%; max-width:26rem; background:#fff; border:1px solid #e2e4ee;
            border-radius:1rem; padding:1.75rem; box-shadow:0 10px 30px rgba(16,19,34,.08); }}
  @media (prefers-color-scheme: dark) {{ .kotak {{ background:#141826; border-color:#252b3d; }} }}
  .lencana {{ display:inline-block; font-size:.7rem; font-weight:600; letter-spacing:.04em;
              text-transform:uppercase; color:#1f47f5; }}
  h1 {{ margin:.5rem 0 .25rem; font-size:1.25rem; }}
  p.sub {{ margin:0 0 1.25rem; font-size:.85rem; color:#5b6178; }}
  @media (prefers-color-scheme: dark) {{ p.sub {{ color:#9aa2bb; }} }}
  label {{ display:block; font-size:.75rem; font-weight:600; margin-bottom:.35rem; }}
  input {{ width:100%; padding:.6rem .7rem; font-size:.9rem; border-radius:.6rem;
           border:1px solid #c9cddd; background:#fff; color:inherit; }}
  @media (prefers-color-scheme: dark) {{ input {{ background:#0f1320; border-color:#2c3348; }} }}
  button {{ margin-top:1rem; width:100%; padding:.65rem; font-size:.9rem; font-weight:600;
            color:#fff; background:#1f47f5; border:0; border-radius:.6rem; cursor:pointer; }}
  button:hover {{ background:#1a3bd0; }}
  .galat {{ margin:0 0 1rem; padding:.6rem .7rem; font-size:.8rem; border-radius:.6rem;
            background:#fdecec; color:#8a1c1c; }}
  @media (prefers-color-scheme: dark) {{ .galat {{ background:rgba(220,60,60,.12); color:#f4b4b4; }} }}
  footer {{ margin-top:1.25rem; font-size:.7rem; color:#7c8299; }}
</style>
</head>
<body>
  <main class="kotak">
    <span class="lencana">SLDK SIMAN v2 · Kejaksaan RI</span>
    <h1>Dasbor BMN</h1>
    <p class="sub">Data berkategori Terbatas. Masukkan kata sandi dasbor untuk melanjutkan.</p>
    {pesan_html}
    <form method="post" action="/api/login">
      <label for="password">Kata sandi</label>
      <input id="password" name="password" type="password" autocomplete="current-password"
             autofocus required/>
      <button type="submit">Masuk</button>
    </form>
    <footer>Kata sandi diatur lewat <code>DASBOR_PASSWORD</code> atau berkas <code>data/auth.json</code>.</footer>
  </main>
</body>
</html>"#
    )
}

/// Middleware: gerbang tunggal untuk seluruh permintaan.
pub async fn gerbang(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();

    if JALAN_BEBAS.iter().any(|p| path == *p) {
        return next.run(req).await;
    }

    if let Some(token) = ambil_cookie(&req, COOKIE_NAME) {
        if sesi_valid(&token) {
            return next.run(req).await;
        }
    }

    // Panggilan data/aksi tanpa sesi ditolak; halaman lain disajikan formulir masuk.
    if path.starts_with("/api/") {
        return (
            StatusCode::UNAUTHORIZED,
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            )],
            Body::from(r#"{"error":"sesi tidak valid — silakan masuk kembali"}"#),
        )
            .into_response();
    }

    // Aset statis tidak lagi dikecualikan: berkas di `/pkg` hanya disajikan
    // kepada pemegang sesi, agar direktori aset tidak menjadi jalan unduh
    // tanpa masuk (CWE-306).
    if path.starts_with("/pkg/") || path.starts_with("/favicon") {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"))],
            Body::from("sesi tidak valid — silakan masuk kembali"),
        )
            .into_response();
    }

    let mut res = Response::new(Body::from(halaman_masuk(None)));
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    res.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

/// Bentuk isi formulir masuk.
#[derive(serde::Deserialize)]
pub struct FormMasuk {
    pub password: String,
}

fn redirect_ke(lokasi: &str, cookie: Option<String>) -> Response {
    let mut res = Response::new(Body::empty());
    *res.status_mut() = StatusCode::SEE_OTHER;
    if let Ok(v) = HeaderValue::from_str(lokasi) {
        res.headers_mut().insert(header::LOCATION, v);
    }
    if let Some(c) = cookie {
        if let Ok(v) = HeaderValue::from_str(&c) {
            res.headers_mut().insert(header::SET_COOKIE, v);
        }
    }
    res
}

/// `POST /api/login` — verifikasi kata sandi lalu mulai sesi.
pub async fn login(req: Request) -> Response {
    let https = pakai_secure(&req);
    let kunci = kunci_klien(&req);
    let body = match axum::body::to_bytes(req.into_body(), 8 * 1024).await {
        Ok(b) => b,
        Err(_) => return halaman_galat("Permintaan tidak terbaca."),
    };
    let form: FormMasuk = match serde_urlencoded::from_bytes(&body) {
        Ok(f) => f,
        Err(_) => return halaman_galat("Permintaan tidak terbaca."),
    };
    // Verifikasi dan penghitungan percobaan berbagi satu gerbang, sehingga
    // permintaan paralel tidak dapat memeriksa kata sandi bersamaan sebelum
    // penundaan berlaku (CWE-307). Kata sandi yang benar tidak pernah ditunda,
    // jadi operator tidak dapat dikunci oleh percobaan orang lain.
    match verifikasi(&form.password, &kunci).await {
        Hasil::Sukses(token) => redirect_ke("/", Some(header_cookie(&token, https))),
        Hasil::Salah => halaman_galat("Kata sandi salah."),
    }
}

/// `POST /api/logout` — akhiri sesi.
pub async fn logout(req: Request) -> Response {
    let https = pakai_secure(&req);
    if let Some(token) = ambil_cookie(&req, COOKIE_NAME) {
        hapus_sesi(&token);
    }
    redirect_ke("/", Some(cookie_hapus(https)))
}

fn halaman_galat(pesan: &str) -> Response {
    let mut res = Response::new(Body::from(halaman_masuk(Some(pesan))));
    *res.status_mut() = StatusCode::UNAUTHORIZED;
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    res.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialkan uji yang menyentuh penghitung percobaan bersama.
    ///
    /// Penghitung global dan per alamat bersifat proses-wide; tanpa kunci ini,
    /// `hapus_percobaan` pada uji lain dapat mengosongkannya di tengah uji dan
    /// membuat hasilnya bergantung waktu.
    fn kunci_uji() -> std::sync::MutexGuard<'static, ()> {
        static K: OnceLock<Mutex<()>> = OnceLock::new();
        K.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// Kata sandi tetap untuk uji agar tidak membaca/menulis `data/` pengembangan.
    ///
    /// Harus dipanggil sebelum uji mana pun menyentuh `cocok`/`verifikasi`.
    fn kata_sandi_uji() -> &'static str {
        static P: OnceLock<String> = OnceLock::new();
        P.get_or_init(|| {
            std::env::set_var("DASBOR_PASSWORD", "sandi-uji-benar");
            password().to_string()
        })
    }

    #[test]
    fn perbandingan_konstan_benar() {
        assert!(sama_konstan("rahasia", "rahasia"));
        assert!(!sama_konstan("rahasia", "rahasib"));
        assert!(!sama_konstan("rahasia", "rahasiaa"));
        assert!(!sama_konstan("", "x"));
        assert!(sama_konstan("", ""));
    }

    #[test]
    fn token_sesi_unik_dan_berlaku() {
        let a = buat_sesi();
        let b = buat_sesi();
        assert_ne!(a, b);
        assert_eq!(a.len(), 64);
        assert!(sesi_valid(&a));
        hapus_sesi(&a);
        assert!(!sesi_valid(&a));
        hapus_sesi(&b);
    }

    #[test]
    fn halaman_masuk_meloloskan_pesan() {
        let html = halaman_masuk(Some("<script>x</script>"));
        assert!(!html.contains("<script>x"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn pembatasan_percobaan_menaikkan_penundaan() {
        let _k = kunci_uji();
        let kunci = "uji-pembatas";
        hapus_percobaan(kunci);
        for i in 1..=MAKS_PERCOBAAN {
            let n = catat_gagal(kunci);
            assert_eq!(n, i, "penghitung per alamat bertambah");
        }
        // Setelah ambang, penundaan naik berlipat dan berhenti di batas atas.
        assert!(tunda_percobaan(MAKS_PERCOBAAN + 1) > tunda_percobaan(MAKS_PERCOBAAN));
        assert!(tunda_percobaan(MAKS_PERCOBAAN + 2) > tunda_percobaan(MAKS_PERCOBAAN + 1));
        assert_eq!(
            tunda_percobaan(MAKS_PERCOBAAN + 50),
            Duration::from_millis(TUNDA_MAKS_MS)
        );
        // Masuk berhasil menghapus catatan sehingga tidak lagi terdampak.
        hapus_percobaan(kunci);
        assert_eq!(catat_gagal(kunci), 1, "penghitung mulai dari awal lagi");
    }

    #[test]
    fn batas_global_menghitung_walau_alamat_berbeda() {
        let _k = kunci_uji();
        // Kosongkan penghitung global lebih dulu.
        *global().lock().unwrap() = (0, Instant::now() + JENDELA_PERCOBAAN);
        assert_eq!(catat_gagal_global(), 1);
        // Percobaan dari banyak "alamat" tetap menaikkan penghitung global.
        for _ in 0..(MAKS_GLOBAL - 2) {
            catat_gagal_global();
        }
        assert_eq!(catat_gagal_global(), MAKS_GLOBAL);
        assert!(tunda_percobaan(MAKS_GLOBAL) > tunda_percobaan(MAKS_PERCOBAAN));
        // Masuk yang sah mengosongkan penghitung.
        hapus_percobaan("alamat-apa-saja");
        assert_eq!(catat_gagal_global(), 1);
    }

    /// Verifikasi paralel harus tetap berurutan: laju percobaan salah dibatasi
    /// oleh gerbang, sehingga percobaan serentak tidak dapat menyelinap.
    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn verifikasi_paralel_tetap_dibatasi() {
        let _k = kunci_uji();
        *global().lock().unwrap() = (0, Instant::now() + JENDELA_PERCOBAAN);
        let mulai = Instant::now();
        let tugas: Vec<_> = (0..3)
            .map(|_| tokio::spawn(verifikasi("salah", "uji-paralel")))
            .collect();
        for t in tugas {
            assert!(matches!(t.await.unwrap(), Hasil::Salah));
        }
        // Tiga penundaan berturut-turut (3 x 0,3 detik) minimal ~0,9 detik bila
        // benar-benar diserialkan; tanpa gerbang hasilnya jauh lebih kecil.
        assert!(
            mulai.elapsed() >= Duration::from_millis(900),
            "percobaan paralel tidak diserialkan: {:?}",
            mulai.elapsed()
        );
    }

    /// Kata sandi benar harus lolos walaupun antrean percobaan salah menumpuk.
    ///
    /// Penundaan percobaan salah tidak boleh menahan operator (CWE-400): jalur
    /// kata sandi benar tidak mengambil izin gerbang sama sekali.
    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn kata_sandi_benar_tidak_terhalang_antrean() {
        let _k = kunci_uji();
        let benar = kata_sandi_uji();
        *global().lock().unwrap() = (0, Instant::now() + JENDELA_PERCOBAAN);
        // Percobaan salah yang panjang menahan izin gerbang.
        let penyerang = tokio::spawn(verifikasi("salah", "uji-antre"));
        tokio::time::sleep(Duration::from_millis(50)).await;
        // Operator mengirim kata sandi benar di tengah penundaan penyerang.
        let mulai = Instant::now();
        let hasil = verifikasi(benar, "uji-operator").await;
        let tunggu = mulai.elapsed();
        assert!(matches!(hasil, Hasil::Sukses(_)));
        assert!(
            tunggu < Duration::from_millis(TUNDA_DASAR_MS),
            "operator terhalang antrean: {tunggu:?}"
        );
        let _ = penyerang.await;
    }

    #[test]
    fn tunda_dasar_di_bawah_ambang() {
        for n in 1..=MAKS_PERCOBAAN {
            assert_eq!(tunda_percobaan(n), Duration::from_millis(TUNDA_DASAR_MS));
        }
    }

    #[test]
    fn cookie_secure_kecuali_host_lokal() {
        let buat = |peer: &str, host: &str, proto: Option<&str>| {
            let mut b = Request::builder()
                .uri("/api/login")
                .header(header::HOST, host);
            if let Some(p) = proto {
                b = b.header("x-forwarded-proto", p);
            }
            let mut req = b.body(Body::empty()).unwrap();
            let alamat: SocketAddr = peer.parse().unwrap();
            req.extensions_mut().insert(ConnectInfo(alamat));
            pakai_secure(&req)
        };
        // Peer publik: selalu Secure, apa pun nama host dan header proxy.
        assert!(buat("203.0.113.7:5000", "dasbor.example.go.id", None));
        assert!(buat(
            "203.0.113.7:5000",
            "dasbor.example.go.id",
            Some("http")
        ));
        assert!(buat(
            "203.0.113.7:5000",
            "dasbor.example.go.id",
            Some("https")
        ));
        // Peer publik yang memalsukan Host lokal tetap Secure.
        assert!(buat("203.0.113.7:5000", "localhost:12000", None));
        // Host berawalan lokal tapi bukan host lokal: tetap Secure.
        assert!(buat("127.0.0.1:12000", "localhost.example.com", None));
        assert!(buat("127.0.0.1:12000", "127.0.0.1.example.com", None));
        // Host lokal sungguhan dari peer lokal boleh tanpa Secure (pengembangan).
        assert!(!buat("127.0.0.1:12000", "localhost:12000", None));
        assert!(!buat("127.0.0.1:12000", "127.0.0.1:12000", None));
        assert!(!buat("[::1]:12000", "[::1]:12000", None));
        // Peer lokal tapi host bukan lokal (uji lewat terowongan) tetap Secure.
        assert!(buat("127.0.0.1:12000", "dasbor.example.go.id", None));
    }

    #[test]
    fn kunci_pembatasan_memakai_alamat_soket_bukan_header() {
        let buat = |peer: &str, xff: Option<&str>| {
            let mut b = Request::builder().uri("/api/login");
            if let Some(v) = xff {
                b = b.header("x-forwarded-for", v);
            }
            let mut req = b.body(Body::empty()).unwrap();
            let alamat: SocketAddr = peer.parse().unwrap();
            req.extensions_mut().insert(ConnectInfo(alamat));
            kunci_klien(&req)
        };
        // Header yang dipalsukan tidak mengubah kunci: alamat soket yang dipakai.
        assert_eq!(buat("198.51.100.9:4000", Some("1.2.3.4")), "198.51.100.9");
        assert_eq!(buat("198.51.100.9:4000", Some("9.9.9.9")), "198.51.100.9");
        assert_eq!(buat("198.51.100.9:4000", None), "198.51.100.9");
    }

    #[test]
    fn alamat_lokal_dikenali() {
        assert!(alamat_lokal(&"127.0.0.1".parse().unwrap()));
        assert!(alamat_lokal(&"10.0.0.5".parse().unwrap()));
        assert!(alamat_lokal(&"192.168.1.10".parse().unwrap()));
        assert!(alamat_lokal(&"::1".parse().unwrap()));
        assert!(!alamat_lokal(&"203.0.113.7".parse().unwrap()));
        assert!(!alamat_lokal(&"8.8.8.8".parse().unwrap()));
    }
}
