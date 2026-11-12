//! Message d'état affiché en bas de la fenêtre : pastille verte (tout va bien) ou rouge (erreur),
//! avec un point qui pulse, puis qui s'estompe. Logique pure, testée.

use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Ok,
    Error,
}

pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub since: Instant,
}

/// Le point pulse pendant ce délai, puis reste fixe.
pub const PULSE_SECS: f32 = 6.0;
const FADE_SECS: f32 = 0.6;

/// Durée d'affichage avant l'estompage (une erreur reste plus longtemps).
fn lifetime(kind: ToastKind) -> f32 {
    match kind {
        ToastKind::Ok => 12.0,
        ToastKind::Error => 20.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToastLook {
    /// 1 = pleinement visible, 0 = disparu.
    pub opacity: f32,
    pub pulsing: bool,
}

/// Aspect de la pastille après `age_secs` secondes ; `None` quand elle a disparu.
pub fn look(age_secs: f32, kind: ToastKind) -> Option<ToastLook> {
    let life = lifetime(kind);
    if age_secs >= life + FADE_SECS {
        return None;
    }
    let opacity = if age_secs <= life { 1.0 } else { 1.0 - (age_secs - life) / FADE_SECS };
    Some(ToastLook { opacity, pulsing: age_secs < PULSE_SECS })
}

/// Texte et couleur du message après un lot : rouge dès qu'une action a échoué.
pub fn batch_toast(prefix: &str, suffix: &str, matched: usize, failed: usize) -> (String, ToastKind) {
    let base = format!("{prefix}{matched} fichier(s) {}{suffix}", if prefix.is_empty() { "organisé(s)" } else { "concerné(s)" });
    match failed {
        0 => (base, ToastKind::Ok),
        f => (format!("{base} · {f} échec(s)"), ToastKind::Error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toast_pulses_then_stays_then_fades_out() {
        assert_eq!(look(1.0, ToastKind::Ok), Some(ToastLook { opacity: 1.0, pulsing: true }));
        assert_eq!(look(8.0, ToastKind::Ok), Some(ToastLook { opacity: 1.0, pulsing: false }));
        let fading = look(12.3, ToastKind::Ok).unwrap();
        assert!(fading.opacity > 0.0 && fading.opacity < 1.0);
        assert_eq!(look(13.0, ToastKind::Ok), None);
    }

    #[test]
    fn errors_stay_longer_than_successes() {
        assert_eq!(look(15.0, ToastKind::Ok), None);
        assert!(look(15.0, ToastKind::Error).is_some());
        assert_eq!(look(21.0, ToastKind::Error), None);
    }

    #[test]
    fn batch_toast_is_red_as_soon_as_something_failed() {
        assert_eq!(batch_toast("", "", 3, 0), ("3 fichier(s) organisé(s)".to_string(), ToastKind::Ok));
        assert_eq!(batch_toast("", "", 3, 1), ("3 fichier(s) organisé(s) · 1 échec(s)".to_string(), ToastKind::Error));
        assert_eq!(batch_toast("", " automatiquement", 2, 0).0, "2 fichier(s) organisé(s) automatiquement");
        assert_eq!(batch_toast("Aperçu : ", "", 2, 0), ("Aperçu : 2 fichier(s) concerné(s)".to_string(), ToastKind::Ok));
    }
}
