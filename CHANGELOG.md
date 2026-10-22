# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Fixed
- The style "…" menu's items can be clicked again. A press on any item except the one
  nearest the "…" counted as a click outside the menu, so the menu closed before the
  click finished.

## [0.6.0] - 2026-10-22

### Added
- **Grain controls** in the Simple and Detailed views and the style editor:
  - **Size** changes how coarse the grain clumps are, with strength kept the same.
  - **Amount** changes the grain's strength (0 removes it).
  - **Roughness** goes from softer and smoother (below 0) to grittier, with more
    isolated dark and bright specks (above 0).
  - **Randomise** picks a new pattern and random settings within a usable range;
    **Film's grain** resets; **New pattern** is unchanged.
  - The defaults are the film's own grain, and the grain readout says when it has been
    adjusted. Styles saved without adjustments are unchanged.

### Changed
- **The Detail window keeps up while you adjust.** Its crop now renders before the full
  preview, so any workflow or image change shows there first. A spinner shows while an
  update is on its way. The window now opens over the left of the image, leaving the
  Styles and Workflow panels free.

### Fixed
- Switching the Detail window to 200% no longer doubles its width and leaves the
  magnified crop on the left. The window also returns to its normal size if an earlier
  version saved it oversized.

## [0.5.0] - 2026-10-21

### Added
- **File-type filter in the file browser.** Raw files (NEF, DNG, ARW) are always listed,
  and are all that's listed by default. **TIFF/PNG** and **JPEG** toggles in the panel
  header add lossless bitmaps and compressed JPEGs. The header counts the hidden files.
  A folder with no raw files offers **Show them**. Arrow-key stepping skips hidden files,
  and the toggles are remembered.

### Changed
- **Push and pull now change development only.** Your photo is already correctly
  exposed, so the film is exposed at box speed. Push or pull picks the published time for
  the pushed or pulled EI: Zone System N+ / N− development. Exposure compensation in
  Detailed view still gives an underexposed push. Detailed view's "Exposure index" is
  now labelled "Develop for EI".
- **The print holds its black point.** Prints used to place mid-grey at the same density
  for every negative, so a push turned existing blacks grey (Tri-X +2, zone 0 went from
  1.86 to 1.51). The enlarger exposure now keeps clear film printing at the black of the
  normally developed negative, as a printer setting "minimum time for maximum black"
  would. Normal development prints exactly as before. A push keeps blacks, opens the
  shadows slightly and brightens the highlights (HP5 HC-110 B +2: zone III goes from
  1.46 to 1.25 and zone VII from 0.17 to 0.03, with zone 0 unchanged). A pull does the
  reverse. Styles saved with a pushed EI change look accordingly.

## [0.4.1] - 2026-10-21

### Changed
- **Compact file browser.** The left panel is now a one-line-per-file list (default
  width 170 px, down from about 260 px) instead of a list of thumbnails:
  - Long names are shortened in the middle so the start and the end, including the
    number, stay visible (for example "Reykjav…042.NEF"), as the Mac Finder and Apple's
    interface guidelines recommend. Hover a row to see the full name.
  - Each row has a short type tag on the right (NEF, DNG, JPG, PNG, TIF), and folders
    have a "Folder" tag.
  - Hovering a file shows a larger thumbnail beside the list.
  - A **Thumbs** toggle in the panel header brings back small inline thumbnails. The
    setting is remembered.
  - The header shows the folder name (full path on hover) and the number of images and
    folders.

### Fixed
- In Detailed view, showing the white-balance sliders no longer switches an image from
  "As shot" to Custom or marks its adjustments as edited. It changes only when you move
  a slider.

## [0.4.0] - 2026-10-21

### Added
- **Style menu (…)** on every row of the Styles list, and on right-click of a gallery
  tile: **Edit…** (an editor window with every setting and a live preview), **Duplicate**,
  **Rename…**, **Export…** (a `.json` file), **Move up / Move down** (your own styles) and
  **Delete…** (with confirmation). Built-in styles are read-only: rename, move and delete
  are disabled with an explanation; duplicate one to change it.
- **Import…** in the Styles section adds styles from exported `.json` files (one style or a
  list), renaming any that clash.

