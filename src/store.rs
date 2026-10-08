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
    replace_table_transaksional(table, rows, stats, synced_at, false)
}

/// Ganti isi satu tabel dan, bila diminta, buang seluruh data contoh lebih dulu
/// dalam satu transaksi yang sama.
///
/// Penggabungan ini penting: bila penghapusan data contoh dan penulisan data
/// langsung dipisah, kegagalan penulisan akan meninggalkan dasbor kosong tanpa
/// jalan kembali ke data contoh. Karena satu transaksi, kegagalan apa pun
/// membatalkan keduanya sehingga isi lama (termasuk data contoh) tetap utuh.
pub fn replace_table_transaksional(
    table: &str,
    rows: &[Value],
    stats: &TableStats,
    synced_at: &str,
    buang_demo: bool,
) -> rusqlite::Result<()> {
    let mut c = conn().lock().unwrap();
    let tx = c.transaction()?;
    if buang_demo {
        tx.execute("DELETE FROM assets", [])?;
        tx.execute("DELETE FROM sync_meta", [])?;
    }
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
    if buang_demo {
        tx.execute(
            "INSERT INTO app_meta (key, value) VALUES ('mode', 'live')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )?;
    }
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

/// Iterasi seluruh baris satu tabel dalam urutan kunci yang stabil.
///
/// Dipakai untuk ekspor besar. Berbeda dari [`page`], fungsi ini memakai koneksi
/// baca-saja tersendiri sehingga tidak menahan mutex basis data utama dan tidak
/// membaca ulang seluruh tabel pada tiap halaman. Kueri berjalan satu kali, jadi
/// seluruh baris berasal dari snapshot yang sama meski sinkronisasi berjalan di
/// sampingnya.
pub fn alir_baris(table: &str, mut f: impl FnMut(&Value)) -> rusqlite::Result<()> {
    // Pastikan basis data dan skemanya sudah dibuat sebelum membuka koneksi
    // baca-saja. Tanpa ini, ekspor yang menjadi permintaan data pertama (sebelum
    // ada pembacaan lain) gagal membuka berkas yang belum ada. Kunci hanya
    // dipegang sesaat di sini, tidak selama baris dialirkan.
    let _ = conn();
    let path = config::data_dir().join("bmn.sqlite");
    let c = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let mut stmt = c.prepare("SELECT data FROM assets WHERE table_name = ?1 ORDER BY row_key")?;
    let mut rows = stmt.query(params![table])?;
    while let Some(r) = rows.next()? {
        let s: String = r.get(0)?;
        if let Ok(v) = serde_json::from_str::<Value>(&s) {
            f(&v);
        }
    }
    Ok(())
}

/// Arahkan uji ke basis data sementara agar tidak menyentuh cache pengembangan.
///
/// Tanpa ini, uji yang memakai `clear_table` akan menghapus data nyata di `data/`.
/// Direktori dibuat unik per proses uji supaya proses paralel tidak saling
/// menimpa, dan dibersihkan lebih dulu agar sisa jalannya sebelumnya tidak
/// terbaca.
#[cfg(test)]
pub fn uji_isolasi() {
    static SEKALI: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    SEKALI.get_or_init(|| {
        let unik = format!(
            "dasbor-bmn-uji-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let dir = std::env::temp_dir().join(unik);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        std::env::set_var("SIMAN_DATA_DIR", &dir);
    });
}

/// Kunci bersama untuk uji yang menyentuh basis data.
///
/// Seluruh uji dalam satu proses berbagi satu berkas SQLite, dan beberapa uji
/// menghapus seluruh tabel. Tanpa serialisasi, uji yang berjalan paralel saling
/// menimpa data sehingga hasilnya bergantung waktu. Setiap uji yang menyentuh
/// basis data harus memegang kunci ini selama berjalan.
#[cfg(test)]
pub fn uji_kunci() -> std::sync::MutexGuard<'static, ()> {
    static KUNCI: OnceLock<Mutex<()>> = OnceLock::new();
    KUNCI
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
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

    fn stats(n: i64) -> crate::analytics::TableStats {
        crate::analytics::TableStats {
            row_count: n,
            ..Default::default()
        }
    }

    fn siap() -> std::sync::MutexGuard<'static, ()> {
        let kunci = uji_kunci();
        uji_isolasi();
        conn();
        kunci
    }

    #[test]
    fn buang_demo_dan_tulis_dalam_satu_transaksi() {
        let _kunci = siap();
        let t = "UJI_TRANSAKSI";
        // Siapkan isi "data contoh" pada tabel lain.
        let _ = replace_table("UJI_CONTOH", &[json!({"kd_brg": "D1"})], &stats(1), "t0");
        assert_eq!(stored_count("UJI_CONTOH"), 1);

        // Satu transaksi membuang data contoh, menulis tabel, dan menandai mode.
        replace_table_transaksional(t, &[json!({"kd_brg": "A1"})], &stats(1), "t1", true)
            .expect("transaksi berhasil");

        assert_eq!(stored_count("UJI_CONTOH"), 0, "data contoh ikut terbuang");
        assert_eq!(stored_count(t), 1, "data langsung tertulis");
        assert_eq!(get_meta("mode").as_deref(), Some("live"));
    }

    #[test]
    fn alir_baris_mengembalikan_seluruh_baris_berurutan() {
        let _kunci = siap();
        let t = "UJI_ALIR";
        let rows: Vec<Value> = (0..5).map(|i| json!({"kd_brg": format!("K{i}")})).collect();
        replace_table(t, &rows, &stats(5), "t0").unwrap();

        let mut terlihat: Vec<String> = Vec::new();
        alir_baris(t, |v| {
            terlihat.push(
                v.get("kd_brg")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
            );
        })
        .expect("alir baris berhasil");
        assert_eq!(terlihat.len(), 5);
        let mut urut = terlihat.clone();
        urut.sort();
        assert_eq!(terlihat, urut, "urutan kunci stabil");
    }
}
