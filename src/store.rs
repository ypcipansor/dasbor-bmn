//! Penyimpanan lokal (SQLite) untuk cache baris aset, agregat, dan status sinkronisasi.

#![cfg(feature = "ssr")]

use crate::analytics::TableStats;
use crate::config;
use crate::model::{AssetPage, AssetRow, ColumnDef, SyncStatus};
use rusqlite::{params, Connection};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

/// Buka (dan siapkan) basis data lokal.
pub fn conn() -> &'static Mutex<Connection> {
    DB.get_or_init(|| {
        let path = config::data_dir().join("bmn.sqlite");
        let c = Connection::open(path).expect("buka basis data lokal");
        c.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS assets (
                 table_name TEXT NOT NULL,
                 row_key    TEXT NOT NULL,
                 data       TEXT NOT NULL,
                 PRIMARY KEY (table_name, row_key)
             );
             CREATE INDEX IF NOT EXISTS idx_assets_table ON assets(table_name);
             CREATE TABLE IF NOT EXISTS sync_meta (
                 table_name TEXT PRIMARY KEY,
                 row_count  INTEGER NOT NULL DEFAULT 0,
                 stats      TEXT,
                 synced_at  TEXT
             );
             CREATE TABLE IF NOT EXISTS app_meta (
                 key   TEXT PRIMARY KEY,
                 value TEXT
             );",
        )
        .expect("siapkan skema basis data");
        Mutex::new(c)
    })
}

/// Kunci unik satu baris aset; fallback ke indeks bila kode barang kosong.
pub fn row_key(v: &Value, idx: usize) -> String {
    for col in ["kd_brg", "no_aset", "no_kib", "kode_register"] {
        let s = v
            .get(col)
            .map(crate::model::cell_to_string)
            .unwrap_or_default();
        if !s.trim().is_empty() {
            return format!("{col}:{}", s.trim());
        }
    }
    format!("idx:{idx}")
}

/// Ganti seluruh isi satu tabel dengan data terbaru, lalu simpan agregatnya.
pub fn replace_table(
    table: &str,
    rows: &[Value],
    stats: &TableStats,
    synced_at: &str,
) -> rusqlite::Result<()> {
    let mut c = conn().lock().unwrap();
    let tx = c.transaction()?;
    tx.execute("DELETE FROM assets WHERE table_name = ?1", params![table])?;
    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO assets (table_name, row_key, data) VALUES (?1, ?2, ?3)",
        )?;
        let mut seen: HashSet<String> = HashSet::new();
        for (i, r) in rows.iter().enumerate() {
            let mut k = row_key(r, i);
            if !seen.insert(k.clone()) {
                k = format!("{k}#{i}");
            }
            stmt.execute(params![table, k, r.to_string()])?;
        }
    }
    tx.execute(
        "INSERT INTO sync_meta (table_name, row_count, stats, synced_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(table_name) DO UPDATE SET
            row_count = excluded.row_count,
            stats = excluded.stats,
            synced_at = excluded.synced_at",
        params![
            table,
            stats.row_count,
            serde_json::to_string(stats).unwrap_or_default(),
            synced_at
        ],
    )?;
    tx.commit()
}

