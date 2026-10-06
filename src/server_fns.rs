use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::catalog;
#[cfg(feature = "ssr")]
use crate::{analytics, config, demo, sldk, store};

use crate::model::{AssetPage, ConnInfo, Overview, SyncStatus};

/// Metadata satu tabel beserta kolomnya untuk kebutuhan tampilan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableMeta {
    pub table: String,
    pub resource: String,
    pub label: String,
    pub nama_data: String,
    pub volume: String,
    pub icon: String,
    pub columns: Vec<crate::model::ColumnDef>,
}

/// Ringkasan dasbor; otomatis memakai data contoh bila kredensial belum ada.
#[server]
pub async fn get_overview() -> Result<Overview, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        if !config::get().is_configured() && store::all_stats().is_empty() {
            demo::seed();
        }
        let stats = store::all_stats();
        let parts: Vec<(String, String, String, analytics::TableStats)> = stats
            .into_iter()
            .filter_map(|(table, st, _)| {
                catalog::find(&table).map(|t| {
                    (
                        t.label.to_string(),
                        t.icon.to_string(),
                        t.volume.to_string(),
                        st,
                    )
                })
            })
            .collect();
        let mut o = analytics::merge(&parts, demo::is_demo());
        o.generated_at = crate::now_string();
        Ok(o)
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("hanya tersedia di server"))
}

/// Daftar tabel BMN beserta definisi kolomnya.
#[server]
pub async fn get_tables() -> Result<Vec<TableMeta>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        Ok(catalog::TABLES
            .iter()
            .map(|t| TableMeta {
                table: t.table.to_string(),
                resource: t.resource.to_string(),
                label: t.label.to_string(),
                nama_data: t.nama_data.to_string(),
                volume: t.volume.to_string(),
                icon: t.icon.to_string(),
                columns: crate::schema::columns_of(t.table),
            })
            .collect())
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("hanya tersedia di server"))
}

/// Status sinkronisasi tiap tabel.
#[server]
pub async fn get_sync_statuses() -> Result<Vec<SyncStatus>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        Ok(store::statuses_with_labels())
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("hanya tersedia di server"))
}

/// Informasi koneksi aktif (tanpa membocorkan rahasia).
#[server]
pub async fn get_conn_info() -> Result<ConnInfo, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let cfg = config::get();
        let (token_valid, last_error) = match sldk::token().await {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.user_message())),
        };
        Ok(ConnInfo {
            base_url_set: !cfg.base_url.is_empty(),
            client_id: if cfg.client_id.is_empty() {
                None
            } else {
                Some(cfg.client_id.clone())
            },
            grant_type: if cfg.grant_type.is_empty() {
                None
            } else {
                Some(cfg.grant_type.clone())
            },
            ba_key: if cfg.ba_key.is_empty() {
                None
            } else {
                Some(mask(&cfg.ba_key))
            },
            client_secret_set: !cfg.client_secret.is_empty(),
            token_valid,
            last_error,
        })
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("hanya tersedia di server"))
}

#[cfg(feature = "ssr")]
fn mask(v: &str) -> String {
    let n = v.chars().count();
    if n <= 4 {
        "•".repeat(n.max(1))
    } else {
        format!("{}{}", "•".repeat(n - 4), &v[n - 4..])
    }
}

/// Simpan kredensial yang diisi pengguna ke berkas konfigurasi lokal.
#[server]
pub async fn save_conn_config(
    client_id: String,
    client_secret: String,
    grant_type: String,
    ba_key: String,
) -> Result<ConnInfo, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let patch = config::FileConfig {
            client_id: client_id.trim().to_string(),
            client_secret: client_secret.trim().to_string(),
            grant_type: if grant_type.trim().is_empty() {
                "client_credentials".to_string()
            } else {
                grant_type.trim().to_string()
            },
            ba_key: ba_key.trim().to_string(),
        };
        config::save_file_config(patch)
            .map_err(|e| ServerFnError::new(format!("gagal menyimpan: {e}")))?;
        let cfg = config::get();
        let (token_valid, last_error) = match sldk::token().await {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.user_message())),
        };
        Ok(ConnInfo {
            base_url_set: true,
            client_id: Some(cfg.client_id.clone()),
            grant_type: Some(cfg.grant_type.clone()),
            ba_key: Some(mask(&cfg.ba_key)),
            client_secret_set: !cfg.client_secret.is_empty(),
            token_valid,
            last_error,
        })
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("hanya tersedia di server"))
}

