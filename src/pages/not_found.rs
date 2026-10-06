use leptos::prelude::*;
use leptos_router::components::A;

/// Halaman 404.
#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="grid min-h-[50vh] place-items-center text-center">
            <div>
                <p class="text-4xl" aria-hidden="true">"🧭"</p>
                <h2 class="mt-3 text-lg font-semibold">"Halaman tidak ditemukan"</h2>
                <p class="mt-1 text-sm text-ink-400">
                    "Tautan yang Anda buka tidak tersedia pada dasbor ini."
                </p>
                <A href="/" attr:class="btn-primary mt-4">
                    "Kembali ke Ringkasan"
                </A>
            </div>
        </div>
    }
}
