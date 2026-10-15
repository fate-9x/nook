//! Volumen general de Windows y silencio del micrófono (solo en nook Pro): en la versión gratuita
//! no se encuentra ninguna salida de audio ni micrófono.

/// Cómo está la salida de audio.
#[derive(Clone, Debug, PartialEq)]
pub struct Salida {
    pub volumen: f32,
    pub silencio: bool,
    pub nombre: String,
}

pub fn leer() -> Option<Salida> {
    None
}

pub fn poner_volumen(_volumen: f32) {}

pub fn poner_silencio(_silencio: bool) {}

pub fn microfono_silenciado() -> Option<bool> {
    None
}

pub fn silenciar_microfono(_silencio: bool) {}
