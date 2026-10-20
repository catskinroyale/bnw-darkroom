# T-Max 100/400, D-96 and FX-55: development data and image-structure sources

Scope: the T-Max 100 / T-Max 400 and D-96 / FX-55 additions to `data/development/mdc-times.csv`, the Kodak
image-structure data for T-Max, and the D-96 formula.

Retrieval date for all sources: **2026-10-20**.

Status vocabulary (same as `source-inventory.md`):

- **confirmed**: matches the primary or original document, which was read.
- **unconfirmed**: the primary document could not be read. Any evidence given is secondary and labelled **[S]**.

Evidence tags: **[P]** primary document read; **[S]** secondary source (encyclopedia, retailer, blog).

---

## 1. FX-55

| Claim | Status | Evidence |
|-------|--------|----------|
| FX-55 is a black-and-white film developer devised by Geoffrey Crawley (the "FX" series) | **confirmed [S]** | Wikipedia, "FX-55 (film developer)": devised by Geoffrey Crawley, first published in *Amateur Photographer*, 13 September 2008. https://en.wikipedia.org/wiki/FX-55_(film_developer). Silvergrain Classics agrees on Crawley and the 2008 *AP* publication, but dates the original to 1982. https://silvergrainclassics.com/en/2024/01/fx55-film-developer/. The original *AP* article was not read. The Digitaltruth formula page (https://www.digitaltruth.com/data/formula.php?FormulaID=63) returned 403, and its Wayback copy could not be fetched |
| Published formula: Part A (stock, used 1+9): potassium carbonate 20 g, sodium bicarbonate 1.5 g, sodium sulfite 25 g, sodium metabisulphite 12 g, water to 1000 ml. Part B, added to 1 L of diluted A just before use: sodium L-ascorbate 1.3 g, Phenidone 0.1 g | **confirmed [S]** (not checked against *AP*) | Wikipedia (above). One-shot, and the working solution keeps for at most 36 h. Reducing metabisulphite to 10 g, or developing about 10% longer, raises CI from 0.58–0.60 to about 0.65 |
| Character: a fine-grain, Phenidone–ascorbate developer with no metol or hydroquinone ("eco-friendly"). It gains 0.5–1 EV of speed and has good latitude | **confirmed [S]** | Wikipedia (above): "fine, even grain"; an exposure index of about 0.5–1 EV over ISO is usually possible. Silvergrain Classics and the nbtg shop describe it as **high-acutance**, with grain finer than Rodinal. **Compensating** behaviour is not claimed by any source read |
| Sold commercially? | **confirmed [S]: kit form only** | Crawley's formula was published for home mixing and was not sold as a branded product (Silvergrain Classics). Kits are now sold: nbtg ("nothing but the grain", NL) sells an FX-55 kit with powder A and liquid B in propylene glycol, used 1+9 plus 20 ml B per litre, €9.99 for 5 L and €19.99 for 10 L. https://nbtg.dev/product/fx-55/. Nik & Trick (UK) sold a revised version, but its product page returned 404 (https://ntphotoworks.com/fx55-bw-eco-film-developer/) |
| MDC developer name and dilutions | **confirmed [P]** | The MDC developer menu lists **"FX-55"** (option value `FX-55`). Film pages list it at dilution **"1+9"** for every film checked, plus **one HP5+ row at "stock"** (EI 640, 15 min, 21 °C). Archived pages are in §2 |
| MDC FX-55 1+9 times are Crawley's published table | **confirmed (match)** | Every MDC 1+9 row equals the Crawley time table reproduced by Wikipedia: T-Max 100 13, T-Max 400 14½, Tri-X 12½, HP5+ 13½, FP4+ 8½, Delta 100 10, Delta 400 12½, Delta 3200 12½ min at 20 °C (4 inversions/min). These rows are therefore one source (Crawley), not independent community data |

## 2. Rows appended to `data/development/mdc-times.csv`

Method: same as the existing rows. Whole-stop EIs from box speed. Where the exact whole stop is not listed, the
nearest listed EI within 1/3 stop is used (ISO nominal thirds, so 64 counts as 1/3 stop from 50).
Rows are 35mm, and the 120 time is used only when no 35mm time is listed. Every MDC row at the same EI is kept,
with a note. Notes come from archived devrow pages. The "Dilution X is 1+N from concentrate" boilerplate is
dropped, and commas are removed from note text. My selector reproduced the existing tri-x/hp5/fp4/double-x/delta
rows for d76/hc110/rodinal exactly, apart from the existing Delta 3200 ISO 1000 special case. The existing rows
are unchanged, and the new rows are appended at the end.

MDC sources (Wayback):