/// Jumlah baris tersimpan untuk satu tabel.
pub fn stored_count(table: &str) -> i64 {
    let c = conn().lock().unwrap();
    c.query_row(
        "SELECT COUNT(*) FROM assets WHERE table_name = ?1",
        params![table],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

/// Status sinkronisasi seluruh tabel yang punya metadata.
pub fn sync_statuses() -> Vec<(String, i64, Option<String>)> {
    let c = conn().lock().unwrap();
    let mut stmt = c
        .prepare("SELECT table_name, row_count, synced_at FROM sync_meta ORDER BY table_name")
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .unwrap();
    rows.filter_map(|r| r.ok()).collect()
}

/// Hapus cache satu tabel.
pub fn clear_table(table: &str) -> rusqlite::Result<()> {
    let c = conn().lock().unwrap();
    c.execute("DELETE FROM assets WHERE table_name = ?1", params![table])?;
    c.execute(
        "DELETE FROM sync_meta WHERE table_name = ?1",
        params![table],
    )?;
    Ok(())
}

/// Hapus seluruh cache.
pub fn clear_all() -> rusqlite::Result<()> {
    let c = conn().lock().unwrap();
    c.execute("DELETE FROM assets", [])?;
    c.execute("DELETE FROM sync_meta", [])?;
    Ok(())
}

/// Simpan agregat satu tabel tanpa menyentuh barisnya (dipakai mode demo).
pub fn upsert_stats(
    table: &str,
    stats: &TableStats,
    synced_at: Option<&str>,
) -> rusqlite::Result<()> {
    let c = conn().lock().unwrap();
    c.execute(
        "INSERT INTO sync_meta (table_name, row_count, stats, synced_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(table_name) DO UPDATE SET
            row_count = excluded.row_count,
            stats = excluded.stats,
            synced_at = excluded.synced_at",
        params![
            table,
            stats.row_count,
            serde_json::to_string(stats).unwrap_or_default(),
            synced_at
        ],
    )?;
    Ok(())
}

/// Ambil agregat tersimpan untuk semua tabel yang sudah disinkronkan.
pub fn all_stats() -> Vec<(String, TableStats, Option<String>)> {
    let c = conn().lock().unwrap();
    let mut stmt = c
        .prepare("SELECT table_name, stats, synced_at FROM sync_meta WHERE stats IS NOT NULL")
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .unwrap();
    rows.filter_map(|r| r.ok())
        .filter_map(|(t, s, at)| {
            serde_json::from_str::<TableStats>(&s)
                .ok()
                .map(|st| (t, st, at))
        })
        .collect()
}

fn value_of(v: &Value, col: &str) -> String {
    v.get(col)
        .map(crate::model::cell_to_string)
        .unwrap_or_default()
}

/// Apakah kolom dibandingkan sebagai angka (memakai tipe dari skema).
pub fn kolom_numerik(columns: &[ColumnDef], name: &str) -> bool {
    columns.iter().find(|c| c.name == name).is_some_and(|c| {
        matches!(
            c.r#type.as_str(),
            "int" | "number" | "decimal" | "float" | "numeric" | "bigint"
        )
    })
}

/// Urutkan nilai baris untuk pengurutan halaman.
fn banding_urut(a: &Value, b: &Value, sc: &str, numerik: bool, dir: &str) -> std::cmp::Ordering {
    let ord = if numerik {
        crate::model::to_f64(a.get(sc).unwrap_or(&Value::Null))
            .total_cmp(&crate::model::to_f64(b.get(sc).unwrap_or(&Value::Null)))
    } else {
        value_of(a, sc)
            .to_lowercase()
            .cmp(&value_of(b, sc).to_lowercase())
    };
    if dir.eq_ignore_ascii_case("desc") {
        ord.reverse()
    } else {
        ord
    }
}

/// Ambil satu halaman baris aset dengan pencarian dan pengurutan sederhana.
pub fn page(
    table: &str,
    columns: &[ColumnDef],
    page_no: i64,
    per_page: i64,
    q: &str,
    sort: Option<&str>,
    dir: &str,
) -> (Vec<AssetRow>, i64) {
    let c = conn().lock().unwrap();
    let total: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM assets WHERE table_name = ?1",
            params![table],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let mut stmt = c
        .prepare("SELECT data FROM assets WHERE table_name = ?1")
        .unwrap();
    let raw = stmt
        .query_map(params![table], |r| r.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .filter_map(|s| serde_json::from_str::<Value>(&s).ok());

    let needle = q.trim().to_lowercase();
    let mut items: Vec<Value> = raw
        .filter(|v| {
            if needle.is_empty() {
                return true;
            }
            columns.iter().any(|col| {
                let s = value_of(v, &col.name).to_lowercase();
                !s.is_empty() && s.contains(&needle)
            })
        })
        .collect();

    // Urutan kosong berarti "tanpa urutan", bukan urut berdasarkan nama kosong.
    if let Some(sc) = sort.map(str::trim).filter(|s| !s.is_empty()) {
        // Kolom teks dibandingkan sebagai teks, walaupun memuat angka (mis. "A2").
        let numerik = kolom_numerik(columns, sc);
        items.sort_by(|a, b| banding_urut(a, b, sc, numerik, dir));
    }

    let filtered = items.len() as i64;
    let start = ((page_no.max(1) - 1) * per_page).max(0) as usize;
    let rows: Vec<AssetRow> = items
        .into_iter()
        .skip(start)
        .take(per_page.max(1) as usize)
        .map(|v| AssetRow {
            values: columns.iter().map(|c| value_of(&v, &c.name)).collect(),
        })
        .collect();
    let _ = total;
    (rows, filtered)
}

/// Susun halaman kosong saat tabel belum tersinkron.
pub fn empty_page(table: &str, label: &str, columns: Vec<ColumnDef>, is_demo: bool) -> AssetPage {
    AssetPage {
        table: table.to_string(),
        label: label.to_string(),
        columns,
        rows: vec![],
        page: 1,
        per_page: 25,
        total: 0,
        is_demo,
    }
}

/// Baca nilai metadata aplikasi.
pub fn get_meta(key: &str) -> Option<String> {
    let c = conn().lock().unwrap();
    c.query_row(
        "SELECT value FROM app_meta WHERE key = ?1",
        params![key],
        |r| r.get::<_, String>(0),
    )
    .ok()
}

/// Tulis nilai metadata aplikasi.
pub fn set_meta(key: &str, value: &str) -> rusqlite::Result<()> {
    let c = conn().lock().unwrap();
    c.execute(
        "INSERT INTO app_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Hapus nilai metadata aplikasi.
pub fn delete_meta(key: &str) -> rusqlite::Result<()> {
    let c = conn().lock().unwrap();
    c.execute("DELETE FROM app_meta WHERE key = ?1", params![key])?;
    Ok(())
}

/// Ubah status sinkronisasi mentah menjadi model tampilan.
pub fn statuses_with_labels() -> Vec<SyncStatus> {
    crate::catalog::TABLES
        .iter()
        .map(|t| {
            let found = sync_statuses().into_iter().find(|(n, _, _)| n == t.table);
            let (count, at) = found.map(|(_, c, a)| (c, a)).unwrap_or((0, None));
            SyncStatus {
                table: t.table.to_string(),
                label: t.label.to_string(),
                row_count: count,
                last_sync: at.clone(),
                status: if at.is_some() {
                    "tersinkron".into()
                } else {
                    "belum".into()
                },
                message: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ColumnDef;
    use serde_json::json;

    fn kolom(nama: &str, tipe: &str) -> ColumnDef {
        ColumnDef {
            name: nama.to_string(),
            label: nama.to_string(),
            r#type: tipe.to_string(),
            len: "-".to_string(),
        }
    }

    #[test]
    fn kolom_teks_memuat_angka_diurutkan_sebagai_teks() {
        // Regresi: dulu "B1" dianggap lebih kecil dari "A2" karena dibandingkan angka.
        let cols = vec![kolom("nama", "text")];
        assert!(!kolom_numerik(&cols, "nama"));
        let a = json!({"nama": "A2"});
        let b = json!({"nama": "B1"});
        assert_eq!(
            banding_urut(&a, &b, "nama", false, "asc"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            banding_urut(&a, &b, "nama", false, "desc"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn kolom_angka_diurutkan_sebagai_angka() {
        let cols = vec![kolom("rph_aset", "int")];
        assert!(kolom_numerik(&cols, "rph_aset"));
        let a = json!({"rph_aset": 90});
        let b = json!({"rph_aset": 100});
        // Sebagai angka 90 < 100; secara teks justru sebaliknya.
        assert_eq!(
            banding_urut(&a, &b, "rph_aset", true, "asc"),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn kolom_tidak_dikenal_dianggap_teks() {
        let cols = vec![kolom("nama", "text")];
        assert!(!kolom_numerik(&cols, "tidak_ada"));
    }
}
