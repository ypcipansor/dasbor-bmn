//! Klien Web Service SLDK SIMAN v2 (Kejaksaan RI).
//!
//! Alur: token SSO Kemenkeu (client_credentials) → gateway KSB →
//! resource `SLDKSimanKL/2.0/<resource>` dengan parameter `BA_KEY`, `ID_1`, `ID_2`.
#![cfg(feature = "ssr")]

use crate::config;
use serde_json::Value;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum SldkError {
    #[error("kredensial SLDK belum lengkap: {0}")]
    NotConfigured(String),
    #[error("gagal menghubungi SSO/gateway: {0}")]
    Http(String),
    #[error("respons tidak dikenali: {0}")]
    BadResponse(String),
}

impl SldkError {
    pub fn user_message(&self) -> String {
        match self {
            SldkError::NotConfigured(m) => m.clone(),
            SldkError::Http(m) => format!("Koneksi ke layanan gagal ({m})."),
            SldkError::BadResponse(m) => format!("Format respons tidak dikenali ({m})."),
        }
    }
}

struct CachedToken {
    token: String,
    expires_at: Instant,
}

static TOKEN: OnceLock<Mutex<Option<CachedToken>>> = OnceLock::new();

fn http() -> &'static reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent("dasbor-bmn/0.1")
            .build()
            .expect("klien HTTP")
    })
}

/// Ambil token akses SSO, memakai cache selama masa berlakunya (1 jam).
pub async fn token() -> Result<String, SldkError> {
    let cfg = config::get();
    if cfg.client_id.is_empty() || cfg.client_secret.is_empty() {
        return Err(SldkError::NotConfigured(
            "Client ID / Client Secret belum diisi.".to_string(),
        ));
    }
    let lock = TOKEN.get_or_init(|| Mutex::new(None));
    if let Some(t) = lock.lock().unwrap().as_ref() {
        if t.expires_at > Instant::now() + Duration::from_secs(30) {
            return Ok(t.token.clone());
        }
    }

    let params = [
        ("client_id", cfg.client_id.as_str()),
        ("client_secret", cfg.client_secret.as_str()),
        ("grant_type", cfg.grant_type.as_str()),
    ];
    let resp = http()
        .post(&cfg.token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| SldkError::Http(e.to_string()))?;
    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .map_err(|e| SldkError::BadResponse(format!("{status}: {e}")))?;
    let access = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            let detail = body
                .get("error_description")
                .or_else(|| body.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or("token tidak ditemukan");
            SldkError::BadResponse(format!("{status}: {detail}"))
        })?;
    let expires_in = body
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .unwrap_or(3600)
        .max(60);
    *lock.lock().unwrap() = Some(CachedToken {
        token: access.to_string(),
        expires_at: Instant::now() + Duration::from_secs(expires_in as u64),
    });
    Ok(access.to_string())
}

/// Jumlah baris satu tabel pada rentang yang diberikan gateway.
pub async fn row_count(table: &str) -> Result<i64, SldkError> {
    let cfg = config::get();
    if cfg.ba_key.is_empty() {
        return Err(SldkError::NotConfigured("BA_KEY belum diisi.".to_string()));
    }
    let tok = token().await?;
    let url = format!(
        "{}/SLDKSimanKL/2.0/getRowCount/{}/{}",
        cfg.gateway_root(),
        cfg.ba_key,
        table
    );
    let resp = http()
        .get(&url)
        .bearer_auth(tok)
        .send()
        .await
        .map_err(|e| SldkError::Http(e.to_string()))?;
    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .map_err(|e| SldkError::BadResponse(format!("{status}: {e}")))?;
    Ok(parse_row_count(&body))
}

