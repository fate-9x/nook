//! Los personajes y accesorios que hay: los que trae el programa y los del usuario, cada uno en su
//! carpeta dentro de la de los personajes (`%APPDATA%\nook\personajes\<carpeta>\`, ver
//! `figura.rs`) o de la de los accesorios (`%APPDATA%\nook\accesorios\<carpeta>\`, ver
//! `accesorio_archivo.rs`). Los accesorios de un personaje también pueden estar en su carpeta, en
//! `accesorios\<carpeta>\`: solo se ofrecen con él. Se vuelven a leer cuando algo cambia en esas
//! carpetas (ver `firma`): así lo que se edita se ve al momento.

use super::accesorio_archivo::Archivo;
use super::figura::Figura;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

/// Los ids de los personajes del usuario son el nombre de su carpeta con esto delante (así no se
/// confunden con los que trae el programa aunque la carpeta se llame igual).
const PREFIJO: &str = "usuario/";

/// Los personajes y los accesorios de archivo que hay y, de los que no se pudieron leer, su
/// carpeta y por qué.
pub struct Coleccion {
    pub figuras: Vec<Arc<Figura>>,
    pub accesorios: Vec<Arc<Archivo>>,
    pub errores: Vec<(String, String)>,
}

/// Las carpetas de un personaje donde puede haber accesorios.
const ACCESORIOS: &str = "accesorios";

impl Coleccion {
    /// Los personajes que trae el programa y, detrás, los de la carpeta `personajes` por su nombre;
    /// y los accesorios de la carpeta `accesorios` y los de cada personaje. De los que no se
    /// pueden leer, se queda el de `anterior` si lo tenía: así, mientras se edita uno, no
    /// desaparece cada vez que se guarda a medias (el error se anota igual).
    pub fn leer(personajes: &Path, accesorios: &Path, anterior: Option<&Coleccion>) -> Coleccion {
        let (figuras, mut errores) = Coleccion::leer_figuras(personajes, anterior);
        let mut archivos = Coleccion::leer_accesorios(accesorios, None, "accesorios/", &mut errores, anterior);
        for figura in figuras.iter().filter(|f| f.id.starts_with(PREFIJO)) {
            let Some(suya) = carpeta_de(personajes, &figura.id) else { continue };
            let nombre = figura.id[PREFIJO.len()..].to_string();
            archivos.extend(Coleccion::leer_accesorios(&suya.join(ACCESORIOS), Some(&figura.id), &format!("{nombre}/{ACCESORIOS}/"), &mut errores, anterior));
        }
        Coleccion { figuras, accesorios: archivos, errores }
    }

    /// Los accesorios de una carpeta (`propietario`: el personaje al que pertenecen; `rotulo`: lo
    /// que va delante de su carpeta en los errores).
    fn leer_accesorios(
        carpeta: &Path,
        propietario: Option<&str>,
        rotulo: &str,
        errores: &mut Vec<(String, String)>,
        anterior: Option<&Coleccion>,
    ) -> Vec<Arc<Archivo>> {
        let mut archivos = Vec::new();
        for entrada in carpetas_de(carpeta) {
            let nombre = entrada.file_name().to_string_lossy().into_owned();
            let id = match propietario {
                Some(p) => format!("{p}/{nombre}"),
                None => format!("{PREFIJO}{nombre}"),
            };
            match Archivo::leer(&entrada.path(), id.clone(), propietario.map(str::to_string)) {
                Ok(archivo) => archivos.push(Arc::new(archivo)),
                Err(e) => {
                    if let Some(antes) = anterior.and_then(|a| a.accesorios.iter().find(|f| f.id == id)) {
                        archivos.push(antes.clone());
                    }
                    errores.push((format!("{rotulo}{nombre}"), e));
                }
            }
        }
        archivos
    }

