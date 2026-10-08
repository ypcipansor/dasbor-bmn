use leptos::prelude::*;

use crate::components::ui::{Badge, ErrorBlock, LoadingBlock, SectionCard};
use crate::model::ConnInfo;
use crate::server_fns::{get_conn_info, save_conn_config};

/// Halaman pengaturan: kredensial koneksi SLDK dan status token.
#[component]
pub fn SettingsPage() -> impl IntoView {
    let info = Resource::new(|| (), |_| async move { get_conn_info().await });
    let (client_id, set_client_id) = signal(String::new());
    let (client_secret, set_client_secret) = signal(String::new());
    let (grant_type, set_grant_type) = signal("client_credentials".to_string());
    let (ba_key, set_ba_key) = signal(String::new());
    let (saving, set_saving) = signal(false);
    let (saved, set_saved) = signal(Option::<Result<ConnInfo, String>>::None);

    view! {
        <Suspense fallback=|| view! { <LoadingBlock rows=8/> }>
            {move || {
                info.get()
                    .map(|res| match res {
                        Ok(c) => view! { <SettingsBody info=c client_id=client_id set_client_id=set_client_id client_secret=client_secret set_client_secret=set_client_secret grant_type=grant_type set_grant_type=set_grant_type ba_key=ba_key set_ba_key=set_ba_key saving=saving set_saving=set_saving saved=saved set_saved=set_saved/> }.into_any(),
                        Err(e) => view! { <ErrorBlock message=format!("Gagal memuat pengaturan: {e}")/> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn SettingsBody(
    info: ConnInfo,
    client_id: ReadSignal<String>,
    set_client_id: WriteSignal<String>,
    client_secret: ReadSignal<String>,
    set_client_secret: WriteSignal<String>,
    grant_type: ReadSignal<String>,
    set_grant_type: WriteSignal<String>,
    ba_key: ReadSignal<String>,
    set_ba_key: WriteSignal<String>,
    saving: ReadSignal<bool>,
    set_saving: WriteSignal<bool>,
    saved: ReadSignal<Option<Result<ConnInfo, String>>>,
    set_saved: WriteSignal<Option<Result<ConnInfo, String>>>,
) -> impl IntoView {
    let token_tone = if info.token_valid { "green" } else { "red" };
    let token_text = if info.token_valid {
        "Token aktif"
    } else {
        "Token belum aktif"
    };

    view! {
        <div class="space-y-4">
            <SectionCard
                title="Status Koneksi"
                subtitle=Some("Ringkasan konfigurasi yang sedang dipakai server".into())
            >
                <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
                    <StatusItem
                        label="Endpoint Gateway"
                        value=if info.base_url_set { "Terpasang".into() } else { "Default".into() }
                        ok=info.base_url_set
                    />
                    <StatusItem
                        label="Client ID"
                        value=info.client_id.clone().unwrap_or_else(|| "Belum diisi".into())
                        ok=info.client_id.is_some()
                    />
                    <StatusItem
                        label="Client Secret"
                        value=if info.client_secret_set { "Tersimpan".into() } else { "Belum diisi".into() }
                        ok=info.client_secret_set
                    />
                    <StatusItem
                        label="BA_KEY"
                        value=info.ba_key.clone().unwrap_or_else(|| "Belum diisi".into())
                        ok=info.ba_key.is_some()
                    />
                </div>
                <div class="mt-3 flex flex-wrap items-center gap-2">
                    <Badge text=token_text.to_string() tone=token_tone/>
                    <span class="text-xs text-ink-400">
                        {info.grant_type.clone().unwrap_or_else(|| "client_credentials".into())}
                    </span>
                </div>
                {info
                    .last_error
                    .clone()
                    .map(|e| {
                        view! {
                            <p class="mt-2 break-words rounded-lg bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:bg-amber-500/10 dark:text-amber-200">
                                {e}
                            </p>
                        }
                    })}
            </SectionCard>

            <SectionCard
                title="Kredensial SLDK"
                subtitle=Some("Nilai disimpan di server (data/config.json) dan tidak pernah dikirim balik ke peramban".into())
            >
                <form
                    class="grid gap-3 sm:grid-cols-2"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        set_saving.set(true);
                        set_saved.set(None);
                        let id = client_id.get_untracked();
                        let secret = client_secret.get_untracked();
                        let gt = grant_type.get_untracked();
                        let ba = ba_key.get_untracked();
                        leptos::task::spawn_local(async move {
                            let res = save_conn_config(id, secret, gt, ba).await;
                            set_saving.set(false);
                            match res {
                                Ok(c) => {
                                    set_saved.set(Some(Ok(c)));
                                    set_client_secret.set(String::new());
                                }
                                Err(e) => set_saved.set(Some(Err(e.to_string()))),
                            }
                        });
                    }
                >
                    <label>
                        <span class="mb-1 block text-xs font-medium text-ink-500 dark:text-ink-300">
                            "Client ID"
                        </span>
                        <input
                            class="input"
                            autocomplete="off"
                            placeholder="mis. simanv2.kejagung"
                            prop:value=move || client_id.get()
                            on:input=move |ev| set_client_id.set(event_target_value(&ev))
                        />
                    </label>

                    <label>
                        <span class="mb-1 block text-xs font-medium text-ink-500 dark:text-ink-300">
                            "Client Secret"
                        </span>
                        <input
                            class="input"
                            type="password"
                            autocomplete="new-password"
                            placeholder=move || {
                                if info.client_secret_set {
                                    "•••••••• (biarkan kosong bila tidak diubah)"
                                } else {
                                    "Masukkan client secret"
                                }
                            }
                            prop:value=move || client_secret.get()
                            on:input=move |ev| set_client_secret.set(event_target_value(&ev))
                        />
                    </label>

                    <label>
                        <span class="mb-1 block text-xs font-medium text-ink-500 dark:text-ink-300">
                            "Grant Type"
                        </span>
                        <input
                            class="input"
                            placeholder="client_credentials"
                            prop:value=move || grant_type.get()
                            on:input=move |ev| set_grant_type.set(event_target_value(&ev))
                        />
                    </label>

                    <label>
                        <span class="mb-1 block text-xs font-medium text-ink-500 dark:text-ink-300">
                            "BA_KEY"
                        </span>
                        <input
                            class="input"
                            placeholder="Kode BA unit Kejaksaan"
                            prop:value=move || ba_key.get()
                            on:input=move |ev| set_ba_key.set(event_target_value(&ev))
                        />
                    </label>

                    <div class="sm:col-span-2">
                        <button type="submit" class="btn-primary" disabled=move || saving.get()>
                            {move || if saving.get() { "Menyimpan…" } else { "💾 Simpan & Uji Token" }}
                        </button>
                    </div>
                </form>

                {move || {
                    saved
                        .get()
                        .map(|r| match r {
                            Ok(c) => {
                                view! {
                                    <div class="mt-3 rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-800 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-200">
                                        <p class="font-semibold">
                                            {if c.token_valid {
                                                "Kredensial tersimpan — token berhasil diambil."
                                            } else {
                                                "Kredensial tersimpan, tetapi token belum berhasil diambil."
                                            }}
                                        </p>
                                        {c.last_error.map(|e| view! { <p class="mt-1 text-xs">{e}</p> })}
                                    </div>
                                }
                                    .into_any()
                            }
                            Err(e) => view! { <div class="mt-3"><ErrorBlock message=e/></div> }.into_any(),
                        })
                }}
            </SectionCard>

            <SectionCard title="Panduan Singkat" subtitle=None>
                <ol class="list-decimal space-y-1.5 pl-5 text-sm text-ink-600 dark:text-ink-300">
                    <li>"Isi Client ID, Client Secret, Grant Type, dan BA_KEY dari Kementerian Keuangan."</li>
                    <li>"Klik Simpan & Uji Token — server akan mengambil token SSO (berlaku 1 jam)."</li>
                    <li>"Buka halaman Sinkronisasi dan tekan Sinkronkan Semua untuk menarik 15 tabel BMN."</li>
                    <li>"Selama kredensial belum aktif, dasbor memakai data contoh agar tetap dapat dievaluasi."</li>
                </ol>
                <p class="mt-3 text-xs text-ink-400">
                    "Endpoint bawaan: "
                    <code class="rounded bg-ink-100 px-1.5 py-0.5 dark:bg-ink-800">
                        "https://apigateway.kemenkeu.go.id/gateway/SLDKSimanKL/2.0"
                    </code>
                </p>
            </SectionCard>
        </div>
    }
}

#[component]
fn StatusItem(label: &'static str, value: String, ok: bool) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-ink-200 p-3 dark:border-ink-800">
            <p class="text-xs font-medium uppercase tracking-wide text-ink-400">{label}</p>
            <p class="mt-1 flex items-center gap-2 text-sm font-semibold">
                <span
                    class=format!(
                        "size-2 rounded-full {}",
                        if ok { "bg-emerald-500" } else { "bg-ink-300" },
                    )
                    aria-hidden="true"
                ></span>
                <span class="truncate" title=value.clone()>{value.clone()}</span>
            </p>
        </div>
    }
}
