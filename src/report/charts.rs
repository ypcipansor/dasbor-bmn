//! Penggambar grafik vektor untuk laporan PDF.
//!
//! Bentuk visual mengikuti komponen SVG pada dasbor (donut, area, bar, kartu KPI)
//! agar laporan terasa satu bahasa desain dengan aplikasi.

#![cfg(feature = "ssr")]

use super::pdf::{lebar_teks, Page};
use crate::model::{format_number, format_rupiah_short};

/// Palet seragam dengan grafik di web.
pub const PALETTE: &[&str] = &[
    "#1f47f5", "#0ea5e9", "#10b981", "#f59e0b", "#ef4444", "#8b5cf6", "#ec4899", "#14b8a6",
    "#f97316", "#6366f1", "#84cc16", "#06b6d4",
];

/// Warna garis bantu dan latar netral.
pub const GARIS: &str = "#d7dced";
pub const TRACK: &str = "#e8ebf5";

pub fn warna_ke(i: usize) -> &'static str {
    PALETTE[i % PALETTE.len()]
}

fn warna_tone(tone: &str) -> &'static str {
    match tone {
        "emerald" => "#10b981",
        "amber" => "#f59e0b",
        "rose" => "#ef4444",
        "violet" => "#8b5cf6",
        _ => "#1f47f5",
    }
}

/// Judul bagian dengan garis aksen di kiri.
pub fn judul_bagian(p: &mut Page, x: f64, y: f64, w: f64, judul: &str, sub: Option<&str>) {
    p.kotak(x, y, 3.5, 14.0, "#1f47f5");
    p.teks(x + 10.0, y + 10.5, 11.5, true, "#0f172a", judul);
    if let Some(s) = sub {
        let lw = lebar_teks(judul, 11.5, true);
        p.teks(x + 16.0 + lw, y + 10.5, 8.5, false, "#8b93a7", s);
    }
    p.garis(x, y + 19.0, x + w, y + 19.0, GARIS, 0.8);
}

/// Kartu KPI: label kecil, nilai besar, keterangan, dan pita warna di tepi.
#[allow(clippy::too_many_arguments)]
pub fn kartu_kpi(
    p: &mut Page,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    label: &str,
    nilai: &str,
    hint: &str,
    tone: &str,
) {
    p.kotak(x, y, w, h, "#f6f8fd");
    p.kotak_tepi(x, y, w, h, GARIS, 0.7);
    p.kotak(x, y, 3.0, h, warna_tone(tone));
    p.teks(
        x + 11.0,
        y + 14.0,
        7.5,
        true,
        "#8b93a7",
        &label.to_uppercase(),
    );
    let uk = if lebar_teks(nilai, 16.0, true) > w - 22.0 {
        12.5
    } else {
        16.0
    };
    p.teks(x + 11.0, y + 33.0, uk, true, "#0f172a", nilai);
    p.teks(x + 11.0, y + 47.0, 7.5, false, "#8b93a7", hint);
}

/// Donut komposisi dengan legenda berkolom.
pub fn donut(
    p: &mut Page,
    cx: f64,
    cy: f64,
    r: f64,
    tebal_ring: f64,
    data: &[(String, f64)],
    label_pusat: &str,
    nilai_pusat: &str,
) {
    let total: f64 = data.iter().map(|(_, v)| *v).sum();
    let r_dalam = r - tebal_ring;
    let r_tengah = (r + r_dalam) / 2.0;
    let mut mulai = -std::f64::consts::FRAC_PI_2;
    let tai = 2.0 * std::f64::consts::PI;

    // Cincin latar, tampak bila ada kategori bernilai nol.
    p.jalur_mulai(cx, cy - r);
    p.jalur_busur(cx, cy, r, r, -std::f64::consts::FRAC_PI_2, tai);
    p.jalur_ke(cx, cy - r_dalam);
    p.jalur_busur(cx, cy, r_dalam, r_dalam, -std::f64::consts::FRAC_PI_2, -tai);
    p.jalur_tutup();
    p.jalur_isi(TRACK);

    for (i, (_, v)) in data.iter().enumerate() {
        if total <= 0.0 || *v <= 0.0 {
            continue;
        }
        let bagi = (*v / total) * tai;
        let akhir = mulai + bagi;
        p.jalur_mulai(cx + r * mulai.cos(), cy + r * mulai.sin());
        p.jalur_busur(cx, cy, r, r, mulai, bagi);
        p.jalur_ke(cx + r_dalam * akhir.cos(), cy + r_dalam * akhir.sin());
        p.jalur_busur(cx, cy, r_dalam, r_dalam, akhir, -bagi);
        p.jalur_tutup();
        p.jalur_isi(warna_ke(i));
        mulai = akhir;
    }

    p.teks_tengah(
        cx,
        cy - 9.0,
        7.0,
        true,
        "#8b93a7",
        &label_pusat.to_uppercase(),
    );
    p.teks_tengah(cx, cy + 12.0, 12.5, true, "#0f172a", nilai_pusat);

    // Legenda di sebelah kanan donut.
    let lx = cx + r + 26.0;
    let mut ly = cy - r + 4.0;
    for (i, (label, v)) in data.iter().enumerate() {
        let pct = if total > 0.0 { v / total * 100.0 } else { 0.0 };
        p.kotak(lx, ly - 5.5, 7.0, 7.0, warna_ke(i));
        let nama = Page::potong(label, 150.0, 8.5, false);
        p.teks(lx + 12.0, ly, 8.5, false, "#334155", &nama);
        p.teks_kanan(lx + 236.0, ly, 8.5, true, "#0f172a", &format!("{pct:.1}%"));
        ly += 16.0;
    }
    let _ = r_tengah;
}