/// Sinkronkan satu tabel dari SLDK; memakai jalur nyata bila kredensial lengkap.
#[server]
pub async fn sync_table(table: String) -> Result<SyncStatus, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let Some(def) = catalog::find(&table) else {
            return Err(ServerFnError::new("tabel tidak dikenal"));
        };
        let cfg = config::get();
        let label = def.label.to_string();
        let now = crate::now_string();

        if !cfg.is_configured() {
            demo::seed();
            let st = store::all_stats()
                .into_iter()
                .find(|(t, _, _)| t == &table)
                .map(|(_, s, _)| s)
                .unwrap_or_default();
            return Ok(SyncStatus {
                table: table.clone(),
                label,
                row_count: st.row_count,
                last_sync: Some(now),
                status: "demo".into(),
                message: Some("Kredensial SLDK belum diisi — memakai data contoh.".into()),
            });
        }

        let total = sldk::row_count(&table)
            .await
            .map_err(|e| ServerFnError::new(e.user_message()))?;

        // Sumber kosong tidak perlu diambil sama sekali.
        if total <= 0 {
            let stats = analytics::TableStats::default();
            // Satu transaksi: buang data contoh (bila ada) sekaligus ganti tabel
            // dan tandai mode langsung, agar kegagalan tidak meninggalkan dasbor kosong.
            store::replace_table_transaksional(&table, &[], &stats, &now, demo::is_demo())
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            return Ok(SyncStatus {
                table: table.clone(),
                label,
                row_count: 0,
                last_sync: Some(now),
                status: "tersinkron".into(),
                message: Some("Sumber tidak memuat baris untuk tabel ini.".into()),
            });
        }

        let mut rows: Vec<serde_json::Value> = Vec::new();
        let page = 1000i64;
        let mut id1 = 1i64;
        while (id1 - 1) < total {
            let chunk = sldk::fetch_rows(def.resource, id1, id1 + page - 1)
                .await
                .map_err(|e| ServerFnError::new(e.user_message()))?;
            match chunk {
                None => break,
                Some(c) if c.is_empty() => break,
                Some(c) => rows.extend(c),
            }
            id1 += page;
        }

        // Unduhan yang tidak lengkap tidak boleh menimpa cache lama.
        if (rows.len() as i64) < total {
            return Err(ServerFnError::new(format!(
                "Unduhan tidak lengkap: terkumpul {} dari {} baris. Cache lama dipertahankan; coba sinkronkan lagi.",
                rows.len(),
                total
            )));
        }

        // Data contoh dibersihkan sebelum data langsung pertama menggantikannya,
        // agar kategori yang belum tersinkron tidak ikut berlabel langsung.
        // Penghapusan dan penulisan berbagi satu transaksi supaya kegagalan
        // penulisan tidak meninggalkan dasbor kosong tanpa data contoh.
        let stats = analytics::aggregate(&rows);
        store::replace_table_transaksional(&table, &rows, &stats, &now, demo::is_demo())
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(SyncStatus {
            table: table.clone(),
            label,
            row_count: stats.row_count,
            last_sync: Some(now),
            status: "tersinkron".into(),
            message: None,
        })
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("hanya tersedia di server"))
}

/// Sinkronkan seluruh tabel secara berurutan.
#[server]
pub async fn sync_all() -> Result<Vec<SyncStatus>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let mut out = Vec::new();
        for t in catalog::TABLES {
            match sync_table(t.table.to_string()).await {
                Ok(s) => out.push(s),
                Err(e) => out.push(SyncStatus {
                    table: t.table.to_string(),
                    label: t.label.to_string(),
                    row_count: 0,
                    last_sync: None,
                    status: "gagal".into(),
                    message: Some(e.to_string()),
                }),
            }
        }
        Ok(out)
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("hanya tersedia di server"))
}

/// Hapus cache lokal (kembali ke mode contoh bila kredensial belum diisi).
#[server]
pub async fn reset_cache() -> Result<Vec<SyncStatus>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        store::clear_all().map_err(|e| ServerFnError::new(e.to_string()))?;
        store::delete_meta("mode").ok();
        if !config::get().is_configured() {
            demo::seed();
        }
        Ok(store::statuses_with_labels())
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("hanya tersedia di server"))
}

