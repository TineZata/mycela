use maud::{html, Markup, PreEscaped};
use crate::config::WidgetConfig;
use pvxs_sys::{Context, Value, MonitorEvent};
use plotters::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ─── Chart colours (dark-theme friendly) ────────────────────────────────────

const SERIES_COLORS: &[RGBColor] = &[
    RGBColor(0, 204, 102),   // green  (--alarm-none)
    RGBColor(100, 180, 255), // blue
    RGBColor(255, 165, 0),   // orange
    RGBColor(200, 100, 255), // purple
    RGBColor(255, 100, 100), // red
    RGBColor(0, 200, 200),   // cyan
];

const CHART_BG: RGBColor = RGBColor(30, 30, 36);
const CHART_GRID: RGBColor = RGBColor(58, 58, 66);
const CHART_TEXT: RGBColor = RGBColor(160, 160, 170);
const CHART_WIDTH: u32 = 640;
const CHART_HEIGHT: u32 = 300;

// ─── Rendering helpers ───────────────────────────────────────────────────────

fn y_range(series: &[(&str, &[f64])]) -> (f64, f64) {
    let (y_min, y_max) = series
        .iter()
        .flat_map(|(_, d)| d.iter())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    let margin = (y_max - y_min).abs() * 0.1 + 0.001;
    (y_min - margin, y_max + margin)
}

fn x_max(series: &[(&str, &[f64])]) -> f64 {
    series.iter().map(|(_, d)| d.len()).max().unwrap_or(1) as f64
}

