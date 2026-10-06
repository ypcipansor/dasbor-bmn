use leptos::prelude::*;

use crate::model::{format_number, format_rupiah_short};

const PALETTE: &[&str] = &[
    "#1f47f5", "#0ea5e9", "#10b981", "#f59e0b", "#ef4444", "#8b5cf6", "#ec4899", "#14b8a6",
    "#f97316", "#6366f1", "#84cc16", "#06b6d4",
];

fn color_at(i: usize) -> &'static str {
    PALETTE[i % PALETTE.len()]
}

/// Cincin (donut) untuk komposisi; dibuat dari `stroke-dasharray` agar ringan
/// dan tetap tajam di semua ukuran layar.
#[component]
pub fn DonutChart(
    data: Vec<(String, f64)>,
    center_label: String,
    center_value: String,
) -> impl IntoView {
    let r = 60.0_f64;
    let c = 2.0 * std::f64::consts::PI * r;
    let total: f64 = data.iter().map(|(_, v)| v).sum();
    let mut acc = 0.0_f64;
    let segments: Vec<(usize, String, String, f64, String)> = data
        .iter()
        .enumerate()
        .map(|(i, (label, v))| {
            let frac = if total > 0.0 { v / total } else { 0.0 };
            let len = frac * c;
            let offset = -acc;
            acc += len;
            (
                i,
                format!("{len:.3} {:.3}", (c - len).max(0.0)),
                format!("{offset:.3}"),
                frac * 100.0,
                label.clone(),
            )
        })
        .collect();

    view! {
        <div class="flex flex-col items-center gap-4 sm:flex-row sm:items-center sm:gap-6">
            <div class="relative shrink-0">
                <svg
                    viewBox="0 0 160 160"
                    class="size-40 sm:size-44"
                    role="img"
                    aria-label="Komposisi nilai aset per kategori"
                >
                    <circle cx="80" cy="80" r="60" fill="none" stroke="currentColor" stroke-width="18" class="text-ink-100 dark:text-ink-800"/>
                    <g transform="rotate(-90 80 80)">
                        {segments
                            .iter()
                            .map(|(i, dash, offset, _, label)| {
                                view! {
                                    <circle
                                        cx="80"
                                        cy="80"
                                        r="60"
                                        fill="none"
                                        stroke=color_at(*i)
                                        stroke-width="18"
                                        stroke-dasharray=dash.clone()
                                        stroke-dashoffset=offset.clone()
                                    >
                                        <title>{label.clone()}</title>
                                    </circle>
                                }
                            })
                            .collect_view()}
                    </g>
                </svg>
                <div class="pointer-events-none absolute inset-0 grid place-items-center text-center">
                    <div>
                        <p class="text-[0.65rem] font-medium uppercase tracking-wide text-ink-400">
                            {center_label}
                        </p>
                        <p class="text-lg font-bold tabular-nums">{center_value}</p>
                    </div>
                </div>
            </div>
            <ul class="grid w-full min-w-0 grid-cols-1 gap-1.5 sm:grid-cols-2">
                {segments
                    .iter()
                    .map(|(i, _, _, pct, label)| {
                        view! {
                            <li class="flex min-w-0 items-center gap-2 text-xs">
                                <span
                                    class="size-2.5 shrink-0 rounded-full"
                                    style=format!("background:{}", color_at(*i))
                                    aria-hidden="true"
                                ></span>
                                <span class="min-w-0 flex-1 truncate" title=label.clone()>
                                    {label.clone()}
                                </span>
                                <span class="shrink-0 tabular-nums text-ink-400">
                                    {format!("{pct:.1}%")}
                                </span>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </div>
    }
}

/// Diagram batang horizontal untuk peringkat (provinsi, satker, kondisi).
#[component]
pub fn BarList(
    data: Vec<(String, i64, f64)>,
    value_kind: &'static str,
    limit: usize,
) -> impl IntoView {
    let max = data.iter().map(|(_, j, _)| *j).max().unwrap_or(1).max(1);
    let rows: Vec<(String, i64, f64, f64)> = data
        .into_iter()
        .take(limit)
        .map(|(n, j, v)| {
            let pct = (j as f64 / max as f64) * 100.0;
            (n, j, v, pct)
        })
        .collect();
    view! {
        <ul class="space-y-2.5">
            {rows
                .iter()
                .enumerate()
                .map(|(i, (name, jumlah, nilai, pct))| {
                    let val = if value_kind == "rupiah" {
                        format_rupiah_short(*nilai)
                    } else {
                        format_number(*jumlah)
                    };
                    view! {
                        <li>
                            <div class="flex items-baseline justify-between gap-3 text-xs">
                                <span class="min-w-0 flex-1 truncate" title=name.clone()>
                                    {name.clone()}
                                </span>
                                <span class="shrink-0 tabular-nums font-medium">{val}</span>
                            </div>
                            <div class="mt-1 h-2 w-full overflow-hidden rounded-full bg-ink-100 dark:bg-ink-800">
                                <div
                                    class="h-full rounded-full"
                                    style=format!("width:{:.1}%;background:{}", pct.max(1.0), color_at(i))
                                ></div>
                            </div>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}

/// Diagram garis dengan area untuk tren perolehan per tahun.
#[component]
pub fn AreaChart(points: Vec<(i32, i64, f64)>, value_kind: &'static str) -> impl IntoView {
    const W: f64 = 640.0;
    const H: f64 = 200.0;
    const PAD_L: f64 = 46.0;
    const PAD_B: f64 = 26.0;
    const PAD_T: f64 = 12.0;
    const PAD_R: f64 = 10.0;

    if points.len() < 2 {
        return view! {
            <p class="py-6 text-center text-sm text-ink-400">"Data tren belum cukup untuk digambar."</p>
        }
        .into_any();
    }

    let y_max = points
        .iter()
        .map(|(_, j, v)| {
            if value_kind == "rupiah" {
                *v
            } else {
                *j as f64
            }
        })
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let n = points.len() as f64;
    let plot_w = W - PAD_L - PAD_R;
    let plot_h = H - PAD_T - PAD_B;

    let xy: Vec<(f64, f64, i32, i64, f64)> = points
        .iter()
        .enumerate()
        .map(|(i, (y, j, v))| {
            let x = PAD_L + (i as f64 / (n - 1.0)) * plot_w;
            let val = if value_kind == "rupiah" {
                *v
            } else {
                *j as f64
            };
            let yp = PAD_T + plot_h - (val / y_max) * plot_h;
            (x, yp, *y, *j, *v)
        })
        .collect();

    let line: String = xy
        .iter()
        .map(|(x, y, _, _, _)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ");
    let area = format!(
        "{:.1},{:.1} {} {:.1},{:.1}",
        PAD_L,
        PAD_T + plot_h,
        line,
        PAD_L + plot_w,
        PAD_T + plot_h
    );
    let first_year = xy.first().map(|p| p.2).unwrap_or(0);
    let last_year = xy.last().map(|p| p.2).unwrap_or(0);
    let grid: Vec<f64> = (0..=4).map(|i| PAD_T + (i as f64 / 4.0) * plot_h).collect();
    let last_val = xy.last().map(|p| (p.3, p.4)).unwrap_or((0, 0.0));

    view! {
        <div class="w-full">
            <svg
                viewBox=format!("0 0 {W} {H}")
                class="h-52 w-full sm:h-60"
                role="img"
                aria-label="Tren jumlah dan nilai perolehan aset per tahun"
                preserveAspectRatio="none"
            >
                <defs>
                    <linearGradient id="areaFill" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="0%" stop-color="#1f47f5" stop-opacity="0.28"/>
                        <stop offset="100%" stop-color="#1f47f5" stop-opacity="0.02"/>
                    </linearGradient>
                </defs>
                {grid
                    .iter()
                    .map(|y| {
                        view! {
                            <line
                                x1=PAD_L
                                x2=W - PAD_R
                                y1=*y
                                y2=*y
                                stroke="currentColor"
                                stroke-width="1"
                                class="text-ink-100 dark:text-ink-800"
                            ></line>
                        }
                    })
                    .collect_view()}
                <polygon points=area fill="url(#areaFill)"/>
                <polyline
                    points=line
                    fill="none"
                    stroke="#1f47f5"
                    stroke-width="2.5"
                    stroke-linejoin="round"
                    stroke-linecap="round"
                ></polyline>
                {xy
                    .iter()
                    .map(|(x, y, year, j, v)| {
                        let label = if value_kind == "rupiah" {
                            format_rupiah_short(*v)
                        } else {
                            format_number(*j)
                        };
                        view! {
                            <circle cx=*x cy=*y r="3" fill="#fff" stroke="#1f47f5" stroke-width="2">
                                <title>{format!("{year}: {label}")}</title>
                            </circle>
                        }
                    })
                    .collect_view()}
                <text x=PAD_L y=H - 8.0 fill="currentColor" font-size="11" class="text-ink-400">
                    {first_year.to_string()}
                </text>
                <text
                    x=W - PAD_R
                    y=H - 8.0
                    fill="currentColor"
                    font-size="11"
                    text-anchor="end"
                    class="text-ink-400"
                >
                    {last_year.to_string()}
                </text>
            </svg>
            <p class="mt-1 text-xs text-ink-400">
                "Periode "
                {first_year.to_string()}
                "–"
                {last_year.to_string()}
                " · puncak "
                {if value_kind == "rupiah" {
                    format_rupiah_short(last_val.1)
                } else {
                    format_number(last_val.0)
                }}
            </p>
        </div>
    }
    .into_any()
}

/// Batang bertumpuk sederhana untuk perbandingan jumlah vs. nilai per kategori.
#[component]
pub fn CategoryBars(data: Vec<(String, i64, f64, String)>) -> impl IntoView {
    let max = data.iter().map(|(_, j, _, _)| *j).max().unwrap_or(1).max(1);
    view! {
        <ul class="space-y-2">
            {data
                .iter()
                .enumerate()
                .map(|(i, (name, jumlah, nilai, icon))| {
                    let pct = (*jumlah as f64 / max as f64) * 100.0;
                    view! {
                        <li class="flex items-center gap-3">
                            <span class="w-6 shrink-0 text-center text-base" aria-hidden="true">
                                {icon.clone()}
                            </span>
                            <span class="w-32 shrink-0 truncate text-xs sm:w-44" title=name.clone()>
                                {name.clone()}
                            </span>
                            <span class="h-2.5 min-w-0 flex-1 overflow-hidden rounded-full bg-ink-100 dark:bg-ink-800">
                                <span
                                    class="block h-full rounded-full"
                                    style=format!("width:{:.1}%;background:{}", pct.max(1.0), color_at(i))
                                ></span>
                            </span>
                            <span class="w-16 shrink-0 text-right text-xs tabular-nums font-medium">
                                {format_number(*jumlah)}
                            </span>
                            <span class="hidden w-24 shrink-0 text-right text-xs tabular-nums text-ink-400 sm:block">
                                {format_rupiah_short(*nilai)}
                            </span>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}
