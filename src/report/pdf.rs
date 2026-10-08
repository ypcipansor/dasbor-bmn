//! Penulis PDF minimal tanpa dependensi luar.
//!
//! Laporan memakai font standar PDF (Helvetica) yang tidak perlu disematkan,
//! sehingga berkas tetap kecil dan tidak bergantung pada berkas font sistem.
//! Seluruh grafik digambar sebagai vektor (poligon, garis, teks), bukan citra.

#![cfg(feature = "ssr")]

/// Lebar satu halaman A4 dalam poin.
pub const A4_W: f64 = 595.28;
/// Tinggi satu halaman A4 dalam poin.
pub const A4_H: f64 = 841.89;

/// Lebar karakter Helvetica (per 1000 unit) untuk kode ASCII 32..=126.
const W_REG: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

/// Lebar karakter Helvetica-Bold (per 1000 unit) untuk kode ASCII 32..=126.
const W_BOLD: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667,
    611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556,
    278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

fn hex_rgb(hex: &str) -> (f64, f64, f64) {
    let h = hex.trim_start_matches('#');
    let ambil = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f64 / 255.0;
    if h.len() >= 6 {
        (ambil(0), ambil(2), ambil(4))
    } else {
        (0.0, 0.0, 0.0)
    }
}

/// Perkiraan lebar teks memakai tabel metrik Helvetica.
pub fn lebar_teks(s: &str, ukuran: f64, tebal: bool) -> f64 {
    let tabel = if tebal { &W_BOLD } else { &W_REG };
    s.chars()
        .map(|c| {
            let b = c as u32;
            if (32..=126).contains(&b) {
                tabel[(b - 32) as usize] as f64
            } else {
                556.0
            }
        })
        .sum::<f64>()
        / 1000.0
        * ukuran
}

/// Petakan satu karakter ke byte WinAnsi/ASCII yang aman.
///
/// Tanda baca tipografi dipetakan ke padanan ASCII agar tetap terbaca dengan
/// font standar, sedangkan karakter di luar cakupan menjadi tanda tanya.
fn map_byte(c: char) -> u8 {
    let b = c as u32;
    match b {
        0x2013 | 0x2014 | 0x2015 => b'-',
        0x00b7 | 0x2022 | 0x25cf => b'-',
        0x2018 | 0x2019 => b'\'',
        0x201c | 0x201d => b'"',
        0x2026 => b'.',
        0x00a0 => b' ',
        0x20..=0x7e | 0xa0..=0xff => b as u8,
        _ => b'?',
    }
}

/// Tulis teks ke aliran PDF dengan escape dan pemetaan ke Latin-1.
fn push_teks(buf: &mut Vec<u8>, s: &str) {
    for c in s.chars() {
        match c {
            '(' => buf.extend_from_slice(b"\\("),
            ')' => buf.extend_from_slice(b"\\)"),
            '\\' => buf.extend_from_slice(b"\\\\"),
            c => buf.push(map_byte(c)),
        }
    }
}

/// Satu halaman; koordinat gambar memakai acuan kiri-atas seperti CSS.
pub struct Page {
    pub w: f64,
    pub h: f64,
    buf: Vec<u8>,
}

impl Page {
    pub fn new(w: f64, h: f64) -> Self {
        Page {
            w,
            h,
            buf: Vec::new(),
        }
    }

    /// Balik ordinat kiri-atas menjadi ordinat PDF (kiri-bawah).
    fn yy(&self, td: f64) -> f64 {
        self.h - td
    }

    fn op(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
    }

    pub fn isi(&mut self, hex: &str) {
        let (r, g, b) = hex_rgb(hex);
        self.op(&format!("{r:.3} {g:.3} {b:.3} rg\n"));
    }

    /// Persegi terisi (acuan kiri-atas).
    pub fn kotak(&mut self, x: f64, y: f64, w: f64, h: f64, warna: &str) {
        let (r, g, b) = hex_rgb(warna);
        self.op(&format!(
            "{r:.3} {g:.3} {b:.3} rg {x:.2} {:.2} {w:.2} {h:.2} re f\n",
            self.yy(y + h)
        ));
    }