/// Render multiple named line series with optional axis labels.
fn render_line_chart(
    series: &[(&str, &[f64])],
    x_label: &str,
    y_label: &str,
) -> String {
    let mut svg_buf = String::new();

    let n = series.len();

    if n != 1 {
        // ── Multi-series: overlaid traces, per-series coloured Y-axis columns ──
        // Each series is normalised to 0..1 for display so different-magnitude
        // PVs are all visible. The left margin holds N coloured Y-axis columns,
        // each showing the actual value scale for that series.
        const Y_COL_W: u32 = 56; // pixels per Y-axis column
        const N_TICKS: usize = 5;
        let left_pad: u32    = Y_COL_W * n.max(1) as u32;
        let top_pad:  u32    = 10;
        let right_pad: u32   = 12;
        let x_label_area: u32 = if x_label.is_empty() { 22 } else { 36 };

        {
            let root = SVGBackend::with_string(&mut svg_buf, (CHART_WIDTH, CHART_HEIGHT))
                .into_drawing_area();
            root.fill(&CHART_BG).ok();

            // Per-series actual value ranges (used for axis label annotation)
            let ranges: Vec<(f64, f64)> = series.iter().map(|(_, data)| {
                if data.is_empty() { return (0.0, 1.0); }
                let lo = data.iter().cloned().fold(f64::INFINITY,     f64::min);
                let hi = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let m  = (hi - lo).abs() * 0.12 + 0.001;
                (lo - m, hi + m)
            }).collect();

            let xm = x_max(series).max(1.0);

            // Chart area occupies the right portion; left_pad pixels reserved for Y columns.
            // DrawingArea::margin(top, bottom, left, right) — all u32.
            let chart_area = root.margin(top_pad, 4u32, left_pad, right_pad);

            let mut chart = ChartBuilder::on(&chart_area)
                .y_label_area_size(0u32)
                .x_label_area_size(x_label_area)
                .build_cartesian_2d(0f64..xm, 0f64..1f64)
                .unwrap();

            {
                let mut mesh = chart.configure_mesh();
                mesh.x_labels(5)
                    .y_labels(0)
                    .light_line_style(CHART_GRID.mix(0.2))
                    .axis_style(CHART_GRID)
                    .label_style(("sans-serif", 10).into_font().color(&CHART_TEXT))
                    .x_label_formatter(&|v| format!("{:.0}", v));
                if !x_label.is_empty() { mesh.x_desc(x_label); }
                mesh.draw().ok();
            }

            // Draw each series normalised to 0..1
            for (idx, (_, data)) in series.iter().enumerate() {
                if data.is_empty() { continue; }
                let color = SERIES_COLORS[idx % SERIES_COLORS.len()];
                let (lo, hi) = ranges[idx];
                let scale = (hi - lo).max(f64::EPSILON);
                let pts: Vec<(f64, f64)> = data.iter().enumerate()
                    .map(|(i, &y)| (i as f64, (y - lo) / scale))
                    .collect();
                chart.draw_series(LineSeries::new(pts, color.stroke_width(2))).ok();
            }

            // Compute plot-area pixel bounds from normalised data coordinates.
            // DrawingArea uses Rc<RefCell> internally so root.draw() is valid
            // while chart is still in scope.
            let (_, y_top) = chart.backend_coord(&(0.0, 1.0_f64));
            let (_, y_bot) = chart.backend_coord(&(0.0, 0.0_f64));

            // Draw per-series Y-axis columns in the left margin.
            // Series 0 is rightmost (closest to the chart), series n-1 is leftmost.
            for (idx, (name, data)) in series.iter().enumerate() {
                let color = SERIES_COLORS[idx % SERIES_COLORS.len()];
                let (lo, hi) = ranges[idx];
                let scale = (hi - lo).max(f64::EPSILON);

                let col_right = (left_pad as i32) - (idx as i32) * (Y_COL_W as i32);
                let col_left  = col_right - Y_COL_W as i32;

                // Vertical axis line
                root.draw(&PathElement::new(
                    vec![(col_right - 1, y_top), (col_right - 1, y_bot)],
                    color.stroke_width(1),
                )).ok();

                // Shortened display name — take the penultimate colon-segment
                // e.g. "demo:pressure:sensor" → "pressure"
                let parts: Vec<&str> = name.split(':').collect();
                let disp = if parts.len() >= 2 { parts[parts.len() - 2] } else { name };
                root.draw(&Text::new(
                    disp.to_string(),
                    (col_left + 2, y_top - 2),
                    ("sans-serif", 9).into_font().color(&color),
                )).ok();

                if data.is_empty() { continue; }

                // Tick marks and value labels
                for t in 0..=N_TICKS {
                    let norm   = t as f64 / N_TICKS as f64;
                    let actual = lo + norm * scale;
                    let (_, py) = chart.backend_coord(&(0.0, norm));

                    // Short tick mark on the right edge of the column
                    root.draw(&PathElement::new(
                        vec![(col_right - 6, py), (col_right - 1, py)],
                        color.stroke_width(1),
                    )).ok();

                    // Value label, left-aligned inside column
                    root.draw(&Text::new(
                        format!("{:.1}", actual),
                        (col_left + 2, py - 6),
                        ("sans-serif", 9).into_font().color(&color),
                    )).ok();
                }
            }

            root.present().ok();
        }
        return svg_buf;
    }

    // ── Single-series: original rendering ───────────────────────────────────
    {
        let root = SVGBackend::with_string(&mut svg_buf, (CHART_WIDTH, CHART_HEIGHT))
            .into_drawing_area();
        root.fill(&CHART_BG).ok();

        let (y_lo, y_hi) = y_range(series);
        let xm = x_max(series);

        let x_label_area: u32 = if x_label.is_empty() { 20 } else { 35 };
        let y_label_area: u32 = if y_label.is_empty() { 45 } else { 60 };

        let mut chart = ChartBuilder::on(&root)
            .margin(8)
            .x_label_area_size(x_label_area)
            .y_label_area_size(y_label_area)
            .build_cartesian_2d(0f64..xm, y_lo..y_hi)
            .unwrap();

        let mut mesh = chart.configure_mesh();
        mesh.x_labels(5)
            .y_labels(5)
            .light_line_style(CHART_GRID.mix(0.3))
            .bold_line_style(CHART_GRID.mix(0.5))
            .axis_style(CHART_GRID)
            .label_style(("sans-serif", 11).into_font().color(&CHART_TEXT))
            .y_label_formatter(&|v| format!("{:.1}", v))
            .x_label_formatter(&|_| String::new());
        if !x_label.is_empty() { mesh.x_desc(x_label); }
        if !y_label.is_empty() { mesh.y_desc(y_label); }
        mesh.draw().ok();

        let color = SERIES_COLORS[0];
        let pts: Vec<(f64, f64)> = series[0].1.iter().enumerate()
            .map(|(i, &y)| (i as f64, y)).collect();
        chart.draw_series(LineSeries::new(pts, color.stroke_width(2))).ok();

        root.present().ok();
    }
    svg_buf
}

