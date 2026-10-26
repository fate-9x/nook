//! Addons: paquetes que añaden un panel a la barra (un minijuego, por ejemplo) y que solo se
//! cargan al abrirlos. Cada uno está en su carpeta de `%APPDATA%\nook\addons\<id>\`, con:
//!
//! - `addon.json`: el manifiesto (`Manifiesto`).
//! - `panel.slint`: su interfaz, que exporta el componente `Panel` (ver `panel.rs`).
//! - `logica.rhai`: lo que hace (ver `guion.rs`).

pub mod archivo;
pub mod catalogo;
pub mod conectar;
pub mod guion;
pub mod mcp;
pub mod panel;
pub mod tuberia;

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Lo que dice de sí mismo un addon en su `addon.json`.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Manifiesto {
    pub id: String,
    /// Nombre y descripción por idioma ("es", "en"…).
    pub nombre: HashMap<String, String>,
    #[serde(default)]
    pub descripcion: HashMap<String, String>,
    pub version: String,
    /// Versión de nook a partir de la que funciona.
    #[serde(default)]
    pub nook_minimo: Option<String>,
    pub panel: Medidas,
    /// Trazado SVG de su icono en una caja de 24×24 (la tira lo tiñe con el color del tema).
    #[serde(default)]
    pub icono: String,
    /// Lo que puede usar la IA del usuario por MCP (ver `mcp.rs`).
    #[serde(default)]
    pub herramientas: Vec<Herramienta>,
}

/// Herramienta MCP de un addon: al usarla se llama a `herramienta_<nombre>` de su script.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Herramienta {
    pub nombre: String,
    #[serde(default)]
    pub descripcion: String,
    /// JSON Schema de sus argumentos.
    #[serde(default = "sin_parametros")]
    pub parametros: serde_json::Value,
}

fn sin_parametros() -> serde_json::Value {
    serde_json::json!({ "type": "object", "properties": {} })
}

impl Manifiesto {
    /// Su nombre en el idioma de la interfaz (o en español, o el primero que haya).
    pub fn nombre(&self) -> String {
        traducido(&self.nombre)
    }

    pub fn descripcion(&self) -> String {
        traducido(&self.descripcion)
    }
}

/// El texto en el idioma de la interfaz, en español o el primero que haya.
pub fn traducido(textos: &HashMap<String, String>) -> String {
    let idioma = crate::idioma::codigo_actual();
    textos
        .get(idioma)
        .or_else(|| textos.get("es"))
        .or_else(|| textos.values().next())
        .cloned()
        .unwrap_or_default()
}

/// Tamaño del panel, en píxeles lógicos.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Medidas {
    pub ancho: f32,
    pub alto: f32,
    /// El alto sigue a lo que ocupa el contenido del panel (su alto preferido), con `alto` como
    /// máximo.
    #[serde(default)]
    pub ajustar_alto: bool,
}

/// Un addon instalado.
#[derive(Clone, Debug)]
pub struct Addon {
    pub manifiesto: Manifiesto,
    pub carpeta: PathBuf,
}

impl Addon {
    /// Si funciona con esta versión de nook.
    pub fn compatible(&self) -> bool {
        compatible(self.manifiesto.nook_minimo.as_deref())
    }

    /// Lee el addon de una carpeta.
    pub fn leer(carpeta: &Path) -> Result<Addon, String> {
        let ruta = carpeta.join("addon.json");
        let texto = std::fs::read_to_string(&ruta).map_err(|e| format!("{}: {e}", ruta.display()))?;
        let manifiesto: Manifiesto = serde_json::from_str(&texto).map_err(|e| format!("{}: {e}", ruta.display()))?;
        if !id_valido(&manifiesto.id) {
            return Err(format!("{}: id no válido", ruta.display()));
        }
        Ok(Addon { manifiesto, carpeta: carpeta.to_path_buf() })
    }
}

/// Los addons instalados, por su nombre. Los que no se pueden leer se anotan y se saltan.
pub fn instalados() -> Vec<Addon> {
    let Ok(carpetas) = std::fs::read_dir(carpeta()) else { return Vec::new() };
    let mut addons: Vec<Addon> = carpetas
        .flatten()
        .filter(|c| c.path().is_dir() && id_valido(&c.file_name().to_string_lossy()))
        .filter_map(|c| match Addon::leer(&c.path()) {
            Ok(addon) if addon.manifiesto.id == c.file_name().to_string_lossy() => Some(addon),
            Ok(addon) => {
                eprintln!("El addon de {} dice que es {}", c.path().display(), addon.manifiesto.id);
                None
            }
            Err(e) => {
                eprintln!("Addon: {e}");
                None
            }
        })
        .collect();
    addons.sort_by_key(|a| a.manifiesto.nombre().to_lowercase());
    addons
}

