//! Halo « lueur externe » lisse, comme dans Photoshop : la silhouette du bouton (rectangle arrondi,
//! légèrement épaissie) floutée par une gaussienne. Calculé une fois par taille de bouton, puis peint
//! sous le bouton avec une opacité qui peut varier. Logique pure, testée.

/// Réglages du halo.
#[derive(Clone, Copy, Debug)]
pub struct GlowSpec {
    /// Écart-type du flou, en pixels (la lueur s'étend sur ≈ 2,5 sigma).
    pub sigma: f32,
    /// De combien la silhouette est épaissie avant le flou (la lueur démarre plus fort au bord du bouton).
    pub spread: f32,
}

/// Aspect par défaut : ≈ 20 px de lueur.
pub const SOFT: GlowSpec = GlowSpec { sigma: 8.0, spread: 3.0 };

/// Marge (en pixels) autour du bouton occupée par le halo.
pub fn margin(spec: GlowSpec) -> usize {
    (spec.sigma * 3.0 + spec.spread).ceil() as usize
}

/// Image de halo : opacités (0 à 1) d'une grille `width × height` ; le bouton occupe le centre,
/// entouré d'une marge de `margin(spec)` pixels.
pub struct GlowMap {
    pub width: usize,
    pub height: usize,
    pub alpha: Vec<f32>,
}

impl GlowMap {
    #[cfg(test)]
    pub fn at(&self, x: usize, y: usize) -> f32 {
        self.alpha[y * self.width + x]
    }
}

/// Distance signée (négative à l'intérieur) d'un point au rectangle arrondi centré en (0, 0).
fn rounded_rect_distance(px: f32, py: f32, half_w: f32, half_h: f32, radius: f32) -> f32 {
    let (qx, qy) = (px.abs() - (half_w - radius), py.abs() - (half_h - radius));
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - radius
}

/// Noyau gaussien normalisé de rayon `3 sigma`.
fn gaussian_kernel(sigma: f32) -> Vec<f32> {
    let radius = (sigma * 3.0).ceil() as i32;
    let mut k: Vec<f32> = (-radius..=radius).map(|i| (-(i as f32).powi(2) / (2.0 * sigma * sigma)).exp()).collect();
    let sum: f32 = k.iter().sum();
    k.iter_mut().for_each(|v| *v /= sum);
    k
}

fn blur_horizontal(src: &[f32], w: usize, h: usize, kernel: &[f32]) -> Vec<f32> {
    let r = (kernel.len() / 2) as i32;
    let mut out = vec![0.0; src.len()];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0;
            for (i, k) in kernel.iter().enumerate() {
                let sx = x as i32 + i as i32 - r;
                if (0..w as i32).contains(&sx) {
                    acc += src[y * w + sx as usize] * k;
                }
            }
            out[y * w + x] = acc;
        }
    }
    out
}

fn blur_vertical(src: &[f32], w: usize, h: usize, kernel: &[f32]) -> Vec<f32> {
    let r = (kernel.len() / 2) as i32;
    let mut out = vec![0.0; src.len()];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0;
            for (i, k) in kernel.iter().enumerate() {
                let sy = y as i32 + i as i32 - r;
                if (0..h as i32).contains(&sy) {
                    acc += src[sy as usize * w + x] * k;
                }
            }
            out[y * w + x] = acc;
        }
    }
    out
}

/// Halo d'un bouton de `width × height` pixels dont les coins ont le rayon `radius`.
pub fn glow_map(width: f32, height: f32, radius: f32, spec: GlowSpec) -> GlowMap {
    let margin = margin(spec);
    let (w, h) = (width.ceil() as usize + 2 * margin, height.ceil() as usize + 2 * margin);
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let mask: Vec<f32> = (0..w * h)
        .map(|i| {
            let (x, y) = ((i % w) as f32 + 0.5 - cx, (i / w) as f32 + 0.5 - cy);
            let d = rounded_rect_distance(x, y, width / 2.0 + spec.spread, height / 2.0 + spec.spread, radius + spec.spread);
            (0.5 - d).clamp(0.0, 1.0)
        })
        .collect();
    let kernel = gaussian_kernel(spec.sigma);
    let alpha = blur_vertical(&blur_horizontal(&mask, w, h, &kernel), w, h, &kernel);
    GlowMap { width: w, height: h, alpha }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> GlowMap {
        glow_map(200.0, 52.0, 26.0, SOFT)
    }

    #[test]
    fn values_stay_between_zero_and_one() {
        let m = map();
        assert!(m.alpha.iter().all(|a| (0.0..=1.0001).contains(a)));
    }

    #[test]
    fn glow_fades_smoothly_away_from_the_button() {
        let m = map();
        let y = m.height / 2;
        // De la fin du bouton (côté droit) vers le bord de l'image : jamais de remontée ni de marche brutale.
        let start = m.width - margin(SOFT);
        let line: Vec<f32> = (start..m.width).map(|x| m.at(x, y)).collect();
        for pair in line.windows(2) {
            assert!(pair[1] <= pair[0] + 1e-5, "remontée : {pair:?}");
            assert!(pair[0] - pair[1] < 0.08, "marche trop brutale : {pair:?}");
        }
        assert!(line[0] > 0.5, "la lueur démarre franche au bord du bouton : {}", line[0]);
        assert!(*line.last().unwrap() < 0.02, "la lueur s'éteint avant le bord de l'image");
    }

    #[test]
    fn glow_is_symmetric() {
        let m = map();
        let (w, h) = (m.width, m.height);
        for (x, y) in [(3, 5), (20, h / 2), (w / 2, 4), (w / 3, h / 3)] {
            assert!((m.at(x, y) - m.at(w - 1 - x, y)).abs() < 1e-4, "gauche/droite");
            assert!((m.at(x, y) - m.at(x, h - 1 - y)).abs() < 1e-4, "haut/bas");
        }
    }

    #[test]
    fn a_larger_sigma_reaches_farther() {
        let (near_spec, far_spec) = (GlowSpec { sigma: 4.0, spread: 3.0 }, GlowSpec { sigma: 10.0, spread: 3.0 });
        let near = glow_map(120.0, 40.0, 20.0, near_spec);
        let far = glow_map(120.0, 40.0, 20.0, far_spec);
        let reach = |m: &GlowMap, spec: GlowSpec| (0..margin(spec)).filter(|i| m.at(*i, m.height / 2) > 0.05).count();
        assert!(reach(&far, far_spec) > reach(&near, near_spec));
    }

    #[test]
    fn the_button_area_is_fully_lit_under_it() {
        let m = map();
        assert!(m.at(m.width / 2, m.height / 2) > 0.99);
    }
}