/// Diagram area dengan sumbu, grid, dan label tahun untuk tren perolehan.
pub fn area(
    p: &mut Page,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    titik: &[crate::model::TrendPoint],
    nilai_rupiah: bool,
) {
    if titik.len() < 2 {
        p.teks(
            x,
            y + h / 2.0,
            9.0,
            false,
            "#8b93a7",
            "Data tren belum cukup untuk digambar.",
        );
        return;
    }
    let pad_kiri = 52.0;
    let pad_bawah = 22.0;
    let pad_atas = 10.0;
    let pw = w - pad_kiri - 8.0;
    let ph = h - pad_atas - pad_bawah;
    let x0 = x + pad_kiri;
    let y0 = y + pad_atas;

    let nilai = |t: &crate::model::TrendPoint| {
        if nilai_rupiah {
            t.nilai
        } else {
            t.jumlah as f64
        }
    };
    let maks = titik.iter().map(nilai).fold(0.0_f64, f64::max).max(1.0);
    let n = titik.len() as f64;

    // Grid mendatar + label nilai.
    for i in 0..=4 {
        let gy = y0 + ph - (i as f64 / 4.0) * ph;
        p.garis(x0, gy, x0 + pw, gy, GARIS, 0.6);
        let v = maks * (i as f64 / 4.0);
        let teks = if nilai_rupiah {
            format_rupiah_short(v)
        } else {
            format_number(v.round() as i64)
        };
        p.teks_kanan(x0 - 6.0, gy + 3.0, 7.0, false, "#8b93a7", &teks);
    }

    let xy: Vec<(f64, f64)> = titik
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let px = x0 + (i as f64 / (n - 1.0)) * pw;
            let py = y0 + ph - (nilai(t) / maks) * ph;
            (px, py)
        })
        .collect();

    // Area terisi sebagai poligon tertutup ke garis alas.
    let mut area_pts: Vec<(f64, f64)> = Vec::with_capacity(xy.len() + 2);
    area_pts.push((x0, y0 + ph));
    area_pts.extend(xy.iter().copied());
    area_pts.push((x0 + pw, y0 + ph));
    p.poligon(&area_pts, "#dbe3fb");

    p.garis_poli(&xy, "#1f47f5", 2.0);
    for (i, (px, py)) in xy.iter().enumerate() {
        p.kotak(px - 2.0, py - 2.0, 4.0, 4.0, "#ffffff");
        p.kotak_tepi(px - 2.0, py - 2.0, 4.0, 4.0, "#1f47f5", 1.2);
        // Label tahun: semua bila sedikit, sebaliknya tiap beberapa titik.
        let langkah = (titik.len() / 9).max(1);
        if i % langkah == 0 || i == titik.len() - 1 {
            p.teks_tengah(
                *px,
                y + h - 6.0,
                7.0,
                false,
                "#8b93a7",
                &titik[i].year.to_string(),
            );
        }
    }
}