/// Satu halaman data aset dengan pencarian dan pengurutan.
#[server]
pub async fn get_asset_page(
    table: String,
    page: i64,
    per_page: i64,
    q: String,
    sort: Option<String>,
    dir: String,
) -> Result<AssetPage, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let Some(def) = catalog::find(&table) else {
            return Err(ServerFnError::new("tabel tidak dikenal"));
        };
        if !config::get().is_configured() && store::all_stats().is_empty() {
            demo::seed();
        }
        let columns = crate::schema::columns_of(&table);
        let is_demo = demo::is_demo();
        let (rows, total) =
            store::page(&table, &columns, page, per_page, &q, sort.as_deref(), &dir);
        Ok(AssetPage {
            table,
            label: def.label.to_string(),
            columns,
            rows,
            page,
            per_page,
            total,
            is_demo,
        })
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("hanya tersedia di server"))
}

/// Netralkan nilai sel agar tidak dieksekusi sebagai formula spreadsheet.
///
/// Sel yang dimulai dengan `= + - @` (atau tab/CR) diawali apostrof sehingga
/// Excel/LibreOffice memperlakukannya sebagai teks (CWE-1236).
pub fn amankan_sel(v: &str) -> String {
    let berbahaya = v
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'));
    if berbahaya {
        format!("'{v}")
    } else {
        v.to_string()
    }
}

/// Data CSV untuk satu tabel (untuk ekspor).
#[server]
pub async fn export_csv(table: String) -> Result<String, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        if catalog::find(&table).is_none() {
            return Err(ServerFnError::new("tabel tidak dikenal"));
        }
        let columns = crate::schema::columns_of(&table);

        let mut out = String::new();
        out.push_str(
            &columns
                .iter()
                .map(|c| c.label.replace('"', "\"\""))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');

        // Satu kueri berurutan stabil dari koneksi baca-saja tersendiri: seluruh
        // baris berasal dari snapshot yang sama dan tabel tidak dipindai berulang.
        store::alir_baris(&table, |v| {
            let line: Vec<String> = columns
                .iter()
                .map(|c| {
                    let raw = v
                        .get(&c.name)
                        .map(crate::model::cell_to_string)
                        .unwrap_or_default();
                    format!("\"{}\"", amankan_sel(&raw).replace('"', "\"\""))
                })
                .collect();
            out.push_str(&line.join(","));
            out.push('\n');
        })
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(out)
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("hanya tersedia di server"))
}

/// Jumlah baris pada sumber SLDK untuk satu tabel (untuk pratinjau sinkronisasi).
#[server]
pub async fn preview_row_count(table: String) -> Result<i64, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        if !config::get().is_configured() {
            return Err(ServerFnError::new("Kredensial SLDK belum diisi."));
        }
        sldk::row_count(&table)
            .await
            .map_err(|e| ServerFnError::new(e.user_message()))
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("hanya tersedia di server"))
}

/// Nama tabel yang dipakai untuk tabel data (untuk validasi klien).
pub fn table_options() -> Vec<(String, String)> {
    catalog::TABLES
        .iter()
        .map(|t| (t.table.to_string(), t.label.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::amankan_sel;

    #[test]
    fn sel_formula_dinetralkan() {
        assert_eq!(amankan_sel("=1+1"), "'=1+1");
        assert_eq!(amankan_sel("+SUM(A1)"), "'+SUM(A1)");
        assert_eq!(amankan_sel("-2+3"), "'-2+3");
        assert_eq!(amankan_sel("@cmd"), "'@cmd");
        assert_eq!(amankan_sel("\t=1"), "'\t=1");
    }

    #[test]
    fn sel_biasa_tidak_diubah() {
        assert_eq!(amankan_sel("Kantor Kejaksaan"), "Kantor Kejaksaan");
        assert_eq!(amankan_sel(""), "");
        assert_eq!(amankan_sel("123"), "123");
        assert_eq!(amankan_sel("A2"), "A2");
    }

    #[test]
    fn ekspor_memakai_seluruh_baris_satu_kueri() {
        // Ekspor besar tidak boleh lagi memindai tabel berulang per halaman;
        // ia memakai alir_baris satu kueri berurutan stabil.
        let _kunci = crate::store::uji_kunci();
        crate::store::uji_isolasi();
        crate::store::conn();
        let t = "UJI_EKSPOR";
        let rows: Vec<serde_json::Value> = (0..25)
            .map(|i| serde_json::json!({"kd_brg": format!("K{i}"), "rph_aset": i}))
            .collect();
        let stats = crate::analytics::aggregate(&rows);
        crate::store::replace_table(t, &rows, &stats, "t0").unwrap();

        let mut n = 0usize;
        crate::store::alir_baris(t, |_| n += 1).unwrap();
        assert_eq!(n, 25, "seluruh baris terbaca dalam sekali alir");
    }
}
