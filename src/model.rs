use serde::{Deserialize, Serialize};

/// Definisi satu tabel BMN pada layanan SLDK SIMAN v2.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TableDef {
    /// Nama tabel pada SIMAN, mis. `SIMAN2_M_ASET_TANAH`.
    pub table: &'static str,
    /// Nama resource pada gateway, mis. `getAsetTanah`.
    pub resource: &'static str,
    /// Label singkat yang ramah pengguna, mis. "Tanah".
    pub label: &'static str,
    /// Nama data resmi dari dokumen SLDK.
    pub nama_data: &'static str,
    /// Ukuran volume data menurut dokumen SLDK.
    pub volume: &'static str,
    /// Ikon (emoji) untuk kartu kategori.
    pub icon: &'static str,
    /// Kolom yang dipakai sebagai sumbu nilai (Rp).
    pub value_column: &'static str,
}

/// Metadata satu kolom pada tabel BMN.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub label: String,
    pub r#type: String,
    pub len: String,
}

/// Kategori aset yang dipakai pada agregasi dasbor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryStat {
    pub label: String,
    pub icon: String,
    pub jumlah: i64,
    pub nilai: f64,
    pub volume: String,
}

/// Agregat sebaran pada satu dimensi (provinsi, satker, kondisi, ...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bucket {
    pub name: String,
    pub jumlah: i64,
    pub nilai: f64,
}

/// Titik data tren perolehan per tahun.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrendPoint {
    pub year: i32,
    pub jumlah: i64,
    pub nilai: f64,
}

/// Ringkasan utama dasbor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Overview {
    pub total_aset: i64,
    pub total_nilai: f64,
    pub total_susut: f64,
    pub total_kategori: i64,
    pub total_provinsi: i64,
    pub total_satker: i64,
    pub aset_idle: i64,
    pub aset_hilang: i64,
    pub aset_rusak: i64,
    pub categories: Vec<CategoryStat>,
    pub provinsi: Vec<Bucket>,
    pub satker: Vec<Bucket>,
    pub kondisi: Vec<Bucket>,
    pub perolehan: Vec<TrendPoint>,
    pub sumber_dana: Vec<Bucket>,
    pub is_demo: bool,
    pub generated_at: String,
}

/// Status sinkronisasi satu tabel terhadap sumber SLDK.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncStatus {
    pub table: String,
    pub label: String,
    pub row_count: i64,
    pub last_sync: Option<String>,
    pub status: String,
    pub message: Option<String>,
}

/// Satu baris data aset yang sudah dinormalisasi untuk tampilan tabel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetRow {
    pub values: Vec<String>,
}

/// Halaman data aset beserta metadata paginasi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetPage {
    pub table: String,
    pub label: String,
    pub columns: Vec<ColumnDef>,
    pub rows: Vec<AssetRow>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub is_demo: bool,
}

/// Konfigurasi koneksi yang aktif (nilai rahasia tidak pernah dikirim ke klien).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnInfo {
    pub base_url_set: bool,
    pub client_id: Option<String>,
    pub grant_type: Option<String>,
    pub ba_key: Option<String>,
    pub client_secret_set: bool,
    pub token_valid: bool,
    pub last_error: Option<String>,
}

impl Overview {
    pub fn is_empty(&self) -> bool {
        self.total_aset == 0
    }
}

/// Format angka rupiah ringkas, mis. `Rp 1,2 T`.
pub fn format_rupiah_short(v: f64) -> String {
    let a = v.abs();
    let (div, suf) = if a >= 1e12 {
        (1e12, " T")
    } else if a >= 1e9 {
        (1e9, " M")
    } else if a >= 1e6 {
        (1e6, " Jt")
    } else if a >= 1e3 {
        (1e3, " Rb")
    } else {
        (1.0, "")
    };
    let n = v / div;
    let s = if n.fract().abs() < 0.05 {
        format!("{:.0}", n)
    } else {
        format!("{:.1}", n)
    };
    format!("Rp {}{}", s.replace('.', ","), suf)
}

/// Format angka dengan pemisah ribuan gaya Indonesia.
pub fn format_number(v: i64) -> String {
    let neg = v < 0;
    let s = v.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    let s: String = out.chars().rev().collect();
    if neg {
        format!("-{s}")
    } else {
        s
    }
}

/// Format rupiah penuh gaya Indonesia.
pub fn format_rupiah(v: f64) -> String {
    format!("Rp {}", format_number(v.round() as i64))
}

/// Ubah nilai sel menjadi teks tampilan yang aman.
pub fn cell_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => (if *b { "Ya" } else { "Tidak" }).to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Ambil angka dari nilai JSON dengan toleransi string berformat.
pub fn to_f64(v: &serde_json::Value) -> f64 {
    match v {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
        serde_json::Value::String(s) => {
            let cleaned: String = s
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect();
            cleaned.parse().unwrap_or(0.0)
        }
        serde_json::Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}
