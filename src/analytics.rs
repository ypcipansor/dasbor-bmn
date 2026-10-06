//! Agregasi analitik atas baris BMN. Fungsi di sini murni dan dipakai
//! baik saat sinkronisasi (SSR) maupun saat menyusun data demo.

use crate::model::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Kolom sumber yang dipakai untuk analitik.
pub const COL_NILAI: &str = "rph_aset";
pub const COL_SUSUT: &str = "rph_susut";
pub const COL_KONDISI: &str = "ur_kondisi";
pub const COL_PROV: &str = "ur_prov";
pub const COL_SATKER: &str = "nama_satker";
pub const COL_TGL_PERLH: &str = "tgl_perlh";
pub const COL_SUMBER_DANA: &str = "ur_sumber_dana";
pub const COL_KELOMPOK: &str = "ur_sskel";
pub const COL_IDLE: &str = "status_bmn_idle";
pub const COL_HILANG: &str = "brg_hilang_yn";
pub const COL_RUSAK: &str = "brg_rusak_yn";
pub const COL_NAMA: &str = "nama";
pub const COL_MERK: &str = "merk";

/// Agregat satu tabel; disimpan sebagai JSON pada tabel `sync_meta`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TableStats {
    pub row_count: i64,
    pub nilai: f64,
    pub susut: f64,
    pub idle: i64,
    pub hilang: i64,
    pub rusak: i64,
    pub kondisi: Vec<Bucket>,
    pub provinsi: Vec<Bucket>,
    pub satker: Vec<Bucket>,
    pub sumber_dana: Vec<Bucket>,
    pub perolehan: Vec<TrendPoint>,
    /// Nama wilayah/satker berbeda yang lengkap (sebelum pemotongan bucket),
    /// agar `merge` dapat menghitung kardinalitas gabungan lintas tabel.
    #[serde(default)]
    pub provinsi_nama: Vec<String>,
    #[serde(default)]
    pub satker_nama: Vec<String>,
}

fn key_of(v: &Value, col: &str) -> String {
    v.get(col).map(cell_to_string).unwrap_or_default()
}

fn truthy(v: &Value, col: &str) -> bool {
    let s = key_of(v, col).trim().to_ascii_uppercase();
    matches!(s.as_str(), "Y" | "YA" | "1" | "TRUE" | "YES")
}

fn push(map: &mut HashMap<String, (i64, f64)>, name: &str, nilai: f64) {
    let key = if name.trim().is_empty() {
        "(tidak diisi)"
    } else {
        name.trim()
    };
    let e = map.entry(key.to_string()).or_insert((0, 0.0));
    e.0 += 1;
    e.1 += nilai;
}

fn to_buckets(map: HashMap<String, (i64, f64)>, limit: usize) -> Vec<Bucket> {
    let mut v: Vec<Bucket> = map
        .into_iter()
        .map(|(name, (jumlah, nilai))| Bucket {
            name,
            jumlah,
            nilai,
        })
        .collect();
    v.sort_by(|a, b| b.jumlah.cmp(&a.jumlah).then(b.nilai.total_cmp(&a.nilai)));
    v.truncate(limit);
    v
}

fn year_of(v: &Value) -> Option<i32> {
    let s = key_of(v, COL_TGL_PERLH);
    if s.len() < 4 {
        return None;
    }
    let head = &s[..4];
    head.parse::<i32>()
        .ok()
        .filter(|y| (1900..=2100).contains(y))
}

/// Hitung agregat satu tabel dari kumpulan baris.
pub fn aggregate(rows: &[Value]) -> TableStats {
    let mut s = TableStats {
        row_count: rows.len() as i64,
        ..Default::default()
    };
    let mut kondisi = HashMap::new();
    let mut provinsi = HashMap::new();
    let mut satker = HashMap::new();
    let mut sumber = HashMap::new();
    let mut tahun: HashMap<i32, (i64, f64)> = HashMap::new();

    for r in rows {
        let nilai = r.get(COL_NILAI).map(to_f64).unwrap_or(0.0);
        s.nilai += nilai;
        s.susut += r.get(COL_SUSUT).map(to_f64).unwrap_or(0.0);
        if truthy(r, COL_IDLE) {
            s.idle += 1;
        }
        if truthy(r, COL_HILANG) {
            s.hilang += 1;
        }
        if truthy(r, COL_RUSAK) {
            s.rusak += 1;
        }
        push(&mut kondisi, &key_of(r, COL_KONDISI), nilai);
        push(&mut provinsi, &key_of(r, COL_PROV), nilai);
        push(&mut satker, &key_of(r, COL_SATKER), nilai);
        push(&mut sumber, &key_of(r, COL_SUMBER_DANA), nilai);
        if let Some(y) = year_of(r) {
            let e = tahun.entry(y).or_insert((0, 0.0));
            e.0 += 1;
            e.1 += nilai;
        }
    }

    s.kondisi = to_buckets(kondisi, 12);
    // Simpan daftar nama lengkap sebelum bucket dipotong untuk tampilan.
    s.provinsi_nama = provinsi.keys().cloned().collect();
    s.satker_nama = satker.keys().cloned().collect();
    s.provinsi = to_buckets(provinsi, 40);
    s.satker = to_buckets(satker, 25);
    s.sumber_dana = to_buckets(sumber, 12);
    let mut perolehan: Vec<TrendPoint> = tahun
        .into_iter()
        .map(|(year, (jumlah, nilai))| TrendPoint {
            year,
            jumlah,
            nilai,
        })
        .collect();
    perolehan.sort_by_key(|t| t.year);
    s.perolehan = perolehan;
    s
}

