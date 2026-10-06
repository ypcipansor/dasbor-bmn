use leptos::prelude::*;

use crate::components::chart::CategoryBars;
use crate::components::ui::{DataModeBanner, EmptyBlock, ErrorBlock, LoadingBlock, SectionCard};
use crate::model::{format_number, format_rupiah, format_rupiah_short, Overview};
use crate::schema::COLUMN_COUNT;
use crate::server_fns::{get_overview, get_tables, TableMeta};

/// Halaman kategori: rincian 15 kategori BMN dan struktur datanya.
#[component]
pub fn CategoryPage() -> impl IntoView {
    let overview = Resource::new(|| (), |_| async move { get_overview().await });
    let tables = Resource::new(|| (), |_| async move { get_tables().await });
    let (selected, set_selected) = signal(String::new());

    view! {
        <Suspense fallback=|| view! { <LoadingBlock rows=6/> }>
            {move || {
                match (overview.get(), tables.get()) {
                    (Some(Ok(o)), Some(Ok(t))) => view! { <CategoryBody data=o tables=t selected=selected set_selected=set_selected/> }.into_any(),
                    (Some(Err(e)), _) | (_, Some(Err(e))) => view! { <ErrorBlock message=format!("Gagal memuat kategori: {e}")/> }.into_any(),
                    _ => view! { <LoadingBlock rows=6/> }.into_any(),
                }
            }}
        </Suspense>
    }
}