/// Render a histogram (value distribution) from a data array.
fn render_histogram(
    data: &[f64],
    x_label: &str,
    y_label: &str,
) -> String {
    if data.is_empty() {
        return render_line_chart(&[], x_label, y_label);
    }
    let mut svg_buf = String::new();
    {
        let root = SVGBackend::with_string(&mut svg_buf, (CHART_WIDTH, CHART_HEIGHT))
            .into_drawing_area();
        root.fill(&CHART_BG).ok();

        let d_min = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let d_max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = (d_max - d_min).max(0.001);

        const NUM_BINS: usize = 20;
        let mut bins = vec![0u32; NUM_BINS];
        for &v in data {
            let bin = (((v - d_min) / range) * NUM_BINS as f64) as usize;
            let bin = bin.min(NUM_BINS - 1);
            bins[bin] += 1;
        }
        let max_count = *bins.iter().max().unwrap_or(&1) as f64;

        let x_label_area: u32 = if x_label.is_empty() { 20 } else { 35 };
        let y_label_area: u32 = if y_label.is_empty() { 45 } else { 60 };

        let mut chart = ChartBuilder::on(&root)
            .margin(8)
            .x_label_area_size(x_label_area)
            .y_label_area_size(y_label_area)
            .build_cartesian_2d((d_min..d_max).step(range / NUM_BINS as f64), 0f64..max_count * 1.1)
            .unwrap();

        let mut mesh = chart.configure_mesh();
        mesh.x_labels(5)
            .y_labels(5)
            .light_line_style(CHART_GRID.mix(0.3))
            .bold_line_style(CHART_GRID.mix(0.5))
            .axis_style(CHART_GRID)
            .label_style(("sans-serif", 11).into_font().color(&CHART_TEXT))
            .y_label_formatter(&|v| format!("{:.0}", v))
            .x_label_formatter(&|v| format!("{:.1}", v));
        if !x_label.is_empty() { mesh.x_desc(x_label); }
        if !y_label.is_empty() { mesh.y_desc(y_label); }
        mesh.draw().ok();

        let bar_color = SERIES_COLORS[0].mix(0.75);
        let bin_width = range / NUM_BINS as f64;
        chart.draw_series(bins.iter().enumerate().map(|(i, &count)| {
            let x0 = d_min + i as f64 * bin_width;
            let x1 = x0 + bin_width * 0.9;
            Rectangle::new(
                [(x0, 0f64), (x1, count as f64)],
                bar_color.filled(),
            )
        })).ok();

        root.present().ok();
    }
    svg_buf
}