/// Si un addon que pide esta versión mínima funciona con este nook.
pub fn compatible(nook_minimo: Option<&str>) -> bool {
    nook_minimo.is_none_or(|minimo| !crate::actualizacion::es_mas_nueva(minimo, crate::actualizacion::VERSION))
}

/// Carpeta, dentro de la de los addons, con lo que guarda cada uno (ver `ruta_datos`).
const DATOS: &str = "datos";

/// Nombre de un addon (el de su carpeta): minúsculas, números y guiones. No puede llamarse como
/// la carpeta de los datos, que está junto a las de los addons.
pub fn id_valido(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id != DATOS
        && !id.starts_with('-')
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Nombre de un archivo de un addon: sin carpetas ni nada raro.
pub fn nombre_de_archivo_valido(nombre: &str) -> bool {
    !nombre.is_empty()
        && !nombre.starts_with('.')
        && nombre.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Escribe un archivo de un addon en su carpeta (si su nombre es válido).
pub fn escribir_archivo(carpeta: &Path, nombre: &str, datos: &[u8]) -> Result<(), String> {
    if !nombre_de_archivo_valido(nombre) {
        return Err(format!("nombre de archivo no válido: {nombre}"));
    }
    std::fs::write(carpeta.join(nombre), datos).map_err(|e| format!("{nombre}: {e}"))
}

/// Instala el addon `id` (en lugar de la versión que hubiera): `escribir` deja sus archivos en una
/// carpeta nueva, que se comprueba (su manifiesto tiene que ser de este addon) y se pone en su
/// sitio. Si algo falla, lo que había se queda como estaba.
pub fn colocar(id: &str, escribir: impl FnOnce(&Path) -> Result<(), String>) -> Result<(), String> {
    if !id_valido(id) {
        return Err(format!("id no válido: {id}"));
    }
    let base = carpeta();
    let destino = base.join(id);
    let nueva = base.join(format!(".{id}.nuevo"));
    let vieja = base.join(format!(".{id}.viejo"));
    let _ = std::fs::remove_dir_all(&nueva);
    std::fs::create_dir_all(&nueva).map_err(|e| e.to_string())?;
    let resultado = escribir(&nueva).and_then(|()| {
        let addon = Addon::leer(&nueva)?;
        if addon.manifiesto.id != id {
            return Err(format!("addon.json dice que es {}", addon.manifiesto.id));
        }
        Ok(())
    });
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

/// Carpeta en la que se instalan los addons.
pub fn carpeta() -> PathBuf {
    crate::config::carpeta().join("addons")
}

/// Archivo con lo que guarda un addon (`guardar` en su script). Va fuera de su carpeta, para que
/// no se pierda al actualizarlo.
pub fn ruta_datos(id: &str) -> PathBuf {
    carpeta().join(DATOS).join(format!("{id}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_el_manifiesto() {
        let json = r#"{"id":"dado","nombre":{"es":"Dado","en":"Dice"},"version":"0.1.0",
            "panel":{"ancho":300,"alto":132},"icono":"M0 0h24v24H0z",
            "herramientas":[{"nombre":"tirar","descripcion":"Tira el dado"}]}"#;
        let m: Manifiesto = serde_json::from_str(json).unwrap();
        assert_eq!(m.id, "dado");
        assert_eq!(m.panel.ancho, 300.0);
        assert_eq!(m.herramientas[0].nombre, "tirar");
        assert_eq!(m.herramientas[0].parametros["type"], "object");
        assert_eq!(m.nook_minimo, None);
        assert!(!m.panel.ajustar_alto);
        let m: Manifiesto = serde_json::from_str(&json.replace(r#""alto":132"#, r#""alto":132,"ajustar_alto":true"#)).unwrap();
        assert!(m.panel.ajustar_alto);
    }

    #[test]
    fn ids_validos() {
        assert!(id_valido("dado"));
        assert!(id_valido("batalla-naval2"));
        assert!(!id_valido(""));
        assert!(!id_valido("Dado"));
        assert!(!id_valido("../dado"));
        assert!(!id_valido("-dado"));
        assert!(!id_valido("da do"));
        // La carpeta de los datos no es un addon (ni se puede quitar como si lo fuera).
        assert!(!id_valido("datos"));
        assert!(id_valido("datos-2"));
    }

    #[test]
    fn nombres_de_archivo() {
        assert!(nombre_de_archivo_valido("addon.json"));
        assert!(nombre_de_archivo_valido("logica.rhai"));
        assert!(!nombre_de_archivo_valido("../addon.json"));
        assert!(!nombre_de_archivo_valido("a/b.slint"));
        assert!(!nombre_de_archivo_valido(".oculto"));
        assert!(!nombre_de_archivo_valido(""));
    }

    #[test]
    fn version_minima() {
        assert!(compatible(None));
        assert!(compatible(Some("0.0.1")));
        assert!(!compatible(Some("999.0.0")));
    }
}
