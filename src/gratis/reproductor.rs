//! El reproductor (solo en nook Pro): en la versión gratuita nunca hay nada que mostrar.

use crate::musica::{Orden, Reproduccion};
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as Controles;

#[derive(Default)]
pub struct Reproductor {
    pub estado: Reproduccion,
}

impl Reproductor {
    pub fn consultar(&mut self, _controles: Option<&Controles>) {}

    pub fn ejecutar(&mut self, _orden: Orden) -> bool {
        false
    }
}
