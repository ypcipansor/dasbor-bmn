//! Penyusun laporan bulanan PDF di atas data `Overview`.
//!
//! Laporan bersifat "per bulan pelaporan": judul dan rentang tanggal mengikuti
//! bulan yang dipilih, sedangkan seluruh grafik merangkum analitik terkini
//! (komposisi, tren per tahun, sebaran, kondisi, dan sumber dana).

#![cfg(feature = "ssr")]

pub mod charts;
pub mod http;
pub mod pdf;

use crate::model::{format_number, format_rupiah, format_rupiah_short, Overview};
use charts::*;
use pdf::{Dokumen, Page, A4_H, A4_W};

/// Bulan pelaporan yang dipilih pengguna.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Periode {
    pub tahun: i32,
    pub bulan: u32,
}

const NAMA_BULAN: [&str; 12] = [
    "Januari",
    "Februari",
    "Maret",
    "April",
    "Mei",
    "Juni",
    "Juli",
    "Agustus",
    "September",
    "Oktober",
    "November",
    "Desember",
];

/// Jumlah hari pada bulan tertentu (memperhitungkan tahun kabisat).
pub fn hari_dalam_bulan(tahun: i32, bulan: u32) -> u32 {
    match bulan {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let kabisat = (tahun % 4 == 0 && tahun % 100 != 0) || tahun % 400 == 0;
            if kabisat {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

impl Periode {
    /// Bulan berjalan menurut zona waktu Indonesia Barat (UTC+7).
    pub fn sekarang() -> Self {
        use chrono::{Datelike, Duration, Utc};
        let now = Utc::now() + Duration::hours(7);
        Periode {
            tahun: now.year(),
            bulan: now.month(),
        }
    }

    /// Baca periode dari bentuk `YYYY-MM`.
    pub fn dari_str(s: &str) -> Option<Self> {
        let (y, m) = s.trim().split_once('-')?;
        let tahun: i32 = y.parse().ok()?;
        let bulan: u32 = m.parse().ok()?;
        if !(1900..=2100).contains(&tahun) || !(1..=12).contains(&bulan) {
            return None;
        }
        Some(Periode { tahun, bulan })
    }

    pub fn kode(&self) -> String {
        format!("{:04}-{:02}", self.tahun, self.bulan)
    }

    pub fn nama_bulan(&self) -> &'static str {
        NAMA_BULAN[(self.bulan.saturating_sub(1) as usize).min(11)]
    }

    pub fn label(&self) -> String {
        format!("{} {}", self.nama_bulan(), self.tahun)
    }

    /// Rentang tanggal lengkap bulan ini, mis. `1-31 Oktober 2026`.
    pub fn rentang(&self) -> String {
        let akhir = hari_dalam_bulan(self.tahun, self.bulan);
        format!("1-{akhir} {} {}", self.nama_bulan(), self.tahun)
    }
}

const MARGIN: f64 = 40.0;
const LEBAR: f64 = A4_W - 2.0 * MARGIN;

/// Header dan footer standar tiap halaman.
fn hiasan(p: &mut Page, judul_laporan: &str, nomor: usize, total: usize) {
    p.kotak(0.0, 0.0, A4_W, 26.0, "#0f172a");
    p.kotak(MARGIN, 6.0, 3.5, 14.0, "#1f47f5");
    p.teks(MARGIN + 10.0, 16.5, 8.0, true, "#ffffff", judul_laporan);
    p.teks_kanan(
        A4_W - MARGIN,
        16.5,
        7.5,
        false,
        "#93a2c4",
        "SIMAN v2 - Kejaksaan RI",
    );
    if nomor > 1 {
        p.garis(MARGIN, A4_H - 34.0, A4_W - MARGIN, A4_H - 34.0, GARIS, 0.6);
        p.teks(
            MARGIN,
            A4_H - 20.0,
            7.5,
            false,
            "#8b93a7",
            "Data bersumber dari Web Service SLDK SIMAN v2. Klasifikasi: Terbatas.",
        );
        p.teks_kanan(
            A4_W - MARGIN,
            A4_H - 20.0,
            7.5,
            false,
            "#8b93a7",
            &format!("Halaman {nomor} dari {total}"),
        );
    }
}

/// Susun laporan lengkap sebagai byte PDF.
pub fn bangun(ov: &Overview, periode: &Periode, dibuat_pada: &str) -> Vec<u8> {
    let judul = format!("Laporan Bulanan BMN - {}", periode.label());
    // Perkiraan jumlah halaman dipakai untuk penomoran footer.
    let total = 5;
    let mut d = Dokumen::new(&judul);

    d.tambah(halaman_sampul(ov, periode, &judul, dibuat_pada));
    d.tambah(halaman_komposisi(ov, periode, &judul));
    d.tambah(halaman_tren(ov, periode, &judul));
    d.tambah(halaman_sebaran(ov, periode, &judul));
    d.tambah(halaman_kondisi(ov, periode, &judul, total));
    d.bangun(&tanggal_pdf(dibuat_pada))
}

/// Ubah "YYYY-MM-DD HH:MM:SS UTC" menjadi "D:YYYYMMDDHHmmSSZ".
fn tanggal_pdf(s: &str) -> String {
    let digit: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digit.len() >= 14 {
        format!("D:{}Z", &digit[..14])
    } else {
        "D:20260101000000Z".to_string()
    }
}

fn halaman_sampul(ov: &Overview, periode: &Periode, judul: &str, dibuat_pada: &str) -> Page {
    let mut p = Page::new(A4_W, A4_H);
    // Pita judul.
    p.kotak(0.0, 0.0, A4_W, 190.0, "#0f172a");
    p.kotak(0.0, 0.0, A4_W, 6.0, "#1f47f5");
    p.kotak(MARGIN, 40.0, 46.0, 46.0, "#1f47f5");
    p.teks_tengah(MARGIN + 23.0, 70.0, 15.0, true, "#ffffff", "BMN");
    p.teks(
        MARGIN + 60.0,
        56.0,
        15.0,
        true,
        "#ffffff",
        "Dasbor Analitik BMN",
    );
    p.teks(
        MARGIN + 60.0,
        72.0,
        8.5,
        false,
        "#93a2c4",
        "Kejaksaan Republik Indonesia",
    );

    p.teks(MARGIN, 118.0, 22.0, true, "#ffffff", "Laporan Bulanan");
    p.teks(MARGIN, 143.0, 13.0, false, "#c7d2ea", &periode.label());
    p.teks(
        MARGIN,
        162.0,
        8.5,
        false,
        "#93a2c4",
        &format!("Periode {} - dicetak {}", periode.rentang(), dibuat_pada),
    );

    // Ringkasan eksekutif.
    judul_bagian(&mut p, MARGIN, 214.0, LEBAR, "Ringkasan Eksekutif", None);
    let rasio_susut = if ov.total_nilai > 0.0 {
        (ov.total_susut / ov.total_nilai) * 100.0
    } else {
        0.0
    };
    let paragraf = format!(
        "Ringkasan ini merangkum {} aset Barang Milik Negara dengan nilai perolehan {} \
         dan akumulasi penyusutan {} ({:.1}% dari nilai aset). Aset tersebar di {} provinsi \
         dan {} satuan kerja. Teridentifikasi {} aset idle, {} barang rusak, dan {} barang hilang \
         yang perlu ditindaklanjuti.",
        format_number(ov.total_aset),
        format_rupiah_short(ov.total_nilai),
        format_rupiah_short(ov.total_susut),
        rasio_susut,
        format_number(ov.total_provinsi),
        format_number(ov.total_satker),
        format_number(ov.aset_idle),
        format_number(ov.aset_rusak),
        format_number(ov.aset_hilang),
    );
    teks_paragraf(&mut p, MARGIN, 248.0, LEBAR, 9.5, &paragraf, 14.0, 4);

    // Kartu KPI.
    let kartu = [
        (
            "Total Aset",
            format_number(ov.total_aset),
            "unit tercatat",
            "blue",
        ),
        (
            "Nilai Aset",
            format_rupiah_short(ov.total_nilai),
            "nilai perolehan",
            "emerald",
        ),
        (
            "Akumulasi Susut",
            format_rupiah_short(ov.total_susut),
            &format!("{rasio_susut:.1}% dari nilai"),
            "amber",
        ),
        (
            "Sebaran Wilayah",
            format_number(ov.total_provinsi),
            &format!("{} satuan kerja", format_number(ov.total_satker)),
            "violet",
        ),
        (
            "Aset Idle",
            format_number(ov.aset_idle),
            "tidak digunakan",
            "amber",
        ),
        (
            "Barang Rusak",
            format_number(ov.aset_rusak),
            "perlu tindak lanjut",
            "rose",
        ),
        (
            "Barang Hilang",
            format_number(ov.aset_hilang),
            "terindikasi hilang",
            "rose",
        ),
        (
            "Kategori BMN",
            format_number(ov.total_kategori),
            "jenis aset",
            "blue",
        ),
    ];
    let kw = (LEBAR - 3.0 * 10.0) / 4.0;
    for (i, (label, nilai, hint, tone)) in kartu.iter().enumerate() {
        let baris = i / 4;
        let kolom = i % 4;
        let x = MARGIN + kolom as f64 * (kw + 10.0);
        let y = 356.0 + baris as f64 * 70.0;
        let nilai_tampil: &str = nilai;
        kartu_kpi(&mut p, x, y, kw, 58.0, label, nilai_tampil, hint, tone);
    }

    // Catatan mode data.
    let (warna, pesan) = if ov.is_demo {
        (
            "#b45309",
            "Mode contoh: kredensial SLDK belum aktif, data pada laporan ini bersifat ilustratif.",
        )
    } else {
        ("#047857", "Data langsung dari Web Service SLDK SIMAN v2.")
    };
    p.kotak(MARGIN, 500.0, LEBAR, 26.0, "#f6f8fd");
    p.kotak(MARGIN, 500.0, 3.0, 26.0, warna);
    p.teks(MARGIN + 11.0, 516.0, 8.5, false, warna, pesan);

    halaman_hias(&mut p, judul, 1, 5);
    p
}

fn halaman_komposisi(ov: &Overview, periode: &Periode, judul: &str) -> Page {
    let mut p = Page::new(A4_W, A4_H);
    hiasan(&mut p, judul, 2, 5);
    judul_bagian(
        &mut p,
        MARGIN,
        60.0,
        LEBAR,
        "Komposisi Nilai per Kategori",
        Some(&periode.label()),
    );

    // Legenda donut dibatasi agar tidak berdesakan dengan tabel di bawahnya;
    // rincian lengkap tetap ada di tabel "Rincian per Kategori".
    const LEGENDA_MAKS: usize = 8;
    let donut_data: Vec<(String, f64)> = {
        let semua: Vec<(String, f64)> = ov
            .categories
            .iter()
            .map(|c| (c.label.clone(), c.nilai))
            .collect();
        if semua.len() <= LEGENDA_MAKS {
            semua
        } else {
            let (kepala, ekor) = semua.split_at(LEGENDA_MAKS - 1);
            let mut v = kepala.to_vec();
            let sisa: f64 = ekor.iter().map(|(_, n)| *n).sum();
            v.push(("Lainnya".to_string(), sisa));
            v
        }
    };
    donut(
        &mut p,
        MARGIN + 118.0,
        210.0,
        82.0,
        24.0,
        &donut_data,
        "Total",
        &format_rupiah_short(ov.total_nilai),
    );

    // Tabel kategori: nama, jumlah, nilai.
    judul_bagian(&mut p, MARGIN, 330.0, LEBAR, "Rincian per Kategori", None);
    let mut y = 360.0;
    p.kotak(MARGIN, y, LEBAR, 18.0, "#eef1fa");
    p.teks(MARGIN + 6.0, y + 12.0, 8.0, true, "#475569", "Kategori");
    p.teks_kanan(
        MARGIN + LEBAR - 190.0,
        y + 12.0,
        8.0,
        true,
        "#475569",
        "Jumlah",
    );
    p.teks_kanan(
        MARGIN + LEBAR - 6.0,
        y + 12.0,
        8.0,
        true,
        "#475569",
        "Nilai",
    );
    y += 18.0;
    for (i, c) in ov.categories.iter().enumerate().take(16) {
        if i % 2 == 0 {
            p.kotak(MARGIN, y, LEBAR, 17.0, "#fafbfe");
        }
        p.kotak(MARGIN + 6.0, y + 5.0, 7.0, 7.0, warna_ke(i));
        p.teks(
            MARGIN + 18.0,
            y + 12.0,
            8.5,
            false,
            "#334155",
            &Page::potong(&c.label, LEBAR - 220.0, 8.5, false),
        );
        p.teks_kanan(
            MARGIN + LEBAR - 190.0,
            y + 12.0,
            8.5,
            false,
            "#475569",
            &format_number(c.jumlah),
        );
        p.teks_kanan(
            MARGIN + LEBAR - 6.0,
            y + 12.0,
            8.5,
            true,
            "#0f172a",
            &format_rupiah_short(c.nilai),
        );
        y += 17.0;
    }
    p
}

fn halaman_tren(ov: &Overview, periode: &Periode, judul: &str) -> Page {
    let mut p = Page::new(A4_W, A4_H);
    hiasan(&mut p, judul, 3, 5);
    judul_bagian(
        &mut p,
        MARGIN,
        60.0,
        LEBAR,
        "Tren Perolehan Aset per Tahun",
        Some(&periode.label()),
    );

    p.teks(MARGIN, 100.0, 9.0, true, "#334155", "Jumlah unit perolehan");
    area(&mut p, MARGIN, 108.0, LEBAR, 200.0, &ov.perolehan, false);
    p.teks(MARGIN, 340.0, 9.0, true, "#334155", "Nilai perolehan (Rp)");
    area(&mut p, MARGIN, 348.0, LEBAR, 200.0, &ov.perolehan, true);

    // Ringkas titik puncak.
    if let Some(puncak) = ov.perolehan.iter().max_by_key(|t| t.jumlah) {
        p.kotak(MARGIN, 574.0, LEBAR, 40.0, "#f6f8fd");
        p.kotak(MARGIN, 574.0, 3.0, 40.0, "#1f47f5");
        p.teks(MARGIN + 11.0, 592.0, 8.5, true, "#0f172a", "Catatan");
        p.teks(
            MARGIN + 11.0,
            606.0,
            8.5,
            false,
            "#475569",
            &format!(
                "Perolehan unit terbanyak pada tahun {} sebanyak {} unit (nilai {}).",
                puncak.year,
                format_number(puncak.jumlah),
                format_rupiah_short(puncak.nilai)
            ),
        );
    }
    p
}

fn halaman_sebaran(ov: &Overview, periode: &Periode, judul: &str) -> Page {
    let mut p = Page::new(A4_W, A4_H);
    hiasan(&mut p, judul, 4, 5);

    judul_bagian(
        &mut p,
        MARGIN,
        60.0,
        LEBAR,
        "Sebaran per Provinsi",
        Some("15 teratas menurut jumlah aset"),
    );
    let prov: Vec<(String, i64, f64)> = ov
        .provinsi
        .iter()
        .map(|b| (b.name.clone(), b.jumlah, b.nilai))
        .collect();
    bar_list(&mut p, MARGIN, 96.0, LEBAR, &prov, false, 15);

    judul_bagian(
        &mut p,
        MARGIN,
        420.0,
        LEBAR,
        "Satuan Kerja Terbesar",
        Some("15 teratas menurut jumlah aset"),
    );
    let sat: Vec<(String, i64, f64)> = ov
        .satker
        .iter()
        .map(|b| (b.name.clone(), b.jumlah, b.nilai))
        .collect();
    bar_list(&mut p, MARGIN, 456.0, LEBAR, &sat, false, 15);

    let _ = periode;
    p
}

fn halaman_kondisi(ov: &Overview, periode: &Periode, judul: &str, total: usize) -> Page {
    let mut p = Page::new(A4_W, A4_H);
    hiasan(&mut p, judul, 5, total);

    judul_bagian(
        &mut p,
        MARGIN,
        60.0,
        LEBAR,
        "Kondisi Aset",
        Some("Klasifikasi kondisi BMN"),
    );
    let kondisi: Vec<(String, f64)> = ov
        .kondisi
        .iter()
        .map(|b| (b.name.clone(), b.jumlah as f64))
        .collect();
    bar_vertikal(&mut p, MARGIN, 92.0, LEBAR, 150.0, &kondisi, false);

    judul_bagian(
        &mut p,
        MARGIN,
        288.0,
        LEBAR,
        "Sumber Dana",
        Some("Asal pembiayaan perolehan"),
    );
    let sumber: Vec<(String, i64, f64)> = ov
        .sumber_dana
        .iter()
        .map(|b| (b.name.clone(), b.jumlah, b.nilai))
        .collect();
    bar_list(&mut p, MARGIN, 324.0, LEBAR, &sumber, false, 10);

    // Tabel ringkas nilai kategori teratas menurut nilai.
    judul_bagian(
        &mut p,
        MARGIN,
        560.0,
        LEBAR,
        "Ringkasan Nilai",
        Some(&periode.label()),
    );
    let mut y = 590.0;
    p.kotak(MARGIN, y, LEBAR, 18.0, "#eef1fa");
    p.teks(
        MARGIN + 6.0,
        y + 12.0,
        8.0,
        true,
        "#475569",
        "Kategori (nilai terbesar)",
    );
    p.teks_kanan(
        MARGIN + LEBAR - 6.0,
        y + 12.0,
        8.0,
        true,
        "#475569",
        "Nilai",
    );
    y += 18.0;
    for c in ov.categories.iter().take(6) {
        p.teks(
            MARGIN + 6.0,
            y + 12.0,
            8.5,
            false,
            "#334155",
            &Page::potong(&c.label, LEBAR - 120.0, 8.5, false),
        );
        p.teks_kanan(
            MARGIN + LEBAR - 6.0,
            y + 12.0,
            8.5,
            true,
            "#0f172a",
            &format_rupiah(c.nilai),
        );
        y += 17.0;
    }
    p
}

/// Gambar header/footer pada halaman pertama (tanpa garis footer ganda).
fn halaman_hias(p: &mut Page, judul: &str, nomor: usize, total: usize) {
    hiasan(p, judul, nomor, total);
    // Halaman sampul: footer cukup nama sistem.
    if nomor == 1 {
        p.garis(MARGIN, A4_H - 34.0, A4_W - MARGIN, A4_H - 34.0, GARIS, 0.6);
        p.teks(
            MARGIN,
            A4_H - 20.0,
            7.5,
            false,
            "#8b93a7",
            "Dasbor Analitik BMN - Kejaksaan Republik Indonesia",
        );
        p.teks_kanan(
            A4_W - MARGIN,
            A4_H - 20.0,
            7.5,
            false,
            "#8b93a7",
            &format!("Halaman 1 dari {total}"),
        );
    }
}

/// Bungkus teks menjadi paragraf dengan lebar tertentu.
fn teks_paragraf(
    p: &mut Page,
    x: f64,
    y: f64,
    lebar: f64,
    ukuran: f64,
    isi: &str,
    tinggi_baris: f64,
    maks_baris: usize,
) {
    let kata: Vec<&str> = isi.split_whitespace().collect();
    let mut baris = String::new();
    let mut ly = y;
    let mut n = 0;
    for k in kata {
        let coba = if baris.is_empty() {
            k.to_string()
        } else {
            format!("{baris} {k}")
        };
        if pdf::lebar_teks(&coba, ukuran, false) > lebar && !baris.is_empty() {
            p.teks(x, ly, ukuran, false, "#334155", &baris);
            ly += tinggi_baris;
            n += 1;
            if n >= maks_baris {
                return;
            }
            baris = k.to_string();
        } else {
            baris = coba;
        }
    }
    if !baris.is_empty() {
        p.teks(x, ly, ukuran, false, "#334155", &baris);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bucket, CategoryStat, TrendPoint};

    fn contoh_overview() -> Overview {
        Overview {
            total_aset: 1234,
            total_nilai: 1_500_000_000_000.0,
            total_susut: 250_000_000_000.0,
            total_kategori: 15,
            total_provinsi: 18,
            total_satker: 25,
            aset_idle: 12,
            aset_hilang: 3,
            aset_rusak: 44,
            categories: vec![
                CategoryStat {
                    label: "Tanah".into(),
                    icon: "🗺️".into(),
                    jumlah: 500,
                    nilai: 900_000_000_000.0,
                    volume: "1 MB".into(),
                },
                CategoryStat {
                    label: "Gedung".into(),
                    icon: "🏢".into(),
                    jumlah: 300,
                    nilai: 400_000_000_000.0,
                    volume: "1 MB".into(),
                },
            ],
            provinsi: vec![Bucket {
                name: "ACEH".into(),
                jumlah: 100,
                nilai: 1.0,
            }],
            satker: vec![Bucket {
                name: "Kejati Aceh".into(),
                jumlah: 80,
                nilai: 1.0,
            }],
            kondisi: vec![
                Bucket {
                    name: "Baik".into(),
                    jumlah: 900,
                    nilai: 1.0,
                },
                Bucket {
                    name: "Rusak Ringan".into(),
                    jumlah: 200,
                    nilai: 1.0,
                },
            ],
            perolehan: vec![
                TrendPoint {
                    year: 2019,
                    jumlah: 100,
                    nilai: 1.0e11,
                },
                TrendPoint {
                    year: 2020,
                    jumlah: 300,
                    nilai: 3.0e11,
                },
                TrendPoint {
                    year: 2021,
                    jumlah: 200,
                    nilai: 2.0e11,
                },
            ],
            sumber_dana: vec![Bucket {
                name: "APBN".into(),
                jumlah: 1000,
                nilai: 1.0,
            }],
            is_demo: true,
            generated_at: "2026-10-08 00:00:00 UTC".into(),
        }
    }

    #[test]
    fn laporan_pdf_lengkap_dan_sah() {
        let ov = contoh_overview();
        let periode = Periode {
            tahun: 2026,
            bulan: 10,
        };
        let b = bangun(&ov, &periode, "2026-10-08 06:00:00 UTC");
        let teks = String::from_utf8_lossy(&b);
        assert!(teks.starts_with("%PDF-1.4"));
        assert!(teks.ends_with("%%EOF\n"));
        assert!(teks.contains("/Count 5"), "laporan harus lima halaman");
        // Judul dan periode ikut tercetak.
        assert!(teks.contains("Laporan Bulanan"));
        assert!(teks.contains("Oktober 2026"));
        assert!(teks.contains("Laporan Bulanan BMN - Oktober 2026"));
        assert!(
            !teks.contains("Laporan Bulanan BMN -? Oktober"),
            "tanda hubung tidak boleh rusak"
        );
        assert!(teks.contains("Ringkasan Eksekutif"));
        assert!(teks.contains("Komposisi Nilai per Kategori"));
        assert!(teks.contains("Tren Perolehan"));
        assert!(teks.contains("Sebaran per Provinsi"));
        assert!(teks.contains("Kondisi Aset"));
        // Ukuran wajar (grafik vektor, bukan citra).
        assert!(b.len() > 4000, "pdf terlalu kecil: {}", b.len());
        // Simpan contoh untuk pemeriksaan manual bila diminta.
        if let Ok(dir) = std::env::var("LAPORAN_TULIS_CONTOH") {
            let _ = std::fs::write(format!("{dir}/contoh-laporan.pdf"), &b);
        }
    }

    #[test]
    fn periode_parsing_dan_rentang() {
        assert_eq!(
            Periode::dari_str("2026-10"),
            Some(Periode {
                tahun: 2026,
                bulan: 10
            })
        );
        assert_eq!(Periode::dari_str("2026-13"), None);
        assert_eq!(Periode::dari_str("x"), None);
        let p = Periode {
            tahun: 2024,
            bulan: 2,
        };
        assert_eq!(hari_dalam_bulan(2024, 2), 29);
        assert_eq!(hari_dalam_bulan(2023, 2), 28);
        assert_eq!(p.rentang(), "1-29 Februari 2024");
        assert_eq!(p.kode(), "2024-02");
    }
}
