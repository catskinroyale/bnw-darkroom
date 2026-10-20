//! EXIF metadata carried from the source image to exports.
//!
//! Read with `kamadak-exif` from JPEG, PNG, TIFF and TIFF-based raw files (NEF, DNG,
//! ARW). On export, the photographic record is kept (camera, lens, exposure, dates,
//! GPS, artist, copyright) and everything that describes the source file's own pixels
//! is dropped or rewritten:
//!
//! - Orientation is set to 1: rotation has already been applied to the pixels.
//! - Pixel dimensions are the exported size; colour space is sRGB.
//! - Software names this application.
//! - MakerNote is dropped: vendor blocks hold internal offsets that break when moved.
//! - The thumbnail IFD, interoperability IFD and source image-structure tags are dropped.
//! - A UserComment records the development recipe, unless the source already has one.

use std::path::Path;

use exif::experimental::Writer;
use exif::{Context, Field, In, Rational, Tag, Value};

/// EXIF tag type, re-exported so callers need no direct `kamadak-exif` dependency.
pub use exif::Tag as ExifTag;

/// Source EXIF fields that are safe to carry over (primary image only).
#[derive(Clone, Debug, Default)]
pub struct Exif {
    fields: Vec<Field>,
}

/// What an export writes into the file's metadata.
#[derive(Clone, Debug, Default)]
pub struct ExportMetadata {
    /// The source's EXIF, if it had any.
    pub exif: Option<Exif>,
    /// Value of the Software tag, e.g. "BW Darkroom 0.1.0".
    pub software: String,
    /// Development recipe for the UserComment tag.
    pub comment: Option<String>,
}

/// TIFF-context tags kept from the source; all other TIFF tags describe the source
/// file's structure (strips, compression, resolution, orientation) and are rebuilt.
const KEPT_TIFF_TAGS: [Tag; 6] = [
    Tag::ImageDescription,
    Tag::Make,
    Tag::Model,
    Tag::DateTime,
    Tag::Artist,
    Tag::Copyright,
];

/// EXIF-context tags dropped because they describe the source pixels or are unsafe to move.
const DROPPED_EXIF_TAGS: [Tag; 9] = [
    Tag::MakerNote,
    Tag::PixelXDimension,
    Tag::PixelYDimension,
    Tag::ColorSpace,
    Tag::ComponentsConfiguration,
    Tag::CompressedBitsPerPixel,
    Tag::SubjectArea,
    Tag::SubjectLocation,
    Tag::Gamma,
];

/// Read EXIF from an image file. `None` when the file has none or it can't be parsed.
pub fn read_exif(path: &Path) -> Option<Exif> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let parsed = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let fields: Vec<Field> = parsed
        .fields()
        .filter(|f| f.ifd_num == In::PRIMARY)
        // Cameras often write an empty UserComment; treat it as absent so the recipe
        // comment can take its place.
        .filter(|f| f.tag != Tag::UserComment || !is_blank_comment(&f.value))
        .filter(|f| match f.tag.context() {
            Context::Tiff => KEPT_TIFF_TAGS.contains(&f.tag),
            Context::Exif => !DROPPED_EXIF_TAGS.contains(&f.tag),
            Context::Gps => true,
            // Interop and any context added by future library versions.
            _ => false,
        })
        .cloned()
        .collect();
    (!fields.is_empty()).then_some(Exif { fields })
}

/// A UserComment with nothing but spaces or NULs after its 8-byte character code.
fn is_blank_comment(value: &Value) -> bool {
    match value {
        Value::Undefined(bytes, _) => bytes.iter().skip(8).all(|b| *b == b' ' || *b == 0),
        _ => false,
    }
}

