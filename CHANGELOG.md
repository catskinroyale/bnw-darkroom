# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- **Camera raw support** (`crates/bw-raw`): Nikon NEF, DNG (including Leica M Monochrom)
  and Sony ARW, decoded with LightCraft's `lightcraft-raw` (MIT OR Apache-2.0, pinned to
  commit `d999e38`). AHD demosaic, DNG colour model, highlight reconstruction and
  embedded-preview thumbnails.
- Colour for NEF and ARW from matrices fitted to measured camera spectral sensitivities
  (rawtoaces-data, Academy Software Foundation), including the Nikon Z f.
- **Image panel** in the desktop app for per-image adjustments before development:
  exposure (with an averaging-meter Auto), white balance (as shot, auto, presets, custom
  temperature and tint), highlight reconstruct/clip, rotation and reset. Adjustments are
  stored non-destructively in a sidecar next to each image (`<file>.bw.json`).
- **Export dialog** (⌘E): JPEG with quality setting, 16-bit TIFF, 16-bit PNG or the
  original's format; full size or a long-edge resize; file-name templates with tokens
  (`{name}`, `{style}`, `{film}`, `{developer}`, `{ei}`, `{date}`, `{time}`, `{n}`);
  incremental numbering with zero padding; add-suffix or overwrite on conflict; export
  folder; optional recipe. Settings persist between sessions.
- Export recipes record the image adjustments and camera details.
- `bw-io`: JPEG and 16-bit PNG writers, Lanczos long-edge resize, orientation transforms.

### Changed
- Export recipes are named after the full file (`print.jpg.json`) so a JPEG and a TIFF
  with the same name keep separate recipes.
- The export folder moved from the top bar into the export dialog.

### Known limitations
- Nikon High Efficiency NEFs (HE / HE★, JPEG XS) cannot be decoded; shoot lossless
  compressed NEF.
- `{date}` and `{time}` in export names are UTC.
- Film and developer parameters are still illustrative, not fitted to sensitometry.

## [0.1.0] - 2026-10-20

### Added
- Design 0001 for the negative simulation model, with verified source research.
- Engine crates: `bw-core` (planes, film geometry, monotone curves, deterministic noise,
  blur), `bw-film`, `bw-developer`, `bw-negative` (exposure, development-growth curves,
  adjacency, Poisson/Selwyn grain), `bw-darkroom`, `bw-profiles`, `bw-io`.
- Films: Tri-X 400, T-Max 100/400, HP5 Plus, FP4 Plus, Delta 100/400/3200, Double-X 5222.
  Developers: D-76 (stock, 1+1), D-96, HC-110 (B, E, H, G), Rodinal (1+25, 1+50, 1+100),
  FX-55 (1+9).
- Development-time table curated from Massive Dev Chart data (whole-stop EIs), with
  `tools/curate_times.py`.
- `bw` CLI with a variations contact sheet.
- BW Darkroom desktop app: folder browser, preview, style gallery, simple and detailed
  workflow views with hover help, push/pull in stops, grain from format and enlargement,
  1:1 detail view, full-resolution TIFF export.

[Unreleased]: https://github.com/catskinroyale/bnw-darkroom/compare/49d00c7...HEAD
[0.1.0]: https://github.com/catskinroyale/bnw-darkroom/commit/49d00c7