#[component]
fn CategoryBody(
    data: Overview,
    tables: Vec<TableMeta>,
    selected: ReadSignal<String>,
    set_selected: WriteSignal<String>,
) -> impl IntoView {
    if data.is_empty() {
        return view! {
            <EmptyBlock
                title="Belum ada data kategori"
                message="Sinkronkan data terlebih dahulu agar rincian kategori muncul."
                action=None
            />
        }
        .into_any();
    }

    let bars: Vec<(String, i64, f64, String)> = data
        .categories
        .iter()
        .map(|c| (c.label.clone(), c.jumlah, c.nilai, c.icon.clone()))
        .collect();

    // Salinan sederhana agar closure tampilan tidak meminjam `data`/`tables`.
    let cards: Vec<(String, String, String, i64, f64)> = data
        .categories
        .iter()
        .map(|c| {
            (
                c.label.clone(),
                c.icon.clone(),
                c.volume.clone(),
                c.jumlah,
                c.nilai,
            )
        })
        .collect();
    let first_label = cards.first().map(|c| c.0.clone()).unwrap_or_default();
    let first_label_for_card = first_label.clone();
    let default_table = tables.first().map(|t| t.table.clone()).unwrap_or_default();
    let default_table_for_chip = default_table.clone();
    let default_table_for_detail = default_table.clone();

    let detail = {
        let tables = tables.clone();
        let default_table = default_table_for_detail.clone();
        move || {
            let sel = selected.get();
            let name = if sel.is_empty() { default_table.clone() } else { sel };
            tables.iter().find(|t| t.table == name).cloned()
        }
    };

    view! {
        <DataModeBanner is_demo=data.is_demo generated_at=data.generated_at.clone()/>

        <SectionCard
            title="Perbandingan Kategori"
            subtitle=Some(format!("{} kategori · {} kolom per tabel", cards.len(), COLUMN_COUNT))
        >
            <CategoryBars data=bars/>
        </SectionCard>

        <div class="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
            {cards
                .iter()
                .map(|(label, icon, volume, jumlah, nilai)| {
                    let label = label.clone();
                    let icon = icon.clone();
                    let volume = volume.clone();
                    let jumlah = *jumlah;
                    let nilai = *nilai;
                    let label_for_click = label.clone();
                    let label_for_active = label.clone();
                    let first = first_label_for_card.clone();
                    view! {
                        <button
                            type="button"
                            class=move || {
                                let sel = selected.get();
                                let active = if sel.is_empty() { first.clone() } else { sel };
                                format!(
                                    "card p-4 text-left transition-shadow hover:shadow-md {}",
                                    if active == label_for_active {
                                        "ring-2 ring-bmn-500"
                                    } else {
                                        ""
                                    },
                                )
                            }
                            on:click={
                                let l = label_for_click.clone();
                                move |_| set_selected.set(l.clone())
                            }
                        >
                            <div class="flex items-start justify-between gap-2">
                                <span class="text-2xl" aria-hidden="true">{icon.clone()}</span>
                                <span class="chip bg-ink-100 text-ink-500 dark:bg-ink-800 dark:text-ink-300">
                                    {volume.clone()}
                                </span>
                            </div>
                            <p class="mt-2 truncate text-sm font-semibold" title=label.clone()>
                                {label.clone()}
                            </p>
                            <p class="mt-1 text-lg font-bold tabular-nums">
                                {format_number(jumlah)}
                                <span class="ml-1 text-xs font-normal text-ink-400">"unit"</span>
                            </p>
                            <p class="mt-0.5 text-xs text-ink-400">{format_rupiah_short(nilai)}</p>
                        </button>
                    }
                })
                .collect_view()}
        </div>

        <div class="mt-4">
            <SectionCard
                title="Struktur Data"
                subtitle=Some("Definisi kolom sesuai dokumen SLDK SIMAN v2".into())
            >
                <div class="mb-3 flex flex-wrap gap-2">
                    {tables
                        .iter()
                        .map(|t| {
                            let table = t.table.clone();
                            let label = t.label.clone();
                            let table_for_active = table.clone();
                            let first = default_table_for_chip.clone();
                            view! {
                                <button
                                    type="button"
                                    class=move || {
                                        let sel = selected.get();
                                        let active = if sel.is_empty() { first.clone() } else { sel };
                                        format!(
                                            "chip border {}",
                                            if active == table_for_active {
                                                "border-bmn-500 bg-bmn-50 text-bmn-700 dark:bg-bmn-900/40 dark:text-bmn-200"
                                            } else {
                                                "border-ink-200 text-ink-500 dark:border-ink-700 dark:text-ink-300"
                                            },
                                        )
                                    }
                                    on:click={
                                        let table = table.clone();
                                        move |_| set_selected.set(table.clone())
                                    }
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>

                {move || {
                    detail()
                        .map(|t| {
                            view! {
                                <div>
                                    <div class="mb-2 flex flex-wrap items-baseline gap-x-3 gap-y-1 text-xs text-ink-400">
                                        <span class="font-semibold text-ink-600 dark:text-ink-200">
                                            {t.nama_data.clone()}
                                        </span>
                                        <code class="rounded bg-ink-100 px-1.5 py-0.5 dark:bg-ink-800">
                                            {t.table.clone()}
                                        </code>
                                        <span>{format!("/{}", t.resource)}</span>
                                        <span>{format!("{} kolom", t.columns.len())}</span>
                                    </div>
                                    <div class="table-wrap max-h-[28rem] overflow-y-auto rounded-lg border border-ink-200 dark:border-ink-800">
                                        <table class="w-full min-w-[40rem] text-left text-xs">
                                            <thead class="sticky top-0 bg-ink-50 text-ink-500 dark:bg-ink-900 dark:text-ink-300">
                                                <tr>
                                                    <th scope="col" class="px-3 py-2 font-semibold">"No."</th>
                                                    <th scope="col" class="px-3 py-2 font-semibold">"Kolom"</th>
                                                    <th scope="col" class="px-3 py-2 font-semibold">"Uraian"</th>
                                                    <th scope="col" class="px-3 py-2 font-semibold">"Tipe"</th>
                                                    <th scope="col" class="px-3 py-2 font-semibold">"Panjang"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {t.columns
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(i, c)| {
                                                        view! {
                                                            <tr class="border-t border-ink-100 odd:bg-white even:bg-ink-50/40 dark:border-ink-800 dark:odd:bg-ink-900 dark:even:bg-ink-900/40">
                                                                <td class="px-3 py-1.5 tabular-nums text-ink-400">
                                                                    {i + 1}
                                                                </td>
                                                                <td class="px-3 py-1.5 font-medium">{c.name.clone()}</td>
                                                                <td class="px-3 py-1.5 text-ink-500 dark:text-ink-300">
                                                                    {c.label.clone()}
                                                                </td>
                                                                <td class="px-3 py-1.5">
                                                                    <span class="chip bg-ink-100 text-ink-500 dark:bg-ink-800 dark:text-ink-300">
                                                                        {c.r#type.clone()}
                                                                    </span>
                                                                </td>
                                                                <td class="px-3 py-1.5 tabular-nums text-ink-400">
                                                                    {c.len.clone()}
                                                                </td>
                                                            </tr>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </tbody>
                                        </table>
                                    </div>
                                </div>
                            }
                        })
                }}

                <p class="mt-3 text-xs text-ink-400">
                    "Nilai agregat seluruh tabel: "
                    <span class="font-semibold text-ink-600 dark:text-ink-200">
                        {format_rupiah(data.total_nilai)}
                    </span>
                </p>
            </SectionCard>
        </div>
    }
    .into_any()
}
