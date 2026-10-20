//! HTML contact sheet for a set of variations.

use std::fmt::Write as _;
use std::path::Path;

use bw_core::FilmGeometry;
use bw_developer::{Agitation, ProcessingConditions};
use bw_negative::sensitometry::IsoSpeed;

pub struct Entry {
    pub stem: String,
    pub label: String,
    pub film_name: String,
    pub developer: String,
    pub exposure_index: f64,
    pub process: ProcessingConditions,
    pub grade: f64,
    pub iso: Option<IsoSpeed>,
    pub curve: Vec<(f64, f64)>,
    pub grey_log_h: f64,
    pub warnings: Vec<String>,
    pub millis: u128,
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn agitation(a: &Agitation) -> String {
    match a {
        Agitation::Continuous => "continuous".into(),
        Agitation::Intermittent { interval_s } => format!("every {interval_s:.0} s"),
        Agitation::SemiStand { mid_agitations } => format!("semi-stand ({mid_agitations})"),
        Agitation::Stand => "stand".into(),
    }
}

/// Characteristic curve as inline SVG: log H on x, density on y.
fn curve_svg(entry: &Entry) -> String {
    let (w, h) = (260.0, 150.0);
    let (x0, x1, y1) = (-4.5, 0.5, 3.0);
    let px = |lh: f64| (lh - x0) / (x1 - x0) * w;
    let py = |d: f64| h - d / y1 * h;
    let mut s = format!(
        r#"<svg viewBox="-28 -6 {} {}" class="curve" role="img" aria-label="Characteristic curve">"#,
        w + 36.0,
        h + 26.0
    );
    for d in [0.0, 1.0, 2.0, 3.0] {
        let _ = write!(
            s,
            r#"<line x1="0" x2="{w}" y1="{y}" y2="{y}" class="grid"/><text x="-6" y="{ty}" class="tick" text-anchor="end">{d:.0}</text>"#,
            y = py(d),
            ty = py(d) + 3.0
        );
    }
    for lh in [-4.0, -3.0, -2.0, -1.0, 0.0] {
        let _ = write!(
            s,
            r#"<line y1="0" y2="{h}" x1="{x}" x2="{x}" class="grid"/><text x="{x}" y="{ty}" class="tick" text-anchor="middle">{lh:.0}</text>"#,
            x = px(lh),
            ty = h + 14.0
        );
    }
    let grey = px(entry.grey_log_h);
    let _ = write!(
        s,
        r#"<line y1="0" y2="{h}" x1="{grey}" x2="{grey}" class="grey"/>"#
    );
    let pts: Vec<String> = entry
        .curve
        .iter()
        .map(|&(lh, d)| format!("{:.1},{:.1}", px(lh), py(d.min(y1))))
        .collect();
    let _ = write!(
        s,
        r#"<polyline points="{}" class="line"/></svg>"#,
        pts.join(" ")
    );
    s
}

pub fn write_html(
    path: &Path,
    input: &str,
    geometry: &FilmGeometry,
    entries: &[Entry],
) -> std::io::Result<()> {
    let mut cards = String::new();
    for e in entries {
        let iso = e.iso.map_or("—".to_string(), |i| {
            format!("{:.0} (ΔD<sub>1.3</sub> {:.2})", i.speed, i.delta_d_1_30)
        });
        let warnings: String = e
            .warnings
            .iter()
            .map(|w| format!(r#"<li>{}</li>"#, escape(w)))
            .collect();
        let _ = write!(
            cards,
            r#"<article class="card">
  <a href="{stem}-print.png"><img src="{stem}-print.png" alt="{label} print" loading="lazy"></a>
  <div class="meta">
    <h2><span class="num">{stem}</span> {label}</h2>
    <dl>
      <dt>Film</dt><dd>{film}</dd>
      <dt>Developer</dt><dd>{dev}</dd>
      <dt>Process</dt><dd>{t:.2} min · {temp:.0} °C · {agit}</dd>
      <dt>EI · grade</dt><dd>{ei:.0} · {grade:.0}</dd>
      <dt>Model ISO speed</dt><dd>{iso}</dd>
    </dl>
    {curve}
    <a class="neg" href="{stem}-negative.png"><img src="{stem}-negative.png" alt="negative" loading="lazy"><span>negative</span></a>
    {warn}
    <p class="time">{ms} ms</p>
  </div>
</article>
"#,
            stem = e.stem,
            label = escape(&e.label),
            film = escape(&e.film_name),
            dev = escape(&e.developer),
            t = e.process.time_min,
            temp = e.process.temperature_c,
            agit = agitation(&e.process.agitation),
            ei = e.exposure_index,
            grade = e.grade,
            curve = curve_svg(e),
            warn = if warnings.is_empty() {
                String::new()
            } else {
                format!(r#"<ul class="warn">{warnings}</ul>"#)
            },
            ms = e.millis,
        );
    }

    let html = format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Emulation Variations</title>
<style>
:root {{ --bg:#f4f3f0; --card:#fff; --ink:#1d1d1b; --muted:#6b6a66; --line:#d9d7d1; --accent:#9a3b1e; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg:#141413; --card:#1f1f1d; --ink:#ecebe7; --muted:#9b9a95; --line:#3a3936; --accent:#e08b62; }} }}
* {{ box-sizing:border-box; }}
body {{ margin:0; background:var(--bg); color:var(--ink); font:14px/1.45 -apple-system, system-ui, sans-serif; }}
header {{ max-width:1500px; margin:0 auto; padding:28px 16px 8px; }}
h1 {{ font-size:22px; margin:0 0 6px; }}
.note {{ color:var(--muted); max-width:80ch; margin:4px 0; }}
.note strong {{ color:var(--accent); }}
main {{ max-width:1500px; margin:0 auto; padding:16px; display:grid; gap:16px; grid-template-columns:repeat(auto-fill, minmax(min(100%, 460px), 1fr)); }}
.card {{ background:var(--card); border:1px solid var(--line); border-radius:6px; overflow:hidden; }}
.card > a img {{ display:block; width:100%; height:auto; background:#000; }}
.meta {{ padding:12px 14px 14px; display:grid; grid-template-columns:1fr auto; gap:4px 12px; }}
h2 {{ grid-column:1 / -1; font-size:15px; margin:0 0 4px; }}
.num {{ color:var(--muted); font-variant-numeric:tabular-nums; margin-right:4px; }}
dl {{ margin:0; display:grid; grid-template-columns:auto 1fr; gap:2px 10px; font-size:13px; }}
dt {{ color:var(--muted); }} dd {{ margin:0; }}
.curve {{ width:200px; height:auto; grid-row:span 2; }}
.curve .grid {{ stroke:var(--line); stroke-width:.6; }}
.curve .tick {{ fill:var(--muted); font-size:9px; }}
.curve .line {{ fill:none; stroke:var(--ink); stroke-width:1.6; }}
.curve .grey {{ stroke:var(--accent); stroke-dasharray:3 3; stroke-width:1; }}
.neg {{ display:flex; align-items:center; gap:8px; color:var(--muted); font-size:12px; text-decoration:none; }}
.neg img {{ width:84px; height:auto; border:1px solid var(--line); }}
.warn {{ grid-column:1 / -1; margin:4px 0 0; padding-left:18px; color:var(--accent); font-size:12px; }}
.time {{ grid-column:1 / -1; margin:0; color:var(--muted); font-size:11px; }}
</style></head><body>
<header>
<h1>Emulation variations</h1>
<p class="note">Input: <code>{input}</code> · {w}×{h} px over {fw:.1} mm of film ({pitch:.1} µm per pixel).</p>
<p class="note"><strong>Illustrative parameters.</strong> The film and developer values are placeholders chosen to exercise the engine, not fitted to manufacturer data. Product-like names indicate the intended target only. The curves show the model's density against log exposure (lux·s); the dashed line marks the metered mid-grey. Each print is exposed so mid-grey prints at the same density, as a printer would.</p>
</header>
<main>
{cards}</main>
</body></html>
"#,
        input = escape(input),
        w = geometry.width_px(),
        h = geometry.height_px(),
        fw = geometry.width_px() as f64 * geometry.pixel_pitch_um() / 1000.0,
        pitch = geometry.pixel_pitch_um(),
    );
    std::fs::write(path, html)
}
