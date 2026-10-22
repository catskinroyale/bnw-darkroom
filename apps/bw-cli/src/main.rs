//! `bw`: command-line front end.
//!
//! `bw variations` renders a set of film/developer/processing variations of one image
//! and writes prints, negatives and an HTML contact sheet with each variation's model
//! characteristic curve.

mod chart;
mod report;

use std::path::PathBuf;

use bw_core::FilmGeometry;
use bw_darkroom::{PrintSettings, print};
use bw_developer::{Agitation, ProcessingConditions};
use bw_negative::{ExposureSettings, Metering, RenderOptions, develop, illustrative, sensitometry};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "bw", about = "Black-and-white photographic emulation engine")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render a standard set of variations of one image.
    Variations {
        /// Input image (PNG/JPEG/TIFF), or `chart` for the built-in synthetic test scene.
        #[arg(long)]
        input: String,
        /// Output directory.
        #[arg(long)]
        out: PathBuf,
        /// Width of film the image spans, in mm (36 = full 35 mm frame; smaller = enlarged crop).
        #[arg(long, default_value_t = 36.0)]
        frame_mm: f64,
        #[arg(long, value_enum, default_value_t = Encoding::Auto)]
        encoding: Encoding,
        /// Scene value treated as metered mid-grey.
        #[arg(long, default_value_t = 0.18)]
        grey: f32,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
}

/// How input pixel values relate to scene light.
#[derive(Clone, Copy, ValueEnum)]
pub enum Encoding {
    /// 8-bit files are sRGB-encoded; 16-bit and float files are linear.
    Auto,
    Srgb,
    Linear,
}

pub struct Variation {
    pub label: &'static str,
    pub film: &'static str,
    pub developer: &'static str,
    pub exposure_index: f64,
    pub process: ProcessingConditions,
    pub grade: f64,
}

fn standard_variations() -> Vec<Variation> {
    let normal = |t| ProcessingConditions {
        time_min: t,
        temperature_c: 20.0,
        agitation: Agitation::Intermittent { interval_s: 30.0 },
    };
    let v = |label, film, developer, exposure_index, process, grade| Variation {
        label,
        film,
        developer,
        exposure_index,
        process,
        grade,
    };
    vec![
        v("Reference", "tri-x", "d76", 400.0, normal(6.75), 2.0),
        v(
            "Developer: HC-110 B",
            "tri-x",
            "hc110",
            400.0,
            normal(3.75),
            2.0,
        ),
        v(
            "Developer: Rodinal 1+50",
            "tri-x",
            "rodinal-50",
            400.0,
            normal(14.0),
            2.0,
        ),
        v("Film: HP5-like", "hp5", "d76", 400.0, normal(7.5), 2.0),
        v("Film: FP4-like", "fp4", "d76", 125.0, normal(8.5), 2.0),
        v(
            "Film: Double-X-like",
            "double-x",
            "d76",
            250.0,
            normal(7.0),
            2.0,
        ),
        v(
            "EI 1600, normal development",
            "tri-x",
            "d76",
            1600.0,
            normal(6.75),
            2.0,
        ),
        v(
            "EI 1600, pushed (11 min)",
            "tri-x",
            "d76",
            1600.0,
            normal(11.0),
            2.0,
        ),
        v(
            "EI 200, pulled (5 min)",
            "tri-x",
            "d76",
            200.0,
            normal(5.0),
            2.0,
        ),
        v(
            "Rodinal 1+100 stand, 60 min",
            "tri-x",
            "rodinal-100",
            400.0,
            ProcessingConditions {
                time_min: 60.0,
                temperature_c: 20.0,
                agitation: Agitation::Stand,
            },
            2.0,
        ),
        v("Print on grade 4", "tri-x", "d76", 400.0, normal(6.75), 4.0),
        v(
            "Developed at 24 °C, same time",
            "tri-x",
            "d76",
            400.0,
            ProcessingConditions {
                temperature_c: 24.0,
                ..normal(6.75)
            },
            2.0,
        ),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Command::Variations {
            input,
            out,
            frame_mm,
            encoding,
            grey,
            seed,
        } => {
            std::fs::create_dir_all(&out)?;
            let scene = if input == "chart" {
                chart::synthetic_scene(2400, 1600)
            } else {
                bw_io::load_scene(
                    std::path::Path::new(&input),
                    match encoding {
                        Encoding::Auto => bw_io::Encoding::Auto,
                        Encoding::Srgb => bw_io::Encoding::Srgb,
                        Encoding::Linear => bw_io::Encoding::Linear,
                    },
                )
                .map_err(|e| -> Box<dyn std::error::Error> { e })?
            };
            let (w, h) = scene.dimensions();
            let geometry = FilmGeometry::from_frame_width(frame_mm, w, h)?;
            let mut entries = Vec::new();

            for (i, var) in standard_variations().into_iter().enumerate() {
                let (film, calibration) = illustrative::combination(var.film, var.developer)
                    .ok_or_else(|| {
                        format!(
                            "no illustrative calibration for {}/{}",
                            var.film, var.developer
                        )
                    })?;
                let exposure = ExposureSettings {
                    exposure_index: var.exposure_index,
                    metering: Metering::Anchor { grey },
                    ev_compensation: 0.0,
                    flare_fraction: 0.0,
                };
                let options = RenderOptions {
                    seed,
                    geometry,
                    grain: Default::default(),
                };
                let start = std::time::Instant::now();
                let negative = develop(
                    &scene,
                    &exposure,
                    &film,
                    &var.process,
                    &calibration,
                    &options,
                );
                let settings = PrintSettings {
                    grade: var.grade,
                    ..Default::default()
                };
                let positive = print(&negative, &settings);
                let elapsed = start.elapsed();

                let stem = format!("{:02}", i + 1);
                bw_io::save_srgb_png(&positive, &out.join(format!("{stem}-print.png")))?;
                bw_io::save_negative_png(&negative, &out.join(format!("{stem}-negative.png")))?;
                bw_io::save_linear_tiff16(&positive, &out.join(format!("{stem}-print.tif")))?;

                let state = negative.state;
                entries.push(report::Entry {
                    stem,
                    label: var.label.to_string(),
                    film_name: film.name.clone(),
                    developer: format!("{} {}", calibration.developer_id, calibration.dilution),
                    exposure_index: var.exposure_index,
                    process: var.process,
                    grade: var.grade,
                    iso: sensitometry::iso_speed(&calibration, &state),
                    curve: sensitometry::characteristic_curve(
                        &calibration,
                        &state,
                        film.base_density,
                        (-4.5, 0.5),
                        101,
                    ),
                    grey_log_h: (bw_negative::METER_QK / var.exposure_index).log10(),
                    warnings: negative.warnings.clone(),
                    millis: elapsed.as_millis(),
                });
                eprintln!(
                    "{:>2}  {:<32} {:>5} ms",
                    i + 1,
                    var.label,
                    elapsed.as_millis()
                );
            }
            report::write_html(&out.join("index.html"), &input, &geometry, &entries)?;
            eprintln!("wrote {}", out.join("index.html").display());
        }
    }
    Ok(())
}