impl Exif {
    /// Build from fields directly (tools and tests). Fields are used as given.
    pub fn from_fields(fields: Vec<Field>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    pub fn get(&self, tag: Tag) -> Option<&Field> {
        self.fields.iter().find(|f| f.tag == tag)
    }
}

fn field(tag: Tag, value: Value) -> Field {
    Field {
        tag,
        ifd_num: In::PRIMARY,
        value,
    }
}

fn ascii(s: &str) -> Value {
    Value::Ascii(vec![s.as_bytes().to_vec()])
}

impl ExportMetadata {
    /// The full field list for an export of `width` × `height` pixels.
    fn fields(&self, width: u32, height: u32) -> Vec<Field> {
        let mut fields: Vec<Field> = self
            .exif
            .as_ref()
            .map(|e| e.fields.clone())
            .unwrap_or_default();
        fields.push(field(Tag::Orientation, Value::Short(vec![1])));
        fields.push(field(Tag::Software, ascii(&self.software)));
        fields.push(field(Tag::ColorSpace, Value::Short(vec![1])));
        fields.push(field(Tag::PixelXDimension, Value::Long(vec![width])));
        fields.push(field(Tag::PixelYDimension, Value::Long(vec![height])));
        let has_comment = fields.iter().any(|f| f.tag == Tag::UserComment);
        if let (Some(comment), false) = (&self.comment, has_comment) {
            // UserComment: 8-byte character-code prefix, then the text.
            let mut bytes = b"ASCII\0\0\0".to_vec();
            // ASCII only: common typographic characters get ASCII stand-ins.
            let text: String = comment
                .chars()
                .filter_map(|c| match c {
                    '·' | '–' | '—' => Some('-'),
                    '°' => None,
                    c if c.is_ascii() => Some(c),
                    _ => None,
                })
                .collect();
            bytes.extend(
                text.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .bytes(),
            );
            fields.push(field(Tag::UserComment, Value::Undefined(bytes, 0)));
        }
        // Exif requires the version tag whenever an Exif IFD is written.
        if !fields.iter().any(|f| f.tag == Tag::ExifVersion) {
            fields.push(field(
                Tag::ExifVersion,
                Value::Undefined(b"0232".to_vec(), 0),
            ));
        }
        fields
    }

    /// TIFF-structured EXIF block (little-endian) for embedding in JPEG and PNG.
    pub fn exif_block(&self, width: u32, height: u32) -> Result<Vec<u8>, String> {
        let fields = self.fields(width, height);
        let mut writer = Writer::new();
        for f in &fields {
            writer.push_field(f);
        }
        let mut out = std::io::Cursor::new(Vec::new());
        writer.write(&mut out, true).map_err(|e| e.to_string())?;
        Ok(out.into_inner())
    }

