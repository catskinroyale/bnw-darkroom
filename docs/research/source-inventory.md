# Source inventory: verification of [verify] claims in design doc 0001

Scope: every **[verify]** statement in `docs/design/0001-negative-simulation-model.md` (focus: §1 challenges 5–6,
§2 reciprocity, §3 fitting/§3.4, §3.5, §5.1, §5.2/§5.3, §7, §8.2). The design doc itself has **not** been edited.

Retrieval date for all sources: **2026-10-20**.

Status vocabulary:

- **confirmed**: the claim matches the primary (manufacturer or original) document, which was read in full.
- **corrected**: the primary document was read and contradicts or materially refines the claim.
- **unconfirmed**: the primary document could not be found or read; any evidence given is secondary and labelled as such.

Evidence-type tags: **[P]** primary document read in full; **[P-hist]** primary but historical (no longer current);
**[S]** secondary source (forum, retailer, patent citation, etc.).

---

## 0. Documents actually read

| ID | Document | Number / revision | URL | Notes |
|----|----------|-------------------|-----|-------|
| K1 | KODAK PROFESSIONAL TRI-X 320 and 400 Films, Technical Data | F-4017, February 2016 ("Revised 2-16"), Kodak Alaris, 9 pp. | https://imaging.kodakalaris.com/sites/default/files/files/products/f4017_TriX.pdf | Read in full [P] |
| K2 | EASTMAN DOUBLE-X Negative Film 5222/7222, Technical Data | H-1-5222, March 2022 ("Revised 3-22"), Eastman Kodak, 3 pp. | https://www.kodak.com/content/products-brochures/Film/DOUBLEX-Technical-Data-EN.pdf | Read in full [P] |
| K3 | KODAK Developer D-76, Technical Data / Chemicals | J-78, December 2017 ("Revised 12-17"), Kodak Alaris, 7 pp. | https://business.kodakmoments.com/sites/default/files/wysiwyg/pro/chemistry/j78.pdf | Read in full [P] |
| K4 | KODAK PROFESSIONAL HC-110 Developer, Technical Data / Chemicals | J-24, December 2017 ("Revised 12-17"), Kodak Alaris, 8 pp. | https://business.kodakmoments.com/sites/default/files/wysiwyg/pro/chemistry/j24.pdf | Read in full [P]. The October 2002 edition (https://125px.com/docs/techpubs/kodak/j24-2002_10.pdf) was text-searched only |
| K5 | Processing KODAK Motion Picture Films, Module 15: Processing Black-and-White Films | H-24.15, "Revised 4/05" | https://www.kodak.com/content/products-brochures/Film/Processing-KODAK-Motion-Picture-Films-Module-15.pdf | Text-searched for D-96 tables [P] |
| K6 | Basic Photographic Sensitometry Workbook | H-740, November 2006, Eastman Kodak | https://www.kodak.com/content/products-brochures/Film/Basic-Photographic-Sensitometry-Workbook.pdf | Text-searched for the Contrast Index definition [P] |
| K7 | Kodak Chemicals and Formulae (Kodak Ltd, UK; undated scan) | n/a | https://archive.org/details/KodakChemicalsAndFormulae | OCR text searched for D-76 [P-hist] |
| K8 | KODAK WRATTEN 2 filter curves, Nos. 8, 15, 25, 29 (one PDF per filter) | undated | https://www.kodak.com/en/motion/page/wratten-2-filters/ (No. 25 PDF read: https://www.kodak.com/content/products-brochures/Film/Basic-Color-Filters-W2-25.pdf) | [P] |
| I1 | ILFORD HP5 PLUS Technical Information | Nov 2018, HARMAN technology, 6 pp. | https://www.ilfordphoto.com/amfile/file/download/file/1903/product/691/product_datasheet_hp5plus.pdf | Read in full [P] |
| I2 | ILFORD FP4 PLUS Technical Information | Nov 2018, HARMAN technology, 6 pp. | https://www.ilfordphoto.com/amfile/file/download/file/1919/product/690/product_datasheet_fp4plus.pdf | Read in full [P] |
| I3 | ILFORD PERCEPTOL, ID-11 and MICROPHEN Film Developers, Technical Information | Aug 2024, HARMAN technology, 9 pp. | https://www.ilfordphoto.com/amfile/file/download/file/1829/product/708/product_datasheet_id11.pdf | Text-extracted and searched [P] |
| I4 | Ilford Formulae and Packed Chemicals, 14th edition | Ilford Ltd, March 1953 | https://www.photomemorabilia.co.uk/Ilford/Chronology/Ilford_Formulae_Dev_1953.pdf | Read pp. 1–25 [P-hist] |
| A1 | AGFA RODINAL instruction leaflet (125 ml / 500 ml), a&o imaging solutions GmbH, print code "13 144 140 - 1106" | undated (print code suggests 2006) | https://www.fotoimpex.com/shop/images/products/media/58420_6_PDF-Datasheet.pdf (Fotoimpex links it as the "Adox Adonal PDF-Datasheet") | Read [P], but the table is a low-resolution image. See R-1 |
| N1 | Newson, Faraj, Galerne, Delon, "Realistic Film Grain Rendering", IPOL | vol. 7 (2017) pp. 165–183, DOI 10.5201/ipol.2017.192 | https://www.ipol.im/pub/art/2017/192/ | Landing page read [P] |
| P1 | US Patent 5,563,717 (cites Nelson 1971) | — | https://patents.google.com/patent/US5563717A/en | [S] for the Nelson citation |

