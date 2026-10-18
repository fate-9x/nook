//! Figura del personaje: de qué está hecho y cómo se mueve cada parte. Sale de una carpeta con un
//! `personaje.json` (qué es cada parte) y su dibujo: un `personaje.svg` por capas o, en el modo
//! de imágenes, unos PNG por animación. Nook y Limón son figuras como cualquier otra, con sus
//! archivos en `personajes/` (dentro del programa), y sirven de ejemplo.
//!
//! Las medidas se pasan a unidades del radio del cuerpo, con el centro del cuerpo en (0, 0): da
//! igual a qué tamaño esté dibujado. El radio es la mitad del ancho o del alto del cuerpo (lo
//! que sea mayor), o algo más si lo que sale de él (un rulo, unas orejas) no cabría en la imagen.
//!
//! Capas del dibujo vectorial (ver `TipoCapa`): el cuerpo, lo que va pintado en él (manchas,
//! coloretes y franjas de pelo), los ojos y la boca (que miran al cursor, parpadean y se
//! sorprenden) y lo que sale de la cabeza (el rulo, unas orejas, una antena). Lo llamado
//! `ancla-<algo>` no se dibuja: marca dónde está `<algo>` (`ancla-cuerpo`, su centro, si no es el
//! de su caja; `ancla-ojo`, `ancla-boca`, `ancla-muelle` o el ancla de una capa de la cabeza).

use super::cara::Vec3;
use super::forma::Punto;
use super::svg::{self, Elemento, Pintura, Subtrazo};
use serde::Deserialize;
use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;
use std::path::Path;
use std::sync::{Arc, LazyLock};

/// Ids de las figuras que trae el programa.
pub const NOOK: &str = "nook";
pub const LIMON: &str = "limon";

/// Hasta dónde puede llegar el dibujo desde el centro del cuerpo, en radios del cuerpo, para que
/// quepa en la imagen con sus saltos: hacia arriba, hacia abajo y a los lados.
const CABE_ARRIBA: f32 = 1.55;
const CABE_ABAJO: f32 = 1.05;
const CABE_LADO: f32 = 1.35;

/// Límites del modo de imágenes: fotogramas en total, lado de cada uno y bytes de cada archivo.
const MAX_FOTOGRAMAS: usize = 64;
const MAX_LADO_FOTOGRAMA: u32 = 1024;
const MAX_BYTES_FOTOGRAMA: usize = 8 * 1024 * 1024;

/// Dónde están los rasgos de una figura con el personaje de frente y en reposo, en unidades de
/// radio respecto al centro del cuerpo: el ojo derecho (el izquierdo es su reflejo), la boca, y
/// cuánto mide el cuerpo hacia arriba, hacia abajo y hacia la derecha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Medidas {
    pub ojo: Punto,
    pub boca: Punto,
    pub arriba: f32,
    pub abajo: f32,
    pub ancho: f32,
}

/// Una parte del dibujo ya en unidades de radio: sus trazos, cómo se pinta (el grosor de las
/// líneas, también en unidades de radio), su color y opacidad, y su centro (el de su caja).
#[derive(Clone, Debug, PartialEq)]
pub struct Forma {
    pub subtrazos: Vec<Subtrazo>,
    pub pintura: Pintura,
    pub color: [u8; 3],
    pub alfa: f32,
    pub centro: Punto,
}

/// Hacia dónde mira lo que sale de la cabeza: con la cabeza (se inclina con ella y se acorta al
/// mirar arriba o abajo, como un cartel) o con el cuerpo (solo se gira y se estira con él).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Orientacion {
    Cabeza,
    Cuerpo,
}

/// Cuándo se ve delante del cuerpo lo que sale de la cabeza: siempre, nunca (solo detrás), según
/// lo que gire la cabeza hacia su lado (como lo que la rodea) o asomando poco a poco al mirar
/// hacia él (de frente se ve su mitad de arriba; mirándolo, entero).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Delante {
    Siempre,
    Nunca,
    Girar,
    Revelar,
}

/// Algo que sale de la cabeza, pegado a ella en `ancla` (x a su derecha, y hacia abajo, z hacia
/// donde mira; unidades de radio). Con `fundir` se suma al cuerpo (como el rulo de Nook, que es
/// del mismo color); con `tapable`, no se dibuja si lo tapa un accesorio de la cabeza (un gorro).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Apendice {
    pub ancla: Vec3,
    pub orientacion: Orientacion,
    pub detras: bool,
    pub delante: Delante,
    pub fundir: bool,
    pub tapable: bool,
}

/// Qué es cada capa del dibujo vectorial.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TipoCapa {
    /// La silueta (la unión de lo que tenga), del color de lo que tiene.
    Cuerpo,
    /// Pintado en la cara: gira con ella sobre una esfera y no se sale del cuerpo.
    Pintado,
    /// Una franja de color pintada en la cara (un flequillo, una barriga): su línea es el borde y
    /// se rellena hasta fuera del cuerpo por arriba o por abajo. Con `encima`, va sobre los ojos
    /// y la boca y puede salirse del cuerpo hasta `recorte` veces su tamaño (el pelo del Limón).
    Franja { arriba: bool, encima: bool, recorte: Option<f32> },
    /// El ojo derecho, dibujado en su sitio; el izquierdo es su reflejo, salvo que haya
    /// `OjoIzquierdo`. `OjoAlegre` es el ojo derecho cuando está contento (si no hay, el ojo se
    /// recorta en arco «∩», si es relleno).
    Ojo,
    OjoIzquierdo,
    OjoAlegre,
    Boca,
    Cabeza(Apendice),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Capa {
    pub tipo: TipoCapa,
    pub formas: Vec<Forma>,
    /// Si se mueve como el pelo, con un muelle (ver `Figura::muelle`).
    pub muelle: bool,
    /// Caja de sus formas: esquina de arriba a la izquierda y de abajo a la derecha.
    pub caja: (Punto, Punto),
}

