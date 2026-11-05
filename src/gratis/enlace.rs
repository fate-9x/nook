//! Imágenes y archivos en la bóveda, y el enlace con el móvil para pasarlos (solo en nook Pro): en
//! la versión gratuita la bóveda solo tiene textos.

use slint::Image;
use std::path::PathBuf;

/// Un archivo de la bóveda.
#[allow(dead_code)] // solo los crea la versión Pro
pub struct Archivo {
    pub id: String,
    pub nombre: String,
    pub tamano: u64,
    pub ruta: PathBuf,
    /// Si es una imagen (tiene miniatura, ver `Enlace::foto`).
    pub foto: bool,
}

/// En qué está el vínculo con el móvil.
#[allow(dead_code)] // solo los crea la versión Pro
pub enum Movil {
    SinVincular,
    /// Esperando a que el móvil se conecte para vincularse.
    Esperando,
    /// Hay que comprobar que `codigo` es el que se ve en el móvil `nombre`; `confirmado`, si ya se
    /// dijo que sí aquí.
    Codigo { nombre: String, codigo: String, confirmado: bool },
    Vinculado(String),
}

/// Lo que avisa el enlace a la interfaz, además de despertarla cada vez que cambia algo.
#[allow(dead_code)] // solo los crea la versión Pro
pub enum Evento {
    /// Llegó un móvil para vincularse: hay que enseñar el código.
    Codigo,
    /// No se pudo guardar el archivo `nombre` en la bóveda.
    NoGuardado { nombre: String, motivo: Motivo },
    /// No se pudo vincular el móvil.
    ErrorMovil(Motivo),
}

/// Por qué no se pudo algo (el texto lo pone la interfaz, que es donde se traduce).
#[allow(dead_code)] // solo los crea la versión Pro
pub enum Motivo {
    /// Un archivo: es una carpeta, pasa de 100 MB o ya no cabe (1 GB en total).
    Carpeta,
    Grande,
    Llena,
    /// Al vincular: en el móvil se dijo que el código no coincide, no se terminó de comprobar u
    /// otro aparato intentó vincularse a la vez (y se canceló, por si era el que no tocaba).
    NoCoincide,
    SinTerminar,
    OtroAparato,
    /// Lo que dijo Windows.
    Otro(String),
}

pub struct Enlace;

impl Enlace {
    pub fn iniciar(_despertar: impl Fn() + Send + Sync + 'static) -> Enlace {
        Enlace
    }

    pub fn archivos(&self) -> Vec<Archivo> {
        Vec::new()
    }

    pub fn carpeta(&self) -> PathBuf {
        PathBuf::new()
    }

    pub fn agregar(&self, _rutas: Vec<PathBuf>) {}

    pub fn pegar(&self, _nombre: String) -> bool {
        false
    }

    pub fn quitar(&self, _id: &str) {}

    pub fn copiar(&self, _id: &str) -> bool {
        false
    }

    pub fn foto(&self, _id: &str) -> Option<Image> {
        None
    }

    pub fn eventos(&self) -> Vec<Evento> {
        Vec::new()
    }

    pub fn movil(&self) -> Movil {
        Movil::SinVincular
    }

    pub fn vincular(&self) {}

    pub fn cancelar(&self) {}

    pub fn responder(&self, _si: bool) {}

    pub fn desvincular(&self) {}
}
