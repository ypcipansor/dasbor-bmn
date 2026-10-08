//! Handler HTTP untuk unduhan laporan PDF.

#![cfg(feature = "ssr")]

use std::collections::HashMap;

use axum::extract::Query;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::report::Periode;
use crate::{analytics, catalog, config, demo, store};

/// Bangun laporan dan kirim sebagai berkas PDF.
///
/// `?periode=YYYY-MM` menentukan bulan pelaporan; bila tidak ada, bulan berjalan
/// menurut zona waktu Indonesia Barat yang dipakai.
pub async fn laporan_pdf(Query(params): Query<HashMap<String, String>>) -> Response {
    let periode = params
        .get("periode")
        .and_then(|s| Periode::dari_str(s))
        .unwrap_or_else(Periode::sekarang);

    if !config::get().is_configured() && store::all_stats().is_empty() {
        demo::seed();
    }
    let stats = store::all_stats();
    let parts: Vec<(String, String, String, analytics::TableStats)> = stats
        .into_iter()
        .filter_map(|(table, st, _)| {
            catalog::find(&table).map(|t| {
                (
                    t.label.to_string(),
                    t.icon.to_string(),
                    t.volume.to_string(),
                    st,
                )
            })
        })
        .collect();
    let mut ov = analytics::merge(&parts, demo::is_demo());
    ov.generated_at = crate::now_string();
    let dibuat_pada = ov.generated_at.clone();

    let bytes = crate::report::bangun(&ov, &periode, &dibuat_pada);
    let nama = format!("laporan-bmn-{}.pdf", periode.kode());

    let mut res = (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/pdf"),
            ),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&format!("attachment; filename=\"{nama}\"")).unwrap(),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        bytes,
    )
        .into_response();
    res.headers_mut().insert(
        header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    res
}
