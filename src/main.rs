#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::routing::post;
    use axum::Router;
    use dasbor_bmn::app::{shell, App};
    use dasbor_bmn::auth;
    use dasbor_bmn::report;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use std::net::SocketAddr;

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // Rute server function Leptos didaftarkan lebih dulu agar gerbang otorisasi
    // dapat membungkus semuanya lewat satu titik.
    let app = Router::new()
        .route("/api/login", post(auth::login))
        .route("/api/logout", post(auth::logout))
        .route(
            "/api/laporan.pdf",
            axum::routing::get(report::http::laporan_pdf),
        )
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(axum::middleware::from_fn(auth::gerbang))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("gagal bind alamat");
    println!("dasbor-bmn siap di http://{addr}");
    // `into_make_service_with_connect_info` mengisi alamat TCP peer dari kernel.
    // Gerbang memakainya untuk pembatasan percobaan masuk dan aturan cookie
    // Secure; header alamat klien tidak dipercaya karena dapat dipalsukan.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("server berhenti tak terduga");
}

#[cfg(not(feature = "ssr"))]
fn main() {}