/// Render a scatter plot. Requires two equal-length arrays (x, y).
fn render_scatter(
    x_data: &[f64],
    y_data: &[f64],
    x_label: &str,
    y_label: &str,
) -> String {
    if x_data.is_empty() || y_data.is_empty() {
        return render_line_chart(&[], x_label, y_label);
    }
    let n = x_data.len().min(y_data.len());
    let mut svg_buf = String::new();
    {
        let root = SVGBackend::with_string(&mut svg_buf, (CHART_WIDTH, CHART_HEIGHT))
            .into_drawing_area();
        root.fill(&CHART_BG).ok();

        let xmin = x_data[..n].iter().cloned().fold(f64::INFINITY, f64::min);
        let xmax = x_data[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let ymin = y_data[..n].iter().cloned().fold(f64::INFINITY, f64::min);
        let ymax = y_data[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let xm = (xmax - xmin).abs() * 0.05 + 0.001;
        let ym = (ymax - ymin).abs() * 0.05 + 0.001;

        let x_label_area: u32 = if x_label.is_empty() { 20 } else { 35 };
        let y_label_area: u32 = if y_label.is_empty() { 45 } else { 60 };

        let mut chart = ChartBuilder::on(&root)
            .margin(8)
            .x_label_area_size(x_label_area)
            .y_label_area_size(y_label_area)
            .build_cartesian_2d((xmin - xm)..(xmax + xm), (ymin - ym)..(ymax + ym))
            .unwrap();

        let mut mesh = chart.configure_mesh();
        mesh.x_labels(5)
            .y_labels(5)
            .light_line_style(CHART_GRID.mix(0.3))
            .bold_line_style(CHART_GRID.mix(0.5))
            .axis_style(CHART_GRID)
            .label_style(("sans-serif", 11).into_font().color(&CHART_TEXT))
            .x_label_formatter(&|v| format!("{:.1}", v))
            .y_label_formatter(&|v| format!("{:.1}", v));
        if !x_label.is_empty() { mesh.x_desc(x_label); }
        if !y_label.is_empty() { mesh.y_desc(y_label); }
        mesh.draw().ok();

        let dot_color = SERIES_COLORS[1];
        chart.draw_series(x_data[..n].iter().zip(y_data[..n].iter()).map(|(&x, &y)| {
            Circle::new((x, y), 3, dot_color.filled())
        })).ok();

        root.present().ok();
    }
    svg_buf
}

/// Render a scatter+histogram: scatter in large top area, x-histogram at bottom.
/// Expects x_data and y_data.
fn render_scatter_histogram(
    x_data: &[f64],
    y_data: &[f64],
    x_label: &str,
    y_label: &str,
) -> String {
    if x_data.is_empty() || y_data.is_empty() {
        return render_line_chart(&[], x_label, y_label);
    }
    let n = x_data.len().min(y_data.len());

    // Render scatter (top 65%) and x-histogram (bottom 35%) as separate SVG areas.
    const W: u32 = CHART_WIDTH;
    const H: u32 = CHART_HEIGHT + 160; // taller for the split layout (scatter + histogram row)

    let mut svg_buf = String::new();
    {
        let root = SVGBackend::with_string(&mut svg_buf, (W, H)).into_drawing_area();
        root.fill(&CHART_BG).ok();

        let scatter_h = (H as f64 * 0.62) as u32;
        let hist_h = H - scatter_h - 6;

        let (scatter_area, rest) = root.split_vertically(scatter_h);
        let (_, hist_area) = rest.split_vertically(6); // 6px gap

        // --- scatter ---
        let xmin = x_data[..n].iter().cloned().fold(f64::INFINITY, f64::min);
        let xmax = x_data[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let ymin = y_data[..n].iter().cloned().fold(f64::INFINITY, f64::min);
        let ymax = y_data[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let xm = (xmax - xmin).abs() * 0.05 + 0.001;
        let ym = (ymax - ymin).abs() * 0.05 + 0.001;

        let y_label_area: u32 = if y_label.is_empty() { 45 } else { 60 };

        let mut sc = ChartBuilder::on(&scatter_area)
            .margin(6)
            .x_label_area_size(0)
            .y_label_area_size(y_label_area)
            .build_cartesian_2d((xmin - xm)..(xmax + xm), (ymin - ym)..(ymax + ym))
            .unwrap();
        let mut mesh = sc.configure_mesh();
        mesh.x_labels(0)
            .y_labels(4)
            .light_line_style(CHART_GRID.mix(0.3))
            .bold_line_style(CHART_GRID.mix(0.5))
            .axis_style(CHART_GRID)
            .label_style(("sans-serif", 11).into_font().color(&CHART_TEXT))
            .y_label_formatter(&|v| format!("{:.1}", v));
        if !y_label.is_empty() { mesh.y_desc(y_label); }
        mesh.draw().ok();
        sc.draw_series(x_data[..n].iter().zip(y_data[..n].iter()).map(|(&x, &y)| {
            Circle::new((x, y), 3, SERIES_COLORS[1].filled())
        })).ok();

        // --- x-histogram ---
        let range = (xmax - xmin).max(0.001);
        const NUM_BINS: usize = 20;
        let mut bins = vec![0u32; NUM_BINS];
        for &v in &x_data[..n] {
            let bin = (((v - xmin) / range) * NUM_BINS as f64) as usize;
            bins[bin.min(NUM_BINS - 1)] += 1;
        }
        let max_count = *bins.iter().max().unwrap_or(&1) as f64;

        let x_label_area: u32 = if x_label.is_empty() { 20 } else { 35 };

        let _ = hist_h; // used implicitly by split_vertically
        let mut hc = ChartBuilder::on(&hist_area)
            .margin_left(y_label_area as i32)
            .margin_right(6)
            .margin_bottom(6)
            .x_label_area_size(x_label_area)
            .y_label_area_size(0)
            .build_cartesian_2d((xmin..xmax).step(range / NUM_BINS as f64), 0f64..max_count * 1.1)
            .unwrap();
        let mut mesh = hc.configure_mesh();
        mesh.x_labels(5)
            .y_labels(0)
            .light_line_style(CHART_GRID.mix(0.3))
            .axis_style(CHART_GRID)
            .label_style(("sans-serif", 11).into_font().color(&CHART_TEXT))
            .x_label_formatter(&|v| format!("{:.1}", v));
        if !x_label.is_empty() { mesh.x_desc(x_label); }
        mesh.draw().ok();

        let bin_width = range / NUM_BINS as f64;
        hc.draw_series(bins.iter().enumerate().map(|(i, &count)| {
            let x0 = xmin + i as f64 * bin_width;
            let x1 = x0 + bin_width * 0.9;
            Rectangle::new([(x0, 0f64), (x1, count as f64)], SERIES_COLORS[0].mix(0.75).filled())
        })).ok();

        root.present().ok();
    }
    svg_buf
}

// ─── Widget struct ──────────────────────────────────────────────────────────

/// Lightweight snapshot of primary-PV metadata (Value is not Clone/Send).
#[derive(Default, Clone)]
struct PrimaryMeta {
    alarm_severity: i32,
    description: String,
    units: String,
    limit_lo: f64,
    limit_hi: f64,
}

impl PrimaryMeta {
    fn from_value(raw: &Value) -> Self {
        Self {
            alarm_severity: raw.get_field_int32("alarm.severity").unwrap_or(0),
            description: raw.get_field_string("display.description").unwrap_or_default(),
            units: raw.get_field_string("display.units").unwrap_or_default(),
            limit_lo: raw.get_field_double("display.limitLow").unwrap_or(0.0),
            limit_hi: raw.get_field_double("display.limitHigh").unwrap_or(0.0),
        }
    }
}

pub struct Chart {
    config: WidgetConfig,
}

impl Chart {
    pub fn new(config: WidgetConfig) -> Self {
        Self { config }
    }

    pub fn into_sse_stream(
        self,
    ) -> impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>
           + Send
           + 'static {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let config = std::sync::Arc::new(self.config);
        let config_thread = config.clone();

        tokio::task::spawn_blocking(move || Self::run_monitor(config_thread, tx));

        async_stream::stream! {
            yield Ok(axum::response::sse::Event::default().data(
                render_inner_disconnected(&config).into_string()
            ));
            let mut rx = rx;
            while let Some(html) = rx.recv().await {
                yield Ok(axum::response::sse::Event::default().data(html));
            }
        }
    }

    pub(crate) fn run_monitor(
        config: std::sync::Arc<WidgetConfig>,
        tx: tokio::sync::mpsc::UnboundedSender<String>,
    ) {
        // Multi-series line chart: each PV needs its own concurrent monitor.
        let chart_type = config.chart_type.as_deref().unwrap_or("line");
        let all_pvs = collect_series_pvs(&config);
        if chart_type == "line" && all_pvs.len() > 1 {
            Self::run_monitor_multi(config, all_pvs, tx);
            return;
        }

        tracing::info!("Chart monitor starting for: {}", config.pv_name);

        let mut ctx = match Context::from_env() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Context creation failed for {}: {}", config.pv_name, e);
                let _ = tx.send(render_inner_disconnected(&config).into_string());
                return;
            }
        };

        let mut monitor = match ctx
            .monitor_builder(&config.pv_name)
            .and_then(|b| b.connect_exception(true).disconnect_exception(true).exec())
        {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Monitor creation failed for {}: {}", config.pv_name, e);
                let _ = tx.send(render_inner_disconnected(&config).into_string());
                return;
            }
        };

        if let Err(e) = monitor.start() {
            tracing::error!("Monitor start failed for {}: {}", config.pv_name, e);
            return;
        }

        loop {
            match monitor.pop() {
                Ok(Some(raw)) => {
                    let html = render_inner_connected(&config, &raw).into_string();
                    if tx.send(html).is_err() { break; }
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
                Err(MonitorEvent::Connected(msg)) => {
                    tracing::info!("Chart {}: connected - {}", config.pv_name, msg);
                }
                Err(MonitorEvent::Disconnected(msg)) => {
                    tracing::warn!("Chart {}: disconnected - {}", config.pv_name, msg);
                    if tx.send(render_inner_disconnected(&config).into_string()).is_err() { break; }
                }
                Err(MonitorEvent::Finished(msg)) => {
                    tracing::info!("Chart {}: finished - {}", config.pv_name, msg);
                    break;
                }
                Err(MonitorEvent::RemoteError(msg) | MonitorEvent::ClientError(msg)) => {
                    tracing::error!("Chart {}: error - {}", config.pv_name, msg);
                    if tx.send(render_inner_disconnected(&config).into_string()).is_err() { break; }
                }
            }
        }

        tracing::info!("Chart monitor stopped for: {}", config.pv_name);
    }

    /// Monitor every PV concurrently, re-rendering the chart whenever any series updates.
    fn run_monitor_multi(
        config: std::sync::Arc<WidgetConfig>,
        all_pvs: Vec<String>,
        tx: tokio::sync::mpsc::UnboundedSender<String>,
    ) {
        tracing::info!("Chart multi-series monitor starting for {} PVs", all_pvs.len());

        // Shared state: latest data per PV name + metadata snapshot from primary PV.
        type SharedState = Arc<Mutex<(HashMap<String, Vec<f64>>, PrimaryMeta)>>;
        let state: SharedState = Arc::new(Mutex::new((HashMap::new(), PrimaryMeta::default())));

        let handles: Vec<_> = all_pvs.iter().cloned().enumerate().map(|(idx, pv_name)| {
            let config    = config.clone();
            let all_pvs   = all_pvs.clone();
            let state     = state.clone();
            let tx        = tx.clone();
            let is_primary = idx == 0;

            std::thread::spawn(move || {
                let mut ctx = match Context::from_env() {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Multi-monitor: Context failed for {}: {}", pv_name, e);
                        return;
                    }
                };

                let mut monitor = match ctx
                    .monitor_builder(&pv_name)
                    .and_then(|b| b.connect_exception(true).disconnect_exception(true).exec())
                {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::error!("Multi-monitor: Monitor failed for {}: {}", pv_name, e);
                        return;
                    }
                };

                if let Err(e) = monitor.start() {
                    tracing::error!("Multi-monitor: Start failed for {}: {}", pv_name, e);
                    return;
                }

                loop {
                    match monitor.pop() {
                        Ok(Some(raw)) => {
                            if let Ok(arr) = raw.get_field_double_array("value") {
                                let mut guard = state.lock().unwrap();
                                guard.0.insert(pv_name.clone(), arr);
                                if is_primary {
                                    guard.1 = PrimaryMeta::from_value(&raw);
                                }
                                let html = render_inner_connected_multi(
                                    &config, &guard.0, &all_pvs, &guard.1,
                                ).into_string();
                                drop(guard);
                                if tx.send(html).is_err() { break; }
                            }
                        }
                        Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
                        Err(MonitorEvent::Connected(msg)) => {
                            tracing::info!("Multi-monitor {}: connected - {}", pv_name, msg);
                        }
                        Err(MonitorEvent::Disconnected(msg)) => {
                            tracing::warn!("Multi-monitor {}: disconnected - {}", pv_name, msg);
                            if tx.send(render_inner_disconnected(&config).into_string()).is_err() { break; }
                        }
                        Err(MonitorEvent::Finished(msg)) => {
                            tracing::info!("Multi-monitor {}: finished - {}", pv_name, msg);
                            break;
                        }
                        Err(MonitorEvent::RemoteError(msg) | MonitorEvent::ClientError(msg)) => {
                            tracing::error!("Multi-monitor {}: error - {}", pv_name, msg);
                            if tx.send(render_inner_disconnected(&config).into_string()).is_err() { break; }
                        }
                    }
                }
                tracing::info!("Multi-monitor stopped for: {}", pv_name);
            })
        }).collect();

        for h in handles { let _ = h.join(); }
    }
}