    /// Persegi berongga (garis tepi).
    pub fn kotak_tepi(&mut self, x: f64, y: f64, w: f64, h: f64, warna: &str, lw: f64) {
        let (r, g, b) = hex_rgb(warna);
        self.op(&format!(
            "{r:.3} {g:.3} {b:.3} RG {lw:.2} w {x:.2} {:.2} {w:.2} {h:.2} re S\n",
            self.yy(y + h)
        ));
    }

    pub fn garis(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, warna: &str, lw: f64) {
        let (r, g, b) = hex_rgb(warna);
        self.op(&format!(
            "{r:.3} {g:.3} {b:.3} RG {lw:.2} w {x1:.2} {:.2} m {x2:.2} {:.2} l S\n",
            self.yy(y1),
            self.yy(y2)
        ));
    }

    /// Poligon terisi dari titik-titik acuan kiri-atas.
    pub fn poligon(&mut self, titik: &[(f64, f64)], warna: &str) {
        if titik.len() < 3 {
            return;
        }
        let (r, g, b) = hex_rgb(warna);
        let mut s = format!("{r:.3} {g:.3} {b:.3} rg ");
        for (i, (x, y)) in titik.iter().enumerate() {
            s.push_str(&format!(
                "{x:.2} {:.2} {} ",
                self.yy(*y),
                if i == 0 { "m" } else { "l" }
            ));
        }
        s.push_str("h f\n");
        self.op(&s);
    }

    /// Garis bersambung (polyline) acuan kiri-atas.
    pub fn garis_poli(&mut self, titik: &[(f64, f64)], warna: &str, lw: f64) {
        if titik.len() < 2 {
            return;
        }
        let (r, g, b) = hex_rgb(warna);
        let mut s = format!("{r:.3} {g:.3} {b:.3} RG {lw:.2} w ");
        for (i, (x, y)) in titik.iter().enumerate() {
            s.push_str(&format!(
                "{x:.2} {:.2} {} ",
                self.yy(*y),
                if i == 0 { "m" } else { "l" }
            ));
        }
        s.push_str("S\n");
        self.op(&s);
    }

    /// Mulai jalur baru pada titik acuan kiri-atas.
    pub fn jalur_mulai(&mut self, x: f64, y: f64) {
        self.op(&format!("{x:.2} {:.2} m\n", self.yy(y)));
    }

    /// Tambah segmen garis lurus.
    pub fn jalur_ke(&mut self, x: f64, y: f64) {
        self.op(&format!("{x:.2} {:.2} l\n", self.yy(y)));
    }

    /// Lanjutkan jalur dengan busur elips dari sudut `a` sejauh `delta` radian
    /// (positif = searah jarum jam matematis/berlawanan di layar). Tidak memulai
    /// subjalur baru, sehingga aman disambung dengan `jalur_ke` untuk cincin.
    #[allow(clippy::too_many_arguments)]
    pub fn jalur_busur(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, mut a: f64, delta: f64) {
        if delta.abs() < 1e-9 {
            return;
        }
        // Target ~30 derajat per segmen supaya busur tetap halus.
        let n = ((delta.abs() / (std::f64::consts::PI / 6.0)).ceil() as usize).max(1);
        let step = delta / n as f64;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        let pindah = |t: f64| (cx + rx * t.cos(), cy + ry * t.sin());
        let (mut x0, mut y0) = pindah(a);
        for _ in 0..n {
            let a1b = a + step;
            let (x1, y1) = pindah(a1b);
            let c1 = (x0 - k * rx * a.sin(), y0 + k * ry * a.cos());
            let c2 = (x1 + k * rx * a1b.sin(), y1 - k * ry * a1b.cos());
            self.op(&format!(
                "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n",
                c1.0,
                self.yy(c1.1),
                c2.0,
                self.yy(c2.1),
                x1,
                self.yy(y1)
            ));
            x0 = x1;
            y0 = y1;
            a = a1b;
        }
    }