/// Animaciones del modo de imágenes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Animacion {
    Reposo,
    Aviso,
    Musica,
    Clic,
}

/// Un fotograma, en RGBA premultiplicado, con una copia a la mitad de tamaño (y esa, con la
/// suya…) para dibujarlo pequeño sin que se vea con dientes.
#[derive(Clone, Debug, PartialEq)]
pub struct Fotograma {
    pub ancho: u32,
    pub alto: u32,
    pub pixeles: Vec<[u8; 4]>,
    pub mitad: Option<Box<Fotograma>>,
}

impl Fotograma {
    fn new(ancho: u32, alto: u32, pixeles: Vec<[u8; 4]>) -> Fotograma {
        let mitad = (ancho >= 32 && alto >= 32).then(|| {
            let (a, h) = (ancho / 2, alto / 2);
            let pixeles = (0..a * h)
                .map(|i| {
                    let (x, y) = (i % a * 2, i / a * 2);
                    let p = |dx: u32, dy: u32| pixeles[((y + dy) * ancho + x + dx) as usize];
                    std::array::from_fn(|c| ((p(0, 0)[c] as u16 + p(1, 0)[c] as u16 + p(0, 1)[c] as u16 + p(1, 1)[c] as u16 + 2) / 4) as u8)
                })
                .collect();
            Box::new(Fotograma::new(a, h, pixeles))
        });
        Fotograma { ancho, alto, pixeles, mitad }
    }
}

/// Figura hecha de imágenes: los fotogramas de cada animación (en el orden de `Animacion`), a
/// cuántos por segundo van, si son pixel art (se amplían sin suavizar, con los píxeles nítidos),
/// dónde está el centro del cuerpo en ellos y cuántos píxeles mide una unidad de radio.
#[derive(Clone, Debug, PartialEq)]
pub struct Imagenes {
    animaciones: [Vec<Fotograma>; 4],
    pub fps: f32,
    pub pixelado: bool,
    pub centro: Punto,
    pub unidad: f32,
}

impl Imagenes {
    /// Los fotogramas de una animación (los del reposo, si no tiene).
    pub fn fotogramas(&self, animacion: Animacion) -> &[Fotograma] {
        let propios = &self.animaciones[animacion as usize];
        if propios.is_empty() { &self.animaciones[0] } else { propios }
    }

    pub fn tiene(&self, animacion: Animacion) -> bool {
        !self.animaciones[animacion as usize].is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Dibujo {
    Vectorial(Vec<Capa>),
    Imagenes(Imagenes),
}

/// Colores del dibujo que cambian con los ajustes: el que toma el color de acento y el que toma
/// el de los rasgos (el que se lee sobre el acento). `aviso` es el del circulito de las
/// notificaciones si el usuario no elige otro.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Colores {
    pub cuerpo: Option<[u8; 3]>,
    pub rasgos: Option<[u8; 3]>,
    pub aviso: Option<[u8; 3]>,
}

#[derive(Debug, PartialEq)]
pub struct Figura {
    pub id: String,
    /// Nombre por idioma ("es", "en"…).
    pub nombre: HashMap<String, String>,
    pub medidas: Medidas,
    pub colores: Colores,
    /// Dónde empieza el pelo (la altura, en unidades de radio, a partir de la que se mueve con
    /// el muelle), si alguna capa lo tiene.
    pub muelle: Option<f32>,
    pub dibujo: Dibujo,
    /// Contorno del cuerpo en reposo (para saber cuánto mide en cada dirección).
    contorno: Vec<Vec<Punto>>,
}

impl Figura {
    pub fn nook() -> Arc<Figura> {
        static FIGURA: LazyLock<Arc<Figura>> = LazyLock::new(|| integrada(NOOK, include_str!("../../personajes/nook/personaje.json"), include_str!("../../personajes/nook/personaje.svg")));
        FIGURA.clone()
    }

    pub fn limon() -> Arc<Figura> {
        static FIGURA: LazyLock<Arc<Figura>> = LazyLock::new(|| integrada(LIMON, include_str!("../../personajes/limon/personaje.json"), include_str!("../../personajes/limon/personaje.svg")));
        FIGURA.clone()
    }

    /// Las que trae el programa, en el orden en que salen en la personalización.
    pub fn integradas() -> [Arc<Figura>; 2] {
        [Figura::nook(), Figura::limon()]
    }

