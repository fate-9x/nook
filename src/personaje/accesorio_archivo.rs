//! Accesorios hechos por el usuario: cada uno es una carpeta con un dibujo, `accesorio.svg`, y su
//! descripción, `accesorio.json` (ver docs/personajes.md). Se dibujan sobre la plantilla de Nook:
//! el (0, 0) del dibujo es el centro de su cuerpo, que mide 98 de ancho; en otra figura, el
//! accesorio se mueve con su `ancla` (en la coronilla, sube lo que mida de más; en los ojos,
//! baja a los ojos de la figura…), como los que trae el programa (ver `accesorios.rs`).
//!
//! Cada color distinto del dibujo es un color que el usuario puede cambiar (una `Ranura`), salvo
//! los que la descripción marca como fijos. Una forma pintada de un color fijo, o de un color que
//! el usuario no cambió, lleva el del dibujo.

use super::accesorios::{Estilo, Pieza, abierto, al_frente, cerrado};
use super::figura::{Figura, color};
use super::forma::Punto;
use super::motor::{Cabeza, UNIDAD};
use super::svg::{self, Pintura, Subtrazo};
use crate::config::Hueco;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use zeno::Command;

/// Los colores distintos que puede cambiar el usuario en un accesorio, como mucho.
pub const MAX_RANURAS: usize = 8;

/// Cómo va puesto: pintado en la cara como los ojos (gira sobre la esfera y no se sale del
/// cuerpo), como un cartel que mira siempre al frente, o rodeando la cabeza (lo que queda del lado
/// que no se ve pasa por detrás del cuerpo).
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Modo {
    Cara,
    Cartel,
    Alrededor,
}

/// Dónde se pega en el personaje: el sitio de la plantilla de Nook del que se mueve con la figura.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Ancla {
    /// Lo alto de la cabeza.
    Coronilla,
    Ojos,
    Boca,
    Cuello,
    /// Detrás del cuerpo (con `cartel`, el accesorio va detrás de él).
    Espalda,
    /// El centro del cuerpo: no se mueve con la figura.
    Cuerpo,
}

