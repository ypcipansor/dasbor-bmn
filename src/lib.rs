pub mod analytics;
pub mod app;
pub mod auth;
pub mod catalog;
pub mod components;
pub mod config;
pub mod demo;
pub mod fs_aman;
pub mod model;
pub mod pages;
pub mod report;
pub mod schema;
pub mod server_fns;
pub mod sldk;
pub mod store;

/// Waktu sekarang dalam format tanggal-jam (UTC).
pub fn now_string() -> String {
    #[cfg(feature = "ssr")]
    {
        chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string()
    }
    #[cfg(not(feature = "ssr"))]
    {
        String::new()
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
