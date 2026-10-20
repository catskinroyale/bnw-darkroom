//! Film stock definitions (design 0001 §6.3).
//!
//! A film stock holds only properties of the emulsion itself. How it responds to a
//! particular developer lives in `bw-developer::DevelopmentCalibration`.

use bw_core::Provenance;

/// Linear projection of scene-linear RGB onto the film's spectral sensitivity
/// (design 0001 §5.1). Weights sum to 1 so that neutrals give `E = Y`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpectralWeights {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl SpectralWeights {
    pub fn is_normalised(&self) -> bool {
        (self.r + self.g + self.b - 1.0).abs() < 1e-4
    }
}

/// Grain statistics (design 0001 §5.2–5.3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrainParams {
    /// Effective grain area `a` in µm² for the film's reference developer, from
    /// `a = (G/1000)² · A_48 / 0.434` where `G` is diffuse RMS granularity.
    pub area_um2: f64,
    /// Spatial correlation (clump) scale as a Gaussian σ in µm.
    pub clump_um: f64,
}

impl GrainParams {
    /// Area of the 48 µm circular aperture used for RMS granularity.
    pub const APERTURE_48_UM2: f64 = std::f64::consts::PI * 24.0 * 24.0;

    /// Effective grain area from published diffuse RMS granularity (×1000 units).
    pub fn area_from_rms_granularity(g: f64) -> f64 {
        let sigma = g / 1000.0;
        sigma * sigma * Self::APERTURE_48_UM2 / 0.434
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FilmStock {
    pub id: String,
    pub name: String,
    pub nominal_iso: f64,
    /// EI the film is usually rated at: the zero point for push and pull. Equal to the
    /// ISO speed for most films; Delta 3200, for example, is ISO 1000 rated at 3200.
    pub box_ei: f64,
    pub base_density: f64,
    /// Weights for linear sRGB/Rec.709 primaries.
    pub spectral_rec709: SpectralWeights,
    /// Emulsion light spread (irradiation) as a Gaussian σ in µm.
    pub irradiation_um: f64,
    pub grain: GrainParams,
    pub provenance: Provenance,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn granularity_17_gives_about_1_2_um2() {
        // Design 0001 §5.2 worked example.
        let a = GrainParams::area_from_rms_granularity(17.0);
        assert!((a - 1.20).abs() < 0.01, "a = {a}");
    }
}
