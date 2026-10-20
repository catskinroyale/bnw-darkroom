//! Numerical foundations for the black-and-white emulation engine.
//!
//! This crate is deliberately free of photographic policy and I/O. It provides the
//! primitives the film, developer and darkroom models are built from:
//!
//! - [`Plane`]: planar `f32` image storage.
//! - [`FilmGeometry`]: the mapping between pixels and film-plane micrometres, so that
//!   grain, irradiation and adjacency are specified in physical units (design 0001 §2.1).
//! - [`MonotoneCurve`] and [`Lut1D`]: characteristic-curve representation (design 0001 §3.2).
//! - [`noise`]: counter-based deterministic Gaussian noise (design 0001 §5.3, D8).
//! - [`blur`]: separable Gaussian filtering with an explicit halo radius (design 0001 §4).

pub mod blur;
pub mod curve;
pub mod geometry;
pub mod noise;
pub mod plane;
pub mod provenance;

pub use curve::{CurveError, Lut1D, MonotoneCurve};
pub use geometry::{FilmGeometry, GeometryError};
pub use plane::{Plane, PlaneError};
pub use provenance::{CalibrationLevel, Provenance};
