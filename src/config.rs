#[cfg(feature = "ssr")]
use std::sync::{OnceLock, RwLock};

/// Konfigurasi koneksi SLDK SIMAN v2.
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub base_url: String,
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub grant_type: String,
    pub ba_key: String,
}

#[cfg(feature = "ssr")]
fn env_any(keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| std::env::var(k).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(feature = "ssr")]
fn config_path() -> std::path::PathBuf {
    let dir = env_any(&["SIMAN_DATA_DIR"]).unwrap_or_else(|| "data".to_string());
    std::path::PathBuf::from(dir).join("config.json")
}

/// Nilai yang boleh ditimpa dari berkas `data/config.json` (tanpa base_url/token_url).
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FileConfig {
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub grant_type: String,
    #[serde(default)]
    pub ba_key: String,
}

#[cfg(feature = "ssr")]
static CONFIG: OnceLock<RwLock<Config>> = OnceLock::new();

#[cfg(feature = "ssr")]
fn read_file_config() -> FileConfig {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[cfg(feature = "ssr")]
fn load() -> Config {
    let file = read_file_config();
    let pick = |env_keys: &[&str], file_val: &str, default: &str| -> String {
        env_any(env_keys).unwrap_or_else(|| {
            if file_val.trim().is_empty() {
                default.to_string()
            } else {
                file_val.trim().to_string()
            }
        })
    };
    Config {
        base_url: env_any(&["SIMAN_BASE_URL", "BASE_URL"])
            .unwrap_or_else(|| "https://apigateway.kemenkeu.go.id".to_string()),
        token_url: env_any(&["SIMAN_TOKEN_URL"])
            .unwrap_or_else(|| "https://sso.kemenkeu.go.id/connect/token".to_string()),
        client_id: pick(&["SIMAN_CLIENT_ID", "CLIENT_ID"], &file.client_id, ""),
        client_secret: pick(
            &["SIMAN_CLIENT_SECRET", "CLIENT_SECRET"],
            &file.client_secret,
            "",
        ),
        grant_type: pick(
            &["SIMAN_GRANT_TYPE", "GRANT_TYPE"],
            &file.grant_type,
            "client_credentials",
        ),
        ba_key: pick(&["SIMAN_BA_KEY", "BA_KEY"], &file.ba_key, ""),
    }
}

/// Konfigurasi aktif; dimuat sekali dari environment lalu dapat ditimpa dari UI.
#[cfg(feature = "ssr")]
pub fn get() -> Config {
    CONFIG.get_or_init(|| RwLock::new(load())).read().unwrap().clone()
}

/// Simpan konfigurasi ke `data/config.json` dan terapkan segera.
#[cfg(feature = "ssr")]
pub fn save_file_config(patch: FileConfig) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut current = read_file_config();
    if !patch.client_id.is_empty() {
        current.client_id = patch.client_id;
    }
    if !patch.client_secret.is_empty() {
        current.client_secret = patch.client_secret;
    }
    if !patch.grant_type.is_empty() {
        current.grant_type = patch.grant_type;
    }
    if !patch.ba_key.is_empty() {
        current.ba_key = patch.ba_key;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&current)?)?;
    let mut c = get();
    c.client_id = current.client_id.clone();
    c.client_secret = current.client_secret.clone();
    c.grant_type = current.grant_type.clone();
    c.ba_key = current.ba_key.clone();
    *CONFIG.get_or_init(|| RwLock::new(load())).write().unwrap() = c;
    Ok(())
}

impl Config {
    /// True bila kredensial inti sudah tersedia sehingga mode demo tidak diperlukan.
    pub fn is_configured(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret.is_empty() && !self.ba_key.is_empty()
    }

    pub fn gateway_root(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/gateway") {
            base.to_string()
        } else {
            format!("{base}/gateway")
        }
    }
}

/// Data direktori penyimpanan lokal (cache & konfigurasi).
#[cfg(feature = "ssr")]
pub fn data_dir() -> std::path::PathBuf {
    let dir = env_any(&["SIMAN_DATA_DIR"]).unwrap_or_else(|| "data".to_string());
    let p = std::path::PathBuf::from(dir);
    let _ = std::fs::create_dir_all(&p);
    p
}