/// Gabungkan agregat seluruh tabel menjadi ringkasan dasbor.
pub fn merge(parts: &[(String, String, String, TableStats)], is_demo: bool) -> Overview {
    let mut o = Overview {
        total_aset: 0,
        total_nilai: 0.0,
        total_susut: 0.0,
        total_kategori: parts.len() as i64,
        total_provinsi: 0,
        total_satker: 0,
        aset_idle: 0,
        aset_hilang: 0,
        aset_rusak: 0,
        categories: vec![],
        provinsi: vec![],
        satker: vec![],
        kondisi: vec![],
        perolehan: vec![],
        sumber_dana: vec![],
        is_demo,
        generated_at: String::new(),
    };
    let mut prov = HashMap::new();
    let mut sat = HashMap::new();
    let mut kond = HashMap::new();
    let mut sumber = HashMap::new();
    let mut tahun: HashMap<i32, (i64, f64)> = HashMap::new();
    let mut prov_unik: HashSet<String> = HashSet::new();
    let mut sat_unik: HashSet<String> = HashSet::new();

    for (label, icon, volume, s) in parts {
        o.total_aset += s.row_count;
        o.total_nilai += s.nilai;
        o.total_susut += s.susut;
        o.aset_idle += s.idle;
        o.aset_hilang += s.hilang;
        o.aset_rusak += s.rusak;
        // Nama wilayah/satker yang sama di beberapa tabel hanya dihitung sekali.
        if s.provinsi_nama.is_empty() {
            // Cache lama belum memuat daftar nama; pakai bucket yang tersedia.
            prov_unik.extend(s.provinsi.iter().map(|b| b.name.clone()));
        } else {
            prov_unik.extend(s.provinsi_nama.iter().cloned());
        }
        if s.satker_nama.is_empty() {
            sat_unik.extend(s.satker.iter().map(|b| b.name.clone()));
        } else {
            sat_unik.extend(s.satker_nama.iter().cloned());
        }
        o.categories.push(CategoryStat {
            label: label.clone(),
            icon: icon.clone(),
            jumlah: s.row_count,
            nilai: s.nilai,
            volume: volume.clone(),
        });
        for b in &s.provinsi {
            let e = prov.entry(b.name.clone()).or_insert((0, 0.0));
            e.0 += b.jumlah;
            e.1 += b.nilai;
        }
        for b in &s.satker {
            let e = sat.entry(b.name.clone()).or_insert((0, 0.0));
            e.0 += b.jumlah;
            e.1 += b.nilai;
        }
        for b in &s.kondisi {
            let e = kond.entry(b.name.clone()).or_insert((0, 0.0));
            e.0 += b.jumlah;
            e.1 += b.nilai;
        }
        for b in &s.sumber_dana {
            let e = sumber.entry(b.name.clone()).or_insert((0, 0.0));
            e.0 += b.jumlah;
            e.1 += b.nilai;
        }
        for t in &s.perolehan {
            let e = tahun.entry(t.year).or_insert((0, 0.0));
            e.0 += t.jumlah;
            e.1 += t.nilai;
        }
    }

    o.categories.sort_by(|a, b| b.nilai.total_cmp(&a.nilai));
    o.provinsi = to_buckets(prov, 15);
    o.satker = to_buckets(sat, 15);
    o.kondisi = to_buckets(kond, 10);
    o.sumber_dana = to_buckets(sumber, 10);
    o.total_provinsi = prov_unik.len() as i64;
    o.total_satker = sat_unik.len() as i64;
    let mut perolehan: Vec<TrendPoint> = tahun
        .into_iter()
        .map(|(year, (jumlah, nilai))| TrendPoint {
            year,
            jumlah,
            nilai,
        })
        .collect();
    perolehan.sort_by_key(|t| t.year);
    if perolehan.len() > 30 {
        perolehan = perolehan.split_off(perolehan.len() - 30);
    }
    o.perolehan = perolehan;
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn agregat_dasar_benar() {
        let rows = vec![
            json!({"rph_aset": 100, "rph_susut": 10, "ur_kondisi": "Baik", "ur_prov": "DKI Jakarta", "tgl_perlh": "2019-05-01 00:00:00"}),
            json!({"rph_aset": "250", "ur_kondisi": "Baik", "ur_prov": "DKI Jakarta", "tgl_perlh": "2020-01-01 00:00:00"}),
            json!({"rph_aset": 50, "ur_kondisi": "Rusak Ringan", "ur_prov": "Jawa Barat", "status_bmn_idle": "Y"}),
        ];
        let s = aggregate(&rows);
        assert_eq!(s.row_count, 3);
        assert_eq!(s.nilai, 400.0);
        assert_eq!(s.susut, 10.0);
        assert_eq!(s.idle, 1);
        assert_eq!(s.kondisi[0].name, "Baik");
        assert_eq!(s.kondisi[0].jumlah, 2);
        assert_eq!(s.provinsi.len(), 2);
        assert_eq!(s.perolehan.len(), 2);
        assert_eq!(s.perolehan[0].year, 2019);
    }

    #[test]
    fn format_rupiah_ringkas() {
        assert_eq!(format_rupiah_short(1_500_000_000_000.0), "Rp 1,5 T");
        assert_eq!(format_rupiah_short(2_000_000_000.0), "Rp 2 M");
        assert_eq!(format_number(1234567), "1.234.567");
    }

    #[test]
    fn ekstraksi_baris_dari_pembungkus() {
        let v = json!({"results": [{"a": 1}, {"a": 2}]});
        assert_eq!(extract_rows_demo(&v).len(), 2);
        let t = json!({"results": "Tidak Ada Data"});
        assert_eq!(extract_rows_demo(&t).len(), 0);
    }

    // Salinan lokal agar tes tidak bergantung pada modul ssr.
    fn extract_rows_demo(body: &Value) -> Vec<Value> {
        match body {
            Value::Array(a) => a.clone(),
            Value::Object(o) => {
                for k in ["results", "result", "data", "rows"] {
                    if let Some(inner) = o.get(k) {
                        if let Value::Array(a) = inner {
                            return a.clone();
                        }
                    }
                }
                vec![]
            }
            _ => vec![],
        }
    }

    fn baris(provinsi: &str, satker: &str) -> Value {
        json!({
            "ur_prov": provinsi,
            "nama_satker": satker,
            "rph_aset": 1000,
            "rph_susut": 100,
            "tgl_perlh": "2020-01-01",
        })
    }

    #[test]
    fn wilayah_sama_lintas_tabel_dihitung_sekali() {
        // Regresi: dulu jumlah unik per tabel dijumlahkan sehingga wilayah yang
        // sama muncul berkali-kali (mis. 15 tabel x 18 provinsi = 270).
        let a = aggregate(&[baris("ACEH", "Kejati Aceh")]);
        let b = aggregate(&[baris("ACEH", "Kejati Aceh")]);
        let c = aggregate(&[baris("BALI", "Kejati Bali")]);
        let parts = vec![
            ("Tanah".to_string(), "🗺️".to_string(), "1 MB".to_string(), a),
            (
                "Gedung".to_string(),
                "🏢".to_string(),
                "1 MB".to_string(),
                b,
            ),
            ("Rumah".to_string(), "🏠".to_string(), "1 MB".to_string(), c),
        ];
        let o = merge(&parts, false);
        assert_eq!(o.total_provinsi, 2, "ACEH + BALI, bukan 3");
        assert_eq!(o.total_satker, 2);
    }

    #[test]
    fn nama_terpotong_dari_bucket_tetap_dihitung() {
        // Daftar bucket tampilan dibatasi; daftar nama lengkap harus tetap utuh.
        let mut b = aggregate(&[baris("PROV-A", "SAT-A")]);
        b.provinsi_nama = (0..60).map(|i| format!("PROV-{i}")).collect();
        b.satker_nama = (0..40).map(|i| format!("SAT-{i}")).collect();
        assert!(b.provinsi.len() < 60, "bucket tampilan memang dipotong");
        let parts = vec![("Tanah".to_string(), "🗺️".to_string(), "1 MB".to_string(), b)];
        let o = merge(&parts, false);
        assert_eq!(o.total_provinsi, 60);
        assert_eq!(o.total_satker, 40);
    }
}
