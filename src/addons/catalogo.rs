//! Catálogo de addons: los que se pueden instalar, en el repositorio fate-9x/nook-addons de
//! GitHub. Su `catalogo.json` dice, de cada addon, su última versión, el tag de git en el que está
//! (sus archivos se descargan de ese tag, así no cambian) y el SHA-256 de cada archivo, que se
//! comprueba antes de instalarlo.
//!
//! Las llamadas a la red bloquean: hay que hacerlas en un hilo aparte.

use crate::red;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Dónde está el repositorio de los addons (sus archivos tal cual, por rama o por tag).
const REPOSITORIO: &str = "https://raw.githubusercontent.com/fate-9x/nook-addons";

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Catalogo {
    pub addons: Vec<Entrada>,
}

/// Un addon del catálogo.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Entrada {
    pub id: String,
    pub nombre: HashMap<String, String>,
    #[serde(default)]
    pub descripcion: HashMap<String, String>,
    pub version: String,
    #[serde(default)]
    pub nook_minimo: Option<String>,
    /// Trazado SVG de su icono (el de su manifiesto).
    #[serde(default)]
    pub icono: String,
    /// Tag de git con los archivos de esta versión.
    pub tag: String,
    /// Nombre de cada archivo y su SHA-256 (en hexadecimal).
    pub archivos: BTreeMap<String, String>,
}

impl Entrada {
    /// Si funciona con esta versión de nook.
    pub fn compatible(&self) -> bool {
        super::compatible(self.nook_minimo.as_deref())
    }
}

/// Dirección del repositorio. En las compilaciones de desarrollo se puede cambiar con la
/// variable de entorno `NOOK_CATALOGO` (para probar con un servidor local).
fn repositorio() -> String {
    if cfg!(debug_assertions) {
        if let Ok(otro) = std::env::var("NOOK_CATALOGO") {
            return otro.trim_end_matches('/').to_string();
        }
    }
    REPOSITORIO.to_string()
}

/// Descarga el catálogo.
pub fn descargar() -> Result<Catalogo, String> {
    let json = red::texto(&format!("{}/main/catalogo.json", repositorio())).map_err(|e| e.message())?;
    leer(&json)
}

fn leer(json: &str) -> Result<Catalogo, String> {
    let catalogo: Catalogo = serde_json::from_str(json).map_err(|e| e.to_string())?;
    // Lo que no tenga un nombre seguro (podría salirse de su carpeta) no se ofrece.
    Ok(Catalogo {
        addons: catalogo
            .addons
            .into_iter()
            .filter(|a| super::id_valido(&a.id) && a.archivos.keys().all(|archivo| nombre_valido(archivo)))
            .collect(),
    })
}

/// Nombre de un archivo de un addon: sin carpetas ni nada raro.
fn nombre_valido(nombre: &str) -> bool {
    !nombre.is_empty()
        && !nombre.starts_with('.')
        && nombre.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Descarga los archivos de un addon, comprueba que son los del catálogo y lo instala (en lugar
/// de la versión que hubiera). Si algo falla, lo que había se queda como estaba.
pub fn instalar(entrada: &Entrada) -> Result<(), String> {
    if !super::id_valido(&entrada.id) {
        return Err(format!("id no válido: {}", entrada.id));
    }
    let carpeta = super::carpeta();
    let destino = carpeta.join(&entrada.id);
    let nueva = carpeta.join(format!(".{}.nuevo", entrada.id));
    let vieja = carpeta.join(format!(".{}.viejo", entrada.id));
    let _ = std::fs::remove_dir_all(&nueva);
    std::fs::create_dir_all(&nueva).map_err(|e| e.to_string())?;
    let resultado = (|| {
        for (archivo, hash) in &entrada.archivos {
            if !nombre_valido(archivo) {
                return Err(format!("nombre de archivo no válido: {archivo}"));
            }
            let url = format!("{}/{}/addons/{}/{archivo}", repositorio(), entrada.tag, entrada.id);
            let datos = red::descargar(&url, 0, |_| {}).map_err(|e| format!("{archivo}: {}", e.message()))?;
            if !sha256(&datos).eq_ignore_ascii_case(hash) {
                return Err(format!("{archivo}: no es el archivo del catálogo"));
            }
            std::fs::write(nueva.join(archivo), &datos).map_err(|e| format!("{archivo}: {e}"))?;
        }
        // Que el manifiesto se pueda leer y sea de este addon.
        let addon = super::Addon::leer(&nueva)?;
        if addon.manifiesto.id != entrada.id {
            return Err(format!("addon.json dice que es {}", addon.manifiesto.id));
        }
        Ok(())
    })();
    if let Err(e) = resultado {
        let _ = std::fs::remove_dir_all(&nueva);
        return Err(e);
    }
    let _ = std::fs::remove_dir_all(&vieja);
    if destino.exists() {
        std::fs::rename(&destino, &vieja).map_err(|e| e.to_string())?;
    }
    if let Err(e) = std::fs::rename(&nueva, &destino) {
        let _ = std::fs::rename(&vieja, &destino);
        return Err(e.to_string());
    }
    let _ = std::fs::remove_dir_all(&vieja);
    Ok(())
}

/// Quita un addon instalado y lo que había guardado.
pub fn quitar(id: &str) -> Result<(), String> {
    if !super::id_valido(id) {
        return Err(format!("id no válido: {id}"));
    }
    let carpeta = super::carpeta().join(id);
    if carpeta.exists() {
        std::fs::remove_dir_all(&carpeta).map_err(|e| e.to_string())?;
    }
    let datos = super::ruta_datos(id);
    if Path::new(&datos).exists() {
        let _ = std::fs::remove_file(datos);
    }
    Ok(())
}

/// SHA-256 en hexadecimal (con el de Windows).
pub fn sha256(datos: &[u8]) -> String {
    use windows::Win32::Security::Cryptography::{BCRYPT_SHA256_ALG_HANDLE, BCryptHash};
    let mut hash = [0u8; 32];
    // SAFETY: el algoritmo es uno de los que Windows tiene siempre abiertos y el resultado cabe.
    let estado = unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, None, datos, &mut hash) };
    debug_assert!(estado.is_ok());
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_conocido() {
        assert_eq!(sha256(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn lee_el_catalogo_y_descarta_lo_peligroso() {
        let json = r#"{"version":1,"addons":[
            {"id":"dado","nombre":{"es":"Dado"},"version":"0.1.0","tag":"dado-v0.1.0",
             "archivos":{"addon.json":"aa","panel.slint":"bb"}},
            {"id":"../malo","nombre":{"es":"Malo"},"version":"1.0.0","tag":"x","archivos":{}},
            {"id":"otro","nombre":{"es":"Otro"},"version":"1.0.0","tag":"x","archivos":{"../../a.exe":"cc"}}
        ]}"#;
        let catalogo = leer(json).unwrap();
        assert_eq!(catalogo.addons.len(), 1);
        assert_eq!(catalogo.addons[0].id, "dado");
        assert_eq!(catalogo.addons[0].archivos.len(), 2);
    }

    #[test]
    fn nombres_de_archivo() {
        assert!(nombre_valido("addon.json"));
        assert!(nombre_valido("logica.rhai"));
        assert!(!nombre_valido("../addon.json"));
        assert!(!nombre_valido("a/b.slint"));
        assert!(!nombre_valido(".oculto"));
        assert!(!nombre_valido(""));
    }
}
