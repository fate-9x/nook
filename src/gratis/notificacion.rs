//! Área de notificación propia (solo en nook Pro): en la versión gratuita no se recoge ningún
//! icono.

use std::sync::Arc;

/// Un icono del área de notificación.
#[derive(Clone, Debug)]
pub struct Icono {
    pub hwnd: isize,
    pub pixeles: Option<Arc<Vec<u8>>>,
    pub lado: u32,
    pub texto: String,
}

pub fn iniciar(_lado: u32) {}

pub fn parar() {}

pub fn activa() -> bool {
    false
}

pub fn cambios() -> u64 {
    0
}

pub fn iconos() -> Vec<Icono> {
    Vec::new()
}

pub fn revisar() {}

pub fn pulsar(_icono: &Icono, _derecho: bool, _cursor: (i32, i32)) {}

/// Icono que Windows deja a la vista en su barra.
#[derive(Clone, Debug, PartialEq)]
pub struct AlaVista;

impl AlaVista {
    pub fn es(&self, _icono: &Icono) -> bool {
        false
    }
}

pub fn a_la_vista_en_windows() -> Vec<AlaVista> {
    Vec::new()
}
