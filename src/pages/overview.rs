use leptos::prelude::*;

use crate::components::chart::{AreaChart, BarList, DonutChart};
use crate::components::ui::{
    DataModeBanner, EmptyBlock, ErrorBlock, LoadingBlock, MoneyCard, NumberCard, SectionCard,
};
use crate::model::{format_number, format_rupiah_short, Overview};
use crate::server_fns::get_overview;

/// Halaman ringkasan: KPI utama, komposisi, peringkat, dan tren perolehan.
#[component]
pub fn OverviewPage() -> impl IntoView {
    let overview = Resource::new(|| (), |_| async move { get_overview().await });

    view! {
        <Suspense fallback=|| view! { <LoadingBlock rows=8/> }>
            {move || {
                overview
                    .get()
                    .map(|res| match res {
                        Ok(o) => view! { <OverviewBody data=o/> }.into_any(),
                        Err(e) => {
                            view! { <ErrorBlock message=format!("Gagal memuat ringkasan: {e}")/> }
                                .into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

#[component]
fn OverviewBody(data: Overview) -> impl IntoView {
    if data.is_empty() {
        return view! {
            <EmptyBlock
                title="Belum ada data"
                message="Lakukan sinkronisasi pada halaman Sinkronisasi, atau isi kredensial SLDK di Pengaturan."
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
    let donut_legend = donut.clone();
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
    let total_susut = d.total_susut;
    let total_nilai = d.total_nilai;
    let rasio_susut = if total_nilai > 0.0 {
        (total_susut / total_nilai) * 100.0
    } else {
        0.0
    };
    let jumlah_kategori = d.categories.len();
    let volume_total: f64 = d.categories.iter().map(|c| parse_volume(&c.volume)).sum();
    let top_kategori = d
        .categories
        .first()
        .map(|c| c.label.clone())
        .unwrap_or_default();
    let _ = donut_legend;

    view! {
        <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
            <div>
                <h1 class="text-lg font-semibold">"Ringkasan Dasbor"</h1>
                <p class="text-xs text-ink-400">"KPI dan analitik utama Barang Milik Negara."</p>
            </div>
            <a class="btn-ghost" href="/laporan">
                <span aria-hidden="true">"🧾"</span>
                <span>"Laporan bulanan PDF"</span>
            </a>
        </div>

        <DataModeBanner is_demo=data.is_demo generated_at=data.generated_at.clone()/>

        <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <NumberCard
                label="Total Aset"
                count=data.total_aset
                hint=Some(format!("{jumlah_kategori} kategori BMN"))
                icon="📦"
                tone="blue"
            />
            <MoneyCard
                label="Nilai Aset"
                amount=data.total_nilai
                hint=Some(format!("Volume sumber ±{:.1} GB", volume_total / (1024.0 * 1024.0)))
                icon="💰"
                tone="emerald"
            />
            <MoneyCard
                label="Akumulasi Susut"
                amount=data.total_susut
                hint=Some(format!("{rasio_susut:.1}% dari nilai aset"))
                icon="📉"
                tone="amber"
            />
            <NumberCard
                label="Sebaran Wilayah"
                count=data.total_provinsi
                hint=Some(format!("{} satuan kerja", format_number(data.total_satker)))
                icon="🗺️"
                tone="violet"
            />
        </div>

        <div class="mt-3 grid grid-cols-3 gap-3">
            <NumberCard
                label="Aset Idle"
                count=data.aset_idle
                hint=Some("Tidak digunakan".into())
                icon="⏸️"
                tone="amber"
            />
            <NumberCard
                label="Barang Rusak"
                count=data.aset_rusak
                hint=Some("Perlu tindak lanjut".into())
                icon="🛠️"
                tone="rose"
            />
            <NumberCard
                label="Barang Hilang"
                count=data.aset_hilang
                hint=Some("Terindikasi hilang".into())
                icon="🚨"
                tone="rose"
            />
        </div>

        <div class="mt-4 grid gap-4 xl:grid-cols-3">
            <div class="xl:col-span-2">
                <SectionCard
                    title="Tren Perolehan Aset"
                    subtitle=Some("Jumlah unit menurut tanggal perolehan".into())
                >
                    <AreaChart points=tren value_kind="jumlah"/>
                </SectionCard>
            </div>
            <SectionCard
                title="Komposisi Nilai per Kategori"
                subtitle=Some(format!("Kategori terbesar: {top_kategori}"))
            >
                <DonutChart
                    data=donut
                    center_label="Total".to_string()
                    center_value=format_rupiah_short(total_nilai)
                />
            </SectionCard>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-2">
            <SectionCard title="Sebaran per Provinsi" subtitle=Some("15 teratas menurut jumlah aset".into())>
                <BarList data=prov value_kind="jumlah" limit=15/>
            </SectionCard>
            <SectionCard title="Satuan Kerja Terbesar" subtitle=Some("15 teratas menurut jumlah aset".into())>
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

/// Ubah teks volume seperti "18 MB" menjadi satuan KB untuk penjumlahan.
fn parse_volume(s: &str) -> f64 {
    let t = s.trim().to_uppercase();
    let num: f64 = t
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .parse()
        .unwrap_or(0.0);
    if t.contains("GB") {
        num * 1024.0 * 1024.0
    } else if t.contains("MB") {
        num * 1024.0
    } else {
        num
    }
}