// ─── Tooltip helpers ─────────────────────────────────────────────────────────

/// Build a chart-specific tooltip that lists every PV and which axis/series it maps to.
fn build_chart_tooltip(config: &WidgetConfig, raw: &Value) -> String {
    let chart_type = config.chart_type.as_deref().unwrap_or("line");
    let x_label = config.axis_label_x.as_deref().unwrap_or("").to_string();
    let y_label = config.axis_label_y.as_deref().unwrap_or("").to_string();

    let mut t = String::new();

    // Chart type line
    let type_str = match chart_type {
        "histogram"         => "Histogram",
        "scatter"           => "Scatter",
        "scatter_histogram" => "Scatter + Histogram",
        _                   => "Line",
    };
    t.push_str(&format!("Chart type: {}\n", type_str));

    // Axis labels
    if !x_label.is_empty() { t.push_str(&format!("X-axis: {}\n", x_label)); }
    if !y_label.is_empty() { t.push_str(&format!("Y-axis: {}\n", y_label)); }

    t.push('\n');

    // Per-PV / per-series breakdown
    match chart_type {
        "scatter" | "scatter_histogram" => {
            // pv_name → X axis, first name in pv_names → Y axis
            t.push_str(&format!("X data:  {}\n", config.pv_name));
            if let Some(names) = &config.pv_names {
                if let Some(y_pv) = names.first() {
                    t.push_str(&format!("Y data:  {}\n", y_pv));
                }
            }
        }
        "histogram" => {
            t.push_str(&format!("Data PV: {}\n", config.pv_name));
        }
        _ => {
            // Line chart — list all series PVs
            let series_pvs = collect_series_pvs(config);
            for (idx, pv) in series_pvs.iter().enumerate() {
                t.push_str(&format!("Series {}: {}\n", idx + 1, pv));
            }
        }
    }

    // Standard PV metadata (from the primary PV's Value)
    t.push('\n');
    if let Ok(v) = raw.get_field_string("display.description") {
        if !v.is_empty() { t.push_str(&format!("{}\n", v)); }
    }
    if let Ok(v) = raw.get_field_string("display.units") {
        if !v.is_empty() { t.push_str(&format!("Units: {}\n", v)); }
    }
    if let Ok(lo) = raw.get_field_double("display.limitLow") {
        if let Ok(hi) = raw.get_field_double("display.limitHigh") {
            t.push_str(&format!("Display range: {:.2} – {:.2}\n", lo, hi));
        }
    }

    let severity = raw.get_field_int32("alarm.severity").unwrap_or(0);
    let sev_str = match pvxs_sys::AlarmSeverity::from(severity) {
        pvxs_sys::AlarmSeverity::NoAlarm => "No Alarm",
        pvxs_sys::AlarmSeverity::Minor   => "Minor",
        pvxs_sys::AlarmSeverity::Major   => "Major",
        pvxs_sys::AlarmSeverity::Invalid => "Invalid",
        _                                => "Unknown",
    };
    t.push_str(&format!("Alarm: {}\n", sev_str));

    t.trim_end().to_string()
}

