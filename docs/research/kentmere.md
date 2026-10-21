# Kentmere Pan 100 / 200 / 400: development data and manufacturer data

Scope: the Kentmere additions to `data/development/mdc-times.csv` (film ids `kentmere-100`, `kentmere-200`,
`kentmere-400`), and what HARMAN technology's current Kentmere Pan technical data sheets do and do not publish.

Retrieval date for all sources: **2026-10-21**.

Status vocabulary (same as `source-inventory.md` and `tmax-d96-fx55.md`):

- **confirmed**: matches the primary or original document, which was read.
- **unconfirmed**: the primary document could not be read, or no primary source makes the claim. Any evidence
  given is secondary and labelled **[S]**.

Evidence tags: **[P]** primary document read; **[S]** secondary source (retailer, blog, user note).

---

## 1. HARMAN technical data sheets

Sheets read (the PDF text was extracted with PDFKit, and each page was also rendered to an image and checked):

| Film | Sheet version | URL |
|------|---------------|-----|
| Kentmere Pan 100 | Jul 2022, 4 pp. | https://www.ilfordphoto.com/wp/wp-content/uploads/2022/12/KENTMERE-PAN-100_04_07_22.pdf |
| Kentmere Pan 200 | Mar 2025, 4 pp. | https://www.ilfordphoto.com/wp/wp-content/uploads/2025/05/KENTMERE-PAN-200-260325.pdf |
| Kentmere Pan 400 | "B26", 4 pp. (newest found) | https://www.hard2buff.it/wp-content/uploads/2026/05/KENTMERE-PAN400-technical-data-sheet-B26.pdf (reseller copy of the HARMAN sheet) |
| Kentmere Pan 400 | Jul 2022, 4 pp. | https://www.ilfordphoto.com/wp/wp-content/uploads/2022/12/KENTMERE-PAN-400_04_07_22.pdf |