    /// Lee la figura de una carpeta (con `personaje.json`, `personaje.svg` o los dos).
    pub fn leer(carpeta: &Path, id: String) -> Result<Figura, String> {
        let leer_texto = |nombre: &str| match std::fs::read_to_string(carpeta.join(nombre)) {
            Ok(texto) => Ok(Some(texto)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{nombre}: {e}")),
        };
        let json = leer_texto("personaje.json")?;
        let svg = leer_texto("personaje.svg")?;
        if json.is_none() && svg.is_none() {
            return Err("falta personaje.json o personaje.svg".into());
        }
        let leer_archivo = |nombre: &str| -> Result<Vec<u8>, String> {
            let ruta = carpeta.join(nombre);
            let bytes = std::fs::metadata(&ruta).map_err(|e| format!("{nombre}: {e}"))?.len();
            if bytes as usize > MAX_BYTES_FOTOGRAMA {
                return Err(format!("{nombre}: es demasiado grande"));
            }
            std::fs::read(&ruta).map_err(|e| format!("{nombre}: {e}"))
        };
        Figura::desde(id, json.as_deref(), svg.as_deref(), &leer_archivo)
    }

    /// Crea la figura de su descripción (`personaje.json`), su dibujo (`personaje.svg`) y, para
    /// el modo de imágenes, cómo leer sus PNG.
    pub fn desde(
        id: String,
        json: Option<&str>,
        svg: Option<&str>,
        leer_archivo: &dyn Fn(&str) -> Result<Vec<u8>, String>,
    ) -> Result<Figura, String> {
        let descripcion: Descripcion = match json {
            Some(json) => serde_json::from_str(json).map_err(|e| format!("personaje.json: {e}"))?,
            None => Descripcion::default(),
        };
        let colores = Colores {
            cuerpo: descripcion.colores.cuerpo.as_deref().map(color).transpose()?,
            rasgos: descripcion.colores.rasgos.as_deref().map(color).transpose()?,
            aviso: descripcion.colores.aviso.as_deref().map(color).transpose()?,
        };
        // Sin nombre, el de su carpeta (el id sin lo de delante, ver `coleccion`).
        let carpeta = id.rsplit('/').next().unwrap_or(&id).to_string();
        let nombre = if descripcion.nombre.is_empty() { HashMap::from([("es".to_string(), carpeta)]) } else { descripcion.nombre.clone() };
        if let Some(imagenes) = &descripcion.imagenes {
            if descripcion.capas.is_some() {
                return Err("personaje.json: lleva «capas» e «imagenes»: es uno u otro".into());
            }
            return de_imagenes(id, nombre, colores, imagenes, leer_archivo);
        }
        let svg = svg.ok_or("falta personaje.svg (o «imagenes» en personaje.json)")?;
        de_svg(id, nombre, colores, descripcion.capas.as_deref(), svg)
    }

    /// Si mira al cursor y tiene cara (las de imágenes, no).
    pub fn mira(&self) -> bool {
        matches!(self.dibujo, Dibujo::Vectorial(_))
    }

    pub fn capas(&self) -> &[Capa] {
        match &self.dibujo {
            Dibujo::Vectorial(capas) => capas,
            Dibujo::Imagenes(_) => &[],
        }
    }

    #[cfg(test)]
    pub fn tiene(&self, tipo: fn(&TipoCapa) -> bool) -> bool {
        self.capas().iter().any(|c| tipo(&c.tipo))
    }

    /// Cuánto mide el cuerpo en reposo desde su centro en la dirección `angulo` (radianes, 0 a la
    /// derecha y π/2 hacia abajo): hasta su borde más lejano en esa dirección.
    pub fn radio_en(&self, angulo: f32) -> f32 {
        radio_en(&self.contorno, angulo)
    }

}

fn integrada(id: &str, json: &str, svg: &str) -> Arc<Figura> {
    let figura = Figura::desde(id.to_string(), Some(json), Some(svg), &|nombre| Err(format!("{nombre}: no existe")));
    Arc::new(figura.unwrap_or_else(|e| panic!("la figura {id} trae un error: {e}")))
}

// ---- Lo que dice personaje.json ----

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Descripcion {
    nombre: HashMap<String, String>,
    colores: DescripcionColores,
    capas: Option<Vec<DescripcionCapa>>,
    imagenes: Option<DescripcionImagenes>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct DescripcionColores {
    cuerpo: Option<String>,
    rasgos: Option<String>,
    aviso: Option<String>,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum Tipo {
    Cuerpo,
    Pintado,
    Franja,
    Ojo,
    OjoIzquierdo,
    OjoAlegre,
    Boca,
    Cabeza,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum Hacia {
    Arriba,
    Abajo,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
struct DescripcionCapa {
    id: String,
    #[serde(default)]
    tipo: Option<Tipo>,
    #[serde(default)]
    muelle: bool,
    // Franja.
    #[serde(default)]
    hacia: Option<Hacia>,
    #[serde(default)]
    encima: bool,
    #[serde(default)]
    recorte: Option<f32>,
    // Cabeza.
    #[serde(default)]
    orientacion: Option<Orientacion>,
    #[serde(default)]
    detras: bool,
    #[serde(default)]
    delante: Option<Delante>,
    #[serde(default)]
    fundir: bool,
    #[serde(default)]
    tapable: Option<bool>,
    #[serde(default)]
    profundidad: Option<f32>,
}

impl DescripcionCapa {
    /// La capa con ese id, con su tipo por defecto (el de su nombre, si es uno de los de la cara
    /// o el cuerpo; si no, pintada).
    fn de(id: &str) -> Self {
        DescripcionCapa {
            id: id.to_string(),
            tipo: None,
            muelle: false,
            hacia: None,
            encima: false,
            recorte: None,
            orientacion: None,
            detras: false,
            delante: None,
            fundir: false,
            tapable: None,
            profundidad: None,
        }
    }

    fn tipo(&self) -> Tipo {
        self.tipo.unwrap_or(match self.id.as_str() {
            "cuerpo" => Tipo::Cuerpo,
            "ojo" => Tipo::Ojo,
            "ojo-izquierdo" => Tipo::OjoIzquierdo,
            "ojo-alegre" => Tipo::OjoAlegre,
            "boca" => Tipo::Boca,
            _ => Tipo::Pintado,
        })
    }
}

/// Capas que se buscan en el dibujo si personaje.json no dice cuáles hay.
const CAPAS_POR_DEFECTO: [&str; 6] = ["cuerpo", "pintado", "ojo", "ojo-izquierdo", "ojo-alegre", "boca"];

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct DescripcionImagenes {
    reposo: Vec<String>,
    aviso: Vec<String>,
    musica: Vec<String>,
    clic: Vec<String>,
    fps: Option<f32>,
    pixelado: bool,
    /// Dónde están el ojo derecho y la boca en las imágenes, en píxeles (para los accesorios).
    ojo: Option<[f32; 2]>,
    boca: Option<[f32; 2]>,
}

/// Un color "#rrggbb" o "#rgb".
pub fn color(texto: &str) -> Result<[u8; 3], String> {
    let hex = texto.trim().trim_start_matches('#');
    let error = || format!("color no válido: {texto}");
    let canal = |s: &str| u8::from_str_radix(s, 16).map_err(|_| error());
    match hex.len() {
        6 => Ok([canal(&hex[0..2])?, canal(&hex[2..4])?, canal(&hex[4..6])?]),
        3 => {
            let c = |i: usize| canal(&hex[i..i + 1]).map(|v| v * 17);
            Ok([c(0)?, c(1)?, c(2)?])
        }
        _ => Err(error()),
    }
}

// ---- Figura vectorial ----

/// A qué capa (o ancla) pertenece un elemento: el primero de sus nombres que lo sea.
fn duenio<'a>(elemento: &'a Elemento, capas: &[DescripcionCapa]) -> Option<&'a str> {
    elemento.nombres.iter().map(String::as_str).find(|n| n.starts_with("ancla-") || capas.iter().any(|c| c.id == *n))
}

fn caja<'a>(puntos: impl Iterator<Item = &'a Punto>) -> Option<(Punto, Punto)> {
    puntos.fold(None, |caja, &(x, y)| match caja {
        None => Some(((x, y), (x, y))),
        Some(((x0, y0), (x1, y1))) => Some(((x0.min(x), y0.min(y)), (x1.max(x), y1.max(y)))),
    })
}

fn centro_de(caja: (Punto, Punto)) -> Punto {
    ((caja.0.0 + caja.1.0) / 2.0, (caja.0.1 + caja.1.1) / 2.0)
}

fn de_svg(id: String, nombre: HashMap<String, String>, colores: Colores, capas: Option<&[DescripcionCapa]>, svg: &str) -> Result<Figura, String> {
    let elementos = svg::leer(svg)?;
    let descripciones: Vec<DescripcionCapa> = match capas {
        Some(capas) => capas.to_vec(),
        None => CAPAS_POR_DEFECTO
            .iter()
            .filter(|&&n| elementos.iter().any(|e| e.nombres.iter().any(|m| m == n)))
            .map(|n| DescripcionCapa::de(n))
            .collect(),
    };
    for (i, d) in descripciones.iter().enumerate() {
        if d.id.starts_with("ancla-") {
            return Err(format!("la capa «{}» no puede llamarse ancla-…: eso marca un sitio", d.id));
        }
        if descripciones[..i].iter().any(|o| o.id == d.id) {
            return Err(format!("la capa «{}» está dos veces", d.id));
        }
    }
    match descripciones.iter().filter(|d| d.tipo() == Tipo::Cuerpo).count() {
        0 => return Err("falta la capa «cuerpo» (el dibujo tiene que tener algo con ese nombre)".into()),
        1 => {}
        _ => return Err("solo puede haber una capa de tipo «cuerpo»".into()),
    }

    // Los elementos de cada capa y las anclas.
    let mut de_capa: Vec<Vec<&Elemento>> = vec![Vec::new(); descripciones.len()];
    let mut anclas: HashMap<&str, (Punto, Punto)> = HashMap::new();
    for elemento in &elementos {
        let Some(nombre) = duenio(elemento, &descripciones) else { continue };
        if nombre.starts_with("ancla-") {
            if let Some(c) = caja(elemento.subtrazos.iter().flat_map(|s| &s.puntos)) {
                anclas.insert(nombre, c);
            }
        } else if let Some(i) = descripciones.iter().position(|d| d.id == nombre) {
            de_capa[i].push(elemento);
        }
    }
    if let Some(i) = de_capa.iter().position(Vec::is_empty) {
        return Err(format!("no hay nada llamado «{}» en el dibujo", descripciones[i].id));
    }

    // Centro y tamaño del cuerpo, y la unidad: lo que haga falta para que todo quepa.
    let i_cuerpo = descripciones.iter().position(|d| d.tipo() == Tipo::Cuerpo).unwrap_or(0);
    let (c0, c1) = caja(de_capa[i_cuerpo].iter().flat_map(|e| e.subtrazos.iter().flat_map(|s| &s.puntos))).ok_or("el cuerpo está vacío")?;
    let centro = anclas.get("ancla-cuerpo").map_or(centro_de((c0, c1)), |&c| centro_de(c));
    // Lo que mide el cuerpo desde su centro, a lo ancho y a lo alto (por el lado que llegue más
    // lejos).
    let mitad_ancho = (centro.0 - c0.0).max(c1.0 - centro.0);
    let mitad_alto = (centro.1 - c0.1).max(c1.1 - centro.1);
    if mitad_ancho <= 0.0 || mitad_alto <= 0.0 {
        return Err("el cuerpo no tiene superficie".into());
    }
    let (mut arriba, mut abajo, mut lado) = (0.0f32, 0.0f32, 0.0f32);
    for e in de_capa.iter().flatten() {
        let margen = match e.pintura {
            Pintura::Linea { grosor, .. } => grosor / 2.0,
            Pintura::Relleno { .. } => 0.0,
        };
        for &(x, y) in e.subtrazos.iter().flat_map(|s| &s.puntos) {
            arriba = arriba.max(centro.1 - y + margen);
            abajo = abajo.max(y - centro.1 + margen);
            lado = lado.max((x - centro.0).abs() + margen);
        }
    }
    let unidad = mitad_ancho.max(mitad_alto).max(arriba / CABE_ARRIBA).max(abajo / CABE_ABAJO).max(lado / CABE_LADO);
    let a_unidades = |(x, y): Punto| ((x - centro.0) / unidad, (y - centro.1) / unidad);
    let ancla = |nombre: &str| anclas.get(nombre).map(|&c| a_unidades(centro_de(c)));

    // Las formas de cada capa en unidades de radio respecto al centro del cuerpo.
    let formas: Vec<Vec<Forma>> = de_capa
        .iter()
        .map(|elementos| {
            elementos
                .iter()
                .map(|e| {
                    let subtrazos: Vec<Subtrazo> = e
                        .subtrazos
                        .iter()
                        .map(|s| Subtrazo { puntos: s.puntos.iter().map(|&p| a_unidades(p)).collect(), cerrado: s.cerrado })
                        .collect();
                    let centro = caja(subtrazos.iter().flat_map(|s| &s.puntos)).map(centro_de).unwrap_or_default();
                    let pintura = match e.pintura {
                        Pintura::Linea { grosor, extremo, union } => Pintura::Linea { grosor: grosor / unidad, extremo, union },
                        relleno => relleno,
                    };
                    Forma { subtrazos, pintura, color: e.color, alfa: e.alfa, centro }
                })
                .collect()
        })
        .collect();

    let contorno: Vec<Vec<Punto>> = formas[i_cuerpo].iter().flat_map(|f| &f.subtrazos).map(|s| s.puntos.clone()).collect();
    let cajas: Vec<Option<(Punto, Punto)>> = formas.iter().map(|f| caja(f.iter().flat_map(|f| f.subtrazos.iter().flat_map(|s| &s.puntos)))).collect();
    let caja_de = |i: usize| cajas[i];
    let capa_de = |tipo: Tipo| descripciones.iter().position(|d| d.tipo() == tipo);
    let ancho = radio_en(&contorno, 0.0);
    let (alto_arriba, alto_abajo) = (radio_en(&contorno, -FRAC_PI_2), radio_en(&contorno, FRAC_PI_2));
    let ojo = ancla("ancla-ojo")
        .or_else(|| capa_de(Tipo::Ojo).and_then(caja_de).map(centro_de))
        .unwrap_or((ancho * 13.0 / 49.0, 0.0));
    let boca = ancla("ancla-boca")
        .or_else(|| capa_de(Tipo::Boca).and_then(caja_de).map(centro_de))
        .unwrap_or((0.0, alto_abajo * 15.0 / 39.0));
    let medidas = Medidas { ojo, boca, arriba: alto_arriba, abajo: alto_abajo, ancho };

    // Cada capa con lo suyo, y sus formas respecto a su sitio si se mueven con algo: el ojo
    // (también el izquierdo y el alegre), la boca o el ancla de lo que sale de la cabeza.
    let mut capas = Vec::new();
    for ((d, formas), i) in descripciones.iter().zip(formas).zip(0..) {
        let (tipo, origen) = match d.tipo() {
            Tipo::Cuerpo => (TipoCapa::Cuerpo, (0.0, 0.0)),
            Tipo::Pintado => (TipoCapa::Pintado, (0.0, 0.0)),
            Tipo::Franja => {
                let recorte = d.recorte.map(|r| r.clamp(1.0, 1.5));
                (TipoCapa::Franja { arriba: d.hacia != Some(Hacia::Abajo), encima: d.encima, recorte }, (0.0, 0.0))
            }
            Tipo::Ojo => (TipoCapa::Ojo, ojo),
            Tipo::OjoAlegre => (TipoCapa::OjoAlegre, ojo),
            Tipo::OjoIzquierdo => (TipoCapa::OjoIzquierdo, (-ojo.0, ojo.1)),
            Tipo::Boca => (TipoCapa::Boca, boca),
            Tipo::Cabeza => {
                // Sin ancla, donde arranca: abajo y en medio de lo que dibuja.
                let (c0, c1) = caja_de(i).unwrap_or_default();
                let (ax, ay) = ancla(&format!("ancla-{}", d.id)).unwrap_or(((c0.0 + c1.0) / 2.0, c1.1));
                // Su profundidad: la del punto de la cabeza (un elipsoide como el cuerpo) que
                // queda delante de él.
                let vertical = if ay < 0.0 { alto_arriba } else { alto_abajo };
                let dentro = 1.0 - (ax / ancho.max(1e-3)).powi(2) - (ay / vertical.max(1e-3)).powi(2);
                let z = d.profundidad.unwrap_or(dentro.max(0.0).sqrt() * vertical);
                let apendice = Apendice {
                    ancla: [ax, ay, z],
                    orientacion: d.orientacion.unwrap_or(Orientacion::Cabeza),
                    detras: d.detras,
                    delante: d.delante.unwrap_or(Delante::Siempre),
                    fundir: d.fundir,
                    tapable: d.tapable.unwrap_or(true),
                };
                (TipoCapa::Cabeza(apendice), (ax, ay))
            }
        };
        let formas: Vec<Forma> = formas
            .into_iter()
            .map(|mut f| {
                let mover = |(x, y): Punto| (x - origen.0, y - origen.1);
                for s in &mut f.subtrazos {
                    s.puntos.iter_mut().for_each(|p| *p = mover(*p));
                }
                f.centro = mover(f.centro);
                f
            })
            .collect();
        let caja = caja(formas.iter().flat_map(|f| f.subtrazos.iter().flat_map(|s| &s.puntos))).unwrap_or_default();
        capas.push(Capa { tipo, formas, muelle: d.muelle, caja });
    }
    let muelle = descripciones.iter().any(|d| d.muelle).then(|| ancla("ancla-muelle").map_or(-0.27 * alto_arriba, |p| p.1));
    Ok(Figura { id, nombre, medidas, colores, muelle, dibujo: Dibujo::Vectorial(capas), contorno })
}

/// Hasta dónde llega el contorno desde el centro en la dirección `angulo`: el corte más lejano
/// con alguno de sus tramos.
fn radio_en(contorno: &[Vec<Punto>], angulo: f32) -> f32 {
    let (dy, dx) = angulo.sin_cos();
    let mut lejos: f32 = 0.0;
    for poligono in contorno {
        for (i, &a) in poligono.iter().enumerate() {
            let b = poligono[(i + 1) % poligono.len()];
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            let det = dx * -ey - dy * -ex;
            if det.abs() < 1e-9 {
                continue;
            }
            // centro + t·d = a + s·e
            let t = (a.0 * -ey - a.1 * -ex) / det;
            let s = (dx * a.1 - dy * a.0) / det;
            if t > 0.0 && (0.0..=1.0).contains(&s) {
                lejos = lejos.max(t);
            }
        }
    }
    if lejos > 0.0 { lejos } else { 1.0 }
}

// ---- Figura de imágenes ----

fn de_imagenes(
    id: String,
    nombre: HashMap<String, String>,
    colores: Colores,
    d: &DescripcionImagenes,
    leer_archivo: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<Figura, String> {
    if d.reposo.is_empty() {
        return Err("personaje.json: «imagenes» necesita al menos una imagen en «reposo»".into());
    }
    let total = d.reposo.len() + d.aviso.len() + d.musica.len() + d.clic.len();
    if total > MAX_FOTOGRAMAS {
        return Err(format!("demasiadas imágenes ({total}; como mucho {MAX_FOTOGRAMAS})"));
    }
    let leer = |nombres: &[String]| -> Result<Vec<Fotograma>, String> {
        nombres
            .iter()
            .map(|n| {
                if !nombre_de_archivo_valido(n) {
                    return Err(format!("{n}: nombre de archivo no válido (tiene que estar en la carpeta del personaje)"));
                }
                leer_png(&leer_archivo(n)?).map_err(|e| format!("{n}: {e}"))
            })
            .collect()
    };
    let animaciones = [leer(&d.reposo)?, leer(&d.aviso)?, leer(&d.musica)?, leer(&d.clic)?];
    let primero = &animaciones[0][0];
    if let Some(otro) = animaciones.iter().flatten().find(|f| (f.ancho, f.alto) != (primero.ancho, primero.alto)) {
        return Err(format!(
            "todas las imágenes tienen que medir lo mismo ({}×{}, y hay una de {}×{})",
            primero.ancho, primero.alto, otro.ancho, otro.alto
        ));
    }

    // El cuerpo es lo que no es transparente en el primer fotograma del reposo.
    let opaco = |x: u32, y: u32| primero.pixeles[(y * primero.ancho + x) as usize][3] > 16;
    let mut caja_px: Option<(Punto, Punto)> = None;
    for y in 0..primero.alto {
        for x in 0..primero.ancho {
            if opaco(x, y) {
                let p = (x as f32, y as f32);
                let q = (x as f32 + 1.0, y as f32 + 1.0);
                caja_px = Some(caja_px.map_or((p, q), |(a, b)| ((a.0.min(p.0), a.1.min(p.1)), (b.0.max(q.0), b.1.max(q.1)))));
            }
        }
    }
    let (c0, c1) = caja_px.ok_or("la primera imagen del reposo es transparente")?;
    let centro = centro_de((c0, c1));
    let unidad = ((c1.0 - c0.0) / 2.0).max((c1.1 - c0.1) / 2.0);

    // Su contorno: hasta dónde llega lo opaco en 64 direcciones.
    let contorno: Vec<Punto> = (0..64)
        .map(|i| {
            let (s, c) = (i as f32 / 64.0 * std::f32::consts::TAU).sin_cos();
            let mut lejos = 0.0;
            let mut r = 0.0;
            while r < unidad * 2.0 {
                let (x, y) = (centro.0 + c * r, centro.1 + s * r);
                if x >= 0.0 && y >= 0.0 && (x as u32) < primero.ancho && (y as u32) < primero.alto && opaco(x as u32, y as u32) {
                    lejos = r;
                }
                r += 0.5;
            }
            (c * lejos.max(1.0) / unidad, s * lejos.max(1.0) / unidad)
        })
        .collect();
    let contorno = vec![contorno];
    let ancho = radio_en(&contorno, 0.0);
    let (arriba, abajo) = (radio_en(&contorno, -FRAC_PI_2), radio_en(&contorno, FRAC_PI_2));
    let en_unidades = |[x, y]: [f32; 2]| ((x - centro.0) / unidad, (y - centro.1) / unidad);
    let medidas = Medidas {
        ojo: d.ojo.map(en_unidades).unwrap_or((ancho * 13.0 / 49.0, 0.0)),
        boca: d.boca.map(en_unidades).unwrap_or((0.0, abajo * 15.0 / 39.0)),
        arriba,
        abajo,
        ancho,
    };
    let fps = d.fps.unwrap_or(8.0).clamp(0.5, 60.0);
    let imagenes = Imagenes { animaciones, fps, pixelado: d.pixelado, centro, unidad };
    Ok(Figura { id, nombre, medidas, colores, muelle: None, dibujo: Dibujo::Imagenes(imagenes), contorno })
}

/// Nombre de un archivo de la carpeta del personaje: sin carpetas ni nada raro.
fn nombre_de_archivo_valido(nombre: &str) -> bool {
    !nombre.is_empty() && !nombre.starts_with('.') && nombre.chars().all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' '))
}

/// Lee un PNG como RGBA premultiplicado.
fn leer_png(bytes: &[u8]) -> Result<Fotograma, String> {
    let mut decodificador = png::Decoder::new(std::io::Cursor::new(bytes));
    decodificador.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
    let mut lector = decodificador.read_info().map_err(|e| e.to_string())?;
    let (ancho, alto) = (lector.info().width, lector.info().height);
    if ancho == 0 || alto == 0 || ancho > MAX_LADO_FOTOGRAMA || alto > MAX_LADO_FOTOGRAMA {
        return Err(format!("mide {ancho}×{alto} (como mucho {MAX_LADO_FOTOGRAMA}×{MAX_LADO_FOTOGRAMA})"));
    }
    let mut datos = vec![0; lector.output_buffer_size().ok_or("no se puede leer")?];
    let info = lector.next_frame(&mut datos).map_err(|e| e.to_string())?;
    let datos = &datos[..info.buffer_size()];
    let por_pixel = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::GrayscaleAlpha => 2,
        otro => return Err(format!("tipo de color no admitido ({otro:?})")),
    };
    let pixeles = datos
        .chunks_exact(por_pixel)
        .take((ancho * alto) as usize)
        .map(|p| {
            let (r, g, b, a) = if por_pixel == 4 { (p[0], p[1], p[2], p[3]) } else { (p[0], p[0], p[0], p[1]) };
            let pre = |c: u8| ((c as u16 * a as u16 + 127) / 255) as u8;
            [pre(r), pre(g), pre(b), a]
        })
        .collect::<Vec<_>>();
    if pixeles.len() != (ancho * alto) as usize {
        return Err("está incompleto".into());
    }
    Ok(Fotograma::new(ancho, alto, pixeles))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sin_archivos(nombre: &str) -> Result<Vec<u8>, String> {
        Err(format!("{nombre}: no existe"))
    }