---

## 1. Kodak Tri-X 400 (F-4017), design doc §7 row 1, §3 step 3, §5.2

| Claim (design doc) | Status | Evidence (K1 = F-4017, Feb 2016) | Correction to design doc |
|---|---|---|---|
| Characteristic curves exist "in D-76; possibly a time family" (§7) | **confirmed (time family)** | p. 8: "Characteristic Curves, TRI-X 400 Film / 400TX, 35 mm" and "120-size". Process: KODAK PROFESSIONAL D-76 Developer, **Large Tank; agitation at 1-minute intervals**, 20 °C (68 °F). Curves at **6, 8, 10, 12 min**. Densitometry: diffuse visual. Axes: density 0–4.0; log exposure **in lux-seconds (absolute)**, from 4̄.0 (−4) to 1.0. p. 7: a second family in **KODAK T-MAX Developer, Small Tank, agitation at 30-second intervals**, 20 °C, at **6, 7, 9, 11 min** (35 mm and 120 shown separately) | State that two time families are published (D-76 large tank; T-MAX Developer small tank) and that each records its own agitation. The 35 mm and 120 curves differ slightly in base+fog (35 mm about 0.3, 120 about 0.2), so pick one format and record it. |
| CI-vs-time curves for "D-76, HC-110 B, XTOL, T-Max developer" (§7) | **confirmed, with additions** | p. 8, "Contrast Index Curves, 400TX, 35 mm and 120": Process **Small Tank at 20 °C**, diffuse visual. Developers: T-MAX, D-76, **D-76 (1:1)**, **XTOL 1:1**, HC-110 (B), **T-MAX RS**, XTOL. Axes: CI 0–1.2 against development time 0–30 min | Add D-76 1:1, XTOL 1:1 and T-MAX RS. **Important:** the 400TX CI curves are *small tank (30 s agitation)*, but the D-76 characteristic-curve family is *large tank (1 min agitation)*. These are two different reference agitations. Each must be recorded separately (§3.4 "reference agitation"), and the curve family cannot be combined with the CI curve without an agitation conversion. |
| HC-110 characteristic curve for Tri-X 400 | **not published** | The only HC-110 (B) characteristic curve in K1 is for **TRI-X 320 (320TXP) sheets** (Large Tank, 1-min agitation; 4, 6, 9, 14 min), p. 8 | §7.1 "curve shape from D-76" for Tri-X+HC-110 stays correct. Do not use the 320TXP HC-110 curve for 400TX. |
| Spectral sensitivity | **confirmed** | p. 7: "Spectral Sensitivity Curves": **log sensitivity**, where sensitivity = reciprocal of the exposure (erg/cm²) needed for a specified density. Two curves: **D = 0.3 and D = 1.0 above gross fog**. Effective exposure 0.5 s; diffuse visual; 250–750 nm axis. Process not stated on the graph | Record the form (log₁₀ of 1/erg·cm⁻² at two density criteria, equal-energy basis). The curve can be used directly as `S(λ)`. |
| MTF | **confirmed** | p. 7: "Modulation Transfer Function": Process **D-76, Large Tank**; densitometry diffuse visual; spatial frequency 1–600 cycles/mm (log axis); response % (log axis, 1–200). The curve sits slightly above 100 % at low frequency and is plotted to about 80 c/mm | Note that the MTF process (D-76) differs from the "Image Structure" process statement (HC-110 B; see next row). |
| Diffuse RMS granularity is published | **confirmed** | p. 6, "Image Structure": "The data in this section is based on development in KODAK HC-110 Developer (Dilution B), 68 °F (20 °C)". **TRI-X 400: 17 (fine)**; TRI-X 320: 16. Footnote: "Read at a net diffuse density of 1.0, using a 48-micrometre aperture, 12x magnification" | §5.2: G = 17 is confirmed. **The reference developer is HC-110 (B), not D-76.** `a_film`/`m_dev` must therefore be referenced to HC-110 B at 20 °C, so `m_dev(HC-110, B) = 1`. |
| Resolving power | **corrected: not published** | K1 has no resolving-power figure. "High resolving power" appears only as a qualitative feature (p. 1) | Remove "resolving power" from the expected content for F-4017. |
| "Kodak publishes a family of curves for some films" (§3 step 3) | **confirmed** | Tri-X 400: D-76 (4 times), T-MAX Developer (4 times); 320TXP: D-76 and HC-110 B families; Double-X: D-96 (5 times, see §2) | — |
| Processing aim | (context) | p. 2: "starting-point recommendations are intended to produce a **contrast index of 0.56**" | Use CI = 0.56 as the Kodak nominal-contrast target. |
| Time/temperature data | (context for §3.4) | Tables only, not charts: 18, 20, 21, 22, 24 °C for each developer and tank type (pp. 3–5) | See §6 below. |
| Rotary times | (context for §3.4) | p. 4, Rotary-Tube table for 400TX: **identical to the Small-Tank (30 s) times** (for example D-76 6¾ min at 20 °C in both). The push tables are also identical for rotary and small tank (p. 5). For 320TXP sheets the rotary times are shorter than large tank (D-76 5¼ vs 7½ min at 20 °C) | See §6. |
| Reciprocity | (context for §2) | p. 2 table: 1 s → +1 stop (or 2 s), −10 % dev; 10 s → +2 stops (50 s), −20 %; 100 s → +3 stops (1200 s), −30 %. 1/1000 s: +10 % dev; 1/10,000: +½ stop, +15 %; 1/100,000: +1 stop, +20 % | Kodak gives a table plus graphs, not a formula. Short-exposure development changes are also given. |
| Wratten filter factors | (context for §5.1) | p. 1: Nos. 8, 11, 12, 15, 25, 47, 58 and polariser, daylight and tungsten factors (320TXP also lists 29) | Usable as a check on the filter model. |

