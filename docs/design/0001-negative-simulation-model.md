# Design 0001 — Negative Simulation Model

Status: **Draft for review** · Date: 2026-10-20 · Scope: Phase 1 (mathematical model) and the shape of Phase 2 (Rust core)

This document defines the mathematical and data models for the negative simulation engine: how a linear
scene image becomes a simulated, developed black-and-white negative. Printing appears only where it
constrains what the negative must store.

Conventions used throughout:

- `log` means log10 unless written `ln`.
- `H` is photometric exposure at the film plane in lux·seconds; `h = log H`.
- `D` is **ISO diffuse transmission density**, which is what manufacturers publish. Specular (Callier)
  density is a printing concern; see §3.6.
- Source claims were checked against the primary documents on 2026-10-20. Evidence is in
  [`docs/research/source-inventory.md`](../research/source-inventory.md). **[unconfirmed]** marks the few
  claims that could not be checked.
- Exception to the diffuse-density convention: Kodak's Double-X 5222 curves are published in **Status M
  blue** density (motion-picture print practice), not ISO diffuse visual density. Calibrations derived from
  them must record the density type and convert or flag it.

---

## 0. Summary of decisions

| # | Decision | Rationale |
|---|----------|-----------|
| D1 | The pipeline works on **absolute log exposure** (log lux·s) at the film plane, not normalised image values. | Characteristic curves, ISO speed and published data are defined against absolute exposure. Without an absolute exposure axis, EI has no physical meaning. |
| D2 | **Exposure index (EI) is an exposure setting, not a processing setting.** | EI decides where the scene lands on the curve. Development decides the shape of the curve. Push processing is the two applied together, not a separate mode. |
| D3 | Each characteristic curve is a **development-growth family** `D(h, τ) = D_fog(τ) + D∞(h)·(1 − e^(−τ/τc(h)))`, fitted to manufacturer curves and contrast-vs-time data. | It reproduces the observed saturating rise of contrast with time. It has few parameters, so it can be fitted from sparse data, and it can be tested against curve families where manufacturers publish them. |
| D4 | Time, temperature and agitation reduce to a single **effective development** value `τ`. Dilution is **calibrated per published dilution**, not modelled continuously. | Published data exists only at discrete dilutions. Dilution changes compensation, solvent action and acutance as well as rate. A continuous model would be invented physics. |
| D5 | **Chemical formulations are descriptive metadata in the MVP.** They do not drive behaviour. | No published model predicts a film/developer curve from developer chemistry with useful accuracy. Behaviour comes from empirical calibrations. The formulation is kept so future models can use it. |
| D6 | Optical spread (irradiation) is applied to **linear exposure**. Adjacency (edge) effects are applied to **density**, using a Nelson-type model that is exactly zero on uniform fields. | That is where each effect happens physically. Zero response on uniform fields keeps large-area sensitometric calibration intact. |
| D7 | **The grain amplitude is fixed by published RMS granularity through a Poisson/Selwyn relation.** Only the spatial shape (clump scale) is an approximate, Level-1 parameter. | It ties grain to measured data. It is independent of resolution by construction, so previews and full-resolution renders agree statistically. |
| D8 | Grain uses a **counter-based hash RNG** keyed on film-plane pixel coordinates and seed. | Output is identical regardless of thread count, tile size or processing order. |
| D9 | The negative stores **f32 total diffuse density** plus metadata: base+fog, geometry, the versions of every calibration used and a recipe hash. | Density fits comfortably in the f32 range and precision. The metadata makes negatives reproducible and reusable for any number of prints. |
| D10 | Add a **`bw-negative` crate** that composes film + developer + processing, and keep calibration data in a **versioned `data/` tree** instead of compiling it into `bw-profiles`. | The original layout gave negative rendering no home that could depend on both film and developer. Data-driven calibrations must be replaceable without recompiling. |
| D11 | **Spectral emulation is a 3-weight linear projection** of scene-linear RGB, fitted on natural reflectance spectra. Monochrome input skips spectral emulation completely. | Three channels cannot reconstruct a spectrum. A fitted projection is the best defensible estimate. One channel carries no spectral information at all. |

---

## 1. Challenges to the brief

These points change the design. Agree with them or overrule them before implementation starts.

1. **Most of "push speed" is underexposure.** Extended development raises mid-tone and highlight density
   much more than shadow density. The real speed gain at the ISO speed point is usually a fraction of a
   stop, even for a "two-stop push". The model must produce that result on its own. It must not contain a
   "push" parameter that adds speed.
2. **Developer chemistry cannot yet predict behaviour.** Keeping formulation separate from behaviour is the
   right split. However, the behaviour has to be empirical (per film, developer and dilution) for the
   foreseeable future. Deriving it from ingredient concentrations would be speculation presented as physics.
3. **Spectral emulation from RGB is limited, and from monochrome RAW it is impossible.** A Monochrom
   file has one channel with the sensor's own spectral response. No processing can recover how Tri-X would
   have rendered a red door compared with a green one. The only real spectral control for that camera is a
   physical filter at capture. The engine should record which filter was used, not pretend to simulate one.
4. **White balance hides the illuminant.** Converted RGB TIFFs are already white-balanced. Panchromatic film
   under tungsten light renders reds lighter than under daylight. With white-balanced input the engine
   cannot know about that shift. The MVP therefore assumes white-balanced input and a stated reference
   illuminant (default D55). Emulation that accounts for the illuminant needs un-balanced camera data,
   which belongs to the RAW phase.
5. **Double-X 5222 has no manufacturer data for D-76, HC-110 or Rodinal.** Kodak's data (H-1-5222) is for
   D-96 only: a curve family at 4–12 min at 21 °C, in Status M density. I recommend adding **D-96 as a
   calibration anchor for Double-X** even though it is not on the MVP developer list. Otherwise all three
   Double-X combinations are Level 1.
6. **Ilford does not publish RMS granularity** in its current data sheets. Kodak publishes it for Tri-X
   (17) and Double-X (14). The Tri-X figure is measured **in HC-110 (B)**, not D-76. FP4 Plus and HP5 Plus
   grain amplitude will be estimated from Level-1 comparisons until we measure it.
