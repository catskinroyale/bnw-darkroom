//! Push and pull on a correctly exposed source: development changes only, printed with
//! the black point held (Adams, The Negative, p. 72: expansion acts mainly on the
//! higher values). Measured on a grey scale, zones 0–X with mid-grey at zone V.

use bw_core::Plane;
use bw_negative::SceneImage;
use bw_profiles::{builtin_styles, render};

const PATCH: usize = 48;

/// Print reflection density of each zone, averaged over the patch centre.
fn print_densities(film: &str, developer: &str, push: f64) -> Vec<f64> {
    let zones: Vec<f32> = (0..=10).map(|z| 0.18 * 2f32.powi(z - 5)).collect();
    let scene = SceneImage::Mono {
        y: Plane::from_fn(PATCH * zones.len(), 24, |x, _| zones[x / PATCH]).unwrap(),
    };
    let mut style = builtin_styles()[0].clone();
    style.film = film.into();
    style.developer = developer.into();
    style.set_push_stops(push);
    let print = render(&scene, &style, 36.0).unwrap().print;
    (0..zones.len())
        .map(|i| {
            let mut sum = 0.0;
            let mut n = 0.0;
            for y in 4..20 {
                for x in i * PATCH + 8..(i + 1) * PATCH - 8 {
                    sum += f64::from(print.data()[y * print.width() + x]);
                    n += 1.0;
                }
            }
            -(sum / n).log10()
        })
        .collect()
}

#[test]
fn normal_development_prints_mid_grey_at_the_default_density() {
    let d = print_densities("hp5", "hc110-b", 0.0);
    assert!((d[5] - 0.75).abs() < 0.08, "mid-grey {:.2}", d[5]);
}

#[test]
fn push_holds_blacks_opens_shadows_and_brightens_highlights() {
    let normal = print_densities("hp5", "hc110-b", 0.0);
    for push in [1.0, 2.0] {
        let pushed = print_densities("hp5", "hc110-b", push);
        assert!(
            (pushed[0] - normal[0]).abs() < 0.03,
            "+{push}: blacks {:.2} → {:.2}",
            normal[0],
            pushed[0]
        );
        assert!(
            pushed[3] < normal[3] - 0.05,
            "+{push}: zone III {:.2} → {:.2}",
            normal[3],
            pushed[3]
        );
        assert!(
            pushed[7] < normal[7],
            "+{push}: zone VII {:.2} → {:.2}",
            normal[7],
            pushed[7]
        );
        // Contrast rises: the shadow-to-highlight span grows.
        assert!(pushed[1] - pushed[7] > normal[1] - normal[7]);
    }
}

#[test]
fn pull_holds_blacks_and_softens_contrast() {
    let normal = print_densities("hp5", "hc110-b", 0.0);
    let pulled = print_densities("hp5", "hc110-b", -1.0);
    assert!((pulled[0] - normal[0]).abs() < 0.03);
    assert!(pulled[3] > normal[3] && pulled[7] > normal[7]);
}
