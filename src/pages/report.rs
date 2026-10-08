use leptos::prelude::*;

use crate::components::chart::{AreaChart, BarList, DonutChart};
use crate::components::ui::{
    DataModeBanner, EmptyBlock, ErrorBlock, LoadingBlock, MoneyCard, NumberCard, SectionCard,
};
use crate::model::{format_number, format_rupiah_short};
use crate::server_fns::get_overview;

/// Halaman laporan bulanan: pilih bulan, pratinjau isi, lalu unduh PDF.
#[component]
pub fn ReportPage() -> impl IntoView {
    let overview = Resource::new(|| (), |_| async move { get_overview().await });
    // Kosong = bulan berjalan; server memakai zona waktu Indonesia Barat.
    let (periode, set_periode) = signal(String::new());

    view! {
        <div class="space-y-4">
            <section class="card overflow-hidden">
                <header class="border-b border-ink-100 px-4 py-4 dark:border-ink-800">
                    <h1 class="text-base font-semibold">"Laporan Bulanan (PDF)"</h1>
                    <p class="mt-0.5 text-sm text-ink-400">
                        "Laporan analitik lengkap berisi visualisasi grafik dalam bentuk PDF siap cetak."
                    </p>
                </header>
                <div class="grid gap-4 p-4 lg:grid-cols-[1fr_20rem]">
                    <div class="space-y-2">
                        <ul class="grid gap-1.5 text-sm text-ink-600 sm:grid-cols-2 dark:text-ink-300">
                            <li class="flex items-start gap-2">
                                <span aria-hidden="true">"📄"</span>
                                <span>"Ringkasan eksekutif & 8 indikator utama"</span>
                            </li>
                            <li class="flex items-start gap-2">
                                <span aria-hidden="true">"🍩"</span>
                                <span>"Donut komposisi nilai per kategori + tabel rincian"</span>
                            </li>
                            <li class="flex items-start gap-2">
                                <span aria-hidden="true">"📈"</span>
                                <span>"Tren perolehan per tahun (jumlah & nilai)"</span>
                            </li>
                            <li class="flex items-start gap-2">
                                <span aria-hidden="true">"📊"</span>
                                <span>"Sebaran provinsi & satuan kerja"</span>
                            </li>
                            <li class="flex items-start gap-2">
                                <span aria-hidden="true">"🛠️"</span>
                                <span>"Kondisi aset & sumber dana"</span>
                            </li>
                            <li class="flex items-start gap-2">
                                <span aria-hidden="true">"🖨️"</span>
                                <span>"Format A4, grafik vektor tajam saat dicetak"</span>
                            </li>
                        </ul>
                    </div>
                    <div class="rounded-xl border border-ink-200 p-4 dark:border-ink-700">
                        <label class="block text-xs font-medium uppercase tracking-wide text-ink-400" for="periode">
                            "Bulan pelaporan"
                        </label>
                        <input
                            id="periode"
                            type="month"
                            class="mt-1 w-full rounded-lg border border-ink-200 bg-white px-3 py-2 text-sm dark:border-ink-700 dark:bg-ink-900"
                            prop:value=move || periode.get()
                            on:input=move |ev| set_periode.set(event_target_value(&ev))
                        />
                        <a
                            class="btn-primary mt-3 flex w-full items-center justify-center gap-2 px-4 py-2.5"
                            href=move || format!("/api/laporan.pdf?periode={}", periode.get())
                            download=move || format!("laporan-bmn-{}.pdf", periode.get())
                        >
                            <span aria-hidden="true">"⬇️"</span>
                            <span>"Unduh laporan PDF"</span>
                        </a>
                        <p class="mt-2 text-xs text-ink-400">
                            "Judul dan rentang tanggal mengikuti bulan terpilih; grafik merangkum data analitik terkini."
                        </p>
                    </div>
                </div>
            </section>

            <Suspense fallback=|| view! { <LoadingBlock rows=6/> }>
                {move || {
                    overview
                        .get()
                        .map(|res| match res {
                            Ok(o) => view! { <Pratinjau data=o/> }.into_any(),
                            Err(e) => {
                                view! { <ErrorBlock message=format!("Gagal memuat pratinjau: {e}")/> }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

/// Pratinjau isi laporan memakai komponen grafik yang sama dengan dasbor.
#[component]
fn Pratinjau(data: crate::model::Overview) -> impl IntoView {
    if data.is_empty() {
        return view! {
            <EmptyBlock
                title="Belum ada data"
                message="Lakukan sinkronisasi terlebih dahulu agar laporan berisi data aset."
                action=None
            />
        }
        .into_any();
    }

    let d = data.clone();
    let donut: Vec<(String, f64)> = d
        .categories
        .iter()
        .map(|c| (c.label.clone(), c.nilai))
        .collect();
    let prov: Vec<(String, i64, f64)> = d
        .provinsi
        .iter()
        .map(|b| (b.name.clone(), b.jumlah, b.nilai))
        .collect();
    let satker: Vec<(String, i64, f64)> = d
        .satker
        .iter()
        .map(|b| (b.name.clone(), b.jumlah, b.nilai))
        .collect();
    let kondisi: Vec<(String, i64, f64)> = d
        .kondisi
        .iter()
        .map(|b| (b.name.clone(), b.jumlah, b.nilai))
        .collect();
    let sumber: Vec<(String, i64, f64)> = d
        .sumber_dana
        .iter()
        .map(|b| (b.name.clone(), b.jumlah, b.nilai))
        .collect();
    let tren: Vec<(i32, i64, f64)> = d
        .perolehan
        .iter()
        .map(|t| (t.year, t.jumlah, t.nilai))
        .collect();
    let total_nilai = d.total_nilai;

    view! {
        <DataModeBanner is_demo=data.is_demo generated_at=data.generated_at.clone()/>
        <p class="text-sm font-semibold text-ink-500 dark:text-ink-300">"Pratinjau isi laporan"</p>

        <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <NumberCard label="Total Aset" count=data.total_aset hint=Some("unit tercatat".into()) icon="📦" tone="blue"/>
            <MoneyCard label="Nilai Aset" amount=data.total_nilai hint=Some("nilai perolehan".into()) icon="💰" tone="emerald"/>
            <MoneyCard label="Akumulasi Susut" amount=data.total_susut hint=Some("penyusutan".into()) icon="📉" tone="amber"/>
            <NumberCard label="Sebaran Wilayah" count=data.total_provinsi hint=Some(format!("{} satuan kerja", format_number(data.total_satker))) icon="🗺️" tone="violet"/>
        </div>

        <div class="mt-4 grid gap-4 xl:grid-cols-3">
            <div class="xl:col-span-2">
                <SectionCard title="Tren Perolehan Aset" subtitle=Some("Jumlah unit menurut tahun perolehan".into())>
                    <AreaChart points=tren value_kind="jumlah"/>
                </SectionCard>
            </div>
            <SectionCard title="Komposisi Nilai per Kategori" subtitle=Some("Bagian laporan halaman 2".into())>
                <DonutChart data=donut center_label="Total".to_string() center_value=format_rupiah_short(total_nilai)/>
            </SectionCard>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-2">
            <SectionCard title="Sebaran per Provinsi" subtitle=Some("15 teratas".into())>
                <BarList data=prov value_kind="jumlah" limit=15/>
            </SectionCard>
            <SectionCard title="Satuan Kerja Terbesar" subtitle=Some("15 teratas".into())>
                <BarList data=satker value_kind="jumlah" limit=15/>
            </SectionCard>
            <SectionCard title="Kondisi Aset" subtitle=Some("Klasifikasi kondisi BMN".into())>
                <BarList data=kondisi value_kind="jumlah" limit=10/>
            </SectionCard>
            <SectionCard title="Sumber Dana" subtitle=Some("Asal pembiayaan perolehan".into())>
                <BarList data=sumber value_kind="jumlah" limit=10/>
            </SectionCard>
        </div>
    }
    .into_any()
}
