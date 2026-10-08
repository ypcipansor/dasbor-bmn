use leptos::prelude::*;

use crate::components::ui::{Badge, ErrorBlock, LoadingBlock, SectionCard};
use crate::model::{format_number, SyncStatus};
use crate::server_fns::{get_sync_statuses, preview_row_count, reset_cache, sync_all, sync_table};

/// Halaman sinkronisasi: status per tabel, tarik data, dan reset cache.
#[component]
pub fn SyncPage() -> impl IntoView {
    let (refresh, set_refresh) = signal(0_u32);
    let (running, set_running) = signal(Option::<String>::None);
    let (log, set_log) = signal(Vec::<String>::new());
    let (preview, set_preview) = signal(Option::<(String, i64)>::None);
    let (error, set_error) = signal(Option::<String>::None);

    let statuses = Resource::new(
        move || refresh.get(),
        |_| async move { get_sync_statuses().await },
    );

    let push_log = move |s: String| {
        set_log.update(|l| {
            l.push(s);
            if l.len() > 200 {
                l.remove(0);
            }
        });
    };

    let do_sync_one = {
        move |table: String, label: String| {
            set_running.set(Some(label.clone()));
            set_error.set(None);
            push_log(format!("Mulai sinkronisasi: {label}…"));
            leptos::task::spawn_local(async move {
                match sync_table(table.clone()).await {
                    Ok(s) => {
                        push_log(format!(
                            "Selesai: {} — {} baris ({})",
                            s.label,
                            format_number(s.row_count),
                            s.status
                        ));
                        if let Some(m) = s.message {
                            push_log(format!("Catatan: {m}"));
                        }
                        set_refresh.update(|v| *v += 1);
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        push_log(format!("Gagal {label}: {msg}"));
                        set_error.set(Some(msg));
                    }
                }
                set_running.set(None);
            });
        }
    };

    view! {
        <div class="space-y-4">
            <SectionCard
                title="Sinkronisasi Data SLDK"
                subtitle=Some("Tarik data terbaru dari Web Service SLDK SIMAN v2".into())
            >
                <div class="flex flex-wrap items-center gap-2">
                    <button
                        type="button"
                        class="btn-primary"
                        disabled=move || running.get().is_some()
                        on:click=move |_| {
                            set_running.set(Some("Semua tabel".into()));
                            set_error.set(None);
                            push_log("Mulai sinkronisasi seluruh tabel…".into());
                            leptos::task::spawn_local(async move {
                                match sync_all().await {
                                    Ok(list) => {
                                        for s in &list {
                                            push_log(format!(
                                                "{} — {} baris ({})",
                                                s.label,
                                                format_number(s.row_count),
                                                s.status
                                            ));
                                        }
                                        set_refresh.update(|v| *v += 1);
                                    }
                                    Err(e) => {
                                        let msg = e.to_string();
                                        push_log(format!("Gagal: {msg}"));
                                        set_error.set(Some(msg));
                                    }
                                }
                                set_running.set(None);
                            });
                        }
                    >
                        {move || if running.get().is_some() { "⏳ Sedang berjalan…" } else { "🔄 Sinkronkan Semua" }}
                    </button>

                    <button
                        type="button"
                        class="btn-ghost"
                        disabled=move || running.get().is_some()
                        on:click=move |_| {
                            push_log("Menghapus cache lokal…".into());
                            leptos::task::spawn_local(async move {
                                match reset_cache().await {
                                    Ok(_) => {
                                        push_log("Cache dihapus.".into());
                                        set_refresh.update(|v| *v += 1);
                                    }
                                    Err(e) => set_error.set(Some(e.to_string())),
                                }
                            });
                        }
                    >
                        "🗑️ Reset Cache"
                    </button>

                    <span class="text-xs text-ink-400">
                        {move || running.get().map(|r| format!("Memproses: {r}")).unwrap_or_default()}
                    </span>
                </div>

                {move || error.get().map(|e| view! { <div class="mt-3"><ErrorBlock message=e/></div> })}
            </SectionCard>

            <Suspense fallback=|| view! { <LoadingBlock rows=8/> }>
                {move || {
                    statuses
                        .get()
                        .map(|res| match res {
                            Ok(list) => view! { <StatusTable list=list do_sync_one=do_sync_one preview=preview set_preview=set_preview set_error=set_error/> }.into_any(),
                            Err(e) => view! { <ErrorBlock message=format!("Gagal memuat status: {e}")/> }.into_any(),
                        })
                }}
            </Suspense>

            <SectionCard title="Catatan Proses" subtitle=None>
                <pre class="max-h-64 overflow-auto rounded-lg bg-ink-950 p-3 text-xs leading-relaxed text-ink-100">
                    {move || {
                        let l = log.get();
                        if l.is_empty() {
                            "Belum ada aktivitas sinkronisasi.".to_string()
                        } else {
                            l.join("\n")
                        }
                    }}
                </pre>
            </SectionCard>
        </div>
    }
}

