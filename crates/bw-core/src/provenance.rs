//! Where a calibration came from and how far it can be trusted (design 0001 §10, §6.2).

/// How a set of model parameters was obtained.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CalibrationLevel {
    /// Placeholder values chosen to exercise the engine. Not derived from any source;
    /// must never be presented as an emulation of the named product.
    Illustrative,
    /// Level 1: published descriptions, manufacturer times and visual comparison.
    Approximation,
    /// Level 2: manufacturer characteristic curves and development measurements.
    Sensitometric,
    /// Level 3: our own controlled exposures and density measurements.
    Experimental,
}

impl CalibrationLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Illustrative => "illustrative (not calibrated)",
            Self::Approximation => "level 1: approximation",
            Self::Sensitometric => "level 2: sensitometric",
            Self::Experimental => "level 3: experimental",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Provenance {
    pub level: CalibrationLevel,
    pub version: String,
    pub sources: Vec<String>,
    pub assumptions: Vec<String>,
    pub limitations: Vec<String>,
}

impl Provenance {
    /// Provenance for placeholder parameters.
    pub fn illustrative(note: impl Into<String>) -> Self {
        Self {
            level: CalibrationLevel::Illustrative,
            version: "0.0.0".into(),
            sources: Vec::new(),
            assumptions: vec![note.into()],
            limitations: vec!["Parameters are placeholders, not fitted to any data".into()],
        }
    }
}