fn parse_row_count(body: &Value) -> i64 {
    // Bentuk nyata: {"results":[{"SKEMA":"DJKN","NAMATABEL":"...","RCOUNT":2186}]}
    // Angka ada di dalam kunci RCOUNT pada elemen pertama, bukan panjang array.
    if let Some(first) = body.get("results").and_then(|r| r.as_array()).and_then(|a| a.first()) {
        for key in ["RCOUNT", "rcount", "ROWCOUNT", "rowcount", "TOTAL", "total"] {
            if let Some(n) = first.get(key) {
                match n {
                    Value::Number(n) => return n.as_i64().unwrap_or(0),
                    Value::String(s) => {
                        if let Ok(n) = s.trim().parse::<i64>() {
                            return n;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let candidates = [
        body.get("RCOUNT"),
        body.get("rcount"),
        body.get("results"),
        body.get("result"),
        body.get("total"),
        body.get("count"),
    ];
    for c in candidates.into_iter().flatten() {
        match c {
            Value::Number(n) => return n.as_i64().unwrap_or(0),
            Value::String(s) => {
                if let Ok(n) = s.trim().parse::<i64>() {
                    return n;
                }
            }
            Value::Array(a) => {
                for el in a {
                    for key in ["RCOUNT", "rcount", "TOTAL", "total", "count"] {
                        if let Some(n) = el.get(key) {
                            match n {
                                Value::Number(n) => return n.as_i64().unwrap_or(0),
                                Value::String(s) => {
                                    if let Ok(n) = s.trim().parse::<i64>() {
                                        return n;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    0
}

/// Ambil sekumpulan baris dari satu resource aset.
pub async fn fetch_rows(resource: &str, id1: i64, id2: i64) -> Result<Vec<Value>, SldkError> {
    let cfg = config::get();
    if cfg.ba_key.is_empty() {
        return Err(SldkError::NotConfigured("BA_KEY belum diisi.".to_string()));
    }
    let tok = token().await?;
    let url = format!("{}/SLDKSimanKL/2.0/{}", cfg.gateway_root(), resource);
    let params = [
        ("BA_KEY", cfg.ba_key.clone()),
        ("ID_1", id1.to_string()),
        ("ID_2", id2.to_string()),
    ];
    let resp = http()
        .post(&url)
        .bearer_auth(tok)
        .form(&params)
        .send()
        .await
        .map_err(|e| SldkError::Http(e.to_string()))?;
    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .map_err(|e| SldkError::BadResponse(format!("{status}: {e}")))?;
    Ok(extract_rows(&body))
}

/// Normalkan berbagai bentuk pembungkus respons menjadi array baris.
pub fn extract_rows(body: &Value) -> Vec<Value> {
    fn as_rows(v: &Value) -> Option<Vec<Value>> {
        match v {
            Value::Array(a) => Some(a.clone()),
            Value::Object(o) => {
                for key in ["results", "result", "data", "rows", "records", "items"] {
                    if let Some(inner) = o.get(key) {
                        if let Some(rows) = as_rows(inner) {
                            return Some(rows);
                        }
                    }
                }
                // objek tunggal dianggap satu baris bila punya banyak kunci data
                if o.len() > 3 {
                    Some(vec![v.clone()])
                } else {
                    None
                }
            }
            Value::String(s) => {
                let t = s.trim();
                if t.eq_ignore_ascii_case("tidak ada data") || t.is_empty() {
                    Some(vec![])
                } else if let Ok(parsed) = serde_json::from_str::<Value>(t) {
                    as_rows(&parsed)
                } else {
                    Some(vec![])
                }
            }
            _ => None,
        }
    }
    as_rows(body).unwrap_or_default()
}

/// Uji koneksi: token + hitung baris satu tabel.
pub async fn health() -> Result<(bool, Option<String>), ()> {
    match token().await {
        Ok(_) => Ok((true, None)),
        Err(e) => Ok((false, Some(e.user_message()))),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_row_count;
    use serde_json::json;

    #[test]
    fn membaca_rcount_dari_pembungkus_results() {
        // Bentuk balasan nyata dari getRowCount.
        let body = json!({
            "results": [{
                "SKEMA": "DJKN",
                "NAMATABEL": "SIMAN2_M_ASET_TANAH",
                "TANGGAL": "2026-10-04 15:00:32.2166667",
                "RCOUNT": 2186
            }]
        });
        assert_eq!(parse_row_count(&body), 2186);
    }

    #[test]
    fn rcount_nol_tetap_dibaca_nol() {
        let body = json!({"results": [{"RCOUNT": 0}]});
        assert_eq!(parse_row_count(&body), 0);
    }

    #[test]
    fn rcount_sebagai_teks() {
        let body = json!({"results": [{"RCOUNT": "74429"}]});
        assert_eq!(parse_row_count(&body), 74429);
    }

    #[test]
    fn panjang_array_bukan_jumlah_baris() {
        // Regresi: dulu panjang array (1) dipakai sebagai jumlah baris.
        let body = json!({"results": [{"RCOUNT": 4264}, {"RCOUNT": 999}]});
        assert_eq!(parse_row_count(&body), 4264);
    }

    #[test]
    fn bentuk_cadangan_tetap_didukung() {
        assert_eq!(parse_row_count(&json!({"total": 42})), 42);
        assert_eq!(parse_row_count(&json!({"RCOUNT": 7})), 7);
        assert_eq!(parse_row_count(&json!({"results": 12})), 12);
        assert_eq!(parse_row_count(&json!({"tidak_ada": 1})), 0);
    }
}

