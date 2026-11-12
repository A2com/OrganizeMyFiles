//! Sons de l'appli : `intro-sound` pendant l'introduction, `dashboard-sound` à l'apparition de l'Aperçu.
//! Les fichiers sont embarqués dans l'exécutable. Lecture par `NSSound` (macOS) ; ailleurs, rien.
// Hors macOS, seuls les tests utilisent les sons et le fondu.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

pub const INTRO_WAV: &[u8] = include_bytes!("../../assets/sounds/intro-sound.wav");
pub const DASHBOARD_WAV: &[u8] = include_bytes!("../../assets/sounds/dashboard-sound.wav");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Intro,
    Dashboard,
}

impl Sound {
    fn bytes(self) -> &'static [u8] {
        match self {
            Sound::Intro => INTRO_WAV,
            Sound::Dashboard => DASHBOARD_WAV,
        }
    }
}

/// Durée du fondu qui arrête le son d'introduction quand on passe l'animation.
pub const FADE_SECS: f32 = 0.25;

/// Volume (1 = plein, 0 = coupé) `elapsed` secondes après le début du fondu.
pub fn fade_volume(elapsed_secs: f32) -> f32 {
    (1.0 - elapsed_secs / FADE_SECS).clamp(0.0, 1.0)
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2::rc::Retained;
    use objc2::AnyThread;
    use objc2_app_kit::NSSound;
    use objc2_foundation::NSData;
    use std::time::Instant;

    use super::*;

    pub struct Sounds {
        intro: Option<Retained<NSSound>>,
        dashboard: Option<Retained<NSSound>>,
        fade_started: Option<Instant>,
    }

    impl Sounds {
        pub fn new() -> Self {
            Sounds { intro: None, dashboard: None, fade_started: None }
        }

        /// Joue le son depuis le début ; renvoie faux si le système ne peut pas le lire (aucune erreur affichée).
        pub fn play(&mut self, sound: Sound) -> bool {
            let data = NSData::with_bytes(sound.bytes());
            let Some(player) = NSSound::initWithData(NSSound::alloc(), &data) else { return false };
            player.setVolume(1.0);
            let started = player.play();
            match sound {
                Sound::Intro => {
                    self.fade_started = None;
                    self.intro = Some(player);
                }
                Sound::Dashboard => self.dashboard = Some(player),
            }
            started
        }

        /// Arrête le son d'introduction en fondu court.
        pub fn fade_out_intro(&mut self) {
            if self.intro.as_ref().is_some_and(|s| s.isPlaying()) && self.fade_started.is_none() {
                self.fade_started = Some(Instant::now());
            }
        }

        pub fn is_fading(&self) -> bool {
            self.fade_started.is_some()
        }

        /// À appeler à chaque image : fait avancer le fondu.
        pub fn tick(&mut self) {
            let Some(since) = self.fade_started else { return };
            let volume = fade_volume(since.elapsed().as_secs_f32());
            if let Some(intro) = &self.intro {
                intro.setVolume(volume);
                if volume <= 0.0 {
                    intro.stop();
                }
            }
            if volume <= 0.0 {
                self.fade_started = None;
                self.intro = None;
            }
        }

        #[cfg(test)]
        pub fn is_playing(&self, sound: Sound) -> bool {
            let player = match sound {
                Sound::Intro => &self.intro,
                Sound::Dashboard => &self.dashboard,
            };
            player.as_ref().is_some_and(|s| s.isPlaying())
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::*;

    pub struct Sounds;

    #[allow(dead_code)]
    impl Sounds {
        pub fn new() -> Self {
            Sounds
        }
        pub fn play(&mut self, _sound: Sound) -> bool {
            false
        }
        pub fn fade_out_intro(&mut self) {}
        pub fn is_fading(&self) -> bool {
            false
        }
        pub fn tick(&mut self) {}
    }
}

pub use imp::Sounds;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_sounds_are_the_wav_files_provided() {
        for (bytes, len) in [(INTRO_WAV, 864_770), (DASHBOARD_WAV, 576_770)] {
            assert_eq!(bytes.len(), len);
            assert_eq!(&bytes[..4], b"RIFF");
            assert_eq!(&bytes[8..12], b"WAVE");
        }
    }

    /// Vrai lecteur (macOS) : le son démarre, puis un fondu court l'arrête. Ignoré s'il n'y a pas de sortie audio
    /// (machine de CI) ; à volume plein, le son d'introduction ne dure ici qu'une fraction de seconde.
    #[cfg(target_os = "macos")]
    #[test]
    fn real_player_starts_and_fades_out() {
        let mut sounds = Sounds::new();
        if !sounds.play(Sound::Intro) {
            eprintln!("pas de sortie audio : test ignoré");
            return;
        }
        assert!(sounds.is_playing(Sound::Intro));
        sounds.fade_out_intro();
        assert!(sounds.is_fading());
        for _ in 0..100 {
            sounds.tick();
            if !sounds.is_fading() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!sounds.is_fading() && !sounds.is_playing(Sound::Intro), "le fondu doit arrêter le son");
    }

    #[test]
    fn fade_goes_from_full_volume_to_silence() {
        assert_eq!(fade_volume(0.0), 1.0);
        assert!((fade_volume(FADE_SECS / 2.0) - 0.5).abs() < 1e-4);
        assert_eq!(fade_volume(FADE_SECS), 0.0);
        assert_eq!(fade_volume(5.0), 0.0);
    }
}
