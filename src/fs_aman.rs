//! Utilitas berkas untuk data sensitif.
//!
//! Berkas yang memuat rahasia (kredensial SSO, kata sandi dasbor) ditulis
//! dengan mode `0600` dan direktori induknya `0700`, sehingga pengguna lokal
//! lain tidak dapat membacanya — apa pun umask proses.

#![cfg(feature = "ssr")]

use std::io::Write;
use std::path::Path;

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> std::io::Result<()> {
    Ok(())
}

/// Tulis berkas rahasia dengan mode `0600`, mengganti isi lama sepenuhnya.
pub fn tulis_privat(path: &Path, isi: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        // Direktori induk juga dibatasi agar isi tidak dapat dibaca pihak lain.
        let _ = set_mode(parent, 0o700);
    }
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    f.write_all(isi)?;
    f.sync_all()?;
    drop(f);
    set_mode(path, 0o600)
}

/// Perketat izin berkas yang sudah ada (dipakai saat memuat konfigurasi lama).
pub fn rapatkan(path: &Path) {
    if path.exists() {
        let _ = set_mode(path, 0o600);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn berkas_ditulis_dengan_mode_0600() {
        let dir = std::env::temp_dir().join(format!("uji-fs-aman-{}", std::process::id()));
        let path = dir.join("rahasia.json");
        tulis_privat(&path, br#"{"a":1}"#).unwrap();
        let isi = std::fs::read_to_string(&path).unwrap();
        assert_eq!(isi, r#"{"a":1}"#);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "mode berkas harus 0600, dapat {mode:o}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn menulis_ulang_mengganti_isi_lama() {
        let dir = std::env::temp_dir().join(format!("uji-fs-aman2-{}", std::process::id()));
        let path = dir.join("rahasia.json");
        tulis_privat(&path, b"panjang-panjang-panjang").unwrap();
        tulis_privat(&path, b"pendek").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "pendek");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