/// Collect all PV names for a line chart (primary + up to 5 extras).
fn collect_series_pvs(config: &WidgetConfig) -> Vec<String> {
    let mut pvs = vec![config.pv_name.clone()];
    if let Some(extra) = &config.pv_names {
        for pv in extra.iter().take(5) {
            pvs.push(pv.clone());
        }
    }
    pvs
}

// ─── HTML rendering helpers ─────────────────────────────────────────────────

fn render_inner_connected(config: &WidgetConfig, raw: &Value) -> Markup {
    let alarm_severity = raw.get_field_int32("alarm.severity").unwrap_or(0);
    let alarm_class = super::alarm_severity_class(alarm_severity);
    let icon: Option<&str> = match alarm_severity {
        1 => Some(super::MINOR_ALARM_SVG),
        2 => Some(super::MAJOR_ALARM_SVG),
        3 => Some(super::INVALID_SVG),
        _ => None,
    };

    let chart_type = config.chart_type.as_deref().unwrap_or("line");
    let x_label = config.axis_label_x.as_deref().unwrap_or("");
    let y_label = config.axis_label_y.as_deref().unwrap_or("");

    let svg_string = match chart_type {
        "histogram" => {
            match raw.get_field_double_array("value") {
                Ok(arr) => render_histogram(&arr, x_label, y_label),
                _       => render_histogram(&[], x_label, y_label),
            }
        }
        "scatter" => {
            // Primary PV = X data; first pv_names entry = Y data.
            // Both PVs are colocated in the same double_array PV via interleaved storage
            // convention: the array holds [x0,y0, x1,y1, ...] OR we read two separate PVs.
            // Here we read from the "value" field: first half X, second half Y.
            match raw.get_field_double_array("value") {
                Ok(arr) if arr.len() >= 2 => {
                    let mid = arr.len() / 2;
                    render_scatter(&arr[..mid], &arr[mid..], x_label, y_label)
                }
                _ => render_scatter(&[], &[], x_label, y_label),
            }
        }
        "scatter_histogram" => {
            match raw.get_field_double_array("value") {
                Ok(arr) if arr.len() >= 2 => {
                    let mid = arr.len() / 2;
                    render_scatter_histogram(&arr[..mid], &arr[mid..], x_label, y_label)
                }
                _ => render_scatter_histogram(&[], &[], x_label, y_label),
            }
        }
        _ => {
            // Line chart — primary PV only (multi-PV requires separate monitors; todo future)
            match raw.get_field_double_array("value") {
                Ok(arr) => {
                    // If pv_names is set the label is the PV name for the legend.
                    let primary_label = if config.pv_names.as_ref().map_or(false, |v| !v.is_empty()) {
                        config.pv_name.as_str()
                    } else {
                        "value"
                    };
                    render_line_chart(&[(primary_label, &arr)], x_label, y_label)
                }
                _ => render_line_chart(&[], x_label, y_label),
            }
        }
    };

    let tooltip = build_chart_tooltip(config, raw);

    render_chart_html(
        config,
        None,
        &format!("chart {}", alarm_class),
        icon,
        &tooltip,
        &svg_string,
    )
}

