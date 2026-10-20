# BW Darkroom

A black-and-white photographic emulation engine in Rust, with a command-line tool and a
desktop app. Film, developer and processing are modelled as separate components that
interact through a simulated negative; printing is a separate stage on that negative.

> **Status:** early prototype. Film and developer parameters are **illustrative**
> placeholders, not yet fitted to manufacturer sensitometry. Product names describe the
> intended target only. See [design 0001](docs/design/0001-negative-simulation-model.md).

## Layout

| Path | Contents |
|---|---|
| `crates/bw-core` | Planes, film geometry, monotone curves, deterministic noise, blur |
| `crates/bw-film`, `crates/bw-developer` | Film stocks; developer behaviour and processing conditions |
| `crates/bw-negative` | Negative simulation: exposure, development, adjacency, grain; development-time table |
| `crates/bw-darkroom` | Minimal printing |
| `crates/bw-profiles` | Styles (film + developer + process + print) |
| `crates/bw-io` | Image loading and export |
| `apps/bw-cli` | `bw` command-line tool (`bw variations`) |
| `apps/bw-desktop` | `bw-darkroom` desktop app (egui) |
| `data/development` | Development times: raw Massive Dev Chart extract and the curated table |
| `tools/curate_times.py` | Rebuilds `times.csv` from the raw extract |
| `docs/` | Design and source research |

## Build and run

Requires the Rust toolchain pinned in `rust-toolchain.toml` (installed automatically by rustup).

```sh
cargo test --release                            # all tests
cargo run --release -p bw-desktop -- ~/Pictures # desktop app (optional folder or image)
cargo run --release -p bw-cli -- variations --input chart --out out/chart
```

After editing `data/development/mdc-times.csv`, run `python3 -I tools/curate_times.py`.

## Data sources

Development times come from the Massive Dev Chart (Digital Truth), read from Internet
Archive snapshots; each raw row links its source. Kodak and Ilford data-sheet findings are
in `docs/research/`.
