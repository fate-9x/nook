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
    /// En los ojos, si es una pupila (su nombre o el de un grupo que la contiene empieza por
    /// «pupila»): cuánto puede moverse dentro del ojo hacia la izquierda, arriba, la derecha y
    /// abajo, en unidades de radio.
    pub pupila: Option<[f32; 4]>,
}

/// Cuánto del sitio libre dentro del ojo recorre una pupila: no llega a tocar el borde.
const RECORRIDO_PUPILA: f32 = 0.6;

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
    /// Una franja de color pintada en la cara (un flequillo, una barriga, el pelo de los lados
    /// de la cabeza): su trazado es el borde y, con relleno, se rellena de ese color hasta fuera
    /// del cuerpo por arriba, por abajo o por un lado; con línea, la línea se dibuja a lo largo
    /// del borde. Con `encima`, va sobre los ojos y la boca y puede salirse del cuerpo
    /// hasta `recorte` veces su tamaño (el pelo del Limón). Una franja de un lado, con su borde
    /// pegado al de la cabeza, casi no se ve de frente; al girar la cabeza hacia el otro lado, el
    /// borde se va con la cara y la franja cubre lo que queda detrás: la parte de atrás de la
    /// cabeza.
    Franja { hacia: Hacia, encima: bool, recorte: Option<f32> },
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
    /// Su id en el dibujo (el de la lista `capas` del json, o el nombre de la parte).
    pub id: String,
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
    /// Cosas que no fallan pero pueden ser un descuido (trazados que no se dibujan, lo que sale
    /// de la cabeza mal enganchado, los ojos pegados al borde…). Los da `personaje::mcp` y
    /// cualquier otra herramienta que quiera ayudar a quien hace el personaje.
    pub avisos: Vec<String>,
    /// Cuánto tapan los párpados la parte de arriba de los ojos (0–1), si los dibuja el motor.
    pub parpado: Option<f32>,
    /// Sus expresiones, por su nombre y en su orden: la misma figura con otras piezas.
    pub expresiones: Vec<(String, Arc<Figura>)>,
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
        let parpado = descripcion.parpado.map(|p| p.clamp(0.0, 1.0));
        de_svg(id, nombre, colores, descripcion.capas.as_deref(), parpado, &descripcion.expresiones, svg)
    }

    /// La figura con la expresión `nombre` o, si no la tiene (o es la normal), ella misma.
    pub fn con_expresion(self: &Arc<Self>, nombre: &str) -> Arc<Figura> {
        self.expresiones.iter().find(|(n, _)| n == nombre).map_or_else(|| self.clone(), |(_, f)| f.clone())
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

/// Un nombre: un texto, que vale en todos los idiomas, o uno por idioma (`{"es": …, "en": …}`).
pub fn nombre<'de, D: serde::Deserializer<'de>>(d: D) -> Result<HashMap<String, String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Nombre {
        Uno(String),
        PorIdioma(HashMap<String, String>),
    }
    Ok(match Nombre::deserialize(d)? {
        Nombre::Uno(texto) => HashMap::from([("es".to_string(), texto)]),
        Nombre::PorIdioma(textos) => textos,
    })
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Descripcion {
    #[serde(deserialize_with = "nombre")]
    nombre: HashMap<String, String>,
    colores: DescripcionColores,
    capas: Option<Vec<DescripcionCapa>>,
    imagenes: Option<DescripcionImagenes>,
    /// Párpados que dibuja el motor: cuánto tapan los ojos por arriba (0–1).
    parpado: Option<f32>,
    #[serde(deserialize_with = "en_orden")]
    expresiones: Vec<(String, DescripcionExpresion)>,
}

/// Una expresión: qué capas cambian por qué piezas del dibujo, y sus párpados.
#[derive(Deserialize, Default)]
struct DescripcionExpresion {
    parpado: Option<f32>,
    #[serde(flatten)]
    cambios: HashMap<String, String>,
}

/// Un objeto del json como lista de sus pares, en el orden en que están escritos.
fn en_orden<'de, D: serde::Deserializer<'de>, V: Deserialize<'de>>(d: D) -> Result<Vec<(String, V)>, D::Error> {
    struct Pares<V>(std::marker::PhantomData<V>);
    impl<'de, V: Deserialize<'de>> serde::de::Visitor<'de> for Pares<V> {
        type Value = Vec<(String, V)>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("un objeto")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(self, mut mapa: M) -> Result<Self::Value, M::Error> {
            let mut pares = Vec::new();
            while let Some(par) = mapa.next_entry()? {
                pares.push(par);
            }
            Ok(pares)
        }
    }
    d.deserialize_map(Pares(std::marker::PhantomData))
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
    /// Una oreja: sale de la cabeza por un lado, fundida con ella (ver `TipoCapa::Cabeza`).
    Oreja,
}