## 2. Kodak Eastman Double-X 5222/7222 (H-1-5222), §1 challenge 5, §7 row 2

| Claim | Status | Evidence (K2 = H-1-5222, March 2022) | Correction |
|---|---|---|---|
| "Kodak's motion-picture data is for D-96" (§1.5) | **confirmed** | Every curve and every image-structure figure is in **KODAK D-96 at 21 °C (70 °F)**. Processing table (p. 2): D-96, 21 ± 0.3 °C, about 7 min, continuous-immersion machine, agitation "by recirculation through submerged spray jets"; control gamma 0.65–0.70 (Status M blue). No D-76, HC-110 or Rodinal data | — |
| Curves and gamma-vs-time in D-96 (§7) | **confirmed, important caveat** | p. 3 "Sensitometric Curves": Exposure daylight, 1/50 s, intensity-scale sensitometer; Process D-96 at 21 °C; **Densitometry: Status M (Blue)**. Curves at **4 min γ=0.50; 5 min γ=0.56; 6½ min γ=0.66; 9 min γ=0.84; 12 min γ=1.05**. Inset: gamma vs development time and net fog vs time. Log exposure in lux-seconds, axis 3̄.0 to 1.0 | **Status M blue density is not ISO diffuse visual density**, which is the design doc's §0 convention. Double-X curves need a densitometry conversion, or a recorded `densitometry = status_m_blue` with an uncertainty flag. The published contrast measure is **gamma**, not CI. The exposure index (200 tungsten / 250 daylight) is for development to γ = 0.65. |
| Spectral sensitivity | **confirmed** | p. 3: log sensitivity (reciprocal erg/cm²), D = 0.3 and D = 1.0 above gross fog, exposure 1.4 s, D-96 at 21 °C, diffuse visual, 250–750 nm | — |
| MTF | **confirmed** | p. 2: exposure tungsten; D-96 at 21 °C to the recommended control gamma; diffuse visual; 1–600 c/mm axis | — |
| "possibly granularity" | **confirmed (published)** | p. 2: **Diffuse rms granularity 14** (net diffuse visual density 1.0, 48 µm aperture), D-96 at 21 °C. **Resolving power: 32 lines/mm (TOC 1.6:1), 100 lines/mm (TOC 1000:1)** (ISO 6328-like method) | Double-X has a Kodak G value (14, in D-96), so grain amplitude can be Level 2 for the D-96 anchor. |
| Reciprocity | (context) | "No filter corrections or exposure adjustments for exposure times from 1/10,000 of a second to 1 second." No data beyond 1 s | — |
| D-96 formula (§1.5 "add D-96 as anchor") | **confirmed (formula published)** | K5 (H-24.15, rev. 4/05), Table 15-6, "KODAK Developer D-96 (Negative)", tank solution per litre: Elon 1.5 g; sodium sulfite (anhyd.) 75.0 g; hydroquinone 1.5 g; potassium bromide (anhyd.) 0.4 g (or sodium bromide); borax (decahydrate) 4.5 g; water to 1.00 L; pH 8.62 ± 0.05 at 25 °C. D-96R replenisher: Elon 2.0, sulfite 80.0, HQ 2.0, NaBr 0.35, borax 5.0 g, pH 8.72 | Secondary sources often quote "metol 2 g" for D-96. That is the replenisher value or a transcription error. Use 1.5 g (tank). |

