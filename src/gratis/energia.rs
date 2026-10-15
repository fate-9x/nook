//! Apagar, reiniciar o suspender el equipo y leer la batería (solo en nook Pro): en la versión
//! gratuita no hacen nada y nunca hay batería.

/// Cómo está la batería.
#[derive(Clone, Debug, PartialEq)]
pub struct Bateria {
    /// De 0 a 1.
    pub nivel: f32,
    pub cargando: bool,
    pub enchufada: bool,
    pub minutos: Option<u32>,
}

pub fn bateria() -> Option<Bateria> {
    None
}

pub fn suspender() {}

pub fn reiniciar() {}

pub fn apagar() {}