/// Render a multi-series line chart from pre-extracted data (no Value needed).
fn render_inner_connected_multi(
    config: &WidgetConfig,
    series_map: &HashMap<String, Vec<f64>>,
    all_pvs: &[String],
    meta: &PrimaryMeta,
) -> Markup {
    let alarm_severity = meta.alarm_severity;
    let alarm_class = super::alarm_severity_class(alarm_severity);
    let icon: Option<&str> = match alarm_severity {
        1 => Some(super::MINOR_ALARM_SVG),
        2 => Some(super::MAJOR_ALARM_SVG),
        3 => Some(super::INVALID_SVG),
        _ => None,
    };
    let x_label = config.axis_label_x.as_deref().unwrap_or("");
    let y_label = config.axis_label_y.as_deref().unwrap_or("");

    // Build series in config-defined order; skip PVs not yet received
    let series_vecs: Vec<(&str, Vec<f64>)> = all_pvs.iter()
        .filter_map(|pv| series_map.get(pv).map(|v| (pv.as_str(), v.clone())))
        .collect();
    let series_refs: Vec<(&str, &[f64])> = series_vecs.iter()
        .map(|(n, v)| (*n, v.as_slice()))
        .collect();
    let svg_string = render_line_chart(&series_refs, x_label, y_label);

    // Build tooltip without needing a Value
    let mut t = format!("Chart type: Line\n");
    if !x_label.is_empty() { t.push_str(&format!("X-axis: {}\n", x_label)); }
    if !y_label.is_empty() { t.push_str(&format!("Y-axis: {}\n", y_label)); }
    t.push('\n');
    for (idx, pv) in all_pvs.iter().enumerate() {
        t.push_str(&format!("Series {}: {}\n", idx + 1, pv));
    }
    t.push('\n');
    if !meta.description.is_empty() { t.push_str(&format!("{}\n", meta.description)); }
    if !meta.units.is_empty() { t.push_str(&format!("Units: {}\n", meta.units)); }
    if meta.limit_lo != 0.0 || meta.limit_hi != 0.0 {
        t.push_str(&format!("Display range: {:.2} \u{2013} {:.2}\n", meta.limit_lo, meta.limit_hi));
    }
    let sev_str = match meta.alarm_severity {
        0 => "No Alarm", 1 => "Minor", 2 => "Major", 3 => "Invalid", _ => "Unknown",
    };
    t.push_str(&format!("Alarm: {}\n", sev_str));
    let tooltip = t.trim_end().to_string();

    render_chart_html(
        config,
        None,
        &format!("chart {}", alarm_class),
        icon,
        &tooltip,
        &svg_string,
    )
}

