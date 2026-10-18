//! Los personajes que hay: los que trae el programa y los del usuario, cada uno en su carpeta
//! dentro de la de los personajes (`%APPDATA%\nook\personajes\<carpeta>\`, ver `figura.rs`). Se
//! vuelven a leer cuando algo cambia en ella (ver `firma`): así lo que se edita se ve al momento.

use super::figura::Figura;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

/// Los ids de los personajes del usuario son el nombre de su carpeta con esto delante (así no se
/// confunden con los que trae el programa aunque la carpeta se llame igual).
const PREFIJO: &str = "usuario/";

/// Los personajes que hay y, de los que no se pudieron leer, el nombre de su carpeta y por qué.
pub struct Coleccion {
    pub figuras: Vec<Arc<Figura>>,
    pub errores: Vec<(String, String)>,
}

impl Coleccion {
    /// Los que trae el programa y, detrás, los de la carpeta `carpeta` por su nombre. De los que
    /// no se pueden leer, se queda el de `anterior` si lo tenía: así, mientras se edita uno, no
    /// desaparece cada vez que se guarda a medias (el error se anota igual).
    pub fn leer(carpeta: &Path, anterior: Option<&Coleccion>) -> Coleccion {
        let mut figuras: Vec<Arc<Figura>> = Figura::integradas().into();
        let mut errores = Vec::new();
        let mut carpetas: Vec<_> = std::fs::read_dir(carpeta)
            .map(|entradas| entradas.flatten().filter(|e| e.path().is_dir()).collect())
            .unwrap_or_default();
        carpetas.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
        for entrada in carpetas {
            let nombre = entrada.file_name().to_string_lossy().into_owned();
            // Las que empiezan por un punto están a medio escribir o escondidas.
            if nombre.starts_with('.') {
                continue;
            }
            let id = format!("{PREFIJO}{nombre}");
            match Figura::leer(&entrada.path(), id.clone()) {
                Ok(figura) => figuras.push(Arc::new(figura)),
                Err(e) => {
                    if let Some(antes) = anterior.and_then(|a| a.figuras.iter().find(|f| f.id == id)) {
                        figuras.push(antes.clone());
                    }
                    errores.push((nombre, e));
                }
            }
        }
        Coleccion { figuras, errores }
    }

    /// El personaje con ese id (si ya no está, Nook).
    pub fn buscar(&self, id: &str) -> Arc<Figura> {
        self.figuras.iter().find(|f| f.id == id).cloned().unwrap_or_else(Figura::nook)
    }
}

impl Default for Coleccion {
    fn default() -> Self {
        Coleccion { figuras: Figura::integradas().into(), errores: Vec::new() }
    }
}

/// Un resumen de lo que hay en la carpeta de los personajes (nombres, tamaños y fechas de sus
/// archivos, hasta un nivel de carpetas dentro de cada uno): si cambia, hay que volver a leerla.
pub fn firma(carpeta: &Path) -> u64 {
    fn anotar(carpeta: &Path, profundidad: u32, h: &mut impl Hasher) {
        let Ok(entradas) = std::fs::read_dir(carpeta) else { return };
        let mut entradas: Vec<_> = entradas.flatten().collect();
        entradas.sort_by_key(|e| e.file_name());
        for entrada in entradas {
            entrada.file_name().hash(h);
            let Ok(datos) = entrada.metadata() else { continue };
            datos.len().hash(h);
            datos.modified().ok().hash(h);
            if datos.is_dir() && profundidad > 0 {
                anotar(&entrada.path(), profundidad - 1, h);
            }
        }
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    anotar(carpeta, 2, &mut h);
    h.finish()
}

/// Crea un personaje nuevo en la carpeta de los personajes a partir del dibujo de Nook, para
/// cambiarlo a gusto, con el nombre `nombre`. Devuelve su id.
pub fn crear(carpeta: &Path, nombre: &str) -> Result<String, String> {
    std::fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
    let base: String = nombre
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let base = if base.is_empty() { "personaje".to_string() } else { base };
    let (destino, nombre_carpeta) = (1..1000)
        .map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") })
        .map(|n| (carpeta.join(&n), n))
        .find(|(ruta, _)| !ruta.exists())
        .ok_or("no queda ningún nombre libre")?;
    let mut descripcion: serde_json::Value =
        serde_json::from_str(include_str!("../../personajes/nook/personaje.json")).map_err(|e| e.to_string())?;
    descripcion["nombre"] = serde_json::json!({ "es": nombre });
    let json = serde_json::to_string_pretty(&descripcion).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&destino).map_err(|e| e.to_string())?;
    let escrito = std::fs::write(destino.join("personaje.json"), json)
        .and_then(|()| std::fs::write(destino.join("personaje.svg"), include_str!("../../personajes/nook/personaje.svg")));
    if let Err(e) = escrito {
        let _ = std::fs::remove_dir_all(&destino);
        return Err(e.to_string());
    }
    Ok(format!("{PREFIJO}{nombre_carpeta}"))
}

/// La carpeta de un personaje del usuario, por su id (los que trae el programa no tienen).
pub fn carpeta_de(carpeta: &Path, id: &str) -> Option<std::path::PathBuf> {
    id.strip_prefix(PREFIJO).map(|nombre| carpeta.join(nombre))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carpeta_de_prueba() -> std::path::PathBuf {
        let carpeta = std::env::temp_dir().join(format!("nook-prueba-personajes-{}", crate::azar::siguiente().to_bits()));
        std::fs::create_dir_all(&carpeta).unwrap();
        carpeta
    }

    #[test]
    fn lee_los_del_usuario_detras_de_los_integrados_y_anota_los_que_fallan() {
        let carpeta = carpeta_de_prueba();
        let id = crear(&carpeta, "Mi gato").unwrap();
        assert_eq!(id, "usuario/mi-gato");
        // Otro con el mismo nombre va a otra carpeta.
        assert_eq!(crear(&carpeta, "Mi gato").unwrap(), "usuario/mi-gato-2");
        std::fs::create_dir_all(carpeta.join("roto")).unwrap();
        std::fs::write(carpeta.join("roto").join("personaje.json"), "{").unwrap();
        let antes = firma(&carpeta);

        let coleccion = Coleccion::leer(&carpeta, None);
        let ids: Vec<_> = coleccion.figuras.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, vec!["nook", "limon", "usuario/mi-gato", "usuario/mi-gato-2"]);
        assert_eq!(coleccion.buscar("usuario/mi-gato").nombre["es"], "Mi gato");
        assert_eq!(coleccion.errores.len(), 1);
        assert_eq!(coleccion.errores[0].0, "roto");
        // Lo que ya no está es Nook.
        assert_eq!(coleccion.buscar("usuario/no-existe").id, "nook");

        // Al cambiar un archivo cambia la firma.
        std::fs::write(carpeta.join("roto").join("personaje.svg"), "<svg/>").unwrap();
        assert_ne!(firma(&carpeta), antes);

        // Si uno que se leía bien se rompe, se queda como estaba (con su error).
        std::fs::write(carpeta.join("mi-gato").join("personaje.json"), "{").unwrap();
        let despues = Coleccion::leer(&carpeta, Some(&coleccion));
        assert!(despues.figuras.iter().any(|f| f.id == "usuario/mi-gato"));
        assert!(despues.errores.iter().any(|(c, _)| c == "mi-gato"));
        assert!(!Coleccion::leer(&carpeta, None).figuras.iter().any(|f| f.id == "usuario/mi-gato"));
        let _ = std::fs::remove_dir_all(&carpeta);
    }
}
