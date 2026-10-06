use leptos::prelude::*;

use crate::model::{format_number, format_rupiah_short};

/// Kartu KPI dengan nilai utama, label, dan keterangan tambahan.
#[component]
pub fn StatCard(
    label: &'static str,
    value: String,
    hint: Option<String>,
    icon: &'static str,
    tone: &'static str,
) -> impl IntoView {
    let tone_class = match tone {
        "emerald" => "bg-emerald-50 text-emerald-700 dark:bg-emerald-500/10 dark:text-emerald-300",
        "amber" => "bg-amber-50 text-amber-700 dark:bg-amber-500/10 dark:text-amber-300",
        "rose" => "bg-rose-50 text-rose-700 dark:bg-rose-500/10 dark:text-rose-300",
        "violet" => "bg-violet-50 text-violet-700 dark:bg-violet-500/10 dark:text-violet-300",
        _ => "bg-bmn-50 text-bmn-700 dark:bg-bmn-500/10 dark:text-bmn-300",
    };
    view! {
        <div class="card p-4">
            <div class="flex items-start justify-between gap-3">
                <p class="text-xs font-medium uppercase tracking-wide text-ink-400">{label}</p>
                <span class=format!("grid size-9 shrink-0 place-items-center rounded-lg text-base {tone_class}") aria-hidden="true">
                    {icon}
                </span>
            </div>
            <p class="mt-2 text-2xl font-bold tabular-nums leading-tight sm:text-[1.7rem]">{value}</p>
            {hint
                .map(|h| {
                    view! { <p class="mt-1 text-xs text-ink-400">{h}</p> }
                })}
        </div>
    }
}

/// Kartu KPI dari angka mentah.
#[component]
pub fn NumberCard(
    label: &'static str,
    count: i64,
    hint: Option<String>,
    icon: &'static str,
    tone: &'static str,
) -> impl IntoView {
    view! { <StatCard label=label value=format_number(count) hint=hint icon=icon tone=tone/> }
}

/// Kartu KPI nilai rupiah.
#[component]
pub fn MoneyCard(
    label: &'static str,
    amount: f64,
    hint: Option<String>,
    icon: &'static str,
    tone: &'static str,
) -> impl IntoView {
    view! { <StatCard label=label value=format_rupiah_short(amount) hint=hint icon=icon tone=tone/> }
}

/// Label kecil berwarna.
#[component]
pub fn Badge(text: String, tone: &'static str) -> impl IntoView {
    let cls = match tone {
        "green" => "bg-emerald-100 text-emerald-800 dark:bg-emerald-500/15 dark:text-emerald-300",
        "amber" => "bg-amber-100 text-amber-800 dark:bg-amber-500/15 dark:text-amber-300",
        "red" => "bg-rose-100 text-rose-800 dark:bg-rose-500/15 dark:text-rose-300",
        "blue" => "bg-bmn-100 text-bmn-800 dark:bg-bmn-500/15 dark:text-bmn-300",
        _ => "bg-ink-100 text-ink-600 dark:bg-ink-800 dark:text-ink-300",
    };
    view! { <span class=format!("chip {cls}")>{text}</span> }
}

/// Kerangka isi kartu dengan judul dan aksi opsional.
#[component]
pub fn SectionCard(
    title: &'static str,
    subtitle: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <section class="card overflow-hidden">
            <header class="flex flex-wrap items-baseline justify-between gap-2 border-b border-ink-100 px-4 py-3 dark:border-ink-800">
                <h2 class="text-sm font-semibold">{title}</h2>
                {subtitle
                    .map(|s| {
                        view! { <p class="text-xs text-ink-400">{s}</p> }
                    })}
            </header>
            <div class="p-4">{children()}</div>
        </section>
    }
}

/// Penanda mode data (contoh vs. langsung).
#[component]
pub fn DataModeBanner(is_demo: bool, generated_at: String) -> impl IntoView {
    if is_demo {
        view! {
            <div class="mb-4 flex flex-wrap items-center gap-2 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-200">
                <span aria-hidden="true">"🧪"</span>
                <p class="min-w-0 flex-1">
                    <span class="font-semibold">"Mode contoh. "</span>
                    "Kredensial SLDK belum aktif, jadi dasbor menampilkan data ilustratif. "
                    "Isi BA_KEY, Client ID, dan Client Secret di halaman Pengaturan untuk menarik data nyata."
                </p>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div class="mb-4 flex flex-wrap items-center gap-2 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-200">
                <span aria-hidden="true">"✅"</span>
                <p class="min-w-0 flex-1">
                    <span class="font-semibold">"Data langsung. "</span>
                    "Sumber: Web Service SLDK SIMAN v2."
                </p>
                <span class="text-xs text-emerald-700/80 dark:text-emerald-300/80">{generated_at}</span>
            </div>
        }
        .into_any()
    }
}

/// Kerangka pemuatan (skeleton) untuk konten yang menunggu data.
#[component]
pub fn LoadingBlock(rows: usize) -> impl IntoView {
    view! {
        <div class="space-y-3" aria-busy="true" aria-live="polite">
            <span class="sr-only">"Memuat data…"</span>
            {(0..rows)
                .map(|_| {
                    view! {
                        <div class="h-4 w-full animate-pulse rounded bg-ink-200/70 dark:bg-ink-800"></div>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// Pesan kesalahan yang ramah pengguna.
#[component]
pub fn ErrorBlock(message: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-rose-200 bg-rose-50 p-4 text-sm text-rose-900 dark:border-rose-500/30 dark:bg-rose-500/10 dark:text-rose-200">
            <p class="font-semibold">"Terjadi kendala"</p>
            <p class="mt-1 break-words">{message}</p>
        </div>
    }
}

/// Pesan ketika belum ada data sama sekali.
#[component]
pub fn EmptyBlock(
    title: &'static str,
    message: &'static str,
    action: Option<AnyView>,
) -> impl IntoView {
    view! {
        <div class="grid place-items-center rounded-xl border border-dashed border-ink-200 p-8 text-center dark:border-ink-700">
            <div class="max-w-md">
                <p class="text-2xl" aria-hidden="true">"📭"</p>
                <p class="mt-2 font-semibold">{title}</p>
                <p class="mt-1 text-sm text-ink-400">{message}</p>
                {action}
            </div>
        </div>
    }
}