/// Hacia dónde se rellena una franja desde su borde.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Hacia {
    Arriba,
    Abajo,
    Izquierda,
    Derecha,
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
            "oreja" | "oreja-izquierda" => Tipo::Oreja,
            _ => Tipo::Pintado,
        })
    }
}

/// Capas que se buscan en el dibujo si personaje.json no dice cuáles hay.
const CAPAS_POR_DEFECTO: [&str; 8] = ["cuerpo", "oreja", "oreja-izquierda", "pintado", "ojo", "ojo-izquierdo", "ojo-alegre", "boca"];

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

fn de_svg(
    id: String,
    nombre: HashMap<String, String>,
    colores: Colores,
    capas: Option<&[DescripcionCapa]>,
    parpado: Option<f32>,
    expresiones: &[(String, DescripcionExpresion)],
    svg: &str,
) -> Result<Figura, String> {
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
    // Avisos de cosas que no fallan pero pueden ser un descuido.
    let mut avisos: Vec<String> = Vec::new();
    // Las piezas de las expresiones no son de ninguna capa, pero se dibujan en su lugar.
    let piezas: Vec<&str> = expresiones.iter().flat_map(|(_, e)| e.cambios.values().map(String::as_str)).collect();
    let es_pieza = |e: &Elemento| e.nombres.iter().any(|n| piezas.contains(&n.as_str()));
    let sin_capa = elementos.iter().filter(|e| duenio(e, &descripciones).is_none() && !es_pieza(e)).count();
    if sin_capa == 1 {
        avisos.push("hay un trazado del dibujo que no es de ninguna capa ni ancla: no se va a dibujar".into());
    } else if sin_capa > 1 {
        avisos.push(format!("hay {sin_capa} trazados del dibujo que no son de ninguna capa ni ancla: no se van a dibujar"));
    }
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
    // Una capa puede estar vacía en la cara normal si alguna expresión le da sus piezas (unas cejas
    // que solo salen al enfadarse).
    let de_expresion = |id: &str| expresiones.iter().any(|(_, e)| e.cambios.contains_key(id));
    if let Some(i) = de_capa.iter().zip(&descripciones).position(|(es, d)| es.is_empty() && !de_expresion(&d.id)) {
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

    // Las formas de unos elementos en unidades de radio respecto al centro del cuerpo.
    let a_formas = |elementos: &[&Elemento]| -> Vec<Forma> {
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
                    let pupila = e.nombres.iter().any(|n| n.starts_with("pupila")).then_some([0.0; 4]);
                    Forma { subtrazos, pintura, color: e.color, alfa: e.alfa, centro, pupila }
            })
            .collect()
    };
    let formas: Vec<Vec<Forma>> = de_capa.iter().map(|elementos| a_formas(elementos)).collect();

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

    // El dibujo cerca del límite de puntos, y los ojos muy pegados al borde de la cabeza (al
    // mirar de lado se aplastan contra él).
    let puntos: usize = formas.iter().flat_map(|fs| fs.iter()).map(|f| f.subtrazos.iter().map(|s| s.puntos.len()).sum::<usize>()).sum();
    if puntos > svg::MAX_PUNTOS * 4 / 5 {
        avisos.push(format!("el dibujo se acerca al límite de puntos: {} de {}; simplifícalo o aplana los trazados", puntos, svg::MAX_PUNTOS));
    }
    if let Some(i_ojo) = capa_de(Tipo::Ojo) {
        if let Some((c0, c1)) = cajas[i_ojo] {
            let (x, y) = (c1.0, (c0.1 + c1.1) / 2.0);
            if x > 0.92 * radio_en(&contorno, y.atan2(x)) {
                avisos.push("los ojos llegan muy cerca del borde de la cabeza: al mirar de lado se verán apretados".into());
            }
        }
    }

    // Cada capa con lo suyo, y sus formas respecto a su sitio si se mueven con algo: el ojo
    // (también el izquierdo y el alegre), la boca o el ancla de lo que sale de la cabeza.
    let construir = |d: &DescripcionCapa, formas: Vec<Forma>, avisos: &mut Vec<String>| -> Capa {
        let caja_suya = caja(formas.iter().flat_map(|f| f.subtrazos.iter().flat_map(|s| &s.puntos)));
        let (tipo, origen) = match d.tipo() {
            Tipo::Cuerpo => (TipoCapa::Cuerpo, (0.0, 0.0)),
            Tipo::Pintado => (TipoCapa::Pintado, (0.0, 0.0)),
            Tipo::Franja => {
                let recorte = d.recorte.map(|r| r.clamp(1.0, 1.5));
                (TipoCapa::Franja { hacia: d.hacia.unwrap_or(Hacia::Arriba), encima: d.encima, recorte }, (0.0, 0.0))
            }
            Tipo::Ojo => (TipoCapa::Ojo, ojo),
            Tipo::OjoAlegre => (TipoCapa::OjoAlegre, ojo),
            Tipo::OjoIzquierdo => (TipoCapa::OjoIzquierdo, (-ojo.0, ojo.1)),
            Tipo::Boca => (TipoCapa::Boca, boca),
            Tipo::Cabeza | Tipo::Oreja => {
                let oreja = d.tipo() == Tipo::Oreja;
                // Sin ancla, donde arranca: abajo y en medio de lo que dibuja; una oreja, por su
                // lado de dentro (el que da a la cabeza), a media altura.
                let (c0, c1) = caja_suya.unwrap_or_default();
                let arranque = if oreja {
                    let dentro = if (c0.0 + c1.0) / 2.0 > 0.0 { c0.0 } else { c1.0 };
                    (dentro, (c0.1 + c1.1) / 2.0)
                } else {
                    ((c0.0 + c1.0) / 2.0, c1.1)
                };
                let (ax, ay) = ancla(&format!("ancla-{}", d.id)).unwrap_or(arranque);
                // Avisos de lo que suele fallar: enganchado a un lado sin profundidad, o con
                // cosas de los dos lados en una sola capa.
                if !oreja && d.profundidad.is_none() && ax.abs() > 0.5 * ancho {
                    avisos.push(format!("la capa «{}» engancha a un lado de la cabeza y no tiene profundidad: al mirar a otros lados se quedará pegada a la cara (pruébale una de 0 a -0.2)", d.id));
                }
                if c0.0 < -0.5 * ancho && c1.0 > 0.5 * ancho && !(d.detras && d.delante == Some(Delante::Nunca)) {
                    avisos.push(format!("la capa «{}» tiene cosas a los dos lados de la cabeza: si son orejas o similares, sepárala en una capa por lado, cada una con su ancla, para que al girar se esconda la del otro lado; si es pelo que la rodea, pruébale «detras» y una profundidad pequeña", d.id));
                }
                // Su profundidad: la del punto de la cabeza (un elipsoide como el cuerpo) que
                // queda delante de él.
                let vertical = if ay < 0.0 { alto_arriba } else { alto_abajo };
                let dentro = 1.0 - (ax / ancho.max(1e-3)).powi(2) - (ay / vertical.max(1e-3)).powi(2);
                let z = d.profundidad.unwrap_or(if oreja { 0.0 } else { dentro.max(0.0).sqrt() * vertical });
                // Una oreja, por defecto, se funde con la cabeza cuando se ve y se esconde detrás
                // al quedar del otro lado, y no la tapa un gorro.
                let apendice = Apendice {
                    ancla: [ax, ay, z],
                    orientacion: d.orientacion.unwrap_or(Orientacion::Cabeza),
                    detras: d.detras || oreja,
                    delante: d.delante.unwrap_or(if oreja { Delante::Girar } else { Delante::Siempre }),
                    fundir: d.fundir || oreja,
                    tapable: d.tapable.unwrap_or(!oreja),
                };
                (TipoCapa::Cabeza(apendice), (ax, ay))
            }
        };
        // Las pupilas de un ojo se mueven por donde marque su ancla (`ancla-pupila`, o
        // `ancla-pupila-izquierda` en el ojo izquierdo: la zona por la que va su centro) o, sin
        // ella, por el sitio que les deja el resto del ojo.
        let mut formas = formas;
        if matches!(tipo, TipoCapa::Ojo | TipoCapa::OjoIzquierdo) {
            let caja_de = |fs: &mut dyn Iterator<Item = &Forma>| caja(fs.flat_map(|f| f.subtrazos.iter().flat_map(|s| &s.puntos)));
            let zona = if tipo == TipoCapa::Ojo { "ancla-pupila" } else { "ancla-pupila-izquierda" };
            let zona = anclas.get(zona).map(|&(z0, z1)| (a_unidades(z0), a_unidades(z1)));
            let ojo = caja_de(&mut formas.iter().filter(|f| f.pupila.is_none()));
            for f in formas.iter_mut().filter(|f| f.pupila.is_some()) {
                let (p0, p1) = caja_de(&mut std::iter::once(&*f)).unwrap_or_default();
                let (cx, cy) = centro_de((p0, p1));
                f.pupila = match (zona, ojo) {
                    (Some((z0, z1)), _) => Some([cx - z0.0, cy - z0.1, z1.0 - cx, z1.1 - cy].map(|v| v.max(0.0))),
                    (None, Some((o0, o1))) => {
                        let libre = |ojo: f32, pupila: f32| ((ojo - pupila) / 2.0 * RECORRIDO_PUPILA).max(0.0);
                        let (rx, ry) = (libre(o1.0 - o0.0, p1.0 - p0.0), libre(o1.1 - o0.1, p1.1 - p0.1));
                        Some([rx, ry, rx, ry])
                    }
                    (None, None) => Some([0.0; 4]),
                };
            }
        }
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
        Capa { id: d.id.clone(), tipo, formas, muelle: d.muelle, caja }
    };
    let mut capas: Vec<Capa> = descripciones.iter().zip(formas).map(|(d, formas)| construir(d, formas, &mut avisos)).collect();

    // Cada expresión: las mismas capas, con las que cambia hechas de sus piezas.
    let mut variantes = Vec::new();
    for (nombre_expresion, expresion) in expresiones {
        let mut variante = capas.clone();
        for (capa, pieza) in &expresion.cambios {
            let i = descripciones
                .iter()
                .position(|d| &d.id == capa)
                .ok_or_else(|| format!("la expresión «{nombre_expresion}» cambia «{capa}», que no es ninguna capa"))?;
            let suyos: Vec<&Elemento> = elementos.iter().filter(|e| e.nombres.iter().any(|n| n == pieza)).collect();
            if suyos.is_empty() {
                return Err(format!("la expresión «{nombre_expresion}» usa «{pieza}», y no hay nada con ese nombre en el dibujo"));
            }
            variante[i] = construir(&descripciones[i], a_formas(&suyos), &mut Vec::new());
        }
        reflejar_oreja(&mut variante);
        variantes.push((nombre_expresion.clone(), variante, expresion.parpado.map(|p| p.clamp(0.0, 1.0)).or(parpado)));
    }
    reflejar_oreja(&mut capas);
    let muelle = descripciones.iter().any(|d| d.muelle).then(|| ancla("ancla-muelle").map_or(-0.27 * alto_arriba, |p| p.1));
    let expresiones = variantes
        .into_iter()
        .map(|(nombre_expresion, capas, parpado)| {
            let figura = Figura {
                id: id.clone(),
                nombre: nombre.clone(),
                medidas,
                colores,
                muelle,
                dibujo: Dibujo::Vectorial(capas),
                avisos: Vec::new(),
                parpado,
                expresiones: Vec::new(),
                contorno: contorno.clone(),
            };
            (nombre_expresion, Arc::new(figura))
        })
        .collect();
    Ok(Figura { id, nombre, medidas, colores, muelle, dibujo: Dibujo::Vectorial(capas), avisos, parpado, expresiones, contorno })
}