    /// Tutup jalur.
    pub fn jalur_tutup(&mut self) {
        self.op("h\n");
    }

    /// Isi jalur dengan warna.
    pub fn jalur_isi(&mut self, hex: &str) {
        let (r, g, b) = hex_rgb(hex);
        self.op(&format!("{r:.3} {g:.3} {b:.3} rg f\n"));
    }

    /// Gambar garis tepi jalur dengan lebar tertentu.
    pub fn jalur_tepi(&mut self, hex: &str, lw: f64) {
        let (r, g, b) = hex_rgb(hex);
        self.op(&format!("{r:.3} {g:.3} {b:.3} RG {lw:.2} w S\n"));
    }

    /// Balik ordinat kiri-atas menjadi ordinat PDF (kiri-bawah) — untuk driver luar.
    pub fn balik(&self, td: f64) -> f64 {
        self.yy(td)
    }

    /// Teks pada garis alas (baseline) dalam acuan kiri-atas.
    pub fn teks(
        &mut self,
        x: f64,
        baseline: f64,
        ukuran: f64,
        tebal: bool,
        warna: &str,
        isi: &str,
    ) {
        let (r, g, b) = hex_rgb(warna);
        let f = if tebal { "/F2" } else { "/F1" };
        let py = self.yy(baseline);
        self.op(&format!(
            "BT {r:.3} {g:.3} {b:.3} rg {f} {ukuran:.2} Tf 1 0 0 1 {x:.2} {py:.2} Tm ("
        ));
        push_teks(&mut self.buf, isi);
        self.op(") Tj ET\n");
    }

    /// Teks rata tengah pada `cx`.
    pub fn teks_tengah(
        &mut self,
        cx: f64,
        baseline: f64,
        ukuran: f64,
        tebal: bool,
        warna: &str,
        isi: &str,
    ) {
        let x = cx - lebar_teks(isi, ukuran, tebal) / 2.0;
        self.teks(x, baseline, ukuran, tebal, warna, isi);
    }

    /// Teks rata kanan pada `x_kanan`.
    pub fn teks_kanan(
        &mut self,
        x_kanan: f64,
        baseline: f64,
        ukuran: f64,
        tebal: bool,
        warna: &str,
        isi: &str,
    ) {
        let x = x_kanan - lebar_teks(isi, ukuran, tebal);
        self.teks(x, baseline, ukuran, tebal, warna, isi);
    }

    /// Potong teks agar muat dalam `lebar` dengan elipsis.
    pub fn potong(isi: &str, lebar: f64, ukuran: f64, tebal: bool) -> String {
        if lebar_teks(isi, ukuran, tebal) <= lebar {
            return isi.to_string();
        }
        let mut s = String::new();
        for c in isi.chars() {
            let coba = format!("{s}{c}…");
            if lebar_teks(&coba, ukuran, tebal) > lebar {
                break;
            }
            s.push(c);
        }
        format!("{s}…")
    }
}

/// Dokumen PDF berisi kumpulan halaman.
pub struct Dokumen {
    pub judul: String,
    halaman: Vec<Page>,
}

impl Dokumen {
    pub fn new(judul: &str) -> Self {
        Dokumen {
            judul: judul.to_string(),
            halaman: Vec::new(),
        }
    }

    pub fn tambah(&mut self, p: Page) {
        self.halaman.push(p);
    }

    pub fn jumlah_halaman(&self) -> usize {
        self.halaman.len()
    }

