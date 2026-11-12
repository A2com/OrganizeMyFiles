//! Écran d'introduction du premier lancement : chronologie de l'animation et tracés du mot « Bienvenue »
//! (écriture à la main). Logique pure, testée ; le dessin est dans `ui.rs`.

use std::sync::OnceLock;

use serde::Deserialize;

pub type Point = [f32; 2];

/// Tracés de stylo du mot, dans l'ordre d'écriture (générés par `scripts/generer-bienvenue.py`).
#[derive(Deserialize)]
pub struct WordStrokes {
    #[serde(rename = "largeur")]
    pub width: f32,
    #[serde(rename = "hauteur")]
    pub height: f32,
    /// Épaisseur de trait d'origine de la police-tracé, dans la même unité que les points.
    #[serde(rename = "trait")]
    pub pen: f32,
    #[serde(rename = "traits")]
    pub strokes: Vec<Vec<Point>>,
}

pub fn word() -> &'static WordStrokes {
    static WORD: OnceLock<WordStrokes> = OnceLock::new();
    WORD.get_or_init(|| serde_json::from_str(include_str!("../../assets/intro/bienvenue.json")).expect("bienvenue.json valide"))
}

fn dist(a: Point, b: Point) -> f32 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
}

pub fn polyline_length(points: &[Point]) -> f32 {
    points.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// Début d'une polyligne sur la fraction `fraction` (0 à 1) de sa longueur.
pub fn trim_polyline(points: &[Point], fraction: f32) -> Vec<Point> {
    let target = polyline_length(points) * fraction.clamp(0.0, 1.0);
    let mut out = Vec::new();
    let mut walked = 0.0;
    for (i, p) in points.iter().enumerate() {
        if i == 0 {
            out.push(*p);
            continue;
        }
        let seg = dist(points[i - 1], *p);
        if walked + seg >= target {
            let k = if seg > 0.0 { (target - walked) / seg } else { 0.0 };
            let prev = points[i - 1];
            out.push([prev[0] + (p[0] - prev[0]) * k, prev[1] + (p[1] - prev[1]) * k]);
            return out;
        }
        walked += seg;
        out.push(*p);
    }
    out
}

impl WordStrokes {
    pub fn total_length(&self) -> f32 {
        self.strokes.iter().map(|s| polyline_length(s)).sum()
    }

    /// Ce qui est écrit quand `fraction` (0 à 1) de la longueur totale a été tracée : les traits terminés,
    /// puis le trait en cours (tronqué). Renvoie aussi la position du stylo.
    pub fn drawn(&self, fraction: f32) -> (Vec<Vec<Point>>, Option<Point>) {
        let target = self.total_length() * fraction.clamp(0.0, 1.0);
        let mut walked = 0.0;
        let mut out = Vec::new();
        let mut tip = None;
        for stroke in &self.strokes {
            let len = polyline_length(stroke);
            if walked + len <= target {
                tip = stroke.last().copied();
                out.push(stroke.clone());
                walked += len;
            } else {
                if target > walked && len > 0.0 {
                    let part = trim_polyline(stroke, (target - walked) / len);
                    tip = part.last().copied();
                    out.push(part);
                }
                break;
            }
        }
        (out, tip)
    }
}

// ----------------------------------------------------------------------------- chronologie

/// Instant (en secondes) où tout est affiché : mot écrit, présentation et bouton visibles.
pub const END_TIME: f32 = 5.4;
/// Durée du fondu de sortie après « Commencer ».
pub const LEAVE_SECS: f32 = 0.45;

const ICON_START: f32 = 0.2;
const ICON_SECS: f32 = 1.0;
const WRITE_START: f32 = 1.3;
const WRITE_SECS: f32 = 3.0;
const SUBTITLE_START: f32 = 4.3;
const SUBTITLE_SECS: f32 = 0.7;
const BUTTON_START: f32 = 4.8;
const BUTTON_SECS: f32 = 0.6;
const BACKDROP_SECS: f32 = 0.7;

fn unit(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

fn smooth(x: f32) -> f32 {
    let x = unit(x);
    x * x * (3.0 - 2.0 * x)
}

/// Arrivée en douceur avec un léger dépassement (rebond).
fn ease_out_back(x: f32) -> f32 {
    let (c1, c3) = (1.2_f32, 2.2_f32);
    let x = unit(x) - 1.0;
    1.0 + c3 * x.powi(3) + c1 * x.powi(2)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntroFrame {
    /// 0 = fond noir, 1 = fond animé pleinement visible.
    pub backdrop: f32,
    pub icon_alpha: f32,
    pub icon_scale: f32,
    /// Fraction de la longueur du mot déjà écrite (0 à 1).
    pub write: f32,
    pub subtitle: f32,
    pub button: f32,
}

/// Aspect de l'intro à l'instant `t` (en secondes depuis le début).
pub fn frame(t: f32) -> IntroFrame {
    let icon = unit((t - ICON_START) / ICON_SECS);
    IntroFrame {
        backdrop: smooth(t / BACKDROP_SECS),
        icon_alpha: smooth(icon * 1.6),
        icon_scale: 0.8 + 0.2 * ease_out_back(icon),
        write: smooth((t - WRITE_START) / WRITE_SECS),
        subtitle: smooth((t - SUBTITLE_START) / SUBTITLE_SECS),
        button: smooth((t - BUTTON_START) / BUTTON_SECS),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_starts_empty_and_ends_complete() {
        let start = frame(0.0);
        assert_eq!((start.backdrop, start.icon_alpha, start.write, start.subtitle, start.button), (0.0, 0.0, 0.0, 0.0, 0.0));
        let end = frame(END_TIME);
        assert_eq!((end.backdrop, end.icon_alpha, end.write, end.subtitle, end.button), (1.0, 1.0, 1.0, 1.0, 1.0));
        assert!((end.icon_scale - 1.0).abs() < 1e-4);
    }

    #[test]
    fn writing_progress_never_goes_backwards() {
        let mut last = 0.0;
        for i in 0..=120 {
            let w = frame(i as f32 * 0.05).write;
            assert!(w >= last - 1e-6, "recul à t={}", i as f32 * 0.05);
            assert!((0.0..=1.0).contains(&w));
            last = w;
        }
    }

    #[test]
    fn steps_follow_the_planned_order() {
        // À 1 s l'icône est là mais le mot n'est pas commencé ; à 3 s il est à moitié écrit ; le bouton vient en dernier.
        assert!(frame(1.0).icon_alpha > 0.5 && frame(1.0).write == 0.0);
        let mid = frame(2.8).write;
        assert!(mid > 0.3 && mid < 0.7, "{mid}");
        assert!(frame(4.0).button == 0.0 && frame(4.0).subtitle == 0.0);
        assert!(frame(5.0).subtitle > 0.9 && frame(5.0).button < 1.0);
    }

    #[test]
    fn trimming_keeps_the_start_and_cuts_at_the_right_length() {
        let line: Vec<Point> = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
        assert_eq!(polyline_length(&line), 20.0);
        let nothing = trim_polyline(&line, 0.0);
        assert_eq!((nothing[0], polyline_length(&nothing)), ([0.0, 0.0], 0.0));
        assert_eq!(trim_polyline(&line, 0.25), vec![[0.0, 0.0], [5.0, 0.0]]);
        let three_quarters = trim_polyline(&line, 0.75);
        assert_eq!(three_quarters.last().copied(), Some([10.0, 5.0]));
        assert_eq!(trim_polyline(&line, 1.0), line);
    }

    #[test]
    fn the_word_asset_is_usable() {
        let w = word();
        assert!(w.strokes.len() >= 9, "au moins une courbe par lettre");
        assert!(w.width > 50.0 && w.height > 10.0 && w.pen > 0.0);
        for s in &w.strokes {
            assert!(s.len() >= 2);
            for p in s {
                assert!((-0.01..=w.width + 0.01).contains(&p[0]) && (-0.01..=w.height + 0.01).contains(&p[1]), "point hors cadre {p:?}");
            }
        }
        // L'écriture avance globalement de gauche à droite : le dernier trait finit plus à droite que le premier.
        let first_end = w.strokes.first().and_then(|s| s.last()).unwrap()[0];
        let last_end = w.strokes.last().and_then(|s| s.last()).unwrap()[0];
        assert!(last_end > first_end);
    }

    #[test]
    fn drawn_grows_with_the_fraction_and_gives_the_pen_position() {
        let w = word();
        let (none, tip) = w.drawn(0.0);
        assert!(none.is_empty() && tip.is_none());
        let (half, tip_half) = w.drawn(0.5);
        let (all, tip_all) = w.drawn(1.0);
        assert!(!half.is_empty() && tip_half.is_some());
        assert_eq!(all.len(), w.strokes.len());
        assert_eq!(tip_all, w.strokes.last().and_then(|s| s.last()).copied());
        let drawn_len = |v: &Vec<Vec<Point>>| v.iter().map(|s| polyline_length(s)).sum::<f32>();
        assert!((drawn_len(&half) - w.total_length() * 0.5).abs() < 0.5);
    }
}
