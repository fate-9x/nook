//! Conversación con Claude Code (solo en nook Pro): en la versión gratuita no se puede abrir.

use std::path::{Path, PathBuf};
use windows::Win32::Foundation::HWND;

/// Lo que va llegando de Claude mientras responde.
#[allow(dead_code)] // solo los crea la versión Pro
#[derive(Clone, Debug, PartialEq)]
pub enum Suceso {
    /// La conversación tiene este id (para seguirla en la siguiente pregunta).
    Sesion(String),
    /// Lo que dice.
    Texto(String),
    /// Usa una herramienta: su nombre y sobre qué (por ejemplo, "Read src/main.rs").
    Herramienta(String),
    /// Ha terminado: las herramientas que no le dejaron usar y, si falló, por qué.
    Fin { denegadas: Vec<String>, error: Option<String> },
}

/// Una respuesta en marcha.
pub struct Respuesta;

impl Respuesta {
    /// Corta la respuesta.
    pub fn parar(&self) {}
}

/// Le pregunta `mensaje` a Claude Code en `carpeta` (siguiendo la conversación `sesion`, si hay)
/// y llama a `al_llegar`, desde otro hilo, con lo que va respondiendo.
pub fn preguntar(
    _mensaje: &str,
    _carpeta: &Path,
    _sesion: Option<&str>,
    _al_llegar: impl Fn(Suceso) + Send + 'static,
) -> std::io::Result<Respuesta> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// Selector de carpetas de Windows.
pub fn elegir_carpeta(_hwnd: Option<HWND>) -> Option<PathBuf> {
    None
}