### Fixed
- Screenshot test runs no longer save panel sizes and window positions into the user's
  settings.

## [0.3.1] - 2026-10-21

### Added
- Detail view zoom: **100%** (one image pixel per screen pixel, exact on Retina displays)
  and **200%** (each image pixel as a crisp 2×2 block for judging grain).
- The main image shows an outline of exactly the area the detail view displays.

### Fixed
- The detail view was not consistently centred on the clicked point: clicks were measured
  against the whole preview panel rather than the drawn image, so the error varied with the
  image's aspect ratio and the panel size. Clicks outside the image are now ignored.

### Changed
- The detail window opens beside the image instead of over it.

## [0.3.0] - 2026-10-21

### Added
- **Kentmere Pan 100, 200 and 400** films (illustrative models; Harman publishes no curves
  or granularity), with 85 Massive Dev Chart rows and Harman data-sheet findings
  (`docs/research/kentmere.md`). Kentmere HC-110 rows labelled "1+31" count as dilution B.
- **Photographer presets** as a third Workflow view, **Photographers**: Don McCullin,
  Ansel Adams, Bill Brandt, Josef Koudelka and Edward Weston. Each preset is an ordinary
  film/developer/process/print recipe with a note separating documented practice from
  interpretation (`docs/research/photographer-presets.md`); presets also appear as
  built-in styles.
- **Contrast** in the Image panel: scene contrast around mid-grey (−100…+100), applied
  before the film on luminance so colour-to-grey conversion is unchanged; saved in sidecars
  (older sidecars load as 0). Presets can set it.
- **Dark mode**: Auto / Light / Dark toggle in the top bar, remembered between sessions.
- **8×10** film format (203 mm), for contact-print workflows.

### Changed
- The 56 mm format is labelled "6×6 / 645".
- Development-time table: 293 curated rows.

## [0.2.0] - 2026-10-20

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
- **EXIF preserved on export** (JPEG, 16-bit TIFF, 16-bit PNG) from JPEG, PNG, TIFF and
  raw sources (NEF, DNG, ARW), via `kamadak-exif`: camera, lens, exposure, ISO, focal
  length, capture dates, GPS, artist and copyright. Orientation is reset to 1 (rotation is
  applied to the pixels), pixel dimensions follow the export, colour space is sRGB,
  Software names BW Darkroom, and a UserComment records the development recipe.
  MakerNote, thumbnails and source image-structure tags are dropped.
- `bw-io`: JPEG and 16-bit PNG writers, Lanczos long-edge resize, orientation transforms.
- macOS packaging: `tools/package-macos.sh` builds `dist/BW Darkroom.app` and a
  drag-to-Applications DMG (Apple Silicon, ad-hoc signed; `SIGN_IDENTITY` for a Developer
  ID), with an app icon generated by `tools/make_icon.py` and a stripped `dist` build profile.
- **About window** from a native macOS app menu (BW Darkroom ▸ About BW Darkroom, plus
  Services, Hide, Quit and a Window menu, via `muda`): version with git commit, the
  project's licences with full texts, data sources, and a filterable list of all
  third-party components and their licences (`tools/gen_licenses.py`).
- **Semantic versioning:** the workspace version is the single source; builds embed the git
  commit; `tools/release.sh patch|minor|major` bumps the version, dates the changelog
  section, commits and tags `vX.Y.Z`.
- Claude Code project hook (`.claude/hooks/require-changelog.sh`) that blocks `git push`
  unless CHANGELOG.md is updated with the changes being pushed.

### Changed
- **Licensing:** `bw-core` is MIT; every other crate, the desktop app, the CLI and plugins
  are Apache-2.0 (previously all declared "MIT OR Apache-2.0" without licence files).
  Added `LICENSE-MIT` and `LICENSE-APACHE`; both are bundled in the macOS app.
- Export recipes are named after the full file (`print.jpg.json`) so a JPEG and a TIFF
  with the same name keep separate recipes.
- The export folder moved from the top bar into the export dialog.
- README documents macOS packaging and licensing.

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

[Unreleased]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/catskinroyale/bnw-darkroom/compare/49d00c7...v0.2.0
[0.1.0]: https://github.com/catskinroyale/bnw-darkroom/commit/49d00c7