- T-Max 100: https://web.archive.org/web/20260816032522/https://www.digitaltruth.com/devchart.php?mdc=Search&TempUnits=C&TimeUnits=T&Film=%25Kodak+TMax+100%25&Developer=
- T-Max 400: https://web.archive.org/web/20260916013645/https://www.digitaltruth.com/devchart.php?TimeUnits=T&Film=%25Kodak+TMax+400%25&Developer=&mdc=Search&TempUnits=C
- Existing films: the same snapshot URLs already in each film's `source_url`.
- Devrow notes: `https://web.archive.org/web/<ts>/https://www.digitaltruth.com/devchart.php?devrow=<n>`, snapshots
  2025-07-27 to 2026-08-19, for devrows 3239, 3242, 3251, 3346, 3386, 3510, 9115, 9121, 9165, 9267, 9404, 9576,
  10859, 11392, 11822–11827, 12130–12135, 12234, 12235, 12267, 12563, 14619, 17971, 19129.

| Film | d76 stock | d76 1+1 | hc110 B | hc110 E | hc110 G | hc110 H | rodinal 1+25 | rodinal 1+50 | rodinal 1+100 | d96 stock | fx55 |
|------|-----------|---------|---------|---------|---------|---------|--------------|--------------|---------------|-----------|------|
| tmax-100 | 4 (100–800) | 2 (64→50, 100) | 7 (100×2, 200×2, 400×2, 3200) | **0** | **0** | 1 (100) | 5 (64→50, 100×3, 400) | 3 (50, 100×2) | 3 (64→50, 100, 1600) | 1 (100) | 1 (1+9 @100) |
| tmax-400 | 3 (400, 800, 1600) | 6 (50, 100, 400×2, 800, 1600) | 4 (400, 800, 1600, 3200) | 2 (200, 400) | **0** | 2 (400×2) | 5 (400×2, 800, 1600, 3200) | 8 (100, 200, 400×3, 800, 1600×2) | 3 (200, 400, 800) | **0** | 1 (1+9 @400) |
| tri-x | – | – | – | – | – | – | – | – | – | 1 (400) | 1 (1+9 @400) |
| hp5 | – | – | – | – | – | – | – | – | – | 1 (400) | 2 (1+9 @400, stock @640→800) |
| fp4 | – | – | – | – | – | – | – | – | – | 1 (125) | 1 (1+9 @125) |
| delta-100 | – | – | – | – | – | – | – | – | – | 1 (100) | 1 (1+9 @100) |
| delta-400 | – | – | – | – | – | – | – | – | – | **0** | 1 (1+9 @400) |
| delta-3200 | – | – | – | – | – | – | – | – | – | 2 (1600, 3200) | 1 (1+9 @3200) |
| double-x | – | – | – | – | – | – | – | – | – | 5 (250, 400→500, 1600→2000, 3200→4000 ×2) | **0** |

Total appended: **79 rows** (tmax-100 27, tmax-400 34, existing films 18).

**Missing combinations (not on MDC at a whole stop):** T-Max 100 HC-110 E and G. T-Max 400 HC-110 G. T-Max 400 D-96
(MDC has no D-96 row for T-Max 400 at all). Delta 400 D-96. Double-X FX-55. FX-55 rows exist only at box speed,
except HP5+ "stock" at 640.

### Data quirks

- **FX-55 dilution label:** MDC labels it "1+9", which is the Part A working dilution, with Part B added. One HP5+ row is
  labelled "stock" (EI 640, 15 min, 21 °C, no notes). It probably means the same working solution, but this is
  unconfirmed, so the dilution id is kept as listed (`stock`).
- **FX-55 and D-96 rows have no devrow notes.** Single-source FX-55 times come from Crawley (see §1).
- **Double-X D-96 EI 3200:** two identical MDC rows (30 min, 21 °C). Both are kept and noted. The Double-X page
  is in decimal time units.
- **tmax-100 d76 stock 800 (11 min)** carries devrow note "Time for Jobo/rotary tube processor". This is rotary
  agitation, not intermittent, and `tools/curate_times.py` does not filter it.
- **tmax-100 d76 1+1 rows** carry the note "Sheet time in tank". It refers to the sheet column. The 35mm time is used.
- **"Same time applies across ASA/ISO range"** (MDC note [56]) marks Kodak's single published time, which is repeated
  at 100/200 (TMX D-76 stock, HC-110 B) and 400/800 (TMY D-76, HC-110 B). It is not an independent push time.
- **Older-version rows:** several TMX rows carry note [29], "Data is taken from a previous version of this film.
  Starting point time should be similar". Older existing rows have "…remains the same". The wording differs between
  snapshot dates.
- **HC-110 "1+31" rows** (T-Max 100 EI 800, 120 only; T-Max 400 "1+250", "D") are not B/E/G/H labels and were excluded.
- **tmax-400 rodinal 1+50 EI 1600** has a 35mm row and a 120-only row (18 min each). The 120 row is kept with format `120`.
- **Stand/semi-stand rows** (noted in `notes`): tmax-100 hc110 B 3200, tmax-400 rodinal 1+100 200 and 400.
  `curate_times.py` drops them via "stand".

## 3. Kodak T-Max technical data