fn render_inner_disconnected(config: &WidgetConfig) -> Markup {
    render_chart_html(config, None, "chart alarm-disconnected", Some(super::OFFLINE_SVG), "", "")
}

fn render_chart_html(
    config: &WidgetConfig,
    _display_value: Option<String>, // kept for future scalar fallback
    _alarm_class: &str,
    icon: Option<&str>,
    tooltip: &str,
    svg_content: &str,
) -> Markup {
    html! {
        div class="widget-inner" {
            label class="widget-label" {
                (config.label)
                @if let Some(src) = icon {
                    img class="widget-status-icon" src=(src) alt="status";
                }
                @if !tooltip.is_empty() {
                    (super::render_info_btn(tooltip))
                }
            }
            div class="chart-container" {
                @if !svg_content.is_empty() {
                    (PreEscaped(svg_content))
                } @else {
                    div class="chart-placeholder" { "Waiting for data…" }
                }
            }
            @if let Some(desc) = &config.description {
                @if !desc.is_empty() {
                    p class="widget-description" { (desc) }
                }
            }
        }
    }
}

pub fn render_chart(widget: &WidgetConfig) -> Markup {
    html! {
        div style=[super::widget_container_style(widget)]
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            hx-sse=(format!("swap:{}", widget.id)) {
            (render_inner_disconnected(widget))
        }
    }
}

