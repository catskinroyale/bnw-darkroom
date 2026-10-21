#!/usr/bin/env python3
"""Curate data/development/times.csv from the raw Massive Dev Chart extract.

Input:  data/development/mdc-times.csv  (raw rows, several per EI, with notes)
Output: data/development/times.csv      (one time per film/developer/dilution/EI)

Rules, in order:
  1. 35 mm rows only.
  2. Drop rotary-processor (Jobo) times and stand or semi-stand rows: the engine's times assume intermittent agitation,
     and stand development is selected separately with an explicit time.
  3. For Tri-X, prefer rows not marked "previous version of this film" when both exist.
  4. Normalise each time to 20 °C with the developer's temperature coefficient
     (t20 = t_T * exp(alpha * (T - 20))); see design 0001 §3.4.
  5. Several rows at one EI: take the median.
  6. Times must not decrease as EI rises. Walking out from the box-speed row (or the
     row nearest it), any pull longer than the next faster EI, or push shorter than the
     next slower EI, is dropped and reported: such inversions are community-data noise.
Usage: python3 -I tools/curate_times.py
"""

import csv
import math
import statistics
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RAW = ROOT / "data/development/mdc-times.csv"
OUT = ROOT / "data/development/times.csv"

# Per-developer temperature coefficients (per °C). D-76 and HC-110 B are fitted from
# Kodak F-4017 tables (docs/research/source-inventory.md); the rest are Level-1 values.
ALPHA = {
    ("d76", "stock"): 0.087,
    ("d76", "1+1"): 0.055,
    ("d96", None): 0.087,
    ("hc110", None): 0.098,
    ("rodinal", None): 0.09,
    ("fx55", None): 0.09,  # Level 1: no published time/temperature data
}


# The EI push and pull are measured from (matches bw-negative's illustrative films).
BOX_EI = {"kentmere-100": 100, "kentmere-200": 200, "kentmere-400": 400, "tmax-100": 100, "tmax-400": 400, "tri-x": 400, "hp5": 400, "fp4": 125, "delta-100": 100, "delta-400": 400,
          "delta-3200": 3200, "double-x": 250}


def monotone_from_box(film: str, ladder: list[tuple[float, float]]) -> tuple[list, list]:
    """Keep rows whose time does not decrease with EI, anchored at box speed."""
    if not ladder:
        return [], []
    box = BOX_EI.get(film, ladder[0][0])
    anchor = min(range(len(ladder)), key=lambda i: abs(math.log2(ladder[i][0] / box)))
    kept, dropped = [ladder[anchor]], []
    for ei, t in reversed(ladder[:anchor]):  # pulls, slowest last
        if t <= kept[0][1]:
            kept.insert(0, (ei, t))
        else:
            dropped.append((ei, t))
    for ei, t in ladder[anchor + 1:]:  # pushes
        if t >= kept[-1][1]:
            kept.append((ei, t))
        else:
            dropped.append((ei, t))
    return kept, dropped


def alpha(developer: str, dilution: str) -> float:
    return ALPHA.get((developer, dilution), ALPHA.get((developer, None), 0.09))


def main() -> int:
    lines = [l for l in RAW.read_text(encoding="utf-8").splitlines() if not l.startswith("#")]
    rows = list(csv.DictReader(lines))
    groups: dict[tuple, list[dict]] = defaultdict(list)
    dropped = defaultdict(int)
    for r in rows:
        notes = r["notes"].lower()
        if r["format"] != "35mm":
            dropped["not 35 mm"] += 1
            continue
        if "stand" in notes:
            dropped["stand / semi-stand"] += 1
            continue
        if "rotary" in notes or "jobo" in notes:
            dropped["rotary processor"] += 1
            continue
        key = (r["film"], r["developer"], r["dilution"], float(r["ei"]))
        groups[key].append(r)

    medians: dict[tuple, list] = defaultdict(list)
    for (film, dev, dil, ei), rs in sorted(groups.items()):
        current = [r for r in rs if "previous version" not in r["notes"].lower()]
        if current and len(current) < len(rs):
            dropped["older film version (newer rows exist)"] += len(rs) - len(current)
            rs = current
        times = [
            float(r["time_min"]) * math.exp(alpha(dev, dil) * (float(r["temp_c"]) - 20.0))
            for r in rs
        ]
        t = statistics.median(times)
        source = f"MDC median of {len(times)}" if len(times) > 1 else "MDC"
        medians[(film, dev, dil)].append((ei, t, source))

    out = []
    inversions = []
    for (film, dev, dil), ladder in sorted(medians.items()):
        ladder.sort()
        kept, bad = monotone_from_box(film, [(ei, t) for ei, t, _ in ladder])
        sources = {ei: src for ei, _, src in ladder}
        inversions += [f"{film} {dev} {dil} EI {ei:g}: {t:.2f} min" for ei, t in bad]
        for ei, t in kept:
            out.append((film, dev, dil, f"{ei:g}", f"{t:.2f}", "20", sources[ei]))

    with OUT.open("w", encoding="utf-8", newline="") as f:
        f.write("# Development times used by the engine: 35 mm, intermittent agitation, normalised to 20 C.\n")
        f.write("# Curated by tools/curate_times.py from data/development/mdc-times.csv (Massive Dev Chart via\n")
        f.write("# Internet Archive snapshots 2025; see that file for per-row sources and notes). Whole-stop EIs only.\n")
        f.write("film,developer,dilution,ei,time_min,temp_c,source\n")
        for row in out:
            f.write(",".join(row) + "\n")

    print(f"{len(rows)} raw rows -> {len(out)} curated rows")
    for reason, n in sorted(dropped.items()):
        print(f"  dropped {n:3d}: {reason}")
    print(f"  dropped {len(inversions):3d}: time inverted against neighbouring EIs")
    for line in inversions:
        print(f"      {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
