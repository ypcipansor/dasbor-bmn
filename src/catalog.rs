use crate::model::TableDef;

/// Katalog 15 tabel Master Aset BMN dari layanan SLDK SIMAN v2 (DJKN).
pub const TABLES: &[TableDef] = &[
    TableDef { table: "SIMAN2_M_ASET_TANAH", resource: "getAsetTanah", label: "Tanah", nama_data: "Master Aset Tanah", volume: "4.3 MB", icon: "🗺️", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_GEDUNG_BANGUNAN", resource: "getAsetGedungBangunan", label: "Gedung & Bangunan", nama_data: "Master Aset Gedung Bangunan", volume: "9 MB", icon: "🏢", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_RUMAH", resource: "getAsetRumah", label: "Rumah Negara", nama_data: "Master Aset Rumah Negara", volume: "7.4 MB", icon: "🏠", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_ANGKUTAN_BERMOTOR", resource: "getAsetAngkutanBermotor", label: "Angkutan Bermotor", nama_data: "Master Aset Angkutan Bermotor", volume: "18 MB", icon: "🚗", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_ALAT_BESAR", resource: "getAsetAlatBesar", label: "Alat Besar", nama_data: "Master Aset Alat Besar", volume: "2.2 MB", icon: "🚜", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_ALAT_PERSENJATAAN", resource: "getAsetAlatPersenjataan", label: "Alat Persenjataan", nama_data: "Master Aset Alat Persenjataan", volume: "1.5 MB", icon: "🎯", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_JALAN_DAN_JEMBATAN", resource: "getAsetJalandanJembatan", label: "Jalan & Jembatan", nama_data: "Master Aset Jalan dan Jembatan", volume: "260 KB", icon: "🛣️", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_BANGUNAN_AIR", resource: "getAsetBangunanAir", label: "Bangunan Air", nama_data: "Master Aset Bangunan Air", volume: "300 KB", icon: "🌊", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_INSTALASI_JARINGAN", resource: "getAsetInstalasiJaringan", label: "Instalasi & Jaringan", nama_data: "Master Aset Instalasi Jaringan", volume: "9.1 MB", icon: "🔌", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_KHUSUS_TIK", resource: "getAsetKhususTIK", label: "Aset Khusus TIK", nama_data: "Master Aset Khusus TIK", volume: "3.8 MB", icon: "🛰️", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_NON_TIK", resource: "getAsetNonTIK", label: "Aset Non TIK", nama_data: "Master Aset Non TIK", volume: "867 MB", icon: "📦", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_ASET_TAK_BERWUJUD", resource: "getAsetTakBerwujud", label: "Aset Tak Berwujud", nama_data: "Master Aset Tak Berwujud", volume: "8.2 MB", icon: "💾", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_ASET_TETAP_LAINNYA", resource: "getAsetTetapLainnya", label: "Aset Tetap Lainnya", nama_data: "Master Aset Tetap Lainnya", volume: "142 MB", icon: "🗄️", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_ASET_TETAP_RENOVASI", resource: "getAsetTetapRenovasi", label: "Aset Tetap Renovasi", nama_data: "Master Aset Tetap Renovasi", volume: "4.8 MB", icon: "🧱", value_column: "rph_aset" },
    TableDef { table: "SIMAN2_M_ASET_KDP", resource: "getAsetKDP", label: "Konstruksi Dalam Pengerjaan", nama_data: "Master Aset KDP", volume: "696 KB", icon: "🏗️", value_column: "rph_aset" },
];

pub fn find(table: &str) -> Option<&'static TableDef> {
    TABLES.iter().find(|t| t.table == table)
}

pub fn label_of(table: &str) -> String {
    find(table).map(|t| t.label.to_string()).unwrap_or_else(|| table.to_string())
}