/// Lo que tapa el accesorio: lo que sale de lo alto de la cabeza (el rulo…), los ojos o la boca.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum Parte {
    Cabeza,
    Ojos,
    Boca,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Colores {
    /// Colores del dibujo que el usuario no puede cambiar.
    fijos: Vec<String>,
    /// Nombre de los que sí, por su color en el dibujo y por idioma.
    nombres: HashMap<String, HashMap<String, String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descripcion {
    #[serde(default, deserialize_with = "super::figura::nombre")]
    nombre: HashMap<String, String>,
    modo: Modo,
    #[serde(default)]
    ancla: Option<Ancla>,
    #[serde(default)]
    hueco: Option<Hueco>,
    #[serde(default)]
    orden: Option<i32>,
    #[serde(default)]
    tapa: Vec<Parte>,
    #[serde(default)]
    colores: Colores,
}

/// Un color de un accesorio que el usuario puede cambiar: el del dibujo y, si lo tiene, su nombre
/// por idioma.
#[derive(Clone, Debug, PartialEq)]
pub struct Ranura {
    pub original: [u8; 3],
    pub nombre: HashMap<String, String>,
}

/// Una forma del dibujo en unidades de radio del cuerpo (con el (0, 0) en el centro de Nook): el
/// color que le toca cambiar (`ranura`, o ninguno si es fijo), el que tiene en el dibujo y su
/// centro.
#[derive(Clone, Debug, PartialEq)]
pub struct Forma {
    pub subtrazos: Vec<Subtrazo>,
    pub pintura: Pintura,
    pub ranura: Option<usize>,
    pub color: [u8; 3],
    pub alfa: f32,
    pub centro: Punto,
}

#[derive(Debug, PartialEq)]
pub struct Archivo {
    /// "usuario/<carpeta>" o, de uno que es de un personaje, "usuario/<personaje>/<carpeta>".
    pub id: String,
    pub nombre: HashMap<String, String>,
    pub modo: Modo,
    pub ancla: Ancla,
    pub hueco: Option<Hueco>,
    pub orden: i32,
    pub tapa_la_cabeza: bool,
    pub tapa_los_ojos: bool,
    pub tapa_la_boca: bool,
    pub ranuras: Vec<Ranura>,
    pub formas: Vec<Forma>,
    /// El personaje al que pertenece (su id), si está en la carpeta de uno: solo se ofrece con él.
    pub propietario: Option<String>,
}

/// Orden en el que se dibuja un accesorio de archivo si no dice otro: encima de los que trae el
/// programa (que van del 0 al 9).
const ORDEN_INICIAL: i32 = 10;

impl Archivo {
    /// Lee el accesorio de una carpeta (con `accesorio.json` y `accesorio.svg`).
    pub fn leer(carpeta: &Path, id: String, propietario: Option<String>) -> Result<Archivo, String> {
        let texto = |nombre: &str| std::fs::read_to_string(carpeta.join(nombre)).map_err(|e| format!("{nombre}: {e}"));
        Archivo::desde(id, propietario, &texto("accesorio.json")?, &texto("accesorio.svg")?)
    }

    /// Crea el accesorio de su descripción y su dibujo.
    pub fn desde(id: String, propietario: Option<String>, json: &str, dibujo: &str) -> Result<Archivo, String> {
        let d: Descripcion = serde_json::from_str(json).map_err(|e| format!("accesorio.json: {e}"))?;
        let elementos = svg::leer_en_unidades(dibujo).map_err(|e| format!("accesorio.svg: {e}"))?;
        let fijos: Vec<[u8; 3]> = d.colores.fijos.iter().map(|c| color(c)).collect::<Result<_, _>>().map_err(|e| format!("accesorio.json: {e}"))?;
        let mut nombres: HashMap<[u8; 3], HashMap<String, String>> = HashMap::new();
        for (c, n) in &d.colores.nombres {
            nombres.insert(color(c).map_err(|e| format!("accesorio.json: {e}"))?, n.clone());
        }

        let mut ranuras: Vec<Ranura> = Vec::new();
        let mut formas = Vec::new();
        for e in elementos.into_iter().filter(|e| e.alfa > 0.0) {
            let ranura = if fijos.contains(&e.color) {
                None
            } else {
                let i = match ranuras.iter().position(|r| r.original == e.color) {
                    Some(i) => i,
                    None => {
                        ranuras.push(Ranura { original: e.color, nombre: nombres.get(&e.color).cloned().unwrap_or_default() });
                        ranuras.len() - 1
                    }
                };
                Some(i)
            };
            let subtrazos: Vec<Subtrazo> = e
                .subtrazos
                .into_iter()
                .map(|s| Subtrazo { puntos: s.puntos.into_iter().map(|(x, y)| (x / UNIDAD, y / UNIDAD)).collect(), cerrado: s.cerrado })
                .collect();
            let pintura = match e.pintura {
                Pintura::Linea { grosor, extremo, union } => Pintura::Linea { grosor: grosor / UNIDAD, extremo, union },
                relleno => relleno,
            };
            let puntos = subtrazos.iter().flat_map(|s| &s.puntos);
            let (x0, x1) = puntos.clone().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
            let (y0, y1) = puntos.fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
            formas.push(Forma { subtrazos, pintura, ranura, color: e.color, alfa: e.alfa, centro: ((x0 + x1) / 2.0, (y0 + y1) / 2.0) });
        }
        if formas.is_empty() {
            return Err("accesorio.svg: no hay nada que dibujar (formas con relleno o con borde)".into());
        }
        if ranuras.len() > MAX_RANURAS {
            return Err(format!(
                "accesorio.svg: tiene {} colores distintos y se pueden cambiar {MAX_RANURAS} como mucho: usa menos o márcalos como fijos en «colores» → «fijos»",
                ranuras.len()
            ));
        }
        if let Some(sin_forma) = nombres.keys().find(|c| !ranuras.iter().any(|r| r.original == **c)) {
            return Err(format!("accesorio.json: «colores» → «nombres» habla de #{:02x}{:02x}{:02x}, que no es un color que se pueda cambiar en el dibujo", sin_forma[0], sin_forma[1], sin_forma[2]));
        }

        let carpeta = id.rsplit('/').next().unwrap_or(&id).to_string();
        let nombre = if d.nombre.is_empty() { HashMap::from([("es".to_string(), carpeta)]) } else { d.nombre };
        let tapa = |p: Parte| d.tapa.contains(&p);
        Ok(Archivo {
            id,
            nombre,
            modo: d.modo,
            ancla: d.ancla.unwrap_or(Ancla::Cuerpo),
            hueco: d.hueco,
            orden: d.orden.unwrap_or(ORDEN_INICIAL),
            tapa_la_cabeza: tapa(Parte::Cabeza),
            tapa_los_ojos: tapa(Parte::Ojos),
            tapa_la_boca: tapa(Parte::Boca),
            ranuras,
            formas,
            propietario,
        })
    }
}

/// Cuánto hay que mover el dibujo para la figura de `cabeza` (unidades de radio): lo que se
/// diferencia su coronilla, sus ojos, su boca o su base de las de Nook, que es la plantilla.
fn desplazamiento(ancla: Ancla, cabeza: &Cabeza) -> Punto {
    let (m, nook) = (cabeza.medidas(), Figura::nook().medidas);
    match ancla {
        Ancla::Coronilla => (0.0, -(m.arriba - nook.arriba)),
        Ancla::Ojos => (0.0, m.ojo.1 - nook.ojo.1),
        Ancla::Boca => (m.boca.0 - nook.boca.0, m.boca.1 - nook.boca.1),
        Ancla::Cuello | Ancla::Espalda => (0.0, m.abajo - nook.abajo),
        Ancla::Cuerpo => (0.0, 0.0),
    }
}

/// Punto de la cabeza (x a su derecha, y hacia abajo, z hacia donde mira) del que cuelga un
/// accesorio de tipo cartel: así se mueve con la cabeza al girarla, sin deformarse.
fn pivote(ancla: Ancla, cabeza: &Cabeza) -> [f32; 3] {
    let m = cabeza.medidas();
    // Altura de un punto de la cara y lo que sale hacia delante (la cara es un elipsoide).
    let en_cara = |x: f32, y: f32| {
        let (a, b) = (x / m.ancho.max(1e-3), y / m.abajo.max(1e-3));
        [x, y, (1.0 - a * a - b * b).max(0.0).sqrt()]
    };
    match ancla {
        Ancla::Coronilla => [0.0, -m.arriba, 0.3],
        Ancla::Ojos => en_cara(0.0, m.ojo.1),
        Ancla::Boca => en_cara(m.boca.0, m.boca.1),
        Ancla::Cuello => [0.0, m.abajo * 0.8, 0.45],
        Ancla::Espalda => [0.0, 0.0, -0.3],
        Ancla::Cuerpo => [0.0; 3],
    }
}

/// Contorno de unos subtrazos con cada punto pasado por `a_pixeles`. Lo relleno se cierra siempre;
/// las líneas, solo si estaban cerradas.
fn trazo(subtrazos: &[Subtrazo], relleno: bool, a_pixeles: &dyn Fn(Punto) -> [f32; 2]) -> Vec<Command> {
    let mut trazo = Vec::new();
    for s in subtrazos {
        let pixeles: Vec<[f32; 2]> = s.puntos.iter().map(|&p| a_pixeles(p)).collect();
        trazo.extend(if relleno || s.cerrado { cerrado(&pixeles) } else { abierto(&pixeles) });
    }
    trazo
}

/// Las piezas de un accesorio de archivo. `colores` son los que eligió el usuario para cada
/// `Ranura` (los que faltan, los del dibujo); `pantalla` pasa de unidades de radio a píxeles y
/// `escala` es cuántos píxeles mide una unidad.
pub fn piezas(a: &Archivo, colores: &[Option<[u8; 3]>], cabeza: &Cabeza, pantalla: &dyn Fn(Punto) -> [f32; 2], escala: f32) -> Vec<Pieza> {
    let (dx, dy) = desplazamiento(a.ancla, cabeza);
    let mover = |(x, y): Punto| (x + dx, y + dy);
    let pivote = pivote(a.ancla, cabeza);
    // En la cara, el cuello y los ojos o la boca, el accesorio se desvanece al pasar al otro lado
    // de la cabeza (como lo pintado en ella) y no se sale del cuerpo.
    let pegado = matches!(a.ancla, Ancla::Ojos | Ancla::Boca | Ancla::Cuello);
    let mut piezas = Vec::new();
    for forma in &a.formas {
        let tono = forma.ranura.and_then(|r| colores.get(r).copied().flatten()).unwrap_or(forma.color);
        let (relleno, estilo) = match forma.pintura {
            Pintura::Relleno { par_impar: false } => (true, Estilo::Relleno),
            Pintura::Relleno { par_impar: true } => (true, Estilo::ConAgujeros),
            Pintura::Linea { grosor, .. } => (false, Estilo::Trazo(grosor * escala)),
        };
        let (contorno, delante, detras, recortar) = match a.modo {
            Modo::Cara => {
                let visible = (cabeza.en_cara(mover(forma.centro)).1 / 0.12).clamp(0.0, 1.0);
                (trazo(&forma.subtrazos, relleno, &|p| pantalla(cabeza.en_cara(mover(p)).0)), visible, false, true)
            }
            Modo::Cartel => {
                let respecto = |p: Punto| {
                    let (x, y) = mover(p);
                    cabeza.cartel(pivote, (x - pivote[0], y - pivote[1]))
                };
                let contorno = trazo(&forma.subtrazos, relleno, &|p| pantalla(respecto(p)));
                if a.ancla == Ancla::Espalda {
                    (contorno, 0.0, true, false)
                } else {
                    let visible = if pegado { (cabeza.profundidad(pivote) / 0.12).clamp(0.0, 1.0) } else { 1.0 };
                    (contorno, visible, false, pegado)
                }
            }
            Modo::Alrededor => {
                // Es del lado en el que está su centro: lo del lado que no se ve pasa por detrás.
                let lado = (mover(forma.centro).0 / cabeza.medidas().ancho.max(1e-3)).clamp(-1.0, 1.0);
                let contorno = trazo(&forma.subtrazos, relleno, &|p| {
                    let (x, y) = mover(p);
                    pantalla(cabeza.punto([x, y, 0.0]))
                });
                (contorno, al_frente(cabeza.profundidad([lado, 0.0, 0.0])), true, false)
            }
        };
        piezas.push(Pieza { trazo: contorno, estilo, color: tono, alfa: forma.alfa, delante, detras, recortar });
    }
    piezas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::personaje::motor::Motor;

    const SOMBRERO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-60 -80 120 120">
        <rect id="ala" x="-30" y="-44" width="60" height="6" rx="3" fill="#202020"/>
        <path id="copa" d="M -18 -44 L -14 -70 L 14 -70 L 18 -44 Z" fill="#3a6ea5"/>
        <rect id="cinta" x="-17" y="-52" width="34" height="5" fill="#e0455a"/>
        <circle id="adorno" cx="0" cy="-49" r="2" fill="#ffffff"/>
    </svg>"##;

    fn sombrero() -> Archivo {
        let json = r##"{"nombre": {"es": "Sombrero"}, "modo": "cartel", "ancla": "coronilla", "hueco": "cabeza", "tapa": ["cabeza"],
            "colores": {"fijos": ["#ffffff"], "nombres": {"#e0455a": {"es": "Cinta"}}}}"##;
        Archivo::desde("usuario/sombrero".into(), None, json, SOMBRERO).unwrap()
    }

    #[test]
    fn cada_color_distinto_es_una_ranura_salvo_los_fijos() {
        let a = sombrero();
        assert_eq!(a.ranuras.iter().map(|r| r.original).collect::<Vec<_>>(), vec![[0x20, 0x20, 0x20], [0x3a, 0x6e, 0xa5], [0xe0, 0x45, 0x5a]]);
        assert_eq!(a.ranuras[2].nombre["es"], "Cinta");
        assert!(a.ranuras[0].nombre.is_empty());
        // El blanco es fijo: su forma no lleva ranura.
        assert_eq!(a.formas.iter().map(|f| f.ranura).collect::<Vec<_>>(), vec![Some(0), Some(1), Some(2), None]);
        assert_eq!((a.modo, a.ancla, a.hueco, a.orden), (Modo::Cartel, Ancla::Coronilla, Some(Hueco::Cabeza), ORDEN_INICIAL));
        assert!(a.tapa_la_cabeza && !a.tapa_los_ojos && !a.tapa_la_boca);
        // Las medidas, en unidades de radio con el (0, 0) en el centro de Nook.
        let ala = &a.formas[0];
        assert!((ala.centro.0).abs() < 1e-3 && (ala.centro.1 + 41.0 / UNIDAD).abs() < 1e-3, "{:?}", ala.centro);
    }

    #[test]
    fn explica_lo_que_esta_mal() {
        let malo = |json: &str, svg: &str| Archivo::desde("usuario/x".into(), None, json, svg).unwrap_err();
        assert!(malo(r#"{"modo": "cartel", "forma": 1}"#, SOMBRERO).contains("forma"));
        assert!(malo("{}", SOMBRERO).contains("modo"));
        assert!(malo(r#"{"modo": "cartel"}"#, "no es un svg").contains("accesorio.svg"));
        assert!(malo(r#"{"modo": "cartel"}"#, r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#).contains("nada que dibujar"));
        assert!(malo(r##"{"modo": "cartel", "colores": {"fijos": ["rojo"]}}"##, SOMBRERO).contains("color no válido"));
        assert!(malo(r##"{"modo": "cartel", "colores": {"nombres": {"#010203": {"es": "A"}}}}"##, SOMBRERO).contains("#010203"));
        // Demasiados colores.
        let muchos: String = (0..9).map(|i| format!(r##"<rect width="5" height="5" fill="#0{i}0000"/>"##)).collect();
        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg">{muchos}</svg>"#);
        assert!(malo(r#"{"modo": "cara"}"#, &svg).contains("9 colores"));
    }

    fn cabeza_de(figura: std::sync::Arc<Figura>) -> Cabeza {
        let mut motor = Motor::new();
        motor.poner_figura(figura);
        motor.muestrear(0.5).cabeza
    }

    /// Alto de la pieza (mínimo y), en unidades de radio: la pantalla es la identidad.
    fn arriba_de(piezas: &[Pieza]) -> f32 {
        piezas
            .iter()
            .flat_map(|p| p.trazo.iter())
            .filter_map(|c| match c {
                Command::MoveTo(p) | Command::LineTo(p) => Some(p.y),
                _ => None,
            })
            .fold(f32::MAX, f32::min)
    }

    #[test]
    fn un_sombrero_de_la_coronilla_sube_con_una_figura_mas_alta_y_toma_los_colores_elegidos() {
        let a = sombrero();
        let identidad = |(x, y): Punto| [x, y];
        let con_colores = |figura, colores: &[Option<[u8; 3]>]| piezas(&a, colores, &cabeza_de(figura), &identidad, 1.0);
        let nook = con_colores(Figura::nook(), &[]);
        let limon = con_colores(Figura::limon(), &[]);
        // El Limón mide más hacia arriba: el sombrero sube lo que mide de más.
        let sube = arriba_de(&nook) - arriba_de(&limon);
        let mide_de_mas = Figura::limon().medidas.arriba - Figura::nook().medidas.arriba;
        assert!((sube - mide_de_mas).abs() < 0.02, "{sube} frente a {mide_de_mas}");
        // Una pieza por forma; de frente, delante del cuerpo y sin recortar (va en la coronilla).
        assert_eq!(nook.len(), 4);
        assert!(nook.iter().all(|p| p.delante > 0.99 && !p.detras && !p.recortar));
        // Colores: los del dibujo, salvo el que eligió el usuario (la cinta).
        assert_eq!(nook.iter().map(|p| p.color).collect::<Vec<_>>(), vec![[0x20; 3], [0x3a, 0x6e, 0xa5], [0xe0, 0x45, 0x5a], [255; 3]]);
        let elegidos = con_colores(Figura::nook(), &[None, None, Some([1, 2, 3])]);
        assert_eq!(elegidos[2].color, [1, 2, 3]);
        assert_eq!(elegidos[1].color, [0x3a, 0x6e, 0xa5]);
        // La parte fija no cambia aunque se pasen más colores de la cuenta.
        assert_eq!(con_colores(Figura::nook(), &[None, None, None, Some([9, 9, 9])])[3].color, [255; 3]);
    }

    #[test]
    fn cada_modo_coloca_las_piezas_a_su_manera() {
        let identidad = |(x, y): Punto| [x, y];
        let con = |json: &str, figura| {
            let a = Archivo::desde("usuario/x".into(), None, json, SOMBRERO).unwrap();
            piezas(&a, &[], &cabeza_de(figura), &identidad, 1.0)
        };
        // En la cara: delante y recortado por el cuerpo.
        let cara = con(r#"{"modo": "cara", "ancla": "ojos"}"#, Figura::nook());
        assert!(cara.iter().all(|p| p.recortar && !p.detras));
        // Alrededor de la cabeza: siempre también detrás del cuerpo.
        let alrededor = con(r#"{"modo": "alrededor"}"#, Figura::nook());
        assert!(alrededor.iter().all(|p| p.detras && !p.recortar));
        // En la espalda, un cartel solo se ve detrás del cuerpo.
        let espalda = con(r#"{"modo": "cartel", "ancla": "espalda"}"#, Figura::nook());
        assert!(espalda.iter().all(|p| p.detras && p.delante == 0.0));
        // Pegado a los ojos o a la boca baja con ellos: el Limón los tiene más abajo que Nook.
        let en_ojos = |figura| arriba_de(&con(r#"{"modo": "cartel", "ancla": "ojos"}"#, figura));
        assert!(en_ojos(Figura::limon()) > en_ojos(Figura::nook()) + 0.05);
        let en_cuerpo = |figura| arriba_de(&con(r#"{"modo": "cartel", "ancla": "cuerpo"}"#, figura));
        assert!((en_cuerpo(Figura::limon()) - en_cuerpo(Figura::nook())).abs() < 1e-4);
    }

    #[test]
    fn los_ejemplos_de_la_guia_se_leen() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/ejemplos-accesorios");
        let leer = |n: &str| Archivo::leer(&base.join(n), format!("usuario/{n}"), None).unwrap();
        let sombrero = leer("sombrero");
        assert_eq!((sombrero.modo, sombrero.ancla, sombrero.ranuras.len(), sombrero.formas.len()), (Modo::Cartel, Ancla::Coronilla, 3, 4));
        assert_eq!(sombrero.ranuras[2].nombre["en"], "Band");
        assert!(sombrero.tapa_la_cabeza && sombrero.formas[3].ranura.is_none());
        let aureola = leer("aureola");
        assert_eq!((aureola.modo, aureola.hueco, aureola.ranuras.len()), (Modo::Alrededor, Some(Hueco::Cabeza), 1));
        // Un aro: solo tiene borde.
        assert!(matches!(aureola.formas[0].pintura, Pintura::Linea { .. }));
        let bigote = leer("bigote");
        assert_eq!((bigote.modo, bigote.ancla, bigote.hueco, bigote.ranuras.len()), (Modo::Cara, Ancla::Boca, Some(Hueco::Boca), 2));
        assert_eq!(bigote.nombre["es"], "Bigote");
    }

    #[test]
    fn la_opacidad_de_la_forma_pasa_a_la_pieza() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect width="10" height="10" fill="#ff0000" fill-opacity="0.4"/></svg>"##;
        let a = Archivo::desde("usuario/x".into(), None, r#"{"modo": "cartel"}"#, svg).unwrap();
        let identidad = |(x, y): Punto| [x, y];
        let p = piezas(&a, &[], &cabeza_de(Figura::nook()), &identidad, 1.0);
        assert!((p[0].alfa - 0.4).abs() < 1e-3);
    }
}