    #[test]
    fn nook_queda_con_sus_medidas_de_siempre() {
        let nook = Figura::nook();
        let m = nook.medidas;
        let cerca = |a: f32, b: f32| (a - b).abs() < 0.01;
        assert!(cerca(m.ojo.0, 13.0 / 49.0) && cerca(m.ojo.1, 0.0), "{m:?}");
        assert!(cerca(m.boca.0, 0.0) && cerca(m.boca.1, 15.0 / 49.0), "{m:?}");
        assert!(cerca(m.arriba, 39.0 / 49.0) && cerca(m.abajo, 37.5 / 49.0) && cerca(m.ancho, 1.0), "{m:?}");
        assert!(nook.colores.cuerpo.is_some() && nook.colores.rasgos.is_some());
        assert!(nook.mira() && nook.muelle.is_none());
    }

    #[test]
    fn el_limon_es_algo_mas_alto_y_tiene_pelo() {
        let limon = Figura::limon();
        assert!(limon.medidas.arriba > Figura::nook().medidas.arriba + 0.1);
        assert!(limon.muelle.is_some());
        assert!(limon.tiene(|t| matches!(t, TipoCapa::Franja { encima: true, .. })));
    }

    #[test]
    fn sin_descripcion_busca_las_capas_por_su_nombre() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200">
            <ellipse id="cuerpo" cx="100" cy="100" rx="80" ry="60" fill="#abcdef"/>
            <circle id="ojo" cx="130" cy="95" r="8" fill="#000"/>
            <path id="boca" d="M90 130 L110 130" stroke="#000" stroke-width="3"/>
            <rect id="sin-capa" width="10" height="10"/>
        </svg>"##;
        let figura = Figura::desde("usuario/gato".into(), None, Some(svg), &sin_archivos).unwrap();
        let tipos: Vec<_> = figura.capas().iter().map(|c| c.tipo).collect();
        assert_eq!(tipos, vec![TipoCapa::Cuerpo, TipoCapa::Ojo, TipoCapa::Boca]);
        // Da igual el tamaño del dibujo: el cuerpo mide 1 de radio a lo ancho.
        assert!((figura.medidas.ancho - 1.0).abs() < 0.01);
        assert!((figura.medidas.ojo.0 - 30.0 / 80.0).abs() < 0.01);
        // Las formas del ojo van respecto a su centro.
        let ojo = &figura.capas()[1].formas[0];
        assert!(ojo.centro.0.abs() < 1e-3 && ojo.centro.1.abs() < 1e-3);
        assert_eq!(figura.nombre["es"], "gato");
    }

