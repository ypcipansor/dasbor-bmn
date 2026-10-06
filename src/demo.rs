//! Data contoh (mode demo) agar dasbor tetap dapat dieksplorasi tanpa kredensial.

#![cfg(feature = "ssr")]

use crate::analytics::{self, TableStats};
use crate::catalog;
use serde_json::{json, Value};

const PROVINSI: &[&str] = &[
    "DKI Jakarta",
    "Jawa Barat",
    "Jawa Tengah",
    "Jawa Timur",
    "Sumatera Utara",
    "Sumatera Selatan",
    "Sulawesi Selatan",
    "Kalimantan Timur",
    "Bali",
    "Papua",
    "Aceh",
    "Riau",
    "Lampung",
    "NTB",
    "NTT",
    "Banten",
    "DI Yogyakarta",
    "Maluku",
];
const KONDISI: &[&str] = &["Baik", "Rusak Ringan", "Rusak Berat"];
const SUMBER: &[&str] = &["APBN", "APBD", "Hibah", "SBSN", "PNBP", "Perolehan Lainnya"];
const KELOMPOK: &[&str] = &[
    "Peralatan dan Mesin",
    "Bangunan",
    "Tanah",
    "Jalan/Irigasi/Jaringan",
    "Aset Tetap Lainnya",
];

/// RNG deterministik sederhana (LCG) agar data demo stabil antar-build.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 16
    }
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }
    fn pick<'a>(&mut self, xs: &'a [&'a str]) -> &'a str {
        xs[self.range(0, xs.len() as u64 - 1) as usize]
    }
}

fn satker_for(rng: &mut Rng, prov: &str) -> String {
    let n = rng.range(1, 3);
    format!("KEJATI {}-{:02}", prov.to_uppercase(), n)
}

/// Bangkitkan baris contoh untuk satu tabel.
pub fn rows_for(table: &str, seed: u64, n: usize) -> Vec<Value> {
    let mut rng = Rng(seed.wrapping_mul(2654435761));
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let prov = rng.pick(PROVINSI);
        let kondisi = if rng.range(0, 100) > 82 {
            rng.pick(&KONDISI[1..])
        } else {
            "Baik"
        };
        let tahun = rng.range(1998, 2025);
        let bulan = rng.range(1, 12);
        let tgl = rng.range(1, 28);
        let nilai = rng.range(5_000_000, 45_000_000_000) as f64;
        let susut = nilai * (rng.range(0, 60) as f64) / 100.0;
        out.push(json!({
            "table": table,
            "kd_brg": format!("{}.{:06}", rng.range(1, 8), rng.range(1, 999_999)),
            "no_aset": i as i64 + 1,
            "nama": format!("{} {} {}", rng.pick(KELOMPOK), rng.pick(&["Unit", "Set", "Paket"]), i + 1),
            "ur_sskel": rng.pick(KELOMPOK),
            "merk": rng.pick(&["Toyota", "Honda", "Mitsubishi", "Samsung", "Lenovo", "Caterpillar"]),
            "tipe": rng.pick(&["Standar", "Tipe A", "Tipe B", "Generasi 3"]),
            "rph_aset": nilai as i64,
            "rph_susut": susut as i64,
            "rph_mutasi": 0,
            "ur_kondisi": kondisi,
            "ur_prov": prov,
            "ur_kab": format!("Kota {}", prov),
            "ur_kec": "Kecamatan Contoh",
            "ur_kel": "Kelurahan Contoh",
            "alamat": format!("Jl. Contoh No. {}", i + 1),
            "nama_satker": satker_for(&mut rng, prov),
            "kode_satker": format!("{:06}", rng.range(100_000, 999_999)),
            "ur_kanwil": format!("Kanwil {}", prov.to_uppercase()),
            "ur_sumber_dana": rng.pick(SUMBER),
            "asl_perlh": rng.pick(&["Pembelian", "Transfer Masuk", "Hibah", "Perolehan Lainnya"]),
            "cara_perlh": rng.pick(&["Pembelian", "Pembangunan", "Hibah"]),
            "tgl_perlh": format!("{tahun}-{bulan:02}-{tgl:02} 00:00:00"),
            "tgl_buku_pertama": format!("{tahun}-{bulan:02}-{tgl:02} 00:00:00"),
            "status_bmn_yn": "Y",
            "status_bmn_idle": if rng.range(0, 100) > 93 { "Y" } else { "N" },
            "brg_hilang_yn": if rng.range(0, 1000) > 995 { "Y" } else { "N" },
            "brg_rusak_yn": if rng.range(0, 100) > 88 { "Y" } else { "N" },
            "umur_sisa": rng.range(0, 40) as i64,
            "luas": rng.range(50, 5000) as i64,
            "last_update": "2025-10-31 00:00:00",
        }));
    }
    out
}

/// Jumlah baris demo per tabel agar agregat terasa realistis namun ringan.
fn demo_count(table: &str) -> usize {
    match table {
        "SIMAN2_M_ASET_NON_TIK" => 260,
        "SIMAN2_M_ASET_ANGKUTAN_BERMOTOR" => 220,
        "SIMAN2_M_ASET_TANAH" => 180,
        "SIMAN2_M_ASET_GEDUNG_BANGUNAN" => 200,
        _ => 120,
    }
}

/// Isi basis data dengan data contoh untuk seluruh tabel katalog.
pub fn seed() {
    let now = crate::now_string();
    for (i, t) in catalog::TABLES.iter().enumerate() {
        let rows = rows_for(t.table, i as u64 + 7, demo_count(t.table));
        let stats: TableStats = analytics::aggregate(&rows);
        let _ = crate::store::replace_table(t.table, &rows, &stats, &now);
    }
    let _ = crate::store::set_meta("mode", "demo");
}

/// Apakah basis data saat ini berisi data contoh.
pub fn is_demo() -> bool {
    crate::store::get_meta("mode").as_deref() == Some("demo")
}
