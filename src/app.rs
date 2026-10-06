use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::components::layout::{AppShell, ThemeInit};
use crate::pages::category::CategoryPage;
use crate::pages::data::DataPage;
use crate::pages::not_found::NotFound;
use crate::pages::overview::OverviewPage;
use crate::pages::settings::SettingsPage;
use crate::pages::sync::SyncPage;

/// Kerangka HTML yang dikirim server (SSR) dan dihidrasi di peramban.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="id" class="scroll-smooth">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover"/>
                <meta name="color-scheme" content="light dark"/>
                <meta name="theme-color" content="#1f47f5"/>
                <meta
                    name="description"
                    content="Dasbor analitik Barang Milik Negara (BMN) Kejaksaan RI — sumber data SLDK SIMAN v2"
                />
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options.clone()/>
                <MetaTags/>
                <Stylesheet id="leptos" href="/pkg/dasbor-bmn.css"/>
                <ThemeInit/>
            </head>
            <body class="min-h-dvh bg-ink-50 text-ink-900 antialiased dark:bg-ink-950 dark:text-ink-100">
                <App/>
            </body>
        </html>
    }
}

/// Aplikasi utama beserta perutean halaman.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Title text="Dasbor BMN — Kejaksaan RI"/>
        <Router>
            <AppShell>
                <Routes fallback=|| view! { <NotFound/> }>
                    <Route path=path!("/") view=OverviewPage/>
                    <Route path=path!("/kategori") view=CategoryPage/>
                    <Route path=path!("/data") view=DataPage/>
                    <Route path=path!("/sinkronisasi") view=SyncPage/>
                    <Route path=path!("/pengaturan") view=SettingsPage/>
                </Routes>
            </AppShell>
        </Router>
    }
}