## 3. Ilford HP5 Plus and FP4 Plus (Nov 2018 sheets), §1 challenge 6, §2, §7 row 3, §7 ID-11 proxy, §7.1

| Claim | Status | Evidence (I1 HP5 Plus, I2 FP4 Plus; both Nov 2018) | Correction |
|---|---|---|---|
| "characteristic curve(s) in ID-11 and others" (§7) | **corrected** | Each sheet has **one** characteristic curve and **it is not in ID-11.** HP5 Plus: "developed in ILFORD ILFOTEC HC (1+31) stock for 6½ minutes at 20 °C/68 °F with intermittent agitation" (I1 p. 5). FP4 Plus: "roll film developed in ILFORD ILFOTEC HC (1+31) for 8 minutes at 20 °C/68 °F with intermittent agitation" (I2 p. 5). Axes: density 0 to about 2.2; **"Relative log exposure" 0–4 (no absolute lux·s scale)** | The current sheets do not support Level 2 via ID-11. The only curve is Ilfotec HC 1+31, a single time, on a relative exposure axis. Absolute placement must come from the ISO speed (FP4: "ISO speed rating was measured using ILFORD ID-11 developer at 20 °C with intermittent agitation in a spiral tank", I2 p. 1). The HP5 2018 sheet does not name the speed-test developer. |
| "Ḡ-vs-time for several Ilford developers" (§7) | **corrected: not in current sheets** | Neither the HP5 Plus nor the FP4 Plus 2018 sheet contains any average-gradient or contrast-vs-time chart. The ID-11 developer sheet (I3, Aug 2024) states only that its times "should produce negatives of normal contrast, **typically around a Gbar of 0.62**" | Remove the expectation. Use Ḡ ≈ 0.62 as Ilford's nominal target, with **one point per developer and dilution** (the published time). Older Ilford sheets (pre-2018) may have had contrast-time curves; **unconfirmed**, as no older copy could be retrieved (digitaltruth mirror returned HTTP 403). |
| Spectral sensitivity "(wedge spectrogram)" | **confirmed, with a modelling caveat** | Both sheets: "Wedge spectrogram to tungsten light (2850K)". Relative "Sensitivity" scale (0.5, 1.0 ticks), about 380–670 nm | A wedge spectrogram to a 2850 K source **includes the source's spectral power distribution**. It is not an equal-energy spectral sensitivity. To get `S(λ)` for §5.1, divide by the 2850 K (CIE A-like) SPD. Record this as an approximate digitisation (Level 1+). |
| "Ilford does not publish RMS granularity in its current data sheets" (§1.6) | **confirmed** | No granularity, MTF or resolving-power data in I1 or I2 | — |
| Reciprocity power law (§2) | **confirmed** | HP5 Plus: "The graph is based on the formulae **Ta = Tm^1.31**" (I1 p. 2). FP4 Plus: **Ta = Tm^1.26** (I2 p. 2). Both: no adjustment for exposures between ½ s and 1/10,000 s; the law applies above ½ s (graph axes 0–50 s metered, 0–200 s adjusted) | Record the exponent per film and the validity range (> ½ s). |
| "development tables that include some Kodak developers" (§7); includes D-76 / HC-110 / Rodinal? | **confirmed** | **HP5 Plus, EI 400, 20 °C, intermittent agitation (I1 p. 3):** Kodak D-76 stock 7½, 1+1 11, 1+3 22 min; Kodak HC-110 A 2½, B 5 min; Agfa Rodinal 1+25 6, 1+50 11 min (pushes also listed, e.g. D-76 stock EI 800 9½, EI 1600 12½). **FP4 Plus, EI 125 (I2 p. 3):** D-76 stock 8, 1+1 11, 1+3 16; HC-110 A 4½, B 9; Rodinal 1+25 9, 1+50 15 min (EI 50 and EI 200 columns also given). Ilford adds that third-party times "are only a general guide" | — |
| Agitation reference | (context for §3.4) | Spiral tank: "invert the tank four times during the first 10 seconds, then invert the tank four times again during the first 10 seconds of each further minute" (I1/I2 p. 5) | The Ilford reference agitation differs from Kodak's small-tank reference (5 s every 30 s). This confirms §3.4's premise. |