#[component]
fn StatusTable(
    list: Vec<SyncStatus>,
    do_sync_one: impl Fn(String, String) + Copy + 'static,
    preview: ReadSignal<Option<(String, i64)>>,
    set_preview: WriteSignal<Option<(String, i64)>>,
    set_error: WriteSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <section class="card overflow-hidden">
            <header class="border-b border-ink-100 px-4 py-3 dark:border-ink-800">
                <h2 class="text-sm font-semibold">"Status per Tabel"</h2>
            </header>
            <div class="table-wrap">
                <table class="w-full min-w-[44rem] text-left text-sm">
                    <thead class="bg-ink-50 text-xs text-ink-500 dark:bg-ink-900 dark:text-ink-300">
                        <tr>
                            <th scope="col" class="px-4 py-2 font-semibold">"Kategori"</th>
                            <th scope="col" class="px-4 py-2 font-semibold">"Baris tersimpan"</th>
                            <th scope="col" class="px-4 py-2 font-semibold">"Sumber SLDK"</th>
                            <th scope="col" class="px-4 py-2 font-semibold">"Terakhir"</th>
                            <th scope="col" class="px-4 py-2 font-semibold">"Aksi"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {list
                            .into_iter()
                            .map(|s| {
                                let table = s.table.clone();
                                let label = s.label.clone();
                                let table2 = s.table.clone();
                                let tone = if s.status == "tersinkron" {
                                    "green"
                                } else if s.status == "gagal" {
                                    "red"
                                } else {
                                    "amber"
                                };
                                view! {
                                    <tr class="border-t border-ink-100 dark:border-ink-800">
                                        <td class="px-4 py-2">
                                            <p class="font-medium">{s.label.clone()}</p>
                                            <code class="text-xs text-ink-400">{s.table.clone()}</code>
                                        </td>
                                        <td class="px-4 py-2 tabular-nums">{format_number(s.row_count)}</td>
                                        <td class="px-4 py-2">
                                            {move || {
                                                let p = preview.get();
                                                match p {
                                                    Some((t, n)) if t == table2 => {
                                                        view! { <span class="tabular-nums">{format_number(n)}</span> }.into_any()
                                                    }
                                                    _ => {
                                                        view! { <span class="text-ink-400">"—"</span> }.into_any()
                                                    }
                                                }
                                            }}
                                        </td>
                                        <td class="px-4 py-2 text-xs text-ink-400">
                                            {s.last_sync.clone().unwrap_or_else(|| "—".into())}
                                        </td>
                                        <td class="px-4 py-2">
                                            <div class="flex flex-wrap items-center gap-1.5">
                                                <Badge text=s.status.clone() tone=tone/>
                                                <button
                                                    type="button"
                                                    class="btn-ghost px-2 py-1 text-xs"
                                                    on:click={
                                                        let t = table.clone();
                                                        let l = label.clone();
                                                        move |_| do_sync_one(t.clone(), l.clone())
                                                    }
                                                >
                                                    "Sinkron"
                                                </button>
                                                <button
                                                    type="button"
                                                    class="btn-ghost px-2 py-1 text-xs"
                                                    on:click={
                                                        let t = table.clone();
                                                        move |_| {
                                                            let t = t.clone();
                                                            leptos::task::spawn_local(async move {
                                                                match preview_row_count(t.clone()).await {
                                                                    Ok(n) => set_preview.set(Some((t, n))),
                                                                    Err(e) => set_error.set(Some(e.to_string())),
                                                                }
                                                            });
                                                        }
                                                    }
                                                >
                                                    "Cek jumlah"
                                                </button>
                                            </div>
                                        </td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </section>
    }
}
