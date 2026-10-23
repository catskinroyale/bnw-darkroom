# Notes for Claude Code

Working notes for AI-assisted sessions on BW Darkroom. Read alongside `README.md`, the
user guide (`docs/USER_GUIDE.md`), design 0001 (`docs/design/0001-negative-simulation-model.md`)
and the decision log (`docs/decisions.md`; record product and scope decisions there).

## What this is

A Rust black-and-white film emulation engine and a macOS desktop app (`bw-darkroom`).
Film → developer → negative → print are separate, physically motivated stages. Film and
developer parameters are **illustrative** (`crates/bw-negative/src/illustrative.rs`): say
so, never invent data, and label approximations. Photographer presets are documented
interpretations (`docs/research/photographer-presets.md`).

## Layout

| Path | What |
|---|---|
| `crates/bw-core` (MIT) | Planes, geometry, curves, hash noise, blur |
| `crates/bw-negative` | Exposure, development, adjacency, grain; `times.rs` reads `data/development/times.csv` |
| `crates/bw-darkroom` | Printing (paper curve, black-point print exposure) |
| `crates/bw-profiles` | `Style` (film + developer + process + print + grain), `render`, `render_region` |
| `crates/bw-raw` | Raw input via LightCraft's `lightcraft-raw` (pinned git rev) |
| `apps/bw-desktop` | egui app: `app.rs` (state, panels, image strip), `folders.rs` (folder tree) + `browser.rs` (listing, places, packages), `workflow.rs` (Develop sections), `image_panel.rs`, `widgets.rs` + `theme.rs` (LightCraft-derived UI), `updates.rs`, `worker.rs` (render thread) |
| `apps/bw-cli` | `bw variations` |

Everything except `bw-core` is Apache-2.0. `theme.rs`, `widgets.rs` and `folders.rs` adapt LightCraft code
(MIT, credited in each file and the About window); Inter is OFL. Never use ArtCraft branding.

## Commands

```sh
source ~/.cargo/env                      # cargo is not on PATH by default
cargo fmt && cargo clippy --release --all-targets -- -D warnings
cargo test --release --workspace         # ~90 tests; keep clippy and tests clean
cargo test --release -p bw-desktop -- --ignored live   # real GitHub update check (network)
cargo run --release -p bw-desktop -- <folder-or-image>
python3 -I tools/gen_licenses.py         # after dependency changes (About window list)
```

`gh` is in `/opt/homebrew/bin` (`export PATH="/opt/homebrew/bin:$PATH"`).

## Model decisions to keep

- **Push/pull change development only.** Styles expose the film at box speed
  (`Style::exposure_ei`); `exposure_index` picks the published time. An underexposed push
  is negative exposure compensation. See design 0001 D2 application note.
- **Prints hold the black point** (`PrintExposure::BaseAt`), calibrated so normal
  development prints as mid-grey anchoring did. Pushes keep blacks, open shadows slightly,
  brighten highlights (`crates/bw-profiles/tests/push.rs`).
- **Grain** amplitude comes from published granularity; `GrainAdjust` (size, amount,
  roughness) defaults to the film's own grain. Grain noise is keyed on **full-frame pixel
  coordinates** (`RenderOptions::grain_origin`), so 1:1 detail crops match the export.
- Development times: whole-stop EIs only, from the Massive Dev Chart extract.

## UI conventions (egui 0.37)

- `App::ui(ui)` and `egui::Panel::left/right/top/bottom(..).show(ui)`; `Id::unique`
  (`Id::new` and `ui.id()` are deprecated).
- A widget wider than its row inside a resizable panel makes the panel grow forever: lay
  overlays out with zero-size `ui.interact` + painting.
- For "pointer inside a container" use `Response::container_contains_pointer`; plain
  `contains_pointer` is false over child widgets (this broke the style menu once).
- Right side: tool strip (`Module::Develop` / `Module::Styles`) + tools panel. Advanced
  controls go in `widgets::flyout` rows inside `widgets::section`s, not separate views.
- Every control gets a one- or two-line hover explanation.

## Screenshot tests (checking UI without the user's screen)

`BW_DARKROOM_SCREENSHOT=<png>` renders, saves a screenshot and quits; test runs never save
settings or touch the network. Options (env vars, prefix `BW_DARKROOM_SCREENSHOT_`):
`FRESH=1` (default layout), `THEME=dark|light`, `VIEW=develop|styles`, `EDIT=1` (open all
sections), `CLOSE=image,film`, `DETAIL=x,y,zoom`, `GRAIN=amount,size,roughness`,
`THEN_PUSH=stops`, `SHOW=bitmaps,jpegs`, `STYLEMENU=index`, `ABOUT=1`,
`EXPORT=1`, `UPDATE=version` + `UPDATE_DIALOG=1` (fake update), `WAIT=seconds`, and
`CLICKS=t:x,y;t:x,y>x2,y2` (scripted clicks and drags through egui input; macOS blocks
synthetic OS clicks). Run each shot with a time limit and retry: a window opened behind
others may never repaint. Scripted clicks can rename/delete real user styles
(`~/Library/Application Support/bw-darkroom/styles.json`): only open dialogs, never confirm.

## Git, changelog and releases

- Commit and push only when the user asks.
- **Every push needs a CHANGELOG.md `[Unreleased]` entry.** A hook
  (`.claude/hooks/require-changelog.sh`) blocks `git push` otherwise; edit the changelog in
  a separate command before the push command.
- End commit messages with the attribution line the session provides.
- Release (version only in `[workspace.package]` of `Cargo.toml`):
  ```sh
  tools/release.sh patch|minor|major      # bumps version, dates the changelog, tags
  git push && git push origin vX.Y.Z
  tools/package-macos.sh                  # → dist/BW-Darkroom-X.Y.Z-arm64.dmg
  gh release create vX.Y.Z dist/BW-Darkroom-X.Y.Z-arm64.dmg --title "BW Darkroom X.Y.Z" \
    --notes-file <changelog section> --latest
  ```
  Before 1.0: minor = features or behaviour changes, patch = fixes.
- Public images (README, releases) use only the author's own photos.
