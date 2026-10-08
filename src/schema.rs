//! Metadata kolom Master Aset BMN (106 kolom) hasil ekstraksi dokumen SLDK.
//! Dibangkitkan otomatis dari lampiran dokumen; jangan disunting manual.

use crate::model::ColumnDef;

/// 106 kolom Master Aset BMN: (nama, uraian, tipe, panjang).
pub const COLUMN_SPECS: &[(&str, &str, &str, &str)] = &[
    ("kd_jns_bmn", "Kode jenis BMN", "int", "-"),
    ("kd_brg", "Kode Barang", "text", "-"),
    ("no_aset", "No. Aset", "int", "-"),
    ("tercatat", "Tercatat", "varchar", "3"),
    ("merk", "Merk", "varchar", "500"),
    ("tipe", "Tipe", "varchar", "500"),
    ("rph_aset", "Nilai Aset (Rp)", "int", "-"),
    ("rph_susut", "Nilai Susut (Rp)", "int", "-"),
    ("rph_mutasi", "Nilai Mutasi (Rp)", "int", "-"),
    ("status_bmn_yn", "Status BMN", "nvarchar", "-"),
    ("bpybds_yn", "BPYBDS (Y/N)", "nvarchar", "-"),
    ("flag_sap", "Flag SAP", "nvarchar", "-"),
    ("jml_photo", "Jml. Foto", "int", "-"),
    ("ur_sskel", "Kelompok Aset", "varchar", "100"),
    ("alamat", "Alamat", "varchar", "200"),
    ("komplek", "Komplek", "varchar", "200"),
    ("kd_rtrw", "Kode RT/RW", "varchar", "15"),
    ("ur_kab", "Kabupaten", "varchar", "100"),
    ("ur_prov", "Provinsi", "varchar", "100"),
    ("kd_pos", "Kode Pos", "varchar", "10"),
    ("status_sbsn", "Status SBSN", "varchar", "50"),
    ("no_dana", "Nomor Dana", "varchar", "200"),
    ("tgl_dana", "Tanggal Dana", "datetime", "-"),
    ("asl_perlh", "Asal Perolehan", "varchar", "255"),
    ("tgl_perlh", "Tgl. Perolehan", "datetime", "-"),
    ("ur_sumber_dana", "Sumber Dana", "varchar", "30"),
    (
        "tgl_akhir_sbsn",
        "Tanggal akhir masa SBSN (Surat Berharga Syariah Negara)",
        "datetime",
        "-",
    ),
    ("status_bmn_idle", "BMN Idle", "varchar", "1"),
    (
        "rencana_hibah_yn",
        "Indikator apakah aset yn direncanakan untuk hibah (Y/N)",
        "varchar",
        "1",
    ),
    (
        "kmk_sbsn",
        "Nomor Keputusan Menteri Keuangan terkait SBSN",
        "varchar",
        "100",
    ),
    ("no_psp", "No. PSP", "varchar", "100"),
    ("tgl_psp", "Tgl. PSP", "datetime", "-"),
    ("luas", "Luas", "number", "-"),
    ("luas_tapak", "Luas tapak bangunan", "number", "-"),
    ("luas_tnhl", "Luas tanah lahan", "number", "-"),
    ("luas_tnhk", "Luas tanah kosong", "number", "-"),
    ("luas_tnhb", "Luas tanah bangunan", "number", "-"),
    ("jml_lantai", "Jml. Lantai", "number", "-"),
    ("jml_bdg", "Jml. Bangunan", "number", "-"),
    (
        "kd_jns_idle",
        "Kode jenis idle (kategori aset tidak digunakan)",
        "varchar",
        "2",
    ),
    (
        "no_perkara_hukum",
        "Nomor perkara hukum um terkait aset",
        "varchar",
        "100",
    ),
    (
        "no_dok_bukti_kepe",
        "Nomor dokumen bukti epemilikan kepemilikan",
        "varchar",
        "200",
    ),
    (
        "jns_dok_bukti_kepe",
        "Jenis dokumen bukti epemilikan kepemilikan",
        "varchar",
        "300",
    ),
    (
        "dihentikan_yn",
        "Indikator apakah penggunaan aset dihentikan (Y/N)",
        "nvarchar",
        "1",
    ),
    ("sbsk", "SBSK", "number", "-"),
    (
        "tgl_rekam_pertama",
        "Tanggal pertama kali data ma direkam",
        "datetime",
        "-",
    ),
    ("tgl_rekam", "Tgl. Rekam", "datetime", "-"),
    ("tgl_hapus", "Tgl. Hapus", "datetime", "-"),
    ("bts_utara", "Batas wilayah bagian utara", "varchar", "100"),
    (
        "bts_selatan",
        "Batas wilayah bagian selatan",
        "varchar",
        "100",
    ),
    ("bts_barat", "Batas wilayah bagian barat", "varchar", "100"),
    ("bts_timur", "Batas wilayah bagian timur", "varchar", "100"),
    ("bentuk", "Bentuk", "varchar", "30"),
    (
        "peruntukan_tnh",
        "Peruntukan tanah (fungsi atau tujuan penggunaan)",
        "varchar",
        "100",
    ),
    (
        "topografi_kontur",
        "Kontur topografi lokasi aset",
        "varchar",
        "30",
    ),
    (
        "topografi_elevasi",
        "Elevasi atau ketinggian lokasi aset",
        "varchar",
        "30",
    ),
    ("aksesibilitas", "Aksesibilitas", "varchar", "30"),
    ("gps_latitude", "Latitude", "number", "-"),
    ("gps_longitude", "Longitude", "number", "-"),
    ("kemitraan_yn", "Kemitraan", "varchar", "1"),
    ("brg_hilang_yn", "Barang Hilang", "varchar", "1"),
    ("dktp_yn", "DKTP", "varchar", "1"),
    ("brg_rusak_yn", "Barang Rusak", "varchar", "1"),
    ("umur_sisa", "Umur Sisa", "float", "-"),
    (
        "no_dok_perolehan",
        "Nomor dokumen perolehan an aset",
        "varchar",
        "200",
    ),
    ("jns_aset", "Jenis aset", "varchar", "70"),
    ("negara", "Negara asal atau lokasi aset", "varchar", "128"),
    ("rph_perolehan", "Nilai Perolehan (Rp)", "int", "-"),
    ("rph_buku", "Nilai Buku (Rp)", "int", "-"),
    ("cara_perlh", "Cara Perolehan", "varchar", "128"),
    ("jns_pengguna", "Jenis Pengguna", "varchar", "128"),
    (
        "kd_unit_pengguna",
        "Kode unit pengguna aset na",
        "varchar",
        "128",
    ),
    ("nm_unit_pengguna", "Unit Pengguna", "varchar", "128"),
    ("ket_pengguna", "Keterangan Pengguna", "varchar", "128"),
    (
        "stat_dok_bukti_kep",
        "Status dokumen bukti kepemilikan kepemilikan",
        "varchar",
        "128",
    ),
    (
        "tgl_dok_bukti_kepe",
        "Tanggal dokumen bukti epemilikan kepemilikan",
        "varchar",
        "-",
    ),
    ("no_kib", "No. KIB", "varchar", "-"),
    ("kode_satker", "Kode Satker", "varchar", "20"),
    ("nama_satker", "Satker", "varchar", "100"),
    (
        "kode_sub_satker",
        "Kode sub satuan kerja r",
        "varchar",
        "20",
    ),
    (
        "nama_sub_satker",
        "Nama sub satuan kerja er",
        "varchar",
        "100",
    ),
    ("ur_kondisi", "Kondisi", "varchar", "50"),
    (
        "nm_penghuni",
        "Nama penghuni aset (jika ada)",
        "varchar",
        "50",
    ),
    (
        "tgl_mulai_huni",
        "Tanggal mulai penghuni menempati aset",
        "datetime",
        "-",
    ),
    (
        "tgl_selesai_huni",
        "Tanggal selesai penghuni menempati aset",
        "datetime",
        "-",
    ),
    ("no_polisi", "No. Polisi", "varchar", "20"),
    ("urkpknl", "KPKNL", "varchar", "100"),
    ("ur_kanwil", "Kanwil", "varchar", "100"),
    ("ur_kl", "K/L", "varchar", "60"),
    ("ur_eselon1", "Eselon I", "varchar", "100"),
    ("ur_korwil", "Korwil", "varchar", "100"),
    ("kode_register", "Kode Register", "varchar", "100"),
    ("optimalisasi", "Optimalisasi", "number", "-"),
    ("tgl_buku_pertama", "Tgl. Buku Pertama", "datetime", "-"),
    ("jns_sertifikat", "Jenis Sertifikat", "varchar", "30"),
    ("intra_extra", "Intra/Extra", "text", "7"),
    ("ur_status", "Uraian status aset", "varchar", "100"),
    ("nama", "Nama Aset", "text", "500"),
    ("ur_kel", "Kelurahan", "varchar", "100"),
    ("ur_kec", "Kecamatan", "varchar", "100"),
    (
        "hapus_lainnya_yn",
        "Indikator penghapusan yn lainnya (Y/N)",
        "varchar",
        "1",
    ),
    ("konjas_yn", "Konstruksi Dalam Pengerjaan", "varchar", "1"),
    ("properti_investasi_y", "Properti Investasi", "varchar", "1"),
    (
        "lokasi_ruang",
        "Lokasi ruang atau tempat aset berada",
        "varchar",
        "500",
    ),
    ("luas_pemanfaatan", "Luas Pemanfaatan", "number", "-"),
    ("last_update", "Update Terakhir", "datetime", "-"),
];

/// Kolom untuk satu tabel; seluruh tabel memakai struktur yang sama.
pub fn columns_of(_table: &str) -> Vec<ColumnDef> {
    COLUMN_SPECS
        .iter()
        .map(|(name, label, tipe, len)| ColumnDef {
            name: name.to_string(),
            label: label.to_string(),
            r#type: tipe.to_string(),
            len: len.to_string(),
        })
        .collect()
}

/// Kolom ringkas untuk tampilan tabel data.
pub fn key_columns() -> Vec<ColumnDef> {
    let names = [
        "kd_brg",
        "nama",
        "ur_sskel",
        "merk",
        "rph_aset",
        "ur_kondisi",
        "ur_prov",
        "nama_satker",
        "tgl_perlh",
    ];
    columns_of("")
        .into_iter()
        .filter(|c| names.contains(&c.name.as_str()))
        .collect()
}

/// Jumlah kolom yang tersedia.
pub const COLUMN_COUNT: usize = COLUMN_SPECS.len();