/// Sin oreja izquierda, la derecha se refleja (como el ojo), justo después de ella.
fn reflejar_oreja(capas: &mut Vec<Capa>) {
    if !capas.iter().any(|c| c.id == "oreja-izquierda") {
        if let Some(i) = capas.iter().position(|c| c.id == "oreja") {
            let mut izquierda = capas[i].clone();
            izquierda.id = "oreja-izquierda".into();
            if let TipoCapa::Cabeza(apendice) = &mut izquierda.tipo {
                apendice.ancla[0] = -apendice.ancla[0];
            }
            for f in &mut izquierda.formas {
                f.subtrazos.iter_mut().flat_map(|s| s.puntos.iter_mut()).for_each(|p| p.0 = -p.0);
                f.centro.0 = -f.centro.0;
            }
            let ((x0, y0), (x1, y1)) = izquierda.caja;
            izquierda.caja = ((-x1, y0), (-x0, y1));
            capas.insert(i + 1, izquierda);
        }
    }
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
    Ok(Figura { id, nombre, medidas, colores, muelle: None, dibujo: Dibujo::Imagenes(imagenes), avisos: Vec::new(), parpado: None, expresiones: Vec::new(), contorno })
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
    fn las_expresiones_cambian_sus_piezas_y_sus_parpados() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-50 -50 100 100">
            <circle id="cuerpo" r="40" fill="#abcdef"/>
            <circle id="ojo" cx="15" cy="0" r="6" fill="#fff"/>
            <path id="boca" d="M -6 20 Q 0 24 6 20" stroke="#000"/>
            <path id="boca-triste" d="M -6 24 Q 0 18 6 24" stroke="#000"/>
            <path id="cejas-enojado" d="M 8 -12 L 22 -16" stroke="#000"/>
        </svg>"##;
        let json = r#"{
            "parpado": 0.3,
            "capas": [{"id": "cuerpo"}, {"id": "cejas"}, {"id": "ojo"}, {"id": "boca"}],
            "expresiones": {
                "Triste": {"boca": "boca-triste"},
                "Enojado": {"cejas": "cejas-enojado", "parpado": 0.5}
            }
        }"#;
        let figura = Arc::new(Figura::desde("x".into(), Some(json), Some(svg), &sin_archivos).unwrap());
        assert!(figura.avisos.is_empty(), "las piezas no cuentan como trazados sueltos: {:?}", figura.avisos);
        assert_eq!(figura.parpado, Some(0.3));
        // En su orden, cada una con sus párpados (o los de la figura).
        let nombres: Vec<&str> = figura.expresiones.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(nombres, ["Triste", "Enojado"]);
        let (triste, enojado) = (figura.con_expresion("Triste"), figura.con_expresion("Enojado"));
        assert_eq!((triste.parpado, enojado.parpado), (Some(0.3), Some(0.5)));
        // Las cejas solo existen al enfadarse; la boca triste sustituye a la normal.
        let capa = |f: &Figura, id: &str| f.capas().iter().find(|c| c.id == id).unwrap().formas.len();
        assert!(capa(&figura, "cejas") == 0 && capa(&enojado, "cejas") > 0);
        let boca = |f: &Figura| f.capas().iter().find(|c| c.id == "boca").unwrap().formas[0].subtrazos[0].puntos[1].1;
        assert_ne!(boca(&triste), boca(&figura), "la boca triste es otra");
        // La normal (o una que no existe) es la misma figura.
        assert!(Arc::ptr_eq(&figura.con_expresion("Normal"), &figura));

        let mal = json.replace(r#""boca": "boca-triste""#, r#""nariz": "boca-triste""#);
        assert!(Figura::desde("x".into(), Some(&mal), Some(svg), &sin_archivos).unwrap_err().contains("nariz"));
        let mal = json.replace(r#""boca": "boca-triste""#, r#""boca": "boca-feliz""#);
        assert!(Figura::desde("x".into(), Some(&mal), Some(svg), &sin_archivos).unwrap_err().contains("boca-feliz"));
    }

    #[test]
    fn el_nombre_puede_ser_un_texto_o_uno_por_idioma() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><circle id="cuerpo" r="40"/></svg>"##;
        let figura = Figura::desde("x".into(), Some(r#"{"nombre": "Rudi"}"#), Some(svg), &sin_archivos).unwrap();
        assert_eq!(crate::addons::traducido(&figura.nombre), "Rudi");
        let figura = Figura::desde("x".into(), Some(r#"{"nombre": {"es": "Gato", "en": "Cat"}}"#), Some(svg), &sin_archivos).unwrap();
        assert_eq!(figura.nombre["en"], "Cat");
    }

    #[test]
    fn las_pupilas_tienen_su_recorrido_dentro_del_ojo() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-50 -50 100 100">
            <circle id="cuerpo" r="40" fill="#abcdef"/>
            <g id="ojo"><circle cx="15" cy="0" r="8" fill="#fff"/><g id="pupila"><circle cx="15" cy="0" r="2" fill="#000"/><circle cx="16" cy="-1" r="0.5" fill="#fff"/></g></g>
        </svg>"##;
        let figura = Figura::desde("x".into(), None, Some(svg), &sin_archivos).unwrap();
        let ojo = figura.capas().iter().find(|c| c.tipo == TipoCapa::Ojo).unwrap();
        let pupilas: Vec<[f32; 4]> = ojo.formas.iter().filter_map(|f| f.pupila).collect();
        assert_eq!(pupilas.len(), 2, "la pupila y su brillo");
        // Con el ojo de 16 y la pupila de 4, en unidades de un radio de 40: (16 - 4) / 2 · 0,6 / 40.
        let esperado = 12.0 / 2.0 * RECORRIDO_PUPILA / 40.0;
        assert!(pupilas[0].iter().all(|v| (v - esperado).abs() < 0.01), "{pupilas:?}");
        assert!(ojo.formas[0].pupila.is_none(), "el blanco del ojo no se mueve");

        // Con su ancla, solo por esa zona: aquí, de 4 a la izquierda a 2 a la derecha, y nada hacia
        // arriba (un párpado) pero 3 hacia abajo.
        let svg = svg.replace("</svg>", r##"<rect id="ancla-pupila" x="11" y="0" width="6" height="3" fill="none"/></svg>"##);
        let figura = Figura::desde("x".into(), None, Some(&svg), &sin_archivos).unwrap();
        let ojo = figura.capas().iter().find(|c| c.tipo == TipoCapa::Ojo).unwrap();
        let recorrido = ojo.formas[1].pupila.unwrap();
        let esperado = [4.0, 0.0, 2.0, 3.0].map(|v| v / 40.0);
        assert!(recorrido.iter().zip(esperado).all(|(a, b)| (a - b).abs() < 0.001), "{recorrido:?}");
    }

    #[test]
    fn una_oreja_sola_se_refleja_y_se_funde_con_la_cabeza() {
        // Sin personaje.json: la oreja se reconoce por su nombre.
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-60 -60 120 120">
            <circle id="cuerpo" r="40" fill="#abcdef"/>
            <path id="oreja" d="M 36 -6 C 48 -10 50 14 36 10 Z" fill="#abcdef"/>
            <circle id="ojo" cx="15" cy="0" r="4"/>
        </svg>"##;
        let figura = Figura::desde("x".into(), None, Some(svg), &sin_archivos).unwrap();
        let orejas: Vec<&Capa> = figura.capas().iter().filter(|c| c.id.starts_with("oreja")).collect();
        assert_eq!(orejas.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["oreja", "oreja-izquierda"]);
        let (TipoCapa::Cabeza(derecha), TipoCapa::Cabeza(izquierda)) = (orejas[0].tipo, orejas[1].tipo) else { panic!("no son de la cabeza") };
        assert!(derecha.fundir && derecha.detras && derecha.delante == Delante::Girar && !derecha.tapable);
        // Enganchada por su lado de dentro (x = 36), a media altura (y = 2), en el plano del centro.
        assert!((derecha.ancla[0] - 0.9).abs() < 0.01 && (derecha.ancla[1] - 0.05).abs() < 0.01 && derecha.ancla[2] == 0.0, "{derecha:?}");
        assert_eq!(izquierda.ancla, [-derecha.ancla[0], derecha.ancla[1], derecha.ancla[2]]);
        let x_de = |c: &Capa| c.formas[0].subtrazos[0].puntos[1].0;
        assert_eq!(x_de(orejas[1]), -x_de(orejas[0]), "la izquierda es su reflejo");
        assert!(figura.avisos.is_empty(), "{:?}", figura.avisos);
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
        use super::super::motor::{Mirar, Motor};
        let ejemplos = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/ejemplos");
        let gato = Figura::leer(&ejemplos.join("gato"), "usuario/gato".into()).unwrap();
        assert!(gato.mira() && gato.capas().len() == 6);
        assert!(gato.tiene(|t| matches!(t, TipoCapa::Cabeza(Apendice { delante: Delante::Girar, detras: true, .. }))));
        // Lo que cuenta la guía: mirando a la derecha, la oreja derecha queda detrás de la cabeza
        // y la izquierda delante, y lo que está hacia atrás (profundidad negativa) se va al revés
        // que la cara.
        let mut motor = Motor::new();
        motor.poner_figura(Arc::new(gato));
        motor.poner_mirada(Mirar { guinada: 35.0, cabeceo: 0.0, mezcla: 1.0, deriva: 0.0 }, 0.0, 0.001);
        let cabeza = motor.muestrear(1.0).cabeza;
        let anclas: Vec<Vec3> = cabeza.figura.capas().iter().filter_map(|c| match c.tipo {
            TipoCapa::Cabeza(a) => Some(a.ancla),
            _ => None,
        }).collect();
        let [izquierda, derecha] = [anclas[0], anclas[1]];
        assert!(izquierda[0] < 0.0 && derecha[0] > 0.0);
        assert!(cabeza.profundidad(derecha) < -0.3, "la oreja derecha no se esconde");
        assert!(cabeza.profundidad(izquierda) > -0.15, "la oreja izquierda no se ve");
        let (centro, delante, detras) = (cabeza.punto([0.0; 3]).0, cabeza.punto([0.0, 0.0, 0.5]).0, cabeza.punto([0.0, 0.0, -0.5]).0);
        assert!(delante > centro + 0.1 && detras < centro - 0.1);
        let fantasma = Figura::leer(&ejemplos.join("fantasma"), "usuario/fantasma".into()).unwrap();
        let Dibujo::Imagenes(imagenes) = &fantasma.dibujo else { panic!("no es de imágenes") };
        assert!(imagenes.pixelado && imagenes.tiene(Animacion::Aviso) && !imagenes.tiene(Animacion::Musica));
        assert_eq!(imagenes.fotogramas(Animacion::Reposo).len(), 4);
    }

    #[test]
    fn el_ejemplo_de_todos_los_trucos_los_usa_todos_y_avisa_de_nada() {
        let ejemplos = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/ejemplos");
        let trucos = Figura::leer(&ejemplos.join("trucos"), "usuario/trucos".into()).unwrap();
        assert!(trucos.avisos.is_empty(), "{:?}", trucos.avisos);
        assert!(trucos.muelle.is_some());
        assert_eq!(trucos.capas().len(), 16);
        let cabezas: Vec<&Apendice> = trucos.capas().iter().filter_map(|c| match &c.tipo {
            TipoCapa::Cabeza(a) => Some(a),
            _ => None,
        }).collect();
        assert_eq!(cabezas.len(), 6);
        assert!(cabezas.iter().any(|a| a.fundir && a.orientacion == Orientacion::Cuerpo));
        assert!(cabezas.iter().any(|a| a.detras && a.delante == Delante::Revelar));
        assert!(cabezas.iter().any(|a| a.detras && a.delante == Delante::Girar));
        assert!(cabezas.iter().any(|a| a.detras && a.delante == Delante::Nunca));
        assert!(cabezas.iter().any(|a| a.detras));
        assert!(cabezas.iter().any(|a| !a.tapable));
        let franjas: Vec<&TipoCapa> = trucos.capas().iter().map(|c| &c.tipo).filter(|t| matches!(t, TipoCapa::Franja { .. })).collect();
        assert_eq!(franjas.len(), 4);
        assert!(franjas.iter().any(|t| matches!(t, TipoCapa::Franja { hacia: Hacia::Arriba, encima: true, recorte: Some(_) })));
        assert!(franjas.iter().any(|t| matches!(t, TipoCapa::Franja { hacia: Hacia::Abajo, .. })));
        assert!(franjas.iter().any(|t| matches!(t, TipoCapa::Franja { hacia: Hacia::Izquierda, .. })));
        assert!(franjas.iter().any(|t| matches!(t, TipoCapa::Franja { hacia: Hacia::Derecha, .. })));
        assert!(trucos.tiene(|t| matches!(t, TipoCapa::OjoIzquierdo)) && trucos.tiene(|t| matches!(t, TipoCapa::OjoAlegre)));
        // El mínimo: sin json, solo cuerpo, ojo y boca, y se llama como su carpeta.
        let minimo = Figura::leer(&ejemplos.join("minimo"), "usuario/minimo".into()).unwrap();
        assert!(minimo.avisos.is_empty());
        assert_eq!(minimo.nombre["es"], "minimo");
        assert_eq!(minimo.capas().iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), vec!["cuerpo", "ojo", "boca"]);
    }

    #[test]
    fn avisa_de_lo_sospechoso() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-60 -60 120 120">
            <circle id="cuerpo" r="40" fill="#abcdef"/>
            <path id="orejas" d="M -35 -20 L -55 -40 L -30 -35 Z M 35 -20 L 55 -40 L 30 -35 Z" fill="#123456"/>
            <circle id="ojo" cx="20" cy="0" r="18" fill="#000"/>
            <rect id="boca" x="-2" y="20" width="4" height="4" fill="#000"/>
            <rect id="olvidada" x="10" y="10" width="5" height="5" fill="#000"/>
        </svg>"##;
        let json = r#"{"capas": [{"id": "cuerpo"}, {"id": "orejas", "tipo": "cabeza"}, {"id": "ojo"}, {"id": "boca"}]}"#;
        let figura = Figura::desde("x".into(), Some(json), Some(svg), &sin_archivos).unwrap();
        let avisos = figura.avisos.join(" | ");
        assert!(avisos.contains("no es de ninguna capa"), "{avisos}");
        assert!(avisos.contains("«orejas»") && avisos.contains("dos lados"), "{avisos}");
        assert!(avisos.contains("ojos llegan muy cerca del borde"), "{avisos}");
        assert_eq!(figura.capas().len(), 4);
        assert_eq!(figura.capas()[1].id, "orejas");
        // Y los ejemplos de la guía no avisan de nada.
        let ejemplos = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/ejemplos");
        assert!(Figura::leer(&ejemplos.join("gato"), "usuario/gato".into()).unwrap().avisos.is_empty());
        assert!(Figura::leer(&ejemplos.join("fantasma"), "usuario/fantasma".into()).unwrap().avisos.is_empty());
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