    fn leer_figuras(carpeta: &Path, anterior: Option<&Coleccion>) -> (Vec<Arc<Figura>>, Vec<(String, String)>) {
        let mut figuras: Vec<Arc<Figura>> = Figura::integradas().into();
        let mut errores = Vec::new();
        for entrada in carpetas_de(carpeta) {
            let nombre = entrada.file_name().to_string_lossy().into_owned();
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
        (figuras, errores)
    }

    /// El personaje con ese id (si ya no está, Nook).
    pub fn buscar(&self, id: &str) -> Arc<Figura> {
        self.figuras.iter().find(|f| f.id == id).cloned().unwrap_or_else(Figura::nook)
    }

    /// Los accesorios de archivo que se pueden poner a la figura `figura`: los de todos y los
    /// suyos.
    pub fn accesorios_de<'a>(&'a self, figura: &'a str) -> impl Iterator<Item = &'a Arc<Archivo>> {
        self.accesorios.iter().filter(move |a| a.propietario.as_deref().is_none_or(|p| p == figura))
    }

    /// El accesorio de archivo con ese id, si se puede poner a la figura `figura`.
    pub fn accesorio<'a>(&'a self, id: &str, figura: &'a str) -> Option<&'a Arc<Archivo>> {
        self.accesorios_de(figura).find(|a| a.id == id)
    }
}

impl Default for Coleccion {
    fn default() -> Self {
        Coleccion { figuras: Figura::integradas().into(), accesorios: Vec::new(), errores: Vec::new() }
    }
}

/// Las carpetas de una carpeta, por su nombre, sin las que empiezan por un punto (están a medio
/// escribir o escondidas).
fn carpetas_de(carpeta: &Path) -> Vec<std::fs::DirEntry> {
    let mut carpetas: Vec<_> = std::fs::read_dir(carpeta)
        .map(|entradas| entradas.flatten().filter(|e| e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.')).collect())
        .unwrap_or_default();
    carpetas.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
    carpetas
}

/// Un resumen de lo que hay en las carpetas de los personajes y de los accesorios (nombres,
/// tamaños y fechas de sus archivos, hasta las carpetas de accesorios dentro de cada personaje): si
/// cambia, hay que volver a leerlas.
pub fn firma(personajes: &Path, accesorios: &Path) -> u64 {
    fn anotar(carpeta: &Path, profundidad: u32, h: &mut impl Hasher) {
        carpeta.hash(h);
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
    anotar(personajes, 3, &mut h);
    anotar(accesorios, 2, &mut h);
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

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-50 -50 100 100"><circle r="10" fill="#336699"/><rect x="-5" y="-30" width="10" height="5" fill="#cc3333"/></svg>"##;

    /// Un accesorio de prueba en `carpeta/nombre`.
    fn accesorio(carpeta: &Path, nombre: &str, json: &str) {
        std::fs::create_dir_all(carpeta.join(nombre)).unwrap();
        std::fs::write(carpeta.join(nombre).join("accesorio.json"), json).unwrap();
        std::fs::write(carpeta.join(nombre).join("accesorio.svg"), SVG).unwrap();
    }

    #[test]
    fn lee_los_del_usuario_detras_de_los_integrados_y_anota_los_que_fallan() {
        let carpeta = carpeta_de_prueba();
        let sin_accesorios = carpeta.join("accesorios");
        let id = crear(&carpeta, "Mi gato").unwrap();
        assert_eq!(id, "usuario/mi-gato");
        // Otro con el mismo nombre va a otra carpeta.
        assert_eq!(crear(&carpeta, "Mi gato").unwrap(), "usuario/mi-gato-2");
        std::fs::create_dir_all(carpeta.join("roto")).unwrap();
        std::fs::write(carpeta.join("roto").join("personaje.json"), "{").unwrap();
        let antes = firma(&carpeta, &sin_accesorios);

        let coleccion = Coleccion::leer(&carpeta, &sin_accesorios, None);
        let ids: Vec<_> = coleccion.figuras.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, vec!["nook", "limon", "usuario/mi-gato", "usuario/mi-gato-2"]);
        assert_eq!(coleccion.buscar("usuario/mi-gato").nombre["es"], "Mi gato");
        assert_eq!(coleccion.errores.len(), 1);
        assert_eq!(coleccion.errores[0].0, "roto");
        // Lo que ya no está es Nook.
        assert_eq!(coleccion.buscar("usuario/no-existe").id, "nook");

        // Al cambiar un archivo cambia la firma.
        std::fs::write(carpeta.join("roto").join("personaje.svg"), "<svg/>").unwrap();
        assert_ne!(firma(&carpeta, &sin_accesorios), antes);

        // Si uno que se leía bien se rompe, se queda como estaba (con su error).
        std::fs::write(carpeta.join("mi-gato").join("personaje.json"), "{").unwrap();
        let despues = Coleccion::leer(&carpeta, &sin_accesorios, Some(&coleccion));
        assert!(despues.figuras.iter().any(|f| f.id == "usuario/mi-gato"));
        assert!(despues.errores.iter().any(|(c, _)| c == "mi-gato"));
        assert!(!Coleccion::leer(&carpeta, &sin_accesorios, None).figuras.iter().any(|f| f.id == "usuario/mi-gato"));
        let _ = std::fs::remove_dir_all(&carpeta);
    }

    #[test]
    fn lee_los_accesorios_de_todos_y_los_de_cada_personaje() {
        let base = carpeta_de_prueba();
        let (personajes, accesorios) = (base.join("personajes"), base.join("accesorios"));
        crear(&personajes, "Gato").unwrap();
        crear(&personajes, "Perro").unwrap();
        accesorio(&accesorios, "sombrero", r#"{"nombre": {"es": "Sombrero"}, "modo": "cartel", "ancla": "coronilla"}"#);
        accesorio(&accesorios, "roto", r#"{"modo": "volando"}"#);
        // Uno que solo es del gato, en la carpeta del gato.
        accesorio(&personajes.join("gato").join("accesorios"), "bufanda", r#"{"modo": "cara", "ancla": "boca"}"#);
        let antes = firma(&personajes, &accesorios);

        let coleccion = Coleccion::leer(&personajes, &accesorios, None);
        let ids = |figura: &str| coleccion.accesorios_de(figura).map(|a| a.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids("usuario/gato"), vec!["usuario/sombrero", "usuario/gato/bufanda"]);
        assert_eq!(ids("usuario/perro"), vec!["usuario/sombrero"]);
        assert_eq!(ids("nook"), vec!["usuario/sombrero"]);
        assert!(coleccion.accesorio("usuario/gato/bufanda", "usuario/gato").is_some());
        assert!(coleccion.accesorio("usuario/gato/bufanda", "usuario/perro").is_none());
        assert_eq!(coleccion.accesorios[0].nombre["es"], "Sombrero");
        // El que falla se anota, con su carpeta.
        assert_eq!(coleccion.errores.len(), 1);
        assert_eq!(coleccion.errores[0].0, "accesorios/roto");
        assert!(coleccion.errores[0].1.contains("accesorio.json"), "{}", coleccion.errores[0].1);

        // Al tocar un accesorio cambia la firma; si se rompe uno que estaba bien, se queda.
        std::fs::write(accesorios.join("sombrero").join("accesorio.json"), "{").unwrap();
        assert_ne!(firma(&personajes, &accesorios), antes);
        let despues = Coleccion::leer(&personajes, &accesorios, Some(&coleccion));
        assert!(despues.accesorios.iter().any(|a| a.id == "usuario/sombrero"));
        assert_eq!(despues.errores.len(), 2);
        // Y la del gato también se nota (está dentro de su carpeta).
        let antes = firma(&personajes, &accesorios);
        accesorio(&personajes.join("gato").join("accesorios"), "bufanda", r#"{"modo": "cartel", "ancla": "cuello"}"#);
        assert_ne!(firma(&personajes, &accesorios), antes);
        let _ = std::fs::remove_dir_all(&base);
    }
}
