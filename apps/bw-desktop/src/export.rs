//! Export settings and output file naming.
//!
//! File names come from a template with tokens:
//! `{name}` original file name without extension, `{style}`, `{film}`, `{developer}`,
//! `{ei}`, `{date}` (YYYY-MM-DD), `{time}` (HHMMSS) and `{n}` (sequence number,
//! zero-padded to the chosen number of digits).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    /// The source's format: JPEG → JPEG, PNG → PNG, TIFF or raw → 16-bit TIFF.
    SameAsOriginal,
    Tiff16,
    Jpeg,
    Png16,
}

impl Format {
    pub const ALL: [Format; 4] = [
        Format::SameAsOriginal,
        Format::Tiff16,
        Format::Jpeg,
        Format::Png16,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Format::SameAsOriginal => "Same as original",
            Format::Tiff16 => "TIFF 16-bit",
            Format::Jpeg => "JPEG",
            Format::Png16 => "PNG 16-bit",
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            Format::SameAsOriginal => {
                "Match the source file: JPEG stays JPEG, PNG stays PNG; TIFF and raw files become 16-bit TIFF."
            }
            Format::Tiff16 => {
                "Lossless 16-bit greyscale TIFF: the most tonal headroom for further editing or printing."
            }
            Format::Jpeg => "Compressed 8-bit JPEG: small files for sharing and the web.",
            Format::Png16 => "Lossless 16-bit greyscale PNG.",
        }
    }

    /// The concrete format for a source file.
    pub fn resolve(self, source: &Path) -> Format {
        if self != Format::SameAsOriginal {
            return self;
        }
        match source
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("jpg" | "jpeg") => Format::Jpeg,
            Some("png") => Format::Png16,
            _ => Format::Tiff16,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Format::Jpeg => "jpg",
            Format::Png16 => "png",
            Format::Tiff16 | Format::SameAsOriginal => "tif",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Conflict {
    /// Keep existing files; add `-2`, `-3`, … to the new name.
    AddSuffix,
    Overwrite,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportSettings {
    pub format: Format,
    /// JPEG quality 1–100.
    pub jpeg_quality: u8,
    pub template: String,
    /// Next value of `{n}`.
    pub next_number: u32,
    /// Zero padding for `{n}`.
    pub digits: u8,
    /// `None`: a `bw-exports` folder next to each image.
    pub folder: Option<PathBuf>,
    pub conflict: Conflict,
    /// `None`: full resolution; otherwise the long edge in pixels.
    pub long_edge: Option<u32>,
    /// Write the JSON recipe next to the image.
    pub write_recipe: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            format: Format::SameAsOriginal,
            jpeg_quality: 90,
            template: "{name}__{style}".into(),
            next_number: 1,
            digits: 3,
            folder: None,
            conflict: Conflict::AddSuffix,
            long_edge: None,
            write_recipe: true,
        }
    }
}

/// Values the template tokens expand to for one export.
pub struct NameContext<'a> {
    pub source: &'a Path,
    pub style: &'a str,
    pub film: &'a str,
    pub developer: &'a str,
    pub ei: f64,
    /// Seconds since the Unix epoch (local time zone is not applied: UTC).
    pub unix_time: u64,
}

pub const TOKENS: [(&str, &str); 8] = [
    ("{name}", "Original file name without its extension"),
    ("{style}", "Style name"),
    ("{film}", "Film"),
    ("{developer}", "Developer and dilution"),
    ("{ei}", "Exposure index the film was rated at"),
    ("{date}", "Export date, YYYY-MM-DD (UTC)"),
    ("{time}", "Export time, HHMMSS (UTC)"),
    (
        "{n}",
        "Sequence number, zero-padded; increases after each export",
    ),
];

/// Lower-case, keep letters, digits, '-', '+' and '.', turn everything else into '-'.
pub fn slug(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '+' | '.') {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

/// Characters not allowed in file names on macOS, Windows or Linux become '_'.
fn sanitize_file_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').to_string();
    if trimmed.is_empty() {
        "export".into()
    } else {
        trimmed
    }
}

/// Civil date and time (UTC) from Unix seconds (Howard Hinnant's days-from-civil inverse).
fn utc_parts(unix: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        y,
        m,
        d,
        (secs / 3600) as u32,
        (secs / 60 % 60) as u32,
        (secs % 60) as u32,
    )
}

impl ExportSettings {
    /// Whether the template uses the sequence number.
    pub fn uses_number(&self) -> bool {
        self.template.contains("{n}")
    }

