//! Actualizaciones desde las releases de GitHub (https://github.com/fate-9x/nook/releases).
//!
//! - La versión del programa sale del tag con el que se compiló (ver build.rs).
//! - `buscar` pregunta a GitHub por la última release y, si es más nueva, la deja anotada.
//! - `instalar` descarga su `nook.exe`, lo pone en lugar del ejecutable actual y abre la versión
//!   nueva; quien la llama cierra después esta. Windows no deja sobrescribir un ejecutable abierto,
//!   pero sí renombrarlo: el actual pasa a `nook.exe.viejo` y se borra al arrancar la versión nueva.
//!
//! Las llamadas a la red bloquean: hay que hacerlas en un hilo aparte.

use crate::idioma::{tr, tr_con};
use crate::red;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Versión de este ejecutable, sin la "v" (por ejemplo "0.2.0").
pub const VERSION: &str = env!("NOOK_VERSION");

/// Argumento con el que se abre la versión nueva tras actualizar: espera a que se cierre esta.
pub const TRAS_ACTUALIZAR: &str = "--tras-actualizar";

const ULTIMA_RELEASE: &str = "https://api.github.com/repos/fate-9x/nook/releases/latest";
/// Nombre del ejecutable adjunto a cada release (lo sube el workflow de GitHub Actions).
const ARCHIVO: &str = "nook.exe";

/// Release más nueva que esta versión.
#[derive(Clone)]
pub struct Nueva {
    /// Versión sin la "v".
    pub version: String,
    url: String,
    bytes: u64,
}

/// Última release encontrada que es más nueva que esta versión.
static DISPONIBLE: Mutex<Option<Nueva>> = Mutex::new(None);

pub fn disponible() -> Option<Nueva> {
    DISPONIBLE.lock().ok()?.clone()
}

/// Pregunta a GitHub por la última release. Si es más nueva que esta versión, la anota y la
/// devuelve.
pub fn buscar() -> Option<Nueva> {
    let json = red::texto(ULTIMA_RELEASE).ok()?;
    let nueva = leer_release(&json)?;
    if !es_mas_nueva(&nueva.version, VERSION) {
        return None;
    }
    if let Ok(mut d) = DISPONIBLE.lock() {
        *d = Some(nueva.clone());
    }
    Some(nueva)
}

/// Descarga la versión nueva, la pone en lugar de este ejecutable y la abre. `progreso` recibe
/// cuánto se ha descargado (0–1). Si va bien, hay que cerrar este programa enseguida.
pub fn instalar(nueva: &Nueva, progreso: impl Fn(f32)) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let datos = red::descargar(&nueva.url, nueva.bytes, progreso).map_err(|e| tr_con("no se pudo descargar: {}", &[&e.to_string()]))?;
    // Un ejecutable de Windows empieza por "MZ": si no, se ha descargado otra cosa.
    if !datos.starts_with(b"MZ") {
        return Err(tr("lo descargado no es un ejecutable").into());
    }
    let nuevo = con_sufijo(&exe, "nuevo");
    let viejo = con_sufijo(&exe, "viejo");
    std::fs::write(&nuevo, &datos).map_err(|e| tr_con("no se pudo guardar: {}", &[&e.to_string()]))?;
    let _ = std::fs::remove_file(&viejo);
    std::fs::rename(&exe, &viejo).map_err(|e| tr_con("no se pudo apartar la versión actual: {}", &[&e.to_string()]))?;
    if let Err(e) = std::fs::rename(&nuevo, &exe) {
        let _ = std::fs::rename(&viejo, &exe);
        return Err(tr_con("no se pudo colocar la versión nueva: {}", &[&e.to_string()]));
    }
    std::process::Command::new(&exe)
        .arg(TRAS_ACTUALIZAR)
        .spawn()
        .map_err(|e| tr_con("no se pudo abrir la versión nueva: {}", &[&e.to_string()]))?;
    Ok(())
}

/// Borra el ejecutable que dejó apartado la última actualización.
pub fn limpiar() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(con_sufijo(&exe, "viejo"));
    }
}

/// Versión, dirección y tamaño del `nook.exe` de una release (la respuesta JSON de GitHub).
fn leer_release(json: &str) -> Option<Nueva> {
    let release: serde_json::Value = serde_json::from_str(json).ok()?;
    let version = release["tag_name"].as_str()?.trim_start_matches('v').to_string();
    let adjunto = release["assets"].as_array()?.iter().find(|a| a["name"] == ARCHIVO)?;
    Some(Nueva {
        version,
        url: adjunto["browser_download_url"].as_str()?.to_string(),
        bytes: adjunto["size"].as_u64().unwrap_or(0),
    })
}

/// Si la versión `a` es posterior a `b` ("1.2.3"; lo que vaya tras un "-" no cuenta).
pub(crate) fn es_mas_nueva(a: &str, b: &str) -> bool {
    let numeros = |v: &str| -> Vec<u64> {
        let v = v.trim_start_matches('v');
        v.split('-').next().unwrap_or(v).split('.').map(|n| n.parse().unwrap_or(0)).collect()
    };
    numeros(a) > numeros(b)
}

fn con_sufijo(exe: &Path, sufijo: &str) -> PathBuf {
    let mut nombre = exe.as_os_str().to_owned();
    nombre.push(".");
    nombre.push(sufijo);
    PathBuf::from(nombre)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compara_versiones() {
        assert!(es_mas_nueva("0.2.0", "0.1.9"));
        assert!(es_mas_nueva("v1.0.0", "0.9.9"));
        assert!(es_mas_nueva("0.10.0", "0.9.0"));
        assert!(!es_mas_nueva("0.2.0", "0.2.0"));
        assert!(!es_mas_nueva("0.1.0", "0.2.0"));
        assert!(!es_mas_nueva("0.2.0", "0.2.0-3-gabc123"));
    }

    #[test]
    fn lee_la_release_de_github() {
        let json = r#"{"tag_name":"v0.3.1","assets":[
            {"name":"otro.zip","browser_download_url":"https://x/otro.zip","size":1},
            {"name":"nook.exe","browser_download_url":"https://x/nook.exe","size":1234}]}"#;
        let nueva = leer_release(json).unwrap();
        assert_eq!(nueva.version, "0.3.1");
        assert_eq!(nueva.url, "https://x/nook.exe");
        assert_eq!(nueva.bytes, 1234);
        assert!(leer_release(r#"{"tag_name":"v1.0.0","assets":[]}"#).is_none());
    }
}