The ilfordphoto.com product pages (https://www.ilfordphoto.com/kentmere-100-35mm, `-200-35mm`, `-400-35mm`) and
the `amfile` download link returned **HTTP 403** to both curl and WebFetch. So it could not be checked which
sheet version the product pages currently link to. The B26 sheet for Pan 400 came from a reseller. Its text
differs from the Jul 2022 HARMAN-hosted sheet in one place only: ILFOTEC DD at 24 °C is 7:30/10:00 in B26 and
9:00/13:00 in Jul 2022. Every other development time is the same. The Jan 2019 Pan 400 sheet
(https://125px.com/docs/film/kentmere/Pan-400_201901.pdf) has the same times as B26, but lists 35mm only (no 120).

### 1.1 What the sheets publish

| Claim | Status | Evidence |
|-------|--------|----------|
| The sheets publish a **characteristic curve** (density vs log exposure, with developer and time) | **confirmed: NOT published** | None of the three current sheets has a characteristic curve. Each has two kinds of figure: one reciprocity graph (p. 1) and two time/temperature nomograms (p. 3, °C and °F). All three PDFs have the same layout, with 6 embedded images each. This is unlike the Ilford HP5+/FP4+ sheets, which have one Ilfotec HC 1+31 curve (see `source-inventory.md`) |
| The sheets publish **spectral sensitivity** (a spectral curve or wedge spectrogram) | **confirmed: NOT published** | Not in any of the three sheets. The only spectral statement is about filters: green and blue filters "may need slightly more exposure" in red-rich late-afternoon or winter light. The older Kentmere Ltd sheets (January 2009; Pan 400 read: https://web.archive.org/web/2024id_/https://www.digitaltruth.com/products/kentmere_tech/kentmere400.pdf) have no spectral data either. "Panchromatic" is only implied by the product name |
| The sheets publish **granularity** (RMS or diffuse granularity number) or MTF | **confirmed: NOT published** | No granularity value and no MTF curve in any sheet. Grain is described in words only (§1.3) |
| **Reciprocity**: no correction from 1 s to 1/10 000 s. Beyond 1 s, Ta = Tm^p with p = **1.26** (Pan 100, Pan 200) and **1.30** (Pan 400), plus a graph of adjusted vs metered time for 0–50 s | **confirmed [P]** | p. 1 of each sheet: "The graph is based on the formulae Ta = Tm1.26" (Pan 100 Jul 2022, Pan 200 Mar 2025) and "Ta = Tm1.30" (Pan 400 B26 and Jul 2022) |
| Temperature compensation is given as a nomogram only, valid about 14–24 °C. Worked example: 4 min at 20 °C becomes 3 min at 23 °C and 6 min at 16 °C | **confirmed [P]** | p. 3 of each sheet. This is the same generic Ilford chart, not a film-specific coefficient |
| ISO speed is measured in **ILFORD ID-11** at 20 °C with intermittent agitation in a spiral tank. Box speeds are ISO 100/21°, 200/24° and 400/27° | **confirmed [P]** | p. 1 "EXPOSURE RATING" of each sheet |
| The "average contrast" target is not given as a number (no CI, no G-bar) | **confirmed [P]** | p. 2 says only: "These times will produce negatives of average contrast suitable for printing in all enlargers." |
| Agitation: 4 inversions in the first 10 s, then 4 inversions in the first 10 s of each minute. Reduce times by up to 15% for continuous (dish) agitation or rotary processing without a pre-rinse. A pre-rinse is "not recommended" | **confirmed [P]** | pp. 2–3 of each sheet |
| Base: 0.125 mm acetate. Pan 200 is described as "acetate **low density** base" | **confirmed [P]** | p. 1. Only the Pan 200 sheet says "low density" |

### 1.2 Manufacturer development times (20 °C, spiral tank, intermittent agitation)

These are recorded here only and are **not** in the CSV, which holds MDC data only. Format: min:sec. "–" means
no time given. The sheets list Kodak D-76 under "Non-ILFORD Developers". **None of the sheets gives HC-110 or
Rodinal times.** The nearest HARMAN equivalent to HC-110 is ILFOTEC HC, shown here for reference. Ilfotec HC
has **not** been shown to be equivalent to HC-110 (see `source-inventory.md`).

**Kentmere Pan 100** (Jul 2022), EI columns 50 / 100 / 200 — **confirmed [P]**

| Developer | Dilution | EI 50 | EI 100 | EI 200 |
|-----------|----------|-------|--------|--------|
| ID-11 | stock | 7:00 | 9:00 | 11:00 |
| ID-11 | 1+1 | 8:30 | 11:30 | 15:30 |
| ID-11 | 1+3 | 17:30 | 21:00 | – |
| Kodak D-76 | stock | 7:00 | 9:00 | 11:00 |
| Kodak D-76 | 1+1 | 8:30 | 11:30 | 15:30 |
| Kodak D-76 | 1+3 | 17:30 | 21:00 | – |
| ILFOTEC HC (ref.) | 1+15 | – | 4:00 | 5:00 |
| ILFOTEC HC (ref.) | 1+31 | 5:00 | 6:30 | 8:00 |
| HC-110 | any | not given | not given | not given |
| Rodinal | any | not given | not given | not given |

**Kentmere Pan 200** (Mar 2025), EI columns 100 / 200 / 400 — **confirmed [P]**

| Developer | Dilution | EI 100 | EI 200 | EI 400 |
|-----------|----------|--------|--------|--------|
| ID-11 | stock | 5:00 | 7:30 | – |
| ID-11 | 1+1 | 6:00 | 8:45 | – |
| ID-11 | 1+3 | 12:30 | – | – |
| Kodak D-76 | stock | 5:00 | 7:30 | – |
| Kodak D-76 | 1+1 | 6:00 | 8:45 | – |
| Kodak D-76 | 1+3 | 12:30 | – | – |
| ILFOTEC HC (ref.) | 1+15 | – | 4:00 | 7:30 |
| ILFOTEC HC (ref.) | 1+31 | 4:45 | 6:00 | 10:00 |
| HC-110 / Rodinal | any | not given | not given | not given |

**Kentmere Pan 400** (B26, same times as Jul 2022), EI columns 320 / 400 / 800 — **confirmed [P]**

| Developer | Dilution | EI 400 | EI 800 |
|-----------|----------|--------|--------|
| ID-11 | stock | 9:30 | 13:00 |
| ID-11 | 1+1 | 16:30 | 20:30 |
| ID-11 | 1+3 | 25:30 | – |
| Kodak D-76 | stock | 9:30 | 12:30 |
| Kodak D-76 | 1+1 | **14:00** | **17:00** |
| Kodak D-76 | 1+3 | 28:00 | – |
| ILFOTEC HC (ref.) | 1+15 | 4:30 | 6:30 |
| ILFOTEC HC (ref.) | 1+31 | 8:00 | 12:30 |
| HC-110 / Rodinal | any | not given | not given |

The EI 320 column has only Perceptol 1+1 (23:00) and ILFOTEC RT Rapid machine times. No D-76 or ID-11 time is
given at 320.

Quirk: for Pan 100 and Pan 200, the D-76 times are the same as the ID-11 times. For **Pan 400 they differ**:
D-76 1+1 is 14:00 / 17:00 but ID-11 1+1 is 16:30 / 20:30, and at EI 800 D-76 stock is 12:30 but ID-11 stock is
13:00. This holds in the Jan 2019, Jul 2022 and B26 sheets. D-76 and ID-11 are nominally the same formula, and
the sheets do not explain the difference.

### 1.3 Grain character and box speed (manufacturer wording)

| Claim | Status | Evidence |
|-------|--------|----------|
| Pan 100: "high quality images with **fine grain and good sharpness** ... a good choice of film if enlargement prints are desired" | **confirmed [P]** | Pan 100 sheet p. 1 |
| Pan 200: "fine grain and good sharpness and will generally yield **mid-high pictorial contrast** at the given development times ... a good choice for high quality enlargements or high-resolution scans" | **confirmed [P]** | Pan 200 sheet p. 1 (spelled "pictoral" in the sheet). It is the only Kentmere sheet that makes a contrast-character claim |
| Pan 400: "capable of generating **sharp, fine grain**, high quality images ... particularly suitable for working with action shoots and available light situations" | **confirmed [P]** | Pan 400 sheet p. 1 |
| The film performs best at box speed. Rating Pan 400 at 320 or 800 (and Pan 100 at other speeds) "diminish[es] ... grain and sharpness" | **confirmed [P] for the 2009 Kentmere Ltd sheets only** | January 2009 Kentmere 400 sheet: "Although it is possible to rate the film at 320/26° and 800/30°, the film performs at its optimum when rated at 400ASA. When rated at any alternate suggested ASA ratings, quality does diminish in terms of grain and sharpness." The 2009 Kentmere 100 sheet has the same wording (https://web.archive.org/web/2024id_/https://www.digitaltruth.com/products/kentmere_tech/kentmere100.pdf). The current HARMAN sheets **dropped** this sentence. They give only the ISO rating and the EI columns 50–200 (Pan 100), 100–400 (Pan 200) and 320–800 (Pan 400) |
| The manufacturer quantifies grain relative to HP5+/FP4+ (e.g. "grainier than HP5") | **unconfirmed** | No HARMAN source compares them. The common view that Pan 400 is grainier than HP5 Plus comes from the community, not the manufacturer |
| Agfa APX 400 (new, 2018+) is a rebadged Kentmere Pan 400 | **unconfirmed [S]** | Asserted only in an MDC user note (devrow 18353). Not checked against a HARMAN or Agfa source |

---

## 2. Rows appended to `data/development/mdc-times.csv`

Method: same as `tmax-d96-fx55.md` §2, using the same selector, note builder and filters as the existing rows.

- Source page: Wayback snapshot of the MDC Kentmere search, **2025-11-07**, Celsius, minutes:seconds.
  This one page covers all three films:
  https://web.archive.org/web/20251107121119/https://www.digitaltruth.com/devchart.php?Film=%25Kentmere%25&Developer=&mdc=Search&TempUnits=C&TimeUnits=T
  It has 455 rows (Kentmere 100: 148, Kentmere 200: 64, Kentmere 400: 243). MDC names the films "Kentmere
  100/200/400", without "Pan". A 2026-02-16 snapshot (`Film=Kentmere`, no TimeUnits) gave no result table.
- Devrow notes come from `https://web.archive.org/web/<ts>/https://www.digitaltruth.com/devchart.php?devrow=<n>`.
  Snapshots are dated 2025-06-07 to 2026-08-19, and every needed page was retrieved. Devrows: 10851, 10943, 11071,
  11103, 11112, 11182, 11387, 11504, 12312, 13445, 13922, 14615, 14889, 15538, 18049, 18340, 18353, 18425, 18449,
  18467, 18473, 18492, 18529, 18577, 18589, 18640, 18649, 18664, 18685, 18793, 18975, 19038, 19048, 19076, 19077,
  19110, 19111, 19120, 19126, 19168, 19184, 19192.
- Whole-stop EIs: box × 2^n. If the exact whole stop is not listed, the nearest listed EI within 1/3 stop is used.
  No Kentmere row needed that substitution, because every selected EI is an exact whole stop.
- 35mm time is used. When only one time is listed, it is used. A row with only a 120 time is kept as `format=120`
  with a note, as in the existing rows. Every MDC row at the same EI is kept, noted "multiple MDC rows at this EI".
- Stand, semi-stand and rotary rows are kept but marked. `tools/curate_times.py` drops any row whose notes contain
  "stand", "rotary" or "jobo".

Total appended: **85 rows** (kentmere-100: 33, kentmere-200: 12, kentmere-400: 40). Existing rows are unchanged
(checked by diff). The header comment was extended with the Kentmere snapshot date, box speeds, the FX-55 "stock"
row and the HC-110 "1+31" mapping.

| Film | d76 stock | d76 1+1 | hc110 B | hc110 E | hc110 G | hc110 H | rodinal 1+25 | rodinal 1+50 | rodinal 1+100 | d96 stock | fx55 |
|------|-----------|---------|---------|---------|---------|---------|--------------|--------------|---------------|-----------|------|
| kentmere-100 | 3 (50, 100, 200) | 5 (50–800) | 5 (100×3, 200, 400) | **0** | **0** | 2 (100, 400) | 6 (100×2, 200, 800, 1600, 3200) | 10 (25, 50, 100×3, 200, 400×2, 800×2) | 1 (800, stand) | **0** | 1 (1+9 @100) |
| kentmere-200 | 2 (100, 200) | 1 (100) | 3 (200, 400, 800; MDC label "1+31") | 1 (200) | **0** | **0** | 3 (200×2, 800) | 1 (200, 120 only) | 1 (200, stand) | **0** | **0** |
| kentmere-400 | 5 (50, 200, 400, 800, 1600) | 3 (400, 800, 1600) | 7 (200, 400, 800, 1600×3, 3200) | 2 (400, 800) | **0** | 3 (25, 50, 200) | 5 (400×2, 800, 1600, 3200) | 8 (50, 100, 200, 400×3, 800, 1600) | 3 (800, 1600, 3200; all stand) | 2 (400×2) | 2 (1+9 @400; "stock" @1600) |

### 2.1 Cross-check of MDC rows against the HARMAN sheets

| Claim | Status | Evidence |
|-------|--------|----------|
| MDC Kentmere 100 D-76 rows (stock 50/100/200; 1+1 50/100/200) are the HARMAN sheet values | **confirmed (match)** | The values are the same as §1.2. MDC also lists 1+1 at EI 400 (20 min, no note) and EI 800 (27 min, user note), which are not on the sheet |
| MDC Kentmere 200 D-76 rows (stock 100/200, 1+1 100) are the HARMAN sheet values | **confirmed (match)** | Devrows 19038, 19076 and 19077: "From Manufacturer data sheet". The values are the same as §1.2. **MDC has no D-76 1+1 EI 200 row**, while the sheet gives 8:45. It was not added, because the CSV is MDC-only |
| MDC Kentmere 400 D-76 stock 400/800 and 1+1 400/800 are the HARMAN sheet values | **confirmed (match)** | 9:30 / 12:30 / 14:00 / 17:00 are the same as §1.2. The MDC EI 1600 rows (stock 20, 1+1 22) and pull rows (stock 50, 200) are not on the sheet |
| MDC Kentmere 100 HC-110 B EI 100 (6:30, 20 °C) rows are HARMAN's **Ilfotec HC 1+31** time, not an HC-110 test | **confirmed [S]** | Devrow 18589 user note: "This time is taken from Harman's film processing sheet for K100 at 20C in IlfoTec HC". The value is the same as the sheet's ILFOTEC HC 1+31 EI 100 (6:30). The other HC-110 B rows for K100 EI 200/400 (8, 10) also equal Ilfotec HC 1+31 at EI 200 (8:00) and are close to its pattern. Treat them as possibly copied from Ilfotec HC |

---

## 3. Quirks and decisions

1. **HC-110 "1+31" label mapped to B (Kentmere only).** Kentmere 200 has most of its HC-110 data under the
   label "1+31", and Kentmere 400 has two such rows at EI 1600. 1+31 from concentrate is MDC's own definition of
   Dilution B ("Dilution B is 1+31 from concentrate", e.g. devrow 10851). These rows are kept as `B` with the note
   "MDC dilution label 1+31 (= Dilution B 1+31 from concentrate)". **This is a departure from the T-Max batch**,
   where the "1+31" rows were excluded (`tmax-d96-fx55.md`). To restore the strict behaviour, drop rows whose note
   contains "MDC dilution label 1+31".
2. **Rows dropped because they have no numeric time.** Kentmere 100 Rodinal 1+50 EI 400: the time is "#" (devrow
   18664, where the user note says "44 mins total"). Kentmere 400 HC-110 1+31 EI 400: the time cells are blank.
   The "44 mins" was not used as a time.
3. **"40*" time.** Kentmere 100 Rodinal 1+25 EI 800 is listed as "40*". The page does not explain the asterisk.
   40 is used, and the asterisk is recorded in the note.
4. **Semi-stand-like rows MDC does not label as stand.** Kentmere 100 Rodinal 1+50 EI 800 (devrow 18467: "Gentle
   agitation every 10 minutes", 60 min) and Rodinal 1+25 EI 800 (devrow 19048: one minute of agitation every
   10 minutes, 40 min) get an added note: "agitation only every 10 min (semi-stand-like regime; not labelled stand
   by MDC)". This lets the curation filter drop them. Kentmere 100 Rodinal 1+25 EI 1600 (60 min, 120-only,
   "agitate every minute") is **not** marked, and is suspicious.
5. **Rotary.** Kentmere 200 Rodinal 1+25 EI 200 at 24 °C (devrow 19126, "Rotary processor") is marked through
   its note.
6. **FX-55 "stock" row.** Kentmere 400 FX-55 stock EI 1600 (17 min) is a "Gainer Mod" variant sold by nbtg.dev
   (devrow 18685), not Crawley's 1+9 working solution. It is kept with the note "MDC dilution stock".
7. **No Rodinal-type substitutes.** The Kentmere MDC page has **no** Fomadon R09, Adonal or other Rodinal-type
   rows. All Rodinal rows are labelled "Rodinal".
8. **Not used (outside the requested set).** MDC also has ID-11 (26 rows), HC-110 A/D/F and "1+49", D-76 1+2/1+3,
   and Ilfotec HC/LC29/DD-X rows for Kentmere.
9. **Conflicting community data.** Kentmere 400 Rodinal 1+25 EI 400 has 7:30 and 11:30 (the 11:30 user calls 7:30
   "severely underdeveloped"). Rodinal 1+50 EI 400 has 17:30, 11:30 ("black negatives at 17:30") and 20 (120
   only). Kentmere 100 Rodinal 1+50 EI 100 has 15, 15 ("grainy/overexposed at 15") and 12:30 (120). A dry run of
   `tools/curate_times.py` on a copy of the data found monotonicity inversions against the medians:
   kentmere-400 Rodinal 1+25 EI 800 (9 min) and Rodinal 1+50 EI 200 (15 min). Those points are dropped there.
10. **Missing combinations (no MDC data in the snapshot):** HC-110 G for all three films. HC-110 E for Kentmere 100.
    HC-110 H and FX-55 1+9 for Kentmere 200. D-96 for Kentmere 100 and 200. D-76 1+1 EI 200 for Kentmere 200 (the
    manufacturer gives 8:45). D-76 and HC-110 at EI 100 for Kentmere 400.
