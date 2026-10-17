//! Archivos `.nookaddon`: un addon entero en un solo archivo, para añadirlo a mano (con "Añadir
//! addon…" en la configuración o arrastrándolo a la barra). Es la forma de instalar addons en
//! Nookly Pro, que no los descarga por su cuenta; en la gratuita, además del catálogo.
//!
//! Es un JSON con los archivos del addon (que son de texto):
//! `{"formato": 1, "archivos": {"addon.json": "…", "panel.slint": "…", "logica.rhai": "…"}}`.
//! Los publica el repositorio nook-addons en sus releases (uno por versión de cada addon).

use super::{Manifiesto, panel};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Extensión de los archivos de addon.
pub const EXTENSION: &str = "nookaddon";
/// Lo más que puede ocupar uno (los addons son unos pocos archivos de texto).
const MAX_BYTES: u64 = 2 * 1024 * 1024;
const FORMATO: u32 = 1;

#[derive(Deserialize)]
struct ArchivoAddon {
    formato: u32,
    archivos: BTreeMap<String, String>,
}

/// Si una ruta es la de un archivo de addon (por su extensión).
pub fn es_archivo_de_addon(ruta: &Path) -> bool {
    ruta.extension().is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

/// Lee un `.nookaddon`: sus archivos y su manifiesto.
fn leer(texto: &str) -> Result<(BTreeMap<String, String>, Manifiesto), String> {
    let archivo: ArchivoAddon = serde_json::from_str(texto).map_err(|e| e.to_string())?;
    if archivo.formato != FORMATO {
        return Err(format!("formato {} (este Nookly lee el {FORMATO})", archivo.formato));
    }
    if let Some(malo) = archivo.archivos.keys().find(|nombre| !super::nombre_de_archivo_valido(nombre)) {
        return Err(format!("nombre de archivo no válido: {malo}"));
    }
    let manifiesto = archivo.archivos.get("addon.json").ok_or("no tiene addon.json")?;
    let manifiesto: Manifiesto = serde_json::from_str(manifiesto).map_err(|e| format!("addon.json: {e}"))?;
    if !super::id_valido(&manifiesto.id) {
        return Err(format!("id no válido: {}", manifiesto.id));
    }
    Ok((archivo.archivos, manifiesto))
}

/// Instala el addon de un `.nookaddon` (en lugar de la versión que hubiera), comprobando antes que
/// funciona con este Nookly y que su panel compila. Devuelve el addon instalado.
pub fn instalar(ruta: &Path) -> Result<Manifiesto, String> {
    let bytes = std::fs::metadata(ruta).map_err(|e| e.to_string())?.len();
    if bytes > MAX_BYTES {
        return Err("es demasiado grande para ser un addon".into());
    }
    let texto = std::fs::read_to_string(ruta).map_err(|e| e.to_string())?;
    let (archivos, manifiesto) = leer(&texto)?;
    if !super::compatible(manifiesto.nook_minimo.as_deref()) {
        return Err(format!("necesita Nookly {} o más nuevo", manifiesto.nook_minimo.as_deref().unwrap_or_default()));
    }
    super::colocar(&manifiesto.id, |carpeta| {
        for (nombre, contenido) in &archivos {
            super::escribir_archivo(carpeta, nombre, contenido.as_bytes())?;
        }
        // Mejor saberlo ahora que al abrirlo.
        panel::compilar(carpeta).map(|_| ())
    })?;
    Ok(manifiesto)
}

/// Abre el selector de archivos de Windows para elegir un `.nookaddon`. `filtro` es el nombre
/// del tipo de archivo que se muestra en él.
pub fn elegir(hwnd: Option<windows::Win32::Foundation::HWND>, filtro: &str) -> Option<PathBuf> {
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{FOS_FILEMUSTEXIST, FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH};
    use windows_core::{HSTRING, PCWSTR};

    let nombre = HSTRING::from(filtro);
    let patron = HSTRING::from(format!("*.{EXTENSION}"));
    // SAFETY: llamadas normales al selector de Windows desde el hilo de la interfaz (con COM
    // ya preparado, ver plataforma::instalar); los textos viven hasta el final.
    unsafe {
        let dialogo: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let tipos = [COMDLG_FILTERSPEC { pszName: PCWSTR(nombre.as_ptr()), pszSpec: PCWSTR(patron.as_ptr()) }];
        dialogo.SetFileTypes(&tipos).ok()?;
        let opciones = dialogo.GetOptions().ok()?;
        dialogo.SetOptions(opciones | FOS_FILEMUSTEXIST).ok()?;
        // Cancelar también da error: entonces no se elige nada.
        dialogo.Show(hwnd).ok()?;
        let elegido = dialogo.GetResult().ok()?.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let ruta = elegido.to_string().ok();
        windows::Win32::System::Com::CoTaskMemFree(Some(elegido.0 as _));
        ruta.map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFIESTO: &str = r#"{\"id\":\"dado\",\"nombre\":{\"es\":\"Dado\"},\"version\":\"0.1.0\",\"panel\":{\"ancho\":300,\"alto\":132}}"#;

    #[test]
    fn lee_un_archivo_de_addon() {
        let texto = format!(r#"{{"formato":1,"archivos":{{"addon.json":"{MANIFIESTO}","panel.slint":"x"}}}}"#);
        let (archivos, manifiesto) = leer(&texto).unwrap();
        assert_eq!(manifiesto.id, "dado");
        assert_eq!(archivos.len(), 2);
    }

    #[test]
    fn rechaza_lo_que_no_es_un_addon() {
        assert!(leer(r#"{"formato":2,"archivos":{}}"#).is_err());
        assert!(leer(r#"{"formato":1,"archivos":{"panel.slint":"x"}}"#).is_err());
        let fuera = format!(r#"{{"formato":1,"archivos":{{"addon.json":"{MANIFIESTO}","../malo.exe":"x"}}}}"#);
        assert!(leer(&fuera).is_err());
        assert!(leer("no es json").is_err());
    }

    #[test]
    fn reconoce_la_extension() {
        assert!(es_archivo_de_addon(Path::new(r"C:\Descargas\dado-0.1.0.nookaddon")));
        assert!(es_archivo_de_addon(Path::new("DADO.NOOKADDON")));
        assert!(!es_archivo_de_addon(Path::new("dado.zip")));
    }
}