    /// Susun berkas PDF lengkap (dengan xref dan trailer yang benar).
    pub fn bangun(&self, tanggal_pdf: &str) -> Vec<u8> {
        let n = self.halaman.len();
        // Penomoran objek: 1 katalog, 2 halaman-induk, 3..5 font, lalu pasangan
        // (objek halaman, aliran isi) per halaman, dan objek Info di akhir.
        let id_info = 6 + 2 * n;
        let total = id_info + 1;

        let mut objs: Vec<(usize, Vec<u8>)> = Vec::new();
        objs.push((1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()));

        let kids: Vec<String> = (0..n).map(|k| format!("{} 0 R", 6 + 2 * k)).collect();
        let induk = format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" "));
        objs.push((2, induk.into_bytes()));

        let font = |id: usize, nama: &str| {
            (
                id,
                format!(
                    "<< /Type /Font /Subtype /Type1 /BaseFont /{nama} /Encoding /WinAnsiEncoding >>"
                )
                .into_bytes(),
            )
        };
        objs.push(font(3, "Helvetica"));
        objs.push(font(4, "Helvetica-Bold"));
        objs.push(font(5, "Helvetica-Oblique"));

        for (k, p) in self.halaman.iter().enumerate() {
            let id_hal = 6 + 2 * k;
            let id_isi = 7 + 2 * k;
            let hal = format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> \
                 /Contents {id_isi} 0 R >>",
                p.w, p.h
            );
            objs.push((id_hal, hal.into_bytes()));
            let mut aliran = format!("<< /Length {} >>\nstream\n", p.buf.len()).into_bytes();
            aliran.extend_from_slice(&p.buf);
            aliran.extend_from_slice(b"\nendstream");
            objs.push((id_isi, aliran));
        }

        let info = format!(
            "<< /Title ({}) /Producer (Dasbor BMN) /Creator (Dasbor BMN) /CreationDate ({tanggal_pdf}) >>",
            escape_info(&self.judul)
        );
        objs.push((id_info, info.into_bytes()));
        objs.sort_by_key(|(id, _)| *id);

        let mut out: Vec<u8> = b"%PDF-1.4\n".to_vec();
        out.extend_from_slice(&[b'%', 0xe2, 0xe3, 0xcf, 0xd3, b'\n']);
        let mut offset = vec![0usize; total];
        for (id, body) in &objs {
            offset[*id] = out.len();
            out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }

        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {total}\n").as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for id in 1..total {
            out.extend_from_slice(format!("{:010} 00000 n \n", offset[id]).as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {total} /Root 1 0 R /Info {id_info} 0 R >>\nstartxref\n{xref}\n%%EOF\n"
            )
            .as_bytes(),
        );
        out
    }
}

fn escape_info(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '(' => o.push_str("\\("),
            ')' => o.push_str("\\)"),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push(' '),
            c if (c as u32) > 0xff => o.push('?'),
            c => o.push(c),
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dokumen_pdf_sah() {
        let mut d = Dokumen::new("Uji");
        let mut p = Page::new(A4_W, A4_H);
        p.kotak(40.0, 40.0, 200.0, 60.0, "#1f47f5");
        p.teks(50.0, 70.0, 14.0, true, "#ffffff", "Halo (uji) \\ miring");
        p.garis_poli(
            &[(40.0, 120.0), (100.0, 90.0), (160.0, 130.0)],
            "#10b981",
            2.0,
        );
        d.tambah(p);
        let b = d.bangun("D:20260101120000Z");
        let teks = String::from_utf8_lossy(&b);
        assert!(teks.starts_with("%PDF-1.4"));
        assert!(teks.trim_end().ends_with("%%EOF"));
        assert!(teks.contains("/Type /Catalog"));
        assert!(teks.contains("/Count 1"));
        assert!(teks.contains("startxref"));
        assert!(teks.contains("Halo \\(uji\\) \\\\ miring"));
    }

    #[test]
    fn lebar_teks_wajar() {
        assert_eq!(lebar_teks("", 12.0, false), 0.0);
        let a = lebar_teks("AAAA", 10.0, false);
        let b = lebar_teks("iiii", 10.0, false);
        assert!(a > b, "huruf lebar harus lebih lebar dari huruf sempit");
        assert!((lebar_teks("W", 10.0, false) - 9.44).abs() < 0.01);
    }

    #[test]
    fn potong_menambah_elipsis() {
        let s = Page::potong("Kalimat yang sangat panjang sekali", 40.0, 10.0, false);
        assert!(s.ends_with('…'));
        assert!(lebar_teks(&s, 10.0, false) <= 40.0);
    }
}