7. **The Ilford data sheets are thinner than assumed.** HP5 Plus and FP4 Plus each publish a single
   characteristic curve, in Ilfotec HC, on a *relative* exposure axis. There is no contrast-vs-time chart.
   They cannot support Level 2 for any MVP developer on their own.
8. **Grain at camera resolution is near the grain scale.** A 6048-pixel-wide image of a 36 mm frame has a
   pixel pitch of about 6 µm. That is only a few times the effective grain size. Gaussian grain models are
   valid when many grains fall in each pixel and are only approximate at this scale. A Boolean (random
   disc) model is the physically correct upgrade, deferred to Phase 6.
10. **Agitation is part of every published data set.** For Tri-X, Kodak's D-76 curve family is for a large
    tank agitated once a minute, while its contrast-index curves are for a small tank agitated every 30 s.
    Data from different sources can only be combined through an explicit agitation conversion.
9. **Published MTF curves already include adjacency effects.** A film MTF above 1.0 at low frequencies is
   the developer's edge effect. Optical spread and adjacency must be fitted together against the published
   MTF. Adding a separate "acutance" boost on top would count the effect twice.

---

## 2. Internal image and exposure representations

### 2.1 Planes and geometry

All image data is **planar `f32`**, one `Vec<f32>` per channel. Planar layout is better for SIMD, separable
filters and channels that are processed independently. Precision: f32 has a 24-bit mantissa. Over the
range used here (log exposure −5…+2, density 0…4) its resolution is under 1e-6, far below any perceptible
or measurable difference. Model fitting runs in f64.

Every image in the negative pipeline carries a **`FilmGeometry`** that maps pixels to film-plane
micrometres:

```
pixel_pitch_um = frame_width_mm * 1000 / width_px        (square pixels assumed; aspect is checked)
```

Grain, irradiation and adjacency are all defined in µm on the film. This makes their appearance correct
for any resolution and any film format: 35 mm, 645, 6×6, 4×5. The same parameters work at preview and
full resolution.

### 2.2 Scene input

```
enum SceneImage {
    Rgb  { r, g, b: Plane, primaries: RgbPrimaries },   // scene-linear, white-balanced
    Mono { y: Plane, capture_filter: Option<FilterId> } // native sensor response, never reconstructed to RGB
}
```

The MVP input is a 16-bit integer or 32-bit float **linear** TIFF. Its primaries come from a CLI flag
(linear Rec.2020, linear ProPhoto or XYZ). A future version will read them from ICC/DNG metadata. One way
to make such files is LibRaw's `dcraw_emu` in linear 16-bit TIFF mode with an explicit output colour
space **[unconfirmed flags]**. This is test-harness tooling, not part of the engine.

16-bit integer linear files quantise the deepest shadows coarsely, because the bottom stops get few code
values. Float TIFF should be preferred where available. The engine always converts to f32 on load.

### 2.3 Film response (spectral stage)

The output is a single plane `E` that is linear in scene radiance and weighted by the film's spectral
sensitivity. For a neutral surface, `E = Y`. See §5.1 for the model.

### 2.4 Exposure model

Scene values are relative, so an anchor is required. The engine uses the same calibration as a
reflected-light meter (ISO 2720), which is also what makes ISO film speed meaningful.

For an averaging meter with calibration constant `K = 12.5` and lens transmission factor `q ≈ 0.65`, the
film-plane exposure given to a metered mid-tone at exposure index `EI` is:

```
H_m = q·K / EI ≈ 8.125 / EI   lux·s
```

The ISO 6 speed point for B&W film is `H_s = 0.8 / S`. With `EI = S`, the metered mid-tone therefore sits
`log(8.125/0.8) ≈ 1.0` log units, about 3.3 stops, above the speed point. This is the standard result and
is used as a test (§8.2).

Mapping from the film-response plane to exposure:

```
H(x,y) = (E(x,y) / E_grey) · (q·K / EI) · 2^(EV_comp)  +  H_flare
h(x,y) = log10( max(H, H_floor) )
```

- `E_grey` is the scene value that the photographer "metered" as mid-grey. The default is 0.18 for
  scene-referred data normalised so that a white reference is 1.0. An `auto` mode uses the log-average of
  `E`, like an averaging meter. It is computed as a reduction in a fixed order so the result is
  deterministic.
- `EV_comp` is ordinary exposure compensation.
- `H_flare = f · mean(H)` is optional **camera veiling flare**. Real camera exposures on film include a
  flare level of roughly 1–2% of mean exposure that lifts the deep shadows into the toe. A digital source
  has already been through a lens with its own flare. The default is therefore `f = 0`, and the parameter
  is exposed and documented. This is Level 1.
- `H_floor` is a tiny positive constant that keeps `log` finite. It is set far below the toe, so it never
  affects density.

Output: an `ExposureMap { log_h: Plane, geometry }` holding absolute log lux·s.

**Reciprocity failure** is deferred. It needs the shutter time, which linear TIFF does not carry. The
calibration files reserve a field for it; Ilford publishes a reciprocity power law for FP4 Plus and HP5
Plus (`Ta = Tm^1.31` for HP5 Plus and `Ta = Tm^1.26` for FP4 Plus, for metered times over ½ s).

---

## 3. Characteristic curves and development

### 3.1 Requirements

A curve representation must:

- be monotone, non-decreasing in `h`;
- reproduce digitised manufacturer curves within digitisation error;
- vary in a principled way with development, so one calibration covers a range of times;
- extrapolate safely, with linear tails outside the calibrated range and a warning;
- evaluate cheaply per pixel.

### 3.2 Base representation: monotone cubic spline