## 4. ID-11 vs D-76 formulas (§7 "ID-11 as a proxy for D-76")

| Claim | Status | Evidence | Correction |
|---|---|---|---|
| "The published formulas of ID-11 and D-76 are generally regarded as equivalent" | **confirmed (published formulas identical), with caveats** | **ID-11** (I4, Ilford Formulae & Packed Chemicals, 1953, p. 14): Metol 2 g; sodium sulphite cryst. 200 g (the book says to halve this for anhydrous, i.e. 100 g); hydroquinone 5 g; borax 2 g; water to 1000 cc. **D-76** (K7, Kodak Chemicals and Formulae, p. 14): 'Elon' 2.0 g; sodium sulphite cryst. 200.0 g (or anhydrous 100.0 g); hydroquinone 5.0 g; borax 2.0 g; water to 1000 cc. **The published formulas are identical.** Caveats: (a) I4 p. 4 "Terms of Business": packed chemicals "will have approximately the same characteristics … the ingredients and weights used by us … may not be the same in all respects as those published". (b) **Neither current manufacturer document discloses the formula.** Kodak J-78 (2017) and Ilford ID-11 sheet (2024) contain no formulation. (c) Ilford's own times differ slightly: HP5 ID-11 stock 7½ = D-76 stock 7½; FP4 ID-11 stock 8½ vs D-76 8; ID-11 1+3 20 vs D-76 1+3 16 (I1, I2) | Keep `method = proxy`, citing the identical *historical* published formulas and noting that the current packaged products are undisclosed. Note: because the Ilford sheets contain no ID-11 sensitometry (see §3), the proxy currently gives only **time points**, not curves. |
| "D-76 is published" (§7 row 4, J-78) | **corrected** | K3 (J-78, Dec 2017) contains **no formula**. The formula is in historical Kodak formularies (K7). Search snippets also name a "Kodak Formulary" (https://125px.com/docs/techpubs/kodak/Kodak_formulary.pdf); not opened | Cite K7 (or another historical Kodak formulary) for the formulation, not J-78. |

## 5. Kodak D-76 (J-78) and HC-110 (J-24), §7 row 4, §3.4

| Claim | Status | Evidence | Correction |
|---|---|---|---|
| J-78 gives times, temperature charts, dilutions, agitation | **partly corrected** | K3: **dilutions: full strength and 1:1 only** (no 1:3). **Times:** tables at 18/20/21/22/24 °C (small tank, 30 s intervals; large tank, 1 min intervals; tray continuous; rotary). **No temperature chart.** **Agitation:** detailed. Small tank: initial agitation "up to 5 cycles", then "agitate for 5 seconds at 30 second intervals … 2 to 5 cycles". Large tank: continuous for the first 15–30 s, then lift-drain-tilt once a minute. Tray: continuous. Sheet tank: once a minute. Also: "If you process one 135-36 roll in a 237 mL tank … increase the development time by 10 percent" | Use tables, not a chart, for α. **Caution:** J-78 lists only the legacy "TRI-X Pan" and "TRI-X Pan Professional", not 400TX. Its TRI-X Pan time (small tank, 20 °C: 8 min) differs from F-4017's 400TX time (6¾ min). **Use F-4017 for Tri-X 400.** |
| J-24 gives formulation, times, temperature charts, dilutions, agitation | **partly corrected** | K4: **no formula**. Dilutions from concentrate: **A 1:15, B 1:31, C 1:19, D 1:39, E 1:47, F 1:79**; from stock (1+3): A 1:3, B 1:7, C 1:4, D 1:9, E 1:11, F 1:19. Times at 18–24 °C (tables), small tank 30 s, large tank 1 min, rotary continuous. **No temperature chart.** 400TX dilution B: small tank 3¾ min at 20 °C; rotary 3¾ min (the same) | **§3.4 lists "HC-110 B/E/H". Dilution H is not a Kodak dilution** (K4 defines A–F only). Replace H with a Kodak dilution or mark it as non-manufacturer (Level 1). Also note that J-24 gives 400TX times **only for dilution B** (and the legacy TRI-X Pan for A and B). |
| "Manufacturers state that continuous (rotary) agitation needs roughly 10–15 % less time" (§3.4) | **corrected** | **Ilford:** "Where continuous agitation is used … reduce these times by **up to 15%**. For use in rotary processors without a pre-rinse, reduce the spiral tank development times by **up to 15%**" (I1/I2 p. 3). ID-11 sheet: "Continuous agitation reduces the recommended development times by **about 15%**" and rotary "reduced by around 15%" (I3). **Kodak states no percentage.** It publishes separate rotary tables. For **400TX the rotary times equal the small-tank (30 s) times** (F-4017 p. 4 and p. 5; J-24 p. 6), i.e. **0 % reduction**. For 320TXP sheets the rotary D-76 time is 5¼ vs 7½ min large tank (−30 %) and 6 min tray-continuous (−12.5 %) | Replace with: "Ilford: up to 15 % (film sheets), about 15 % (ID-11 sheet). Kodak: no general figure; film-specific rotary tables (400TX: no reduction relative to the 30-s small-tank scheme)". `r_A` must be calibrated per source and not taken as a universal constant. |

## 6. Time/temperature compensation (§3.4 "Ilford and Kodak both publish one", §7 Ilford chart row, §8.2)

| Claim | Status | Evidence | Correction |
|---|---|---|---|
| "Ilford and Kodak both publish one [time/temperature compensation chart]" | **corrected** | **Ilford** publishes one **generic chart** (°C and °F versions) in each film sheet (I1/I2 p. 4; temperature axis 14–24 °C, time axis 1½–32 min). It applies to all developers. Worked examples: HP5 sheet, "if 6 minutes at 20 °C … at 23 °C will be 4½ min and at 16 °C will be 9 min"; FP4 sheet, 4 min → 3 min (23 °C) and 6 min (16 °C). The ID-11 sheet (I3) states the rule: "increase … by **10% for each 1 °C** drop … decrease … by 10% for each 1 °C rise" (range 20–24 °C). The same diagonal-line chart appears in I4 (1953). **Kodak** publishes **developer-specific tables** at 18/20/21/22/24 °C (F-4017, J-78, J-24), not a chart | Ilford gives one developer-independent α (derived from its examples, α ≈ 0.096–0.10 per °C). It cannot be "fitted per developer". Kodak's tables allow per-developer α. Derived from F-4017 400TX small-tank tables (my calculation, ln(t18/t24)/6): D-76 ≈ 0.087/°C, HC-110 B ≈ 0.098/°C, XTOL ≈ 0.087/°C, **D-76 1:1 ≈ 0.055/°C**. So α does depend on developer and dilution. The "18–24 °C" validity range is confirmed for Kodak. Ilford's chart extends to about 14 °C. |

## 7. Rodinal / Adox Adonal (§7 row 6, §3.4 dilutions)

| Claim | Status | Evidence | Correction |
|---|---|---|---|
| Rodinal (Adonal) data: "times at 1+25/1+50, temperature notes" | **confirmed for Agfa Rodinal leaflet; Adox-branded Adonal sheet unconfirmed** | A1 (AGFA RODINAL leaflet, a&o imaging solutions; served by Fotoimpex as the Adonal "PDF-Datasheet"): dilutions **1+25 and 1+50**; "Developing times at 20 °C"; small tank; agitation "continuously for the first minute, and then tilt every 30 seconds"; **Diagram 1: "Contrast γ = 0,65"**. Read from a low-resolution image: Kodak Tri-X 7 / 14 min; Ilford HP5 Plus 8 min (1+25 only); Ilford FP4 Plus 8 / 18 min. Double-X is not listed. **Diagram 2:** time/temperature graphs 18–24 °C **for Agfapan APX 100 and APX 400 only**. Retailer page (Fotoimpex, [S]): Adonal dilutions "1+25 to 1+500", "based on Agfa Leverkusen's 2004 Rodinal formula" | Note that the only manufacturer leaflet found is Agfa-branded, with γ = 0.65 as its contrast aim. It has no temperature data for the MVP films and no 1+100 times. **Ilford and Agfa times disagree** (HP5 1+25: Ilford 6 min vs Agfa 8 min), which supports keeping Rodinal at Level 1. Rodinal 1+100 (§3.4) has no manufacturer time for the MVP films. Agfa/Adonal formula not disclosed in A1. |

## 8. Definitions (§8.2 "verify both definitions")

| Claim | Status | Evidence | Correction |
|---|---|---|---|
| Kodak Contrast Index definition | **confirmed** | K6 (H-740, Nov 2006): mark a straight edge at 0.0, at 0.2 log E, and at 2.2 log E (i.e. 2.0 further). Place the 0.0 mark on the D-min (base+fog) line, with the 0.2 and 2.2 marks both touching the curve. "The slope of this straight line is the Contrast Index." CI includes the toe, unlike gamma | Implement exactly: find h₁ < h₂ with h₂ − h₁ = 2.0, both on the curve, such that the line through them passes through (h₁ − 0.2, D_min). CI = slope. |
| Ilford Ḡ (average gradient) definition: span from 0.1 above fog over 1.5 log exposure | **unconfirmed (secondary only)** | No Ilford primary document defining Ḡ was found. Forum sources [S] (photrio.com threads "Average Gradient Methods", "Effective film speed (as in Lambrecht's method)"): Ḡ is the slope from the point 0.10 above base+fog to the point 1.50 log-H further. Ilford uses the term "Gbar" with a target of about 0.62 (I3) but does not define it there | Treat the definition as provisional until it is checked against an Ilford primary source (e.g. *The Ilford Manual of Photography*, or an older Ilford technical sheet). |

## 9. Wratten filter data (§5.1 "[verify publication]")

| Claim | Status | Evidence | Correction |
|---|---|---|---|
| "Wratten 8, 11, 15, 25 and 29 transmission data are published by Kodak" | **partly corrected** | K8: Kodak's WRATTEN 2 page lists **Nos. 8, 15, 25, 29** with downloadable curves. **No. 11 is not listed.** The No. 25 PDF is a **graph of diffuse density vs wavelength (300–900 nm)**, not a table ("typical of production coatings"). Tabulated spectral data historically appeared in Kodak Publication B-3 (*Kodak Filters for Scientific and Technical Uses* / *Photographic Filters Handbook*); not retrieved (Internet Archive copy access-restricted) [S] | No. 11 needs another source (historical B-3, or a measurement). The curves must be digitised from density plots: T = 10^(−D). |

## 10. Citations (§3.5, §5.3)

| Claim | Status | Evidence | Correction |
|---|---|---|---|
| C. N. Nelson, *Photogr. Sci. Eng.* 15, 1971 | **confirmed (citation details from secondary source); equation form unconfirmed** | P1 reference list [S]: "Nelson, C. N., *Photographic Science and Engineering*, vol. 15, No. 1, Jan.–Feb. 1971, pp. 82–97". An RIT thesis bibliography [S] gives the title as "Prediction of Densities in Fine Detail…" (title not fully confirmed). P1 describes the model as a 1-D density profile using a "chemical spread function φ(ξ)", with terms "the integral" and "B·D²(x)" that cancel for patches ≥ 1 mm. That is consistent with the design doc's form `D − B·D·[(s∗D) − D] = D + B·D² − B·D·(s∗D)`. The equation itself is an image in P1 and was not seen. P1 also cites Dainty & Shaw, *Image Science* (1974) pp. 53–55, which reportedly reproduces it | Use the citation: Nelson, C. N., *Photogr. Sci. Eng.* **15**(1), 82–97 (1971). Check the exact equation against the paper or Dainty & Shaw before implementing. |
| Newson et al., "Realistic Film Grain Rendering", IPOL 2017 | **confirmed** | N1: Alasdair Newson, Noura Faraj, Bruno Galerne, Julie Delon; *Image Processing On Line* **7** (2017) 165–183; DOI 10.5201/ipol.2017.192; published 2017-07-18. The abstract says only "a physically realistic film grain model" (it does not use the word "Boolean"; the body of the paper was not read) | Use the full citation with DOI. |

## 11. Way Beyond Monochrome (§7 "[verify which combinations]")

| Claim | Status | Evidence | Correction |
|---|---|---|---|
| Lambrecht & Woodhouse publish independent sensitometric tests of film/developer pairs | **unconfirmed** | The book could not be accessed. Secondary [S]: a PDF fragment of the book shows a figure comparing **Pan F and HP5** characteristic curves in **ID-11, Perceptol and Rodinal**. A forum post refers to a graphical CI calculator in the book. A GitHub project (RafalLukawiecki/film_tests) uses the book's CI targets. No evidence found for Tri-X, FP4 Plus or Double-X curves | Keep it as a possible cross-check only. Do not assign Level 2 until the book is consulted. If confirmed, the HP5+Rodinal curve would be the only published Rodinal sensitometry for an MVP film. |

---

## 12. Consequences for §7.1 starting levels

| Combination | Design doc | Supported by sources found | Comment |
|---|---|---|---|
| Tri-X + D-76 | 2 | **2** | D-76 curve family (large tank, 1 min) plus CI-vs-time (small tank, 30 s). Agitation references differ and must be reconciled |
| Tri-X + HC-110 | 2− | **2−** | CI-vs-time for HC-110 (B), small tank; no 400TX curve |
| HP5 / FP4 + D-76 | 2− (ID-11 proxy) | **1+** | Current Ilford sheets have no ID-11 curve and no Ḡ-vs-time. Only an Ilford time point for D-76 itself, plus Ḡ ≈ 0.62 nominal |
| HP5 / FP4 + HC-110 | 1+ | 1+ | Ilford time points (A and B). One Ilfotec HC 1+31 curve exists (relative log E). Ilfotec HC ≈ HC-110 is **not** established (times differ: HP5 HC 1+31 6½ vs HC-110 B 5 min) |
| Double-X + any MVP developer | 1 (D-96 shape) | 1 | D-96 family with gamma labels, granularity 14 and MTF all exist, but they are Status M blue densitometry |
| any + Rodinal | 1 | 1 | Agfa leaflet γ = 0.65 time points; Ilford times disagree |
