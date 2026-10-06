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
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::Request;
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
/// Percobaan gagal maksimum per jendela sebelum alamat dikunci sementara.
const MAKS_PERCOBAAN: u32 = 5;

fn sessions() -> &'static Mutex<HashMap<String, Instant>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn percobaan() -> &'static Mutex<HashMap<String, (u32, Instant)>> {
    PERCOBAAN.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Kunci pembatasan: alamat klien dari header proxy, atau "lokal".
fn kunci_klien(req: &Request) -> String {
    for nama in ["x-forwarded-for", "x-real-ip"] {
        if let Some(v) = req.headers().get(nama).and_then(|v| v.to_str().ok()) {
            let first = v.split(',').next().unwrap_or("").trim();
            if !first.is_empty() {
                return first.to_string();
            }
        }
    }
    "lokal".to_string()
}

/// Berapa detik lagi alamat ini harus menunggu sebelum boleh mencoba lagi.
fn sisa_tunggu(kunci: &str) -> Option<u64> {
    let mut p = percobaan().lock().unwrap();
    let now = Instant::now();
    p.retain(|_, (_, until)| *until > now);
    let (jumlah, until) = *p.get(kunci)?;
    if jumlah >= MAKS_PERCOBAAN && until > now {
        Some((until - now).as_secs().max(1))
    } else {
        None
    }
}

/// Catat satu percobaan gagal; kembalikan sisa tunggu bila sudah terkunci.
fn catat_gagal(kunci: &str) -> Option<u64> {
    let mut p = percobaan().lock().unwrap();
    let now = Instant::now();
    p.retain(|_, (_, until)| *until > now);
    let entri = p
        .entry(kunci.to_string())
        .or_insert((0, now + JENDELA_PERCOBAAN));
    entri.0 += 1;
    entri.1 = now + JENDELA_PERCOBAAN;
    if entri.0 >= MAKS_PERCOBAAN {
        Some((entri.1 - now).as_secs().max(1))
    } else {
        None
    }
}

/// Hapus catatan percobaan setelah masuk berhasil.
fn hapus_percobaan(kunci: &str) {
    percobaan().lock().unwrap().remove(kunci);
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
/// Gagal-tertutup: `Secure` dipasang secara bawaan, dan hanya dilepas bila
/// permintaan jelas berasal dari host lokal untuk pengembangan. Dengan begitu
/// sesi tidak pernah dapat dikirim lewat HTTP biasa hanya karena proxy lupa
/// mengirim `x-forwarded-proto` (CWE-614).
fn pakai_secure(req: &Request) -> bool {
    if permintaan_https(req) {
        return true;
    }
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let lokal =
        host.starts_with("localhost") || host.starts_with("127.0.0.1") || host.starts_with("[::1]");
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
    if let Some(sisa) = sisa_tunggu(&kunci) {
        return halaman_galat(&format!(
            "Terlalu banyak percobaan. Coba lagi dalam {sisa} detik."
        ));
    }
    let body = match axum::body::to_bytes(req.into_body(), 8 * 1024).await {
        Ok(b) => b,
        Err(_) => return halaman_galat("Permintaan tidak terbaca."),
    };
    let form: FormMasuk = match serde_urlencoded::from_bytes(&body) {
        Ok(f) => f,
        Err(_) => return halaman_galat("Permintaan tidak terbaca."),
    };
    if !cocok(&form.password) {
        // Perlambat sedikit agar percobaan berturut-turut tidak gratis.
        tokio::time::sleep(Duration::from_millis(300)).await;
        if let Some(sisa) = catat_gagal(&kunci) {
            return halaman_galat(&format!(
                "Terlalu banyak percobaan. Coba lagi dalam {sisa} detik."
            ));
        }
        return halaman_galat("Kata sandi salah.");
    }
    hapus_percobaan(&kunci);
    let token = buat_sesi();
    redirect_ke("/", Some(header_cookie(&token, https)))
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
    fn pembatasan_percobaan_mengunci_setelah_batas() {
        let kunci = "uji-pembatas";
        hapus_percobaan(kunci);
        for i in 0..MAKS_PERCOBAAN {
            let hasil = catat_gagal(kunci);
            if i + 1 < MAKS_PERCOBAAN {
                assert!(hasil.is_none(), "belum boleh terkunci pada percobaan {i}");
            } else {
                assert!(hasil.is_some(), "harus terkunci pada percobaan terakhir");
            }
        }
        assert!(sisa_tunggu(kunci).is_some());
        // Masuk berhasil menghapus catatan sehingga tidak lagi terkunci.
        hapus_percobaan(kunci);
        assert!(sisa_tunggu(kunci).is_none());
    }

    #[test]
    fn cookie_secure_kecuali_host_lokal() {
        let buat = |host: &str, proto: Option<&str>| {
            let mut b = Request::builder()
                .uri("/api/login")
                .header(header::HOST, host);
            if let Some(p) = proto {
                b = b.header("x-forwarded-proto", p);
            }
            pakai_secure(&b.body(Body::empty()).unwrap())
        };
        // Produksi tanpa header proxy: tetap Secure.
        assert!(buat("dasbor.example.go.id", None));
        assert!(buat("dasbor.example.go.id", Some("http")));
        // Proxy menyatakan https.
        assert!(buat("dasbor.example.go.id", Some("https")));
        // Host lokal untuk pengembangan boleh tanpa Secure.
        assert!(!buat("localhost:12000", None));
        assert!(!buat("127.0.0.1:12000", None));
    }
}