/// Diagram batang horizontal untuk peringkat (provinsi, satker, kondisi).
pub fn bar_list(
    p: &mut Page,
    x: f64,
    y: f64,
    w: f64,
    data: &[(String, i64, f64)],
    nilai_rupiah: bool,
    limit: usize,
) {
    let maks = data.iter().map(|(_, j, _)| *j).max().unwrap_or(1).max(1);
    let mut ly = y;
    for (i, (nama, jumlah, nilai)) in data.iter().take(limit).enumerate() {
        let val = if nilai_rupiah {
            format_rupiah_short(*nilai)
        } else {
            format_number(*jumlah)
        };
        let lebar_teks_val = 78.0;
        let nama_max = w - lebar_teks_val - 4.0;
        let nama_tampil = Page::potong(nama, nama_max, 8.0, false);
        p.teks(x, ly, 8.0, false, "#334155", &nama_tampil);
        p.teks_kanan(x + w, ly, 8.0, true, "#0f172a", &val);
        let track_y = ly + 3.5;
        let track_w = w;
        p.kotak(x, track_y, track_w, 5.0, TRACK);
        let pct = (*jumlah as f64 / maks as f64).clamp(0.0, 1.0);
        p.kotak(x, track_y, (track_w * pct).max(2.0), 5.0, warna_ke(i));
        ly += 20.0;
    }
}

/// Diagram batang vertikal sederhana untuk perbandingan antar kategori.
pub fn bar_vertikal(
    p: &mut Page,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    data: &[(String, f64)],
    nilai_rupiah: bool,
) {
    if data.is_empty() {
        return;
    }
    let pad_bawah = 34.0;
    let pad_atas = 16.0;
    let ph = h - pad_bawah - pad_atas;
    let maks = data
        .iter()
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let n = data.len() as f64;
    let celah = 6.0;
    let lebar_batang = ((w - celah * (n + 1.0)) / n).max(4.0);
    for (i, (label, v)) in data.iter().enumerate() {
        let bx = x + celah + i as f64 * (lebar_batang + celah);
        let bh = (*v / maks) * ph;
        let by = y + pad_atas + (ph - bh);
        p.kotak(bx, by, lebar_batang, bh, warna_ke(i));
        if nilai_rupiah {
            p.teks_tengah(
                bx + lebar_batang / 2.0,
                by - 3.0,
                6.5,
                true,
                "#0f172a",
                &format_rupiah_short(*v),
            );
        }
        // Label di bawah, dibungkus dua baris bila perlu.
        let singkat = Page::potong(label, lebar_batang + 14.0, 6.5, false);
        let belah = belah_dua(&singkat, 12);
        for (k, baris) in belah.iter().enumerate() {
            p.teks_tengah(
                bx + lebar_batang / 2.0,
                y + pad_atas + ph + 9.0 + k as f64 * 8.0,
                6.5,
                false,
                "#8b93a7",
                baris,
            );
        }
    }
}

fn belah_dua(s: &str, maks: usize) -> Vec<String> {
    if s.chars().count() <= maks {
        return vec![s.to_string()];
    }
    let mut a = String::new();
    let mut b = String::new();
    for (i, c) in s.chars().enumerate() {
        if i < maks {
            a.push(c);
        } else if b.chars().count() < maks {
            b.push(c);
        }
    }
    if b.chars().count() > maks {
        b = format!("{}…", &b[..maks.saturating_sub(1)]);
    }
    vec![a, b]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::pdf::{Dokumen, A4_H, A4_W};

    #[test]
    fn grafik_menggambar_tanpa_panik() {
        let mut d = Dokumen::new("Uji Grafik");
        let mut p = Page::new(A4_W, A4_H);
        judul_bagian(&mut p, 40.0, 40.0, 515.0, "Komposisi", Some("uji"));
        kartu_kpi(
            &mut p, 40.0, 70.0, 120.0, 56.0, "Total", "1.234", "unit", "blue",
        );
        donut(
            &mut p,
            120.0,
            260.0,
            54.0,
            16.0,
            &[("Tanah".into(), 60.0), ("Gedung".into(), 40.0)],
            "Total",
            "Rp 1 M",
        );
        area(
            &mut p,
            40.0,
            380.0,
            515.0,
            140.0,
            &[
                crate::model::TrendPoint {
                    year: 2019,
                    jumlah: 10,
                    nilai: 5.0,
                },
                crate::model::TrendPoint {
                    year: 2020,
                    jumlah: 30,
                    nilai: 9.0,
                },
                crate::model::TrendPoint {
                    year: 2021,
                    jumlah: 20,
                    nilai: 7.0,
                },
            ],
            false,
        );
        bar_list(
            &mut p,
            40.0,
            560.0,
            515.0,
            &[("A".into(), 10, 1.0), ("B".into(), 5, 2.0)],
            false,
            10,
        );
        bar_vertikal(
            &mut p,
            40.0,
            650.0,
            515.0,
            120.0,
            &[("Kelompok A".into(), 10.0), ("Kelompok B".into(), 20.0)],
            false,
        );
        d.tambah(p);
        let b = d.bangun("D:20260101120000Z");
        assert!(String::from_utf8_lossy(&b).starts_with("%PDF-1.4"));
    }
}
