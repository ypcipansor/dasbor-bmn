use leptos::prelude::*;

use crate::components::ui::{Badge, ErrorBlock, LoadingBlock, SectionCard};
use crate::model::{format_number, AssetPage};
use crate::server_fns::{export_csv, get_asset_page, get_tables, TableMeta};

const PER_PAGE: i64 = 25;

/// Halaman data: penelusuran, pencarian, pengurutan, paginasi, dan ekspor CSV.
#[component]
pub fn DataPage() -> impl IntoView {
    let tables = Resource::new(|| (), |_| async move { get_tables().await });
    let (table, set_table) = signal(String::new());
    let (page, set_page) = signal(1_i64);
    let (q, set_q) = signal(String::new());
    let (sort, set_sort) = signal(Option::<String>::None);
    let (dir, set_dir) = signal("asc".to_string());

    let page_res = Resource::new(
        move || (table.get(), page.get(), q.get(), sort.get(), dir.get()),
        |(t, p, q, s, d)| async move {
            let name = if t.is_empty() {
                "SIMAN2_M_ASET_TANAH".to_string()
            } else {
                t
            };
            get_asset_page(name, p, PER_PAGE, q, s, d).await
        },
    );

    view! {
        <Suspense fallback=|| view! { <LoadingBlock rows=10/> }>
            {move || {
                tables
                    .get()
                    .map(|res| match res {
                        Ok(list) => view! { <DataBody tables=list table=table set_table=set_table set_page=set_page q=q set_q=set_q sort=sort set_sort=set_sort dir=dir set_dir=set_dir page_res=page_res/> }.into_any(),
                        Err(e) => view! { <ErrorBlock message=format!("Gagal memuat daftar tabel: {e}")/> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn DataBody(
    tables: Vec<TableMeta>,
    table: ReadSignal<String>,
    set_table: WriteSignal<String>,
    set_page: WriteSignal<i64>,
    q: ReadSignal<String>,
    set_q: WriteSignal<String>,
    sort: ReadSignal<Option<String>>,
    set_sort: WriteSignal<Option<String>>,
    dir: ReadSignal<String>,
    set_dir: WriteSignal<String>,
    page_res: Resource<Result<AssetPage, ServerFnError>>,
) -> impl IntoView {
    let (busy, set_busy) = signal(false);
    let (csv, set_csv) = signal(Option::<String>::None);

    let tables_for_current = tables.clone();
    let tables_for_columns = tables.clone();
    let current_table = move || {
        let t = table.get();
        if t.is_empty() {
            tables_for_current
                .first()
                .map(|x| x.table.clone())
                .unwrap_or_default()
        } else {
            t
        }
    };
    let current_table_for_columns = current_table.clone();
    let columns = move || {
        let t = current_table_for_columns();
        tables_for_columns
            .iter()
            .find(|x| x.table == t)
            .map(|x| x.columns.clone())
            .unwrap_or_default()
    };

    view! {
        <div class="space-y-4">
            <SectionCard
                title="Telusuri Data Aset"
                subtitle=Some("Cari, urutkan, lalu ekspor ke CSV".into())
            >
                <div class="flex flex-wrap items-end gap-3">
                    <label class="min-w-[12rem] flex-1">
                        <span class="mb-1 block text-xs font-medium text-ink-500 dark:text-ink-300">
                            "Kategori BMN"
                        </span>
                        <select
                            class="input"
                            on:change=move |ev| {
                                set_table.set(event_target_value(&ev));
                                set_page.set(1);
                                set_csv.set(None);
                            }
                        >
                            {tables
                                .iter()
                                .map(|t| {
                                    let table_name = t.table.clone();
                                    let label = t.label.clone();
                                    let current_table = current_table.clone();
                                    view! {
                                        <option value=table_name.clone() selected=move || current_table() == table_name>
                                            {label}
                                        </option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    </label>

                    <label class="min-w-[12rem] flex-[2]">
                        <span class="mb-1 block text-xs font-medium text-ink-500 dark:text-ink-300">
                            "Pencarian"
                        </span>
                        <input
                            type="search"
                            class="input"
                            placeholder="Kode barang, nama, provinsi, satker…"
                            prop:value=move || q.get()
                            on:input=move |ev| {
                                set_q.set(event_target_value(&ev));
                                set_page.set(1);
                            }
                        />
                    </label>

                    <label class="min-w-[10rem]">
                        <span class="mb-1 block text-xs font-medium text-ink-500 dark:text-ink-300">
                            "Urutkan"
                        </span>
                        <select
                            class="input"
                            on:change=move |ev| {
                                let v = event_target_value(&ev);
                                set_sort.set(if v.trim().is_empty() { None } else { Some(v) });
                                set_page.set(1);
                            }
                        >
                            <option value="">"— tanpa urutan —"</option>
                            {columns()
                                .into_iter()
                                .filter(|c| {
                                    matches!(
                                        c.name.as_str(),
                                        "kd_brg" | "nama" | "ur_sskel" | "merk" | "rph_aset" | "rph_susut"
                                            | "ur_kondisi" | "ur_prov" | "nama_satker" | "tgl_perlh" | "umur_sisa"
                                    )
                                })
                                .map(|c| view! { <option value=c.name.clone()>{c.label.clone()}</option> })
                                .collect_view()}
                        </select>
                    </label>

                    <button
                        type="button"
                        class="btn-ghost"
                        on:click=move |_| {
                            set_dir
                                .set(
                                    if dir.get_untracked() == "asc" {
                                        "desc".to_string()
                                    } else {
                                        "asc".to_string()
                                    },
                                )
                        }
                        title="Ubah arah pengurutan"
                    >
                        {move || if dir.get() == "asc" { "⬆️ Menaik" } else { "⬇️ Menurun" }}
                    </button>

                    <button
                        type="button"
                        class="btn-primary"
                        disabled=move || busy.get()
                        on:click={
                            let current_table = current_table.clone();
                            move |_| {
                                let t = current_table();
                                set_busy.set(true);
                                leptos::task::spawn_local(async move {
                                    let res = export_csv(t).await;
                                    set_busy.set(false);
                                    match res {
                                        Ok(data) => set_csv.set(Some(data)),
                                        Err(e) => set_csv.set(Some(format!("ERROR:{e}"))),
                                    }
                                });
                            }
                        }
                    >
                        {move || if busy.get() { "Menyiapkan…" } else { "⬇️ Ekspor CSV" }}
                    </button>
                </div>

                {{
                    let current_table = current_table.clone();
                    move || {
                        csv.get()
                            .map(|data| {
                                if let Some(err) = data.strip_prefix("ERROR:") {
                                    view! { <p class="mt-3 text-sm text-rose-600 dark:text-rose-300">{format!("Gagal mengekspor: {err}")}</p> }.into_any()
                                } else {
                                    let t = current_table();
                                    let href = format!(
                                        "data:text/csv;charset=utf-8,{}",
                                        urlencoding(&data),
                                    );
                                    let download = format!("{t}.csv");
                                    let lines = format_number(data.lines().count() as i64 - 1);
                                    view! {
                                        <a
                                            class="mt-3 inline-flex items-center gap-2 text-sm font-medium text-bmn-600 underline dark:text-bmn-300"
                                            href=href
                                            download=download
                                        >
                                            "⬇️ Unduh berkas CSV ("
                                            {lines}
                                            " baris)"
                                        </a>
                                    }
                                    .into_any()
                                }
                            })
                    }
                }}
            </SectionCard>

            <Suspense fallback=|| view! { <LoadingBlock rows=8/> }>
                {move || {
                    page_res
                        .get()
                        .map(|res| match res {
                            Ok(p) => view! { <DataTable data=p set_page=set_page set_sort=set_sort set_dir=set_dir sort=sort dir=dir/> }.into_any(),
                            Err(e) => view! { <ErrorBlock message=format!("Gagal memuat data: {e}")/> }.into_any(),
                        })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn DataTable(
    data: AssetPage,
    set_page: WriteSignal<i64>,
    set_sort: WriteSignal<Option<String>>,
    set_dir: WriteSignal<String>,
    sort: ReadSignal<Option<String>>,
    dir: ReadSignal<String>,
) -> impl IntoView {
    let total_pages = ((data.total as f64) / (data.per_page as f64))
        .ceil()
        .max(1.0) as i64;
    let page = data.page;
    let columns = data.columns.clone();
    let rows = data.rows.clone();
    let numeric = |name: &str| {
        matches!(
            name,
            "rph_aset"
                | "rph_susut"
                | "rph_mutasi"
                | "rph_perolehan"
                | "rph_buku"
                | "umur_sisa"
                | "luas"
        )
    };
    // Hanya kolom rupiah yang memakai format mata uang; umur & luas tetap angka biasa.
    let rupiah = |name: &str| {
        matches!(
            name,
            "rph_aset" | "rph_susut" | "rph_mutasi" | "rph_perolehan" | "rph_buku"
        )
    };

    view! {
        <section class="card overflow-hidden">
            <header class="flex flex-wrap items-center justify-between gap-2 border-b border-ink-100 px-4 py-3 dark:border-ink-800">
                <div class="flex items-center gap-2">
                    <h2 class="text-sm font-semibold">{data.label.clone()}</h2>
                    {if data.is_demo {
                        view! { <Badge text="Data contoh".into() tone="amber"/> }.into_any()
                    } else {
                        view! { <Badge text="Data langsung".into() tone="green"/> }.into_any()
                    }}
                </div>
                <p class="text-xs text-ink-400">
                    {format_number(data.total)}
                    " baris · halaman "
                    {page}
                    " dari "
                    {total_pages}
                </p>
            </header>

            <div class="table-wrap max-h-[70vh] overflow-y-auto">
                <table class="w-full min-w-[52rem] text-left text-xs">
                    <thead class="sticky top-0 z-10 bg-ink-50 text-ink-500 dark:bg-ink-900 dark:text-ink-300">
                        <tr>
                            <th scope="col" class="px-3 py-2 font-semibold">"#"</th>
                            {columns
                                .iter()
                                .map(|c| {
                                    let name = c.name.clone();
                                    let label = c.label.clone();
                                    let clickable = numeric(&name);
                                    let name2 = name.clone();
                                    view! {
                                        <th scope="col" class="whitespace-nowrap px-3 py-2 font-semibold">
                                            {if clickable {
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="inline-flex items-center gap-1 hover:text-bmn-600"
                                                        on:click=move |_| {
                                                            if sort.get_untracked().as_deref() == Some(name2.as_str()) {
                                                                set_dir
                                                                    .set(
                                                                        if dir.get_untracked() == "asc" {
                                                                            "desc".to_string()
                                                                        } else {
                                                                            "asc".to_string()
                                                                        },
                                                                    );
                                                            } else {
                                                                set_sort.set(Some(name2.clone()));
                                                                set_dir.set("asc".to_string());
                                                            }
                                                        }
                                                    >
                                                        {label}
                                                        <span aria-hidden="true">"↕"</span>
                                                    </button>
                                                }
                                                    .into_any()
                                            } else {
                                                view! { <span>{label}</span> }.into_any()
                                            }}
                                        </th>
                                    }
                                })
                                .collect_view()}
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! {
                                <tr>
                                    <td colspan=columns.len() + 1 class="px-3 py-10 text-center text-ink-400">
                                        "Tidak ada baris yang cocok dengan pencarian."
                                    </td>
                                </tr>
                            }
                                .into_any()
                        } else {
                            rows
                                .iter()
                                .enumerate()
                                .map(|(i, r)| {
                                    let offset = ((page - 1) * data.per_page) as usize + i + 1;
                                    view! {
                                        <tr class="border-t border-ink-100 odd:bg-white even:bg-ink-50/40 hover:bg-bmn-50/50 dark:border-ink-800 dark:odd:bg-ink-900 dark:even:bg-ink-900/40 dark:hover:bg-bmn-900/20">
                                            <td class="px-3 py-1.5 tabular-nums text-ink-400">{offset}</td>
                                            {r.values
                                                .iter()
                                                .enumerate()
                                                .map(|(ci, v)| {
                                                    let col = columns.get(ci);
                                                    let is_num = col.map(|c| numeric(&c.name)).unwrap_or(false);
                                                    let is_rupiah = col.map(|c| rupiah(&c.name)).unwrap_or(false);
                                                    let text = if v.is_empty() {
                                                        "—".to_string()
                                                    } else {
                                                        v.clone()
                                                    };
                                                    let is_num_cell = is_num && !v.is_empty();
                                                    let display = if is_num_cell && is_rupiah {
                                                        crate::model::format_rupiah(
                                                            crate::model::to_f64(&serde_json::Value::String(v.clone())),
                                                        )
                                                    } else {
                                                        text.clone()
                                                    };
                                                    view! {
                                                        <td
                                                            class=format!(
                                                                "max-w-[16rem] truncate px-3 py-1.5 {}",
                                                                if is_num { "tabular-nums text-right" } else { "" },
                                                            )
                                                            title=text.clone()
                                                        >
                                                            {display}
                                                        </td>
                                                    }
                                                })
                                                .collect_view()}
                                        </tr>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </tbody>
                </table>
            </div>

            <footer class="flex flex-wrap items-center justify-between gap-2 border-t border-ink-100 px-4 py-3 dark:border-ink-800">
                <p class="text-xs text-ink-400">
                    {format!("{} kolom per baris", data.columns.len())}
                </p>
                <div class="flex items-center gap-2">
                    <button
                        type="button"
                        class="btn-ghost"
                        disabled=move || page <= 1
                        on:click=move |_| set_page.set((page - 1).max(1))
                    >
                        "← Sebelumnya"
                    </button>
                    <button
                        type="button"
                        class="btn-ghost"
                        disabled=move || page >= total_pages
                        on:click=move |_| set_page.set(page + 1)
                    >
                        "Berikutnya →"
                    </button>
                </div>
            </footer>
        </section>
    }
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