| Claim | Status | Evidence |
|-------|--------|----------|
| Current T-Max 100 data sheet is F-4016, February 2016 ("Revised 2-16"), Kodak Alaris, 9 pp | **confirmed [P]** | https://imaging.kodakalaris.com/sites/default/files/files/products/f4016_TMax_100.pdf (301 → business.kodakmoments.com) |
| T-Max 100 diffuse rms granularity **8**, in **KODAK Developer D-76 at 20 °C** | **confirmed [P]** | F-4016 p. 8, "Image Structure": "data in this section are based on development in KODAK Developer D-76, at 68°F (20°C)". Read at net diffuse density 1.00, 48 µm aperture, 12× magnification. Resolving power 63 lines/mm (TOC 1.6:1) and 200 lines/mm (TOC 1000:1) |
| T-Max 100 characteristic curves | **confirmed [P], with a defect** | p. 8 has three families, all with daylight exposure and diffuse visual densitometry. (1) **D-76, small tank, 20 °C**: three curves, but the legend reads "PL Q" with **no times printed**, which looks like a template placeholder left in the published PDF. (2) **T-MAX RS Developer and Replenisher, large tank, 20 °C**: 8, 10.5, 13 and 15 min. (3) **T-MAX Developer, small tank, 20 °C**: 6, 7, 10 and 12 min. p. 9 has contrast-index vs time curves (small tank, large tank, rotary, tray) for D-76, D-76 1:1, T-MAX, T-MAX RS, XTOL, XTOL 1:1 and HC-110 (B) |
| T-Max 100 spectral sensitivity given | **confirmed [P]** | p. 8: log sensitivity at D = 0.3 and 1.0 above D-min, 1.4 s effective exposure, D-76 at 20 °C, diffuse visual, 250–750 nm |
| T-Max 100 MTF given | **confirmed [P]** | p. 8: "Modulation Transfer Curves", tungsten exposure, small tank D-76 at 20 °C, diffuse visual, 1–600 c/mm axis (curve plotted to about 150 c/mm) |
| Current T-Max 400 data sheet is F-4043, February 2016 ("Revised 2-16") | **confirmed [P]** | https://imaging.kodakalaris.com/sites/default/files/files/products/f4043_TMax_400.pdf (301 → business.kodakmoments.com). It describes the film as 400TMY with "Optimized Light Filtration technology" and calls it the "world's sharpest 400-speed black-and-white film". The sheet does **not** use the name "TMY-2" |
| T-Max 400 diffuse rms granularity **10**, in **D-76 at 20 °C** | **confirmed [P]** | F-4043 p. 7, same footnote conditions (net D 1.00, 48 µm, 12×). Resolving power 50 lines/mm (TOC 1.6:1) and 200 lines/mm (TOC 1000:1) |
| T-Max 400 characteristic curves | **confirmed [P]** | p. 8, daylight exposure, diffuse visual densitometry. (1) **D-76, small tank, 20 °C: 6, 8, 11 min**. (2) **T-MAX Developer, small tank, 24 °C: 5, 7, 9 min**. (3) **T-MAX RS Developer and Replenisher, large tank, 24 °C: 5, 7, 9 min**. Contrast-index vs time curves (small tank, 20 °C): HC-110 (B), XTOL, D-76, XTOL 1:1, D-76 1:1 |
| T-Max 400 spectral sensitivity given | **confirmed [P]** | p. 7: D-76 at 20 °C, diffuse visual, D = 0.3 and 1.0 above D-min, 250–750 nm |
| T-Max 400 MTF given | **confirmed [P]** | p. 7: tungsten exposure, small tank D-76 at 20 °C, diffuse visual |

## 4. D-96

| Claim | Status | Evidence |
|-------|--------|----------|
| D-96 per litre (tank): Elon 1.5 g, sodium sulfite (anhydrous) 75 g, hydroquinone 1.5 g, potassium bromide (anhydrous) 0.4 g, borax (decahydrate) 4.5 g | **confirmed [P]** | Kodak H-24.15, *Processing KODAK Motion Picture Films, Module 15: Processing Black-and-White Films*, rev. 4/05, Table 15-6, "KODAK Developer D-96 (Negative)". Water at about 50 °C, 750 mL, then water to make 1.00 L. KBr can be replaced by sodium bromide. pH 8.62 ± 0.05 at 25 °C, specific gravity 1.069. D-96R replenisher: Elon 2.0, sulfite 80.0, HQ 2.0, bromide 0.35, borax 5.0 g. https://www.kodak.com/content/products-brochures/Film/Processing-KODAK-Motion-Picture-Films-Module-15.pdf (also recorded in `source-inventory.md`, row K5) |
| Usual use: motion-picture negative developer | **confirmed [P]** | H-24.15 Table 15-4 "Processing Specifications for Negative Films" lists D-96 at 21 °C (70 °F), with the time "Will vary with each film. Consult the Data Sheet", in a continuous machine process with recirculation, agitation and filtration. The text says to process in D-96 "or if higher contrast is needed, in KODAK Developer D-97". The Eastman Double-X 5222 data are all in D-96 (`source-inventory.md`) |
| MDC lists D-96 as "D-96", dilution "stock" | **confirmed [P]** | MDC developer menu option value `D-96`. All D-96 rows on the film pages in §2 are "stock" |