    #[test]
    fn explica_lo_que_falta_o_sobra() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><circle id="ojo" cx="10" cy="10" r="5"/></svg>"##;
        let error = Figura::desde("x".into(), None, Some(svg), &sin_archivos).unwrap_err();
        assert!(error.contains("cuerpo"), "{error}");
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><circle id="cuerpo" cx="10" cy="10" r="5"/></svg>"##;
        let json = r#"{"capas": [{"id": "cuerpo"}, {"id": "cola", "tipo": "cabeza"}]}"#;
        let error = Figura::desde("x".into(), Some(json), Some(svg), &sin_archivos).unwrap_err();
        assert!(error.contains("cola"), "{error}");
        let json = r#"{"capas": [{"id": "cuerpo", "encma": true}]}"#;
        let error = Figura::desde("x".into(), Some(json), Some(svg), &sin_archivos).unwrap_err();
        assert!(error.contains("encma"), "{error}");
        assert!(Figura::desde("x".into(), None, None, &sin_archivos).is_err());
    }

    #[test]
    fn lo_que_sale_de_la_cabeza_no_se_sale_de_la_imagen() {
        // Una antena muy alta: la unidad crece para que quepa.
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-60 -200 120 260">
            <circle id="cuerpo" r="40" fill="#abcdef"/>
            <path id="antena" d="M0 -40 L0 -180" stroke="#000" stroke-width="4"/>
        </svg>"##;
        let json = r#"{"capas": [{"id": "cuerpo"}, {"id": "antena", "tipo": "cabeza"}]}"#;
        let figura = Figura::desde("x".into(), Some(json), Some(svg), &sin_archivos).unwrap();
        let antena = &figura.capas()[1];
        let TipoCapa::Cabeza(apendice) = antena.tipo else { panic!("no es de la cabeza") };
        let arriba = apendice.ancla[1] + antena.caja.0.1;
        assert!(arriba >= -CABE_ARRIBA - 0.05, "{arriba}");
    }

    #[test]
    fn los_ejemplos_de_la_guia_se_leen() {
        let ejemplos = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/ejemplos");
        let gato = Figura::leer(&ejemplos.join("gato"), "usuario/gato".into()).unwrap();
        assert!(gato.mira() && gato.capas().len() == 5);
        assert!(gato.tiene(|t| matches!(t, TipoCapa::Cabeza(Apendice { delante: Delante::Girar, detras: true, .. }))));
        let fantasma = Figura::leer(&ejemplos.join("fantasma"), "usuario/fantasma".into()).unwrap();
        let Dibujo::Imagenes(imagenes) = &fantasma.dibujo else { panic!("no es de imágenes") };
        assert!(imagenes.pixelado && imagenes.tiene(Animacion::Aviso) && !imagenes.tiene(Animacion::Musica));
        assert_eq!(imagenes.fotogramas(Animacion::Reposo).len(), 4);
    }

    #[test]
    fn lee_colores() {
        assert_eq!(color("#ff8000"), Ok([255, 128, 0]));
        assert_eq!(color("#f80"), Ok([255, 136, 0]));
        assert!(color("rojo").is_err());
    }

    #[test]
    fn figura_de_imagenes() {
        // Un PNG de 4×2 con un cuadrado opaco de 2×2 en medio.
        let mut png = Vec::new();
        {
            let mut codificador = png::Encoder::new(&mut png, 4, 2);
            codificador.set_color(png::ColorType::Rgba);
            codificador.set_depth(png::BitDepth::Eight);
            let mut escritor = codificador.write_header().unwrap();
            let mut datos = vec![0u8; 4 * 2 * 4];
            for y in 0..2 {
                for x in 1..3 {
                    datos[(y * 4 + x) * 4..][..4].copy_from_slice(&[200, 100, 50, 255]);
                }
            }
            escritor.write_image_data(&datos).unwrap();
        }
        let leer = move |nombre: &str| if nombre == "a.png" { Ok(png.clone()) } else { Err("no".into()) };
        let json = r#"{"imagenes": {"reposo": ["a.png", "a.png"], "fps": 4, "pixelado": true}}"#;
        let figura = Figura::desde("x".into(), Some(json), None, &leer).unwrap();
        assert!(!figura.mira());
        let Dibujo::Imagenes(imagenes) = &figura.dibujo else { panic!() };
        assert_eq!(imagenes.fotogramas(Animacion::Aviso).len(), 2);
        assert!(imagenes.pixelado);
        assert_eq!(imagenes.centro, (2.0, 1.0));
        assert_eq!(imagenes.unidad, 1.0);
        let json = r#"{"imagenes": {"reposo": ["../fuera.png"]}}"#;
        let error = Figura::desde("x".into(), Some(json), None, &sin_archivos).unwrap_err();
        assert!(error.contains("no válido"), "{error}");
    }
}