    /// A complete 16-bit greyscale TIFF (uncompressed, little-endian) with this
    /// metadata in proper EXIF and GPS directories. `samples` are row-major.
    pub fn tiff16(&self, width: u32, height: u32, samples: &[u16]) -> Result<Vec<u8>, String> {
        const ROWS_PER_STRIP: u32 = 64;
        let mut fields = self.fields(width, height);
        let short = |tag, v: u16| field(tag, Value::Short(vec![v]));
        let resolution = Value::Rational(vec![Rational { num: 300, denom: 1 }]);
        fields.extend([
            field(Tag::ImageWidth, Value::Long(vec![width])),
            field(Tag::ImageLength, Value::Long(vec![height])),
            short(Tag::BitsPerSample, 16),
            short(Tag::Compression, 1),
            short(Tag::PhotometricInterpretation, 1), // black is zero
            short(Tag::SamplesPerPixel, 1),
            short(Tag::PlanarConfiguration, 1),
            field(Tag::RowsPerStrip, Value::Long(vec![ROWS_PER_STRIP])),
            field(Tag::XResolution, resolution.clone()),
            field(Tag::YResolution, resolution),
            short(Tag::ResolutionUnit, 2), // inches
        ]);
        let bytes: Vec<u8> = samples.iter().flat_map(|v| v.to_le_bytes()).collect();
        let strip_len = (width * ROWS_PER_STRIP * 2) as usize;
        let strips: Vec<&[u8]> = bytes.chunks(strip_len.max(1)).collect();
        let mut writer = Writer::new();
        for f in &fields {
            writer.push_field(f);
        }
        writer.set_strips(&strips, In::PRIMARY);
        let mut out = std::io::Cursor::new(Vec::new());
        writer.write(&mut out, true).map_err(|e| e.to_string())?;
        Ok(out.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_exif() -> Exif {
        Exif {
            fields: vec![
                field(Tag::Make, ascii("NIKON CORPORATION")),
                field(Tag::Model, ascii("NIKON Z f")),
                field(
                    Tag::ExposureTime,
                    Value::Rational(vec![Rational { num: 1, denom: 60 }]),
                ),
                field(
                    Tag::FNumber,
                    Value::Rational(vec![Rational { num: 56, denom: 10 }]),
                ),
                field(Tag::DateTimeOriginal, ascii("2023:10:18 17:18:20")),
                field(Tag::GPSLatitudeRef, ascii("N")),
            ],
        }
    }

    fn meta() -> ExportMetadata {
        ExportMetadata {
            exif: Some(sample_exif()),
            software: "BW Darkroom test".into(),
            comment: Some("Tri-X D-76".into()),
        }
    }

    fn parse(bytes: Vec<u8>) -> exif::Exif {
        exif::Reader::new().read_raw(bytes).unwrap()
    }

    #[test]
    fn exif_block_carries_record_and_rewrites_structure() {
        let parsed = parse(meta().exif_block(640, 480).unwrap());
        let get = |t| {
            parsed
                .get_field(t, In::PRIMARY)
                .map(|f| f.display_value().to_string())
        };
        assert_eq!(get(Tag::Model).as_deref(), Some("\"NIKON Z f\""));
        assert_eq!(
            get(Tag::DateTimeOriginal).as_deref(),
            Some("2023-10-18 17:18:20")
        );
        assert!(get(Tag::GPSLatitudeRef).is_some());
        assert_eq!(
            get(Tag::Orientation).as_deref(),
            Some("row 0 at top and column 0 at left")
        );
        assert_eq!(get(Tag::PixelXDimension).as_deref(), Some("640"));
        assert!(get(Tag::Software).unwrap().contains("BW Darkroom"));
        let comment = parsed.get_field(Tag::UserComment, In::PRIMARY).unwrap();
        let Value::Undefined(bytes, _) = &comment.value else {
            panic!("UserComment type")
        };
        assert!(bytes.starts_with(b"ASCII\0\0\0") && bytes.ends_with(b"Tri-X D-76"));
    }

    #[test]
    fn tiff16_is_a_readable_image_with_exif() {
        let (w, h) = (130u32, 70u32);
        let samples: Vec<u16> = (0..w * h).map(|i| (i * 7 % 65536) as u16).collect();
        let bytes = meta().tiff16(w, h, &samples).unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        assert_eq!((img.width(), img.height()), (w, h));
        let image::DynamicImage::ImageLuma16(buf) = img else {
            panic!("not 16-bit grey")
        };
        assert_eq!(buf.as_raw()[..], samples[..]);
        let parsed = parse(bytes);
        assert!(parsed.get_field(Tag::Make, In::PRIMARY).is_some());
        assert!(parsed.get_field(Tag::FNumber, In::PRIMARY).is_some());
    }

    #[test]
    fn blank_camera_comments_are_ignored() {
        assert!(is_blank_comment(&Value::Undefined(
            b"ASCII\0\0\0      ".to_vec(),
            0
        )));
        assert!(is_blank_comment(&Value::Undefined(vec![0; 44], 0)));
        assert!(!is_blank_comment(&Value::Undefined(
            b"ASCII\0\0\0hello".to_vec(),
            0
        )));
    }

    #[test]
    fn works_without_source_exif() {
        let m = ExportMetadata {
            exif: None,
            software: "BW Darkroom".into(),
            comment: None,
        };
        let parsed = parse(m.exif_block(10, 10).unwrap());
        assert!(parsed.get_field(Tag::Software, In::PRIMARY).is_some());
        assert!(parsed.get_field(Tag::Make, In::PRIMARY).is_none());
    }
}