    /// File name (with extension) for one export.
    pub fn file_name(&self, ctx: &NameContext) -> String {
        let (y, mo, d, h, mi, s) = utc_parts(ctx.unix_time);
        let stem = ctx
            .source
            .file_stem()
            .map_or("image".into(), |s| s.to_string_lossy().into_owned());
        let name = self
            .template
            .replace("{name}", &stem)
            .replace("{style}", &slug(ctx.style))
            .replace("{film}", &slug(ctx.film))
            .replace("{developer}", &slug(ctx.developer))
            .replace("{ei}", &format!("{:.0}", ctx.ei))
            .replace("{date}", &format!("{y:04}-{mo:02}-{d:02}"))
            .replace("{time}", &format!("{h:02}{mi:02}{s:02}"))
            .replace(
                "{n}",
                &format!(
                    "{:0width$}",
                    self.next_number,
                    width = usize::from(self.digits)
                ),
            );
        format!(
            "{}.{}",
            sanitize_file_name(&name),
            self.format.resolve(ctx.source).extension()
        )
    }

    /// Folder an image's export goes to.
    pub fn folder_for(&self, source: &Path) -> PathBuf {
        self.folder.clone().unwrap_or_else(|| {
            source
                .parent()
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
                .join("bw-exports")
        })
    }
}

/// Apply the conflict policy: the path as is, or the first free `name-2.ext`, `name-3.ext`, ….
pub fn resolve_conflict(path: PathBuf, conflict: Conflict) -> PathBuf {
    if conflict == Conflict::Overwrite || !path.exists() {
        return path;
    }
    let stem = path
        .file_stem()
        .map_or(String::new(), |s| s.to_string_lossy().into_owned());
    let ext = path
        .extension()
        .map_or(String::new(), |e| format!(".{}", e.to_string_lossy()));
    (2..)
        .map(|i| path.with_file_name(format!("{stem}-{i}{ext}")))
        .find(|p| !p.exists())
        .expect("unbounded search")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(source: &Path) -> NameContext<'_> {
        NameContext {
            source,
            style: "Tri-X · D-76",
            film: "Tri-X-like 400",
            developer: "D-76 Stock",
            ei: 1600.0,
            unix_time: 1_792_454_400, // 2026-10-20 00:00:00 UTC
        }
    }

    #[test]
    fn template_tokens_expand() {
        let s = ExportSettings {
            template: "{name}_{film}_{developer}_EI{ei}_{date}_{n}".into(),
            next_number: 7,
            digits: 4,
            format: Format::Jpeg,
            ..Default::default()
        };
        let src = PathBuf::from("/p/DSC_0040.NEF");
        assert_eq!(
            s.file_name(&ctx(&src)),
            "DSC_0040_tri-x-like-400_d-76-stock_EI1600_2026-10-20_0007.jpg"
        );
    }

    #[test]
    fn same_as_original_follows_the_source() {
        let s = ExportSettings::default();
        assert_eq!(s.format.resolve(Path::new("a.JPG")), Format::Jpeg);
        assert_eq!(s.format.resolve(Path::new("a.png")), Format::Png16);
        assert_eq!(s.format.resolve(Path::new("a.nef")), Format::Tiff16);
        assert!(s.file_name(&ctx(Path::new("/p/x.jpeg"))).ends_with(".jpg"));
    }

    #[test]
    fn unsafe_characters_are_replaced() {
        let s = ExportSettings {
            template: "a/b:c*?".into(),
            ..Default::default()
        };
        assert_eq!(s.file_name(&ctx(Path::new("x.tif"))), "a_b_c__.tif");
        let empty = ExportSettings {
            template: "  ".into(),
            ..Default::default()
        };
        assert_eq!(empty.file_name(&ctx(Path::new("x.tif"))), "export.tif");
    }

    #[test]
    fn utc_date_conversion() {
        assert_eq!(utc_parts(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(utc_parts(951_782_400), (2000, 2, 29, 0, 0, 0));
        assert_eq!(utc_parts(1_792_454_400 + 3_723), (2026, 10, 20, 1, 2, 3));
    }

    #[test]
    fn conflicts_add_suffix_or_overwrite() {
        let dir = std::env::temp_dir().join(format!("bw-conflict-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.jpg");
        assert_eq!(resolve_conflict(p.clone(), Conflict::AddSuffix), p);
        std::fs::write(&p, b"x").unwrap();
        assert_eq!(
            resolve_conflict(p.clone(), Conflict::AddSuffix),
            dir.join("a-2.jpg")
        );
        assert_eq!(resolve_conflict(p.clone(), Conflict::Overwrite), p);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