Digitised `(h, D)` points are interpolated with a **monotone piecewise-cubic Hermite spline**
(Fritsch–Carlson, or Steffen's method, which avoids overshoot by construction). Outside the knots the curve
continues linearly with the end slopes. The lower tail has slope clamped to ≥ 0. Above the last knot the
shoulder slope is clamped to ≥ 0, and the extrapolated region is flagged.

For per-pixel speed, the evaluation path samples the spline into a **uniform LUT** (4096 entries over
`h ∈ [−5, +2]`) with linear interpolation. The spline remains the reference implementation, and a test
requires `max |LUT − spline| < 1e-4 D`.

A parametric sigmoid is **not** used as the primary representation. Real toes and shoulders are
asymmetric and differ between emulsions. Parametric fits impose a shape the data may not have. Parametric
forms can still be used in fitting tools.

### 3.3 Development-growth family

The classical empirical result is that, at fixed exposure, density approaches a limit exponentially as
development time increases. As a consequence, gamma or contrast index rises and saturates with time.
This is the shape seen in every manufacturer contrast-vs-time chart.

```
D(h, τ) = D_b + D_f(τ) + D∞(h) · G(h, τ)

G(h, τ)  = 1 − exp( −τ / τc(h) )
τc(h)    = τc0 · (1 + κ · D∞(h) / D_ref)          (κ = 0 for normal development)
D_f(τ)   = f0 + f1 · τ                           (fog growth)
```

| Symbol | Meaning | Source |
|--------|---------|--------|
| `D_b` | base density | data sheet or measurement |
| `D∞(h)` | asymptotic net density at full development (monotone spline) | derived from the reference curve (below) |
| `τc0` | development time constant at reference conditions | fitted to contrast-vs-time data |
| `κ` | compensation: slower effective development where density is high, from local exhaustion | 0 normally; Level 1 for stand and semi-stand |
| `f0`, `f1` | fog and fog growth | data sheet curves; otherwise Level 1 |

Contrast follows from the model:

```
γ(τ) ≈ γ∞ · (1 − e^(−τ/τc0))       (κ = 0)
```

**Fitting procedure for one combination of film, developer and dilution.** Use the best data available, in
this order:

1. **Curve family available** (Tri-X in D-76: 6, 8, 10, 12 min; Double-X in D-96: 4–12 min). Fit `D∞(h)`
   and `τc0` jointly to every curve in the family. This is the strongest case.
2. **Contrast-vs-time plus one reference curve.** Fit `γ∞` and `τc0` to the published contrast index (Kodak
   CI) against time: two parameters, typically 4–6 points. Then recover the asymptotic curve from the
   reference curve at `t_ref` with `D∞(h) = D_net,ref(h) / (1 − e^(−t_ref/τc0))`. If the curve and the CI
   data used different agitation, convert the CI data to the curve's agitation first (§3.4).
3. **Times only** (manufacturer or community time for "normal" contrast, such as Kodak's target CI 0.56 or
   Ilford's Ḡ ≈ 0.62). This gives one point on the contrast-vs-time curve. `τc0` is taken from a
   comparable, better-documented combination, and the result is Level 1.

**Validate:** with a family, hold out one curve and predict it. If the toe residuals are systematic, enable
an `h`-dependent `τc(h)` as a monotone spline. Do not invent a different functional form.

Behaviour that emerges without special cases:

- **Push:** the higher EI moves the scene down the curve, and the larger `τ` raises mid-tone and highlight
  density more than toe density. Shadows are lost, mid-tones and highlights gain contrast, and fog and
  grain rise (§5.3). There is no "push" parameter.
- **Pull:** the reverse. Lower contrast and full shadows.
- **Compensating and stand development:** `κ > 0` slows development where density is high. That
  compresses the highlights over large areas, while local adjacency (§3.5) gives the edge effects.

**Limitations.** The exponential growth law is an empirical description, not a kinetic model of
development. It describes development *within* the supported range of times. It does not predict
behaviour far outside that range, and the calibration records the valid range (§6.4).

### 3.4 Effective development `τ`

```
τ = t · r_T(T) · r_A(agitation)

r_T(T) = exp( α · (T − T_ref) )          T_ref = 20 °C
```

- **Temperature:** `α` is fitted **per developer and dilution** from Kodak's per-developer time tables
  (18–24 °C in F-4017). Fits to those tables give roughly 0.087/°C for D-76 stock, 0.098/°C for HC-110 B
  and 0.055/°C for D-76 1:1. These are our derivation, not Kodak figures, and confirm that a single
  constant is wrong. Ilford publishes one chart for all developers (about 10% per °C). That is acceptable as
  a Level-1 default but cannot be fitted per developer. The model is valid only inside the published
  range.
- **Agitation:** each calibration records **the reference agitation scheme its source data used**,
  because Kodak and Ilford schemes differ (Challenge 10). `r_A` is a calibrated multiplier relative to
  that reference. Manufacturers disagree on continuous agitation: Ilford says to reduce time by "up to 15%"
  (film sheets) or "about 15%" (ID-11 sheet), while Kodak's 400TX rotary times equal its small-tank times.
  `r_A` is therefore Level 1, and the default follows the manufacturer of the calibration's source data.
  Agitation also controls the strength of exhaustion effects. It therefore scales `κ` and the adjacency
  strength `B` (§3.5) as well as `r_A`.
- **Dilution is not part of `τ`.** Each published dilution (D-76 stock and 1:1; HC-110 A–F as defined in
  Kodak J-24, of which B is the usual; Rodinal 1+25/1+50; 1+100 is community practice only) is a separate
  calibration entry, because dilution changes several things: rate,
  compensation (`κ`), solvent action (grain, §5.3) and acutance (`B`). Interpolation between two published
  dilutions in log-concentration is allowed for `τc0` and `γ∞` only, and is flagged. Dilutions outside the
  published set are refused unless extrapolation is explicitly enabled.

```
enum Agitation {
    Continuous,                                        // rotary
    Intermittent { initial_s: f32, interval_s: f32, duration_s: f32 },
    SemiStand    { initial_s: f32, mid_agitations: u8 },
    Stand        { initial_s: f32 },
}
```

The agitation enum maps to a scalar **agitation index** `a ∈ [0, 1]`, where 0 is stand and 1 is
continuous. `r_A`, the `κ` scaling and the `B` scaling are monotone functions of `a`, fitted per
calibration and Level 1 unless measured.

### 3.5 Adjacency (edge) effects

During development, developer becomes exhausted and bromide is released near dense areas. Both diffuse
sideways, which suppresses density next to dense regions (Eberhard and Kostinsky effects, Mackie lines).
This raises acutance and pushes the film MTF above 1 at low frequencies.

A **Nelson-type** model (C. N. Nelson, "Prediction of densities in fine detail in photographic images",
*Photogr. Sci. Eng.* 15(1), 82–97, 1971; citation from a secondary reference list, **[unconfirmed]**
equation form against the original) applied in the density domain:

```
D_adj(x) = D(x) − B · D(x) · [ (s ∗ D)(x) − D(x) ]
```

- `s` is a normalised, radially symmetric spread function with diffusion length `ℓ_adj`, typically tens of
  µm. It is implemented as a Gaussian or a sum of Gaussians.
- `B` is the strength, which depends on developer, dilution and agitation. It is low for high-sulfite
  developers such as D-76 stock, higher for dilute low-sulfite developers such as Rodinal 1+50, and higher
  again with reduced agitation.
- **The term is exactly zero on uniform fields**, so the large-patch sensitometry from §3.3 is preserved.
  This is a required property and has its own test.

Clamping: `D_adj ≥ D_b`.

### 3.6 Optical spread (irradiation)

Light scatters inside the emulsion during exposure. It is applied to **linear `H` before the curve**:

```
H'(x) = (p ∗ H)(x)
```

`p` is modelled as an exponential line-spread (Lorentzian-type MTF), which suits a turbid medium. For the
MVP it is approximated by a sum of two Gaussians.

**Joint calibration:** the published film MTF `M_pub(f)` is fitted as `M_opt(f) · A(f)`. Here `A(f)` is the
small-signal frequency response of the adjacency model, linearised around the test density. `ℓ_adj` and
`B` are shared with §3.5. This avoids double-counting (Challenge 8).

**Callier effect:** condenser enlargers see higher effective contrast than diffusion enlargers (Callier
`Q` > 1), and the size of the effect depends on the grain structure. That is a printing parameter. The
negative therefore stores diffuse density and `bw-darkroom` applies `Q`.

---

## 4. Stage order

```
SceneImage ──► [spectral] E (linear) ──► [exposure] H (linear, lux·s)
          ──► [irradiation] H' ──► log ──► [curve family, τ] D_mean
          ──► [adjacency] D_adj ──► [grain] D ──► Negative
```

Each stage declares a **halo radius in µm**, the support of its kernel. Tiled execution can then compute
overlapping tiles exactly. The MVP may run on the whole frame in memory. At 60 MP each f32 plane is about
240 MB, so the pipeline must reuse buffers. The halo contract keeps tiling possible later without changing
the API.

---

## 5. Spectral and grain models

### 5.1 Spectral projection

Film response to a surface with reflectance `R(λ)` under illuminant `I(λ)`, through an optional lens filter
`F(λ)`:

```
e_film = ∫ S_film(λ) · F(λ) · I(λ) · R(λ) dλ
```

From scene-linear RGB the engine can only form `ê = w_r·R + w_g·G + w_b·B`. The weights `w` are fitted
**offline** by least squares over a training set of natural reflectance spectra `R_i`:

```
w = argmin Σ_i ( e_film(R_i) − Σ_c w_c · ∫ c̄_c(λ) I(λ) R_i(λ) dλ )²    subject to   ê(neutral) = Y(neutral)
```

- `c̄_c` are the colour-matching functions of the working primaries (CIE 1931 2° transformed to those
  primaries).
- The constraint normalises the result so that the meter, which is calibrated on neutrals, behaves
  correctly.
- **Filters:** with a filter, the weights are refitted with `F(λ)` included **without** renormalisation.
  The filter therefore reduces exposure, as a real one does. A `compensate_filter_factor` option (default
  on) applies the neutral attenuation, which is the filter factor, back as exposure compensation. Kodak
  publishes Wratten 8, 15, 25 and 29 as density-vs-wavelength graphs (300–900 nm), which will be digitised.
  No. 11 (yellow-green) data was not found.
- **Source data formats differ.** Kodak gives equal-energy log spectral sensitivity at D = 0.3 and 1.0
  above gross fog, which is directly usable as `S_film`. Ilford gives a wedge spectrogram exposed to
  2850 K tungsten light. It includes the light source's spectrum, so it must be divided by a 2850 K
  Planckian spectrum to recover `S_film`. A wedge spectrogram is also semi-quantitative, so Ilford spectral
  data is Level 1.
- Results are stored per film as data (`weights[primaries][illuminant][filter]`), with the fit residuals
  recorded as a confidence measure.
- Negative weights can occur, for example with blue-heavy orthochromatic-like sensitivity. `E` is clamped
  to ≥ 0 before exposure.

**Limitations:** metamerism. Two surfaces with the same RGB but different spectra get the same film
response. The fit residual on the training set measures the error. The colour-checker test in §8.4
validates it against real film.

**Mono input:** `E = Y_sensor`, unchanged. Spectral weights are not applied. The negative metadata records
`spectral: native-sensor`.

### 5.2 Grain amplitude from granularity (Poisson / Selwyn)

**Principle.** A developed negative is a random arrangement of opaque silver grains or clumps. For `N`
grains per unit area, each with effective projected area `a`, the Nutting relation gives the net density
`D ≈ 0.434·N·a`. If grains are placed independently (Poisson), the density measured through an aperture
of area `A` has variance:

```
σ_D²(A) = 0.434 · a · D / A
```

This contains both **Selwyn's law** (`σ·√A` is constant) and the observed **rise of granularity with
density** at low and medium densities.

**Calibration from published data.** Kodak's diffuse RMS granularity `G` is `1000 × σ_D` at net density 1.0
measured with a 48 µm circular aperture (`A_48 ≈ 1810 µm²`). From this:

```
a_eff = (G/1000)² · A_48 / 0.434
```

Worked example, using Kodak's published `G = 17` for Tri-X (measured in HC-110 B at 20 °C):
`a_eff ≈ 0.000289 × 1810 / 0.434 ≈ 1.2 µm²`. That is an equivalent diameter of about 1.2 µm, which is a
physically plausible effective size for a fast cubic-grain emulsion including clumping. This is a useful
sanity check. Note that `a_eff` is a statistical parameter, not a micrograph measurement.

### 5.3 Grain field generation

Per pixel, with pixel area `A_pix = pitch²`:

```
D(x) = D_adj(x) + sqrt( 0.434 · a_eff · ρ(D_net(x)) · D_net(x) / A_pix ) · n(x)

n    = K_ℓ ∗ W              W: unit-variance white noise from hash(seed, x, y)
K_ℓ  = normalised kernel (Σk = 1) with clump scale ℓ_clump (µm → px); identity if ℓ_clump < ~0.5 px
D_net = D_adj − D_b         (fog is grainy too; base is not)
ρ(D) = empirical high-density roll-off multiplier (default 1); Level 1 unless granularity-vs-density data exists
```

**Why this is independent of resolution.** Convolving white noise with a normalised kernel leaves the
variance integrated over large apertures unchanged and reduces only the variance per pixel. The 48 µm
aperture statistic therefore matches `G` at every resolution and every clump scale. At full resolution,
`ℓ_clump` shapes visible grain structure. At preview resolution, when pixels are larger than the clump
scale, the kernel becomes the identity and the Selwyn scaling gives the correct, lower per-pixel noise.
Previews are **statistically equivalent** to downsampled full-resolution renders, not pixel-identical.

**Effects of developer and development** act on `a_eff`:

```
a_eff = a_film · m_dev(developer, dilution) · (τ / τ_ref)^p
```

- `m_dev < 1` for solvent developers (high sulfite, such as D-76 stock), which round and shrink grain.
  `m_dev > 1` for non-solvent, high-pH developers (Rodinal).
- `p > 0`: extended development increases granularity.
- **Both are Level 1.** Kodak publishes granularity for a single developer only. For Tri-X that is HC-110
  (B), so **HC-110 B is the `m_dev = 1` reference** for Tri-X, and D-76 is relative to it. Relative figures
  come from comparative literature until measured.

**What is deliberately not on the negative.** Grain looks most prominent in print mid-tones. That
appearance comes from three things together: negative `σ(D)`, the local gradient of the paper curve, and
visual sensitivity. Modelling the negative correctly lets the print show this without any special case.
No "mid-tone grain boost" exists anywhere.

**Determinism.** `W` comes from a counter-based hash (for example a SplitMix64/PCG-style mixer of
`seed ⊕ (x, y, plane_id)`) converted to a Gaussian by a deterministic transform. No RNG state is shared, so
any tiling or thread order gives the same output bit for bit at a given resolution.

**Limitations and upgrade path.** Gaussian statistics assume many grains per pixel, which is only marginal
at about 6 µm per pixel (Challenge 7). Real grain is also correlated across scales. Phase 6 upgrade: a
multi-octave clump spectrum, then a Boolean-model renderer (Newson, Faraj, Galerne & Delon, "Realistic
Film Grain Rendering", *IPOL* 7, 165–183, 2017, doi:10.5201/ipol.2017.192) as a full-quality option.

---

## 6. Rust data structures and interfaces

### 6.1 Workspace (revised)

```
bw-engine/
├── Cargo.toml                 # workspace
├── crates/
│   ├── bw-core/               # Plane, FilmGeometry, MonotoneCurve, Lut1D, blur, hash RNG, Provenance; no I/O
│   ├── bw-io/                 # TIFF read/write (tiff crate), sidecar JSON
│   ├── bw-film/               # FilmStock, SpectralWeights, GrainParams, IrradiationPsf, loaders
│   ├── bw-developer/          # DeveloperFormulation, Dilution, ProcessingConditions, DevelopmentCalibration
│   ├── bw-negative/           # NEW: develop() pipeline: exposure → curve → adjacency → grain → Negative
│   ├── bw-darkroom/           # Phase 4 (MVP: minimal inversion + paper curve for viewing)
│   ├── bw-profiles/           # Phase 5: recipes composing the above; preset serialisation
│   └── bw-raw/                # Phase 6
├── apps/
│   └── bw-cli/
├── data/                      # versioned, human-reviewable calibration data (TOML + CSV)
│   ├── films/  developers/  calibrations/  spectral/  filters/
│   └── sources/               # raw digitised points + provenance for every curve
├── tools/                     # fitting binaries (Rust), digitisation notes
└── docs/
```

Dependency direction: `core ← film, developer ← negative ← darkroom ← profiles ← cli`. `bw-io` depends
only on `core`. Nothing depends on a UI.

### 6.2 Core types (`bw-core`)

```rust
pub struct Plane { width: u32, height: u32, data: Vec<f32> }

#[derive(Clone, Copy)]
pub struct FilmGeometry { pub frame_width_mm: f32, pub frame_height_mm: f32, pub width_px: u32, pub height_px: u32 }
impl FilmGeometry { pub fn pixel_pitch_um(&self) -> f32; pub fn um_to_px(&self, um: f32) -> f32; }

/// Monotone cubic Hermite (Steffen). f64 knots; linear, slope-clamped extrapolation.
pub struct MonotoneCurve { xs: Vec<f64>, ys: Vec<f64>, ms: Vec<f64> }
impl MonotoneCurve {
    pub fn new(points: &[(f64, f64)]) -> Result<Self, CurveError>;  // rejects non-monotone/unsorted data
    pub fn eval(&self, x: f64) -> f64;
    pub fn slope(&self, x: f64) -> f64;
    pub fn domain(&self) -> (f64, f64);
    pub fn to_lut(&self, lo: f64, hi: f64, n: usize) -> Lut1D;
}
pub struct Lut1D { lo: f32, inv_step: f32, values: Vec<f32> }       // hot path

pub struct Provenance {
    pub sources: Vec<SourceRef>,          // document, revision, figure/table, URL, retrieval date
    pub method: CalibrationMethod,        // DigitisedCurve, FittedContrastTime, ProxyFrom{..}, Measured{..}, Estimated
    pub level: CalibrationLevel,          // Approximation | Sensitometric | Experimental
    pub confidence: Confidence,           // Low | Medium | High, with free-text justification
    pub assumptions: Vec<String>,
    pub limitations: Vec<String>,
    pub version: semver::Version,
}
```

### 6.3 Film (`bw-film`)

```rust
pub struct FilmStock {
    pub id: FilmId,                       // "kodak-tri-x-400"
    pub name: String,
    pub nominal_iso: f32,
    pub technology: EmulsionTechnology,   // Cubic | Tabular | Other(String)
    pub base_density: f32,
    pub spectral: SpectralModel,          // fitted weights per (primaries, illuminant, filter) + residuals
    pub irradiation: IrradiationPsf,      // sum-of-Gaussians in µm
    pub grain: GrainParams,               // a_film_um2, clump_um, rho: Option<MonotoneCurve>
    pub reciprocity: Option<Reciprocity>, // reserved
    pub provenance: Provenance,
}
```

### 6.4 Developer and calibration (`bw-developer`)

```rust
pub struct DeveloperFormulation {
    pub id: DeveloperId,                  // "kodak-d76"
    pub status: FormulaStatus,            // Published | Proprietary | PartiallyDisclosed
    pub ingredients: Vec<Ingredient>,     // name, g/L (Option), role
    pub dilutions: Vec<Dilution>,         // id "1+1", parts concentrate/water
    pub temperature_alpha: Calibrated<f32>,
    pub provenance: Provenance,
}

pub struct ProcessingConditions {
    pub dilution: DilutionId,
    pub time_min: f32,
    pub temperature_c: f32,
    pub agitation: Agitation,
}

/// Empirical behaviour of one (film, developer, dilution). The only place behaviour lives.
pub struct DevelopmentCalibration {
    pub film: FilmId, pub developer: DeveloperId, pub dilution: DilutionId,
    pub reference_agitation: Agitation,
    pub d_inf: MonotoneCurve,             // h → asymptotic net density
    pub tau_c0_min: f32,
    pub tau_c_shape: Option<MonotoneCurve>, // h-dependent τc, only if validation demands it
    pub compensation_kappa: AgitationCurve,
    pub fog: FogModel,                    // f0, f1
    pub agitation_rate: AgitationCurve,   // r_A(a)
    pub adjacency: AdjacencyParams,       // B(a), length_um
    pub grain: GrainModifier,             // m_dev, p, tau_ref
    pub supported: SupportedRange,        // time, temperature, EI ranges actually backed by data
    pub provenance: Provenance,
}
```

### 6.5 Negative pipeline (`bw-negative`)

```rust
pub struct ExposureSettings {
    pub exposure_index: f32,
    pub metering: Metering,               // Anchor { grey: f32 } | LogAverage
    pub ev_compensation: f32,
    pub flare_fraction: f32,
    pub filter: Option<FilterId>,
    pub compensate_filter_factor: bool,
}

pub struct RenderOptions {
    pub seed: u64,
    pub out_of_range: RangePolicy,        // Refuse | ExtrapolateWithWarning
    pub geometry: FilmGeometry,
}

pub struct Negative {
    pub density: Plane,                   // total diffuse density, f32
    pub base_fog: f32,
    pub geometry: FilmGeometry,
    pub record: NegativeRecord,           // all inputs, calibration versions, warnings, recipe hash, engine version
}

pub fn develop(
    scene: &SceneImage,
    exposure: &ExposureSettings,
    film: &FilmStock,
    process: &ProcessingConditions,
    calibration: &DevelopmentCalibration,
    options: &RenderOptions,
) -> Result<Negative, DevelopError>;

/// Same model, no image: the "virtual sensitometer". Used by tests, the CLI and fitting tools.
pub fn characteristic_curve(film, process, calibration, h_range) -> Vec<(f64, f64)>;
```

Internally each stage is a plain function over planes. There is no trait-object stage graph in the MVP.
Caching is keyed by a hash of each stage's inputs. A trait-based graph should be introduced only when the
desktop app needs partial recomputation.

`DevelopError` distinguishes invalid input, conditions outside the calibrated range (under `Refuse`), and
missing data. Warnings, such as extrapolation or proxy calibrations, go into `NegativeRecord` and are never
silently dropped.

### 6.6 Data file shape (illustrative — values are placeholders, not data)

```toml
# data/calibrations/kodak-tri-x-400__kodak-d76__stock.toml
schema_version = 1
film = "kodak-tri-x-400"
developer = "kodak-d76"
dilution = "stock"

[provenance]
version = "0.1.0"
level = "sensitometric"
method = "fitted-contrast-time"
confidence = "medium"
sources = [{ doc = "Kodak F-4017", revision = "<rev>", figure = "<fig>", retrieved = "<date>" }]
assumptions = ["Reference agitation as stated in source"]
limitations = ["Toe below h = <x> extrapolated"]

[development]
tau_c0_min = "<fitted>"
d_inf = { source = "sources/tri-x_d76_<fig>.csv", transform = "asymptotic-from-reference", t_ref_min = "<t>" }
reference_agitation = { intermittent = { initial_s = "<>", interval_s = "<>", duration_s = "<>" } }

[supported]
time_min = ["<lo>", "<hi>"]
temperature_c = [18.0, 24.0]
```

Raw digitised points live in `data/sources/` with the source page and figure, the axis calibration used in
digitisation (WebPlotDigitizer project file), and an estimated digitisation uncertainty (typically ±0.02 D,
±0.02 log H). The fitting tool reads these points, writes calibration TOML, and records the residuals.

---

## 7. Calibration data sources

Checked against the primary documents on 2026-10-20. Page and figure references, URLs and the remaining
gaps are in [`docs/research/source-inventory.md`](../research/source-inventory.md).

| Source | Content (confirmed) | Use |
|--------|---------------------|-----|
| Kodak Tri-X 400, F-4017 (Feb 2016) | Curve families: D-76 at 6/8/10/12 min (large tank, agitation every 60 s) and T-MAX Developer at 6/7/9/11 min (small tank, every 30 s). CI-vs-time (small tank) for D-76, D-76 1:1, XTOL, XTOL 1:1, HC-110 B, T-MAX, T-MAX RS; target CI 0.56. Equal-energy spectral sensitivity at D = 0.3/1.0. MTF. Diffuse RMS granularity 17, measured in HC-110 B. Per-developer time/temperature tables, 18–24 °C. No resolving-power figure; no 400TX curve in HC-110. | Tri-X + D-76 Level 2 (family fit); Tri-X + HC-110 B from CI; grain amplitude; MTF; spectral; `α` |
| Kodak Double-X 5222, H-1-5222 (Mar 2022) | D-96 family at 21 °C: 4, 5, 6½, 9, 12 min (γ 0.50–1.05) with gamma-vs-time inset; **Status M** density. Spectral sensitivity, MTF, RMS granularity 14, resolving power 32/100 lines/mm. | Double-X anchor via D-96 |
| Kodak H-24.15 | D-96 formula: Elon 1.5 g, sodium sulfite 75 g, hydroquinone 1.5 g, KBr 0.4 g, borax 4.5 g per litre; pH 8.62 | D-96 formulation |
| Ilford HP5 Plus / FP4 Plus sheets (Nov 2018) | One characteristic curve each, in Ilfotec HC 1+31 (HP5 6½ min, FP4 8 min), on a **relative** exposure axis. No Ḡ-vs-time chart. Wedge spectrogram (2850 K). No RMS granularity. Time tables include D-76, HC-110 A/B and Rodinal. Reciprocity `Ta = Tm^1.31` / `Tm^1.26`. | Curve shape; times; reciprocity |
| Ilford ID-11 sheet (Aug 2024) | Times; Ḡ ≈ 0.62 normal-contrast target; continuous agitation "about 15%" less time. No formula. | Contrast target; agitation |
| Kodak D-76, J-78 (Dec 2017); HC-110, J-24 (Dec 2017) | D-76: stock and 1:1 only, no formula, lists old Tri-X Pan (not 400TX). HC-110: dilutions A–F (B = 1:31 from concentrate), no formula, no temperature chart. | Dilution definitions; agitation. Use F-4017 for Tri-X times. |
| Historical formula books (Ilford 1953; Kodak) | ID-11 and D-76 published formulas identical: metol 2 g, anhydrous sulfite 100 g, hydroquinone 5 g, borax 2 g per litre. Ilford warns packaged chemicals may differ. | D-76/ID-11 formulation metadata |
| Ilford time/temperature chart | One chart for all developers, about 10% per °C | Level-1 `α` default |
| Agfa Rodinal leaflet (via Fotoimpex/Adonal) | Times at 1+25/1+50 for γ = 0.65: Tri-X 7/14, FP4 8/18, HP5 8 (1+25 only). No 1+100, no Double-X; temperature graphs for APX only. Disagrees with Ilford's Rodinal times. | Rodinal time points (Level 1) |
| Kodak H-740 | Contrast Index definition | test definition (§8) |
| Kodak Wratten data | Nos. 8, 15, 25, 29 as density-vs-wavelength graphs, 300–900 nm. No. 11 not found. | filter model |
| ISO 6 (B&W speed), ISO 5-2/5-3 (density), ISO 2720 (meters) | definitions | test definitions (§8) |
| CIE 1931 CMFs, D-series and A illuminants | standards | spectral fit |
| Reflectance training sets (e.g. Munsell/Joensuu spectral database, ColorChecker spectra) **[check licences]** | natural reflectances | spectral fit |
| Lambrecht & Woodhouse, *Way Beyond Monochrome* **[unconfirmed]** | Not accessed. A secondary source shows HP5 and Pan F curves in ID-11, Perceptol and Rodinal; nothing for Tri-X, FP4 or Double-X. | possible cross-check for HP5 + Rodinal |
| Massive Dev Chart and similar | development *times* only, for nominal contrast | one point on the CI-vs-time curve for Level-1 combinations; not sensitometry |

**ID-11 and D-76.** The historical published formulas are identical, which supports treating the two
developers as one *formulation*. This no longer raises any calibration level, because Ilford publishes no
ID-11 curves for HP5 Plus or FP4 Plus.

**Anchoring Ilford's relative exposure axis.** Ilford's curves have no absolute exposure scale. They can be
placed on the absolute axis only by assuming that the published curve meets the ISO 6 speed criterion at
box speed. This is a stated assumption, and the calibration records it.

### 7.1 Starting level for the twelve MVP combinations

| Film \ Developer | D-76 | HC-110 | Rodinal |
|---|---|---|---|
| Tri-X 400 | **2** (Kodak D-76 curve family) | **2−** (Kodak CI-vs-time; curve shape from D-76; agitation converted) | 1+ (Agfa times) |
| HP5 Plus | 1+ (Ilford D-76 times; Ilfotec HC curve shape) | 1+ | 1+ |
| FP4 Plus | 1+ | 1+ | 1+ |
| Double-X 5222 | 1 (D-96 shape; Status M converted) | 1 | 1 |

"2−" means Level 2 data with one stated proxy assumption. "1+" means Level 1 with a manufacturer time
point. Only Tri-X has manufacturer sensitometry for MVP developers. HP5 Plus and FP4 Plus can reach Level 2
or 3 only through our own measurements (§8.4), so a densitometer or calibrated scanner matters more than
first assumed. Rodinal stays Level 1 until measured: it is the developer with the strongest film-specific
behaviour, and the two manufacturers' times for it disagree.

---

## 8. Validation strategy

### 8.1 Unit and property tests (`cargo test`, every commit)

- `MonotoneCurve`: interpolates its knots exactly; monotone on dense sampling (property test); rejects
  unsorted or non-monotone input; extrapolation is continuous with clamped slope.
- `Lut1D` vs spline: `max error < 1e-4`.
- Exposure: at `EI = S`, a mid-grey pixel maps to `h = log(8.125/S)`; +1 EV is exactly +log 2.
- Development family: `γ(τ)` is monotone increasing and saturating; at `τ = t_ref` it reproduces the
  reference curve exactly.
- Adjacency: zero change on a constant field (to f32 tolerance); on an edge it produces a Mackie-line
  overshoot of the expected sign.
- Grain: on a uniform field at `D_net = 1.0`, the RMS density through a simulated 48 µm aperture
  equals `G/1000` within statistical tolerance, at several resolutions and clump scales (this is the Selwyn
  test). Doubling `D_net` multiplies variance by `ρ·2`.
- Determinism: identical output bits for 1, 4 and N threads and for different tile sizes.

### 8.2 Sensitometric tests (the virtual sensitometer)

A synthetic **21-step tablet** (0.15 log H increments) is rendered through the real pipeline. Density is
measured on the patch centres, away from adjacency effects at the patch edges.

- **Curve fidelity:** for each Level-2 calibration, RMS error against the digitised manufacturer curve
  ≤ 0.03 D over the useful range, which is within digitisation error.
- **ISO speed:** apply the ISO 6 method (0.10 above base+fog; ΔD 0.80 over Δlog H 1.30) under the
  standard-conforming development. The computed speed is within ⅓ stop of the box speed.
- **CI/Ḡ vs time:** computed CI reproduces Kodak's published CI-vs-time curves. Kodak's definition (H-740)
  uses a template with arcs of 0.2 and 2.2 log-E radius centred on the base+fog line. Ilford's Ḡ (0.1 above
  fog, 1.5 log-E span) is **[unconfirmed]**, taken from secondary sources only, and is used for reporting,
  not calibration.
- **Push behaviour:** with the published push time for EI 1600 (Tri-X/D-76 or HP5/ID-11), the ISO speed
  point moves by **less than one stop**, and mid-tone contrast increases as published.
- **Temperature:** times from the published compensation chart give the same CI within tolerance.

### 8.3 Spatial tests

- **MTF:** a slanted-edge (ISO 12233-style) target is rendered at film scale. The measured MTF of the
  negative matches the published film MTF, including any value above 1 at low frequencies.
- **Granularity spectrum:** a log-log plot of the radially averaged noise power, used to check the clump
  scale against reference scans once those exist.

### 8.4 Reference images (towards Level 3)

1. **Controlled shoot:** a ColorChecker, a step tablet (for example a Stouffer 21-step transmission
   tablet contact-printed or back-lit), and a real scene, each photographed with a digital camera and on
   film with the same exposure.
2. **Measurement:** negative densities read with a densitometer, or with a scanner calibrated by scanning
   the step tablet with all corrections disabled.
3. **Comparisons:**
   - step tablet → characteristic curve (Level 3 curve);
   - ColorChecker patch densities → spectral weights;
   - scene crops → grain and MTF.
4. **Golden regression images:** fixed inputs and seeds, with output hashes stored per calibration
   version. A change in output is either deliberate, with the calibration version bumped, or a bug.

Visual comparison against reference scans is useful as a sanity check, but it never by itself promotes a
calibration level (constraint 7: visual similarity is not physical accuracy).

---

## 9. Prototype scope

### 9.1 In the first working prototype

- `bw-core`: Plane, FilmGeometry, MonotoneCurve/Lut1D, separable Gaussian blur, hash RNG, Provenance.
- `bw-io`: linear TIFF in (16-bit int / 32-bit float, RGB or mono); float density TIFF + JSON sidecar out;
  16-bit TIFF out for the positive.
- Spectral projection with precomputed weights. Filter support only if the weights are ready; otherwise
  next.
- Exposure model (anchor or log-average metering, EI, EV compensation, flare parameter).
- Development family with `τ` (time, temperature, agitation index); fog growth; dilution as discrete
  calibrations.
- Irradiation (Gaussian approximation) and single-scale adjacency.
- Grain: Poisson/Selwyn amplitude, single clump scale, density dependence with `ρ = 1`.
- Minimal positive: `T = 10^(−Q·D)`, enlarger exposure, a single generic paper curve, so the negative can
  be viewed. Proper paper modelling is Phase 4.
- `bw-cli`: `develop`, `print-basic`, `sensitometer` (step-tablet render plus curve/ISO/CI report as CSV),
  `calibrations` (list with levels and warnings).
- First calibration: **Tri-X + D-76 at Level 2**, end to end, with all §8.1–8.2 tests passing before any
  other combination is added.

### 9.2 Deferred

| Item | Phase | Reason |
|------|-------|--------|
| Remaining 11 combinations | 3 | Fit, then validate one at a time |
| `h`-dependent `τc`, compensation `κ` for stand | 3 | Only if the validation residuals require it |
| Paper curves, grades, VC/split-grade, dodging and burning | 4 | Printing engine |
| Reciprocity failure | 3–6 | Needs shutter time (RAW/EXIF) |
| Multi-octave and Boolean-model grain | 6 | Needs reference scans to validate against |
| Illuminant-aware spectral emulation | 6 | Needs un-white-balanced RAW data |
| Halation | 6 | Mostly suppressed in these films; low priority |
| Tiled execution, caching, GPU (Metal) | 2b/6 | Halo contract is in place; optimise after profiling |
| RAW decoding (NEF, Leica DNG, ARW) | 6 | Kept separate from the photographic model |
| Desktop UI | 5 | Only after the model is validated |

---

## 10. Open questions for the owner

1. **Toolchain:** the Rust toolchain is not installed on this machine yet (checked 2026-10-20).
2. **Densitometer or calibrated scanner:** is either available for Level-3 work? With no Ilford
   sensitometry for the MVP developers, this decides whether HP5 Plus and FP4 Plus (curves and grain) and
   all Rodinal combinations can ever leave Level 1.
3. **Adding D-96** as a fourth developer, only as a Double-X anchor (Challenge 5).
4. **Linear TIFF source:** which converter will produce the test inputs, and in which primaries? This fixes
   the first set of spectral weights to compute.
