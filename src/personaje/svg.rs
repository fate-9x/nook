//! Lectura del dibujo SVG de un personaje con `usvg` (el mismo que usa Slint para sus SVG): cada
//! trazado, ya con sus transformaciones aplicadas y sus curvas convertidas en tramos rectos, con
//! los nombres que lo identifican y cómo se pinta.
//!
//! Un elemento se reconoce por su `id` o por el de los grupos que lo contienen, y también por la
//! etiqueta de la capa de Inkscape (`inkscape:label`), que es como Inkscape nombra las capas.

use super::forma::Punto;
use std::collections::HashMap;
use usvg::tiny_skia_path::PathSegment;

/// Lo más que puede ocupar un dibujo y cuántos puntos puede tener en total (para que dibujarlo
/// 60 veces por segundo no pese).
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
/// Lo que más o menos avisa `figura` de que el dibujo se acerca a los puntos que admite.
pub const MAX_PUNTOS: usize = 30_000;

/// Un tramo continuo de un trazado: sus puntos y si se cierra.
#[derive(Clone, Debug, PartialEq)]
pub struct Subtrazo {
    pub puntos: Vec<Punto>,
    pub cerrado: bool,
}

/// Cómo acaban y cómo se unen los tramos de una línea.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extremo {
    Plano,
    Redondo,
    Cuadrado,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Union {
    Inglete,
    Redonda,
    Bisel,
}

/// Cómo se pinta un elemento: relleno (con la regla par-impar o no) o como una línea.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pintura {
    Relleno { par_impar: bool },
    Linea { grosor: f32, extremo: Extremo, union: Union },
}

/// Un trazado del dibujo pintado de una manera: un elemento con relleno y borde da dos.
#[derive(Clone, Debug, PartialEq)]
pub struct Elemento {
    /// Nombres del elemento y de los grupos que lo contienen, del más cercano al más lejano.
    pub nombres: Vec<String>,
    pub subtrazos: Vec<Subtrazo>,
    pub pintura: Pintura,
    pub color: [u8; 3],
    /// Opacidad (la suya por la de sus grupos), de 0 a 1.
    pub alfa: f32,
}

/// Lee un SVG: sus elementos, en el orden en que se dibujan, con las coordenadas del lienzo (el
/// origen es la esquina de arriba a la izquierda, y el tamaño el del dibujo ya escalado).
pub fn leer(texto: &str) -> Result<Vec<Elemento>, String> {
    leer_con_tamano(texto).map(|(elementos, _)| elementos)
}

/// Lo mismo, pero con las coordenadas del propio dibujo: las de su `viewBox`, tal como las escribió
/// su autor (el origen es el (0, 0) del dibujo, esté donde esté el viewBox). Es como se leen los
/// accesorios, que se dibujan sobre la plantilla de Nook.
pub fn leer_en_unidades(texto: &str) -> Result<Vec<Elemento>, String> {
    let (mut elementos, tamano) = leer_con_tamano(texto)?;
    let (minimo, ancho) = caja_de_vista(texto).unwrap_or(((0.0, 0.0), tamano.0));
    // Cuánto más grande es el lienzo que las unidades del dibujo.
    let escala = if ancho > 0.0 { tamano.0 / ancho } else { 1.0 };
    let escala = if escala.is_finite() && escala > 0.0 { escala } else { 1.0 };
    for e in &mut elementos {
        for s in &mut e.subtrazos {
            for p in &mut s.puntos {
                *p = (p.0 / escala + minimo.0, p.1 / escala + minimo.1);
            }
        }
        if let Pintura::Linea { grosor, .. } = &mut e.pintura {
            *grosor /= escala;
        }
    }
    Ok(elementos)
}

/// El `viewBox` del SVG (esquina de arriba a la izquierda y ancho), si lo tiene.
fn caja_de_vista(texto: &str) -> Option<(Punto, f32)> {
    let documento = usvg::roxmltree::Document::parse(texto).ok()?;
    let valores: Vec<f32> =
        documento.root_element().attribute("viewBox")?.split(|c: char| c.is_whitespace() || c == ',').filter(|t| !t.is_empty()).filter_map(|t| t.parse().ok()).collect();
    (valores.len() == 4 && valores[2] > 0.0).then(|| ((valores[0], valores[1]), valores[2]))
}

fn leer_con_tamano(texto: &str) -> Result<(Vec<Elemento>, (f32, f32)), String> {
    if texto.len() > MAX_BYTES {
        return Err("el dibujo es demasiado grande".into());
    }
    let arbol = usvg::Tree::from_str(texto, &usvg::Options::default()).map_err(|e| format!("el dibujo no se puede leer: {e}"))?;
    let etiquetas = etiquetas_de_inkscape(texto);
    // Las curvas se parten en tramos de una fracción del tamaño del dibujo.
    let caja = arbol.root().abs_bounding_box();
    let paso = (caja.width().hypot(caja.height()) / 400.0).max(1e-3);
    let mut elementos = Vec::new();
    recorrer(arbol.root(), &mut Vec::new(), 1.0, &etiquetas, paso, &mut elementos);
    let puntos: usize = elementos.iter().flat_map(|e| &e.subtrazos).map(|s| s.puntos.len()).sum();
    if puntos > MAX_PUNTOS {
        return Err("el dibujo tiene demasiados puntos: simplifícalo".into());
    }
    Ok((elementos, (arbol.size().width(), arbol.size().height())))
}

/// La etiqueta de Inkscape de cada elemento que la tiene, por su id.
fn etiquetas_de_inkscape(texto: &str) -> HashMap<String, String> {
    const INKSCAPE: &str = "http://www.inkscape.org/namespaces/inkscape";
    let Ok(documento) = usvg::roxmltree::Document::parse(texto) else { return HashMap::new() };
    documento
        .descendants()
        .filter_map(|n| Some((n.attribute("id")?.to_string(), n.attribute((INKSCAPE, "label"))?.to_string())))
        .collect()
}

/// Los nombres de un nodo: su id y su etiqueta de Inkscape (si tiene).
fn nombres_de(id: &str, etiquetas: &HashMap<String, String>) -> Vec<String> {
    let mut nombres = Vec::new();
    if !id.is_empty() {
        nombres.push(id.to_string());
        if let Some(etiqueta) = etiquetas.get(id) {
            nombres.push(etiqueta.clone());
        }
    }
    nombres
}

fn recorrer(
    grupo: &usvg::Group,
    nombres: &mut Vec<String>,
    alfa: f32,
    etiquetas: &HashMap<String, String>,
    paso: f32,
    elementos: &mut Vec<Elemento>,
) {
    for nodo in grupo.children() {
        match nodo {
            usvg::Node::Group(g) => {
                let propios = nombres_de(g.id(), etiquetas);
                let n = propios.len();
                // Los del grupo van delante: son los más cercanos de lo que tiene dentro.
                nombres.splice(0..0, propios);
                recorrer(g, nombres, alfa * g.opacity().get(), etiquetas, paso, elementos);
                nombres.drain(0..n);
            }
            usvg::Node::Path(p) => {
                let mut todos = nombres_de(p.id(), etiquetas);
                todos.extend(nombres.iter().cloned());
                let Some(trazado) = p.data().clone().transform(p.abs_transform()) else { continue };
                let subtrazos = aplanar(&trazado, paso);
                if subtrazos.is_empty() {
                    continue;
                }
                // Lo que no se ve (sin relleno ni borde, u oculto) cuenta por dónde está: así se
                // marcan las anclas. No se pinta.
                if !p.is_visible() || (p.fill().is_none() && p.stroke().is_none()) {
                    let pintura = Pintura::Relleno { par_impar: false };
                    elementos.push(Elemento { nombres: todos, subtrazos, pintura, color: [0; 3], alfa: 0.0 });
                    continue;
                }
                let (ex, ey) = p.abs_transform().get_scale();
                if let Some(relleno) = p.fill() {
                    if let Some((color, opacidad)) = color_de(relleno.paint()) {
                        elementos.push(Elemento {
                            nombres: todos.clone(),
                            subtrazos: subtrazos.clone(),
                            pintura: Pintura::Relleno { par_impar: relleno.rule() == usvg::FillRule::EvenOdd },
                            color,
                            alfa: alfa * relleno.opacity().get() * opacidad,
                        });
                    }
                }
                if let Some(linea) = p.stroke() {
                    if let Some((color, opacidad)) = color_de(linea.paint()) {
                        let extremo = match linea.linecap() {
                            usvg::LineCap::Butt => Extremo::Plano,
                            usvg::LineCap::Round => Extremo::Redondo,
                            usvg::LineCap::Square => Extremo::Cuadrado,
                        };
                        let union = match linea.linejoin() {
                            usvg::LineJoin::Round => Union::Redonda,
                            usvg::LineJoin::Bevel => Union::Bisel,
                            _ => Union::Inglete,
                        };
                        elementos.push(Elemento {
                            nombres: todos,
                            subtrazos,
                            pintura: Pintura::Linea { grosor: linea.width().get() * (ex * ey).abs().sqrt(), extremo, union },
                            color,
                            alfa: alfa * linea.opacity().get() * opacidad,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// El color de una pintura y su opacidad. De un degradado, el de su primera parada.
fn color_de(pintura: &usvg::Paint) -> Option<([u8; 3], f32)> {
    let de = |c: usvg::Color| [c.red, c.green, c.blue];
    match pintura {
        usvg::Paint::Color(c) => Some((de(*c), 1.0)),
        usvg::Paint::LinearGradient(g) => g.stops().first().map(|s| (de(s.color()), s.opacity().get())),
        usvg::Paint::RadialGradient(g) => g.stops().first().map(|s| (de(s.color()), s.opacity().get())),
        usvg::Paint::Pattern(_) => None,
    }
}

/// Los subtrazos de un trazado, con las curvas partidas en tramos de unos `paso` de largo.
fn aplanar(trazado: &usvg::tiny_skia_path::Path, paso: f32) -> Vec<Subtrazo> {
    let mut subtrazos = Vec::new();
    let mut actual: Option<Subtrazo> = None;
    let mut ultimo = (0.0, 0.0);
    let tramos = |largo: f32| ((largo / paso).ceil() as usize).clamp(2, 64);
    let terminar = |actual: &mut Option<Subtrazo>, subtrazos: &mut Vec<Subtrazo>| {
        if let Some(s) = actual.take().filter(|s| s.puntos.len() >= 2) {
            subtrazos.push(s);
        }
    };
    for segmento in trazado.segments() {
        match segmento {
            PathSegment::MoveTo(p) => {
                terminar(&mut actual, &mut subtrazos);
                ultimo = (p.x, p.y);
                actual = Some(Subtrazo { puntos: vec![ultimo], cerrado: false });
            }
            PathSegment::LineTo(p) => {
                ultimo = (p.x, p.y);
                actual.get_or_insert_with(|| Subtrazo { puntos: Vec::new(), cerrado: false }).puntos.push(ultimo);
            }
            PathSegment::QuadTo(c, p) => {
                let (p0, c, p1) = (ultimo, (c.x, c.y), (p.x, p.y));
                let n = tramos(distancia(p0, c) + distancia(c, p1));
                let s = actual.get_or_insert_with(|| Subtrazo { puntos: vec![p0], cerrado: false });
                for k in 1..=n {
                    let t = k as f32 / n as f32;
                    let u = 1.0 - t;
                    s.puntos.push((u * u * p0.0 + 2.0 * u * t * c.0 + t * t * p1.0, u * u * p0.1 + 2.0 * u * t * c.1 + t * t * p1.1));
                }
                ultimo = p1;
            }
            PathSegment::CubicTo(c1, c2, p) => {
                let (p0, c1, c2, p1) = (ultimo, (c1.x, c1.y), (c2.x, c2.y), (p.x, p.y));
                let n = tramos(distancia(p0, c1) + distancia(c1, c2) + distancia(c2, p1));
                let s = actual.get_or_insert_with(|| Subtrazo { puntos: vec![p0], cerrado: false });
                for k in 1..=n {
                    s.puntos.push(super::forma::cubica(p0, c1, c2, p1, k as f32 / n as f32));
                }
                ultimo = p1;
            }
            PathSegment::Close => {
                if let Some(s) = actual.as_mut() {
                    s.cerrado = true;
                    // Si acaba donde empezó, el último punto sobra.
                    if s.puntos.len() > 2 && s.puntos.first().zip(s.puntos.last()).is_some_and(|(a, b)| distancia(*a, *b) < 1e-4) {
                        s.puntos.pop();
                    }
                    ultimo = s.puntos[0];
                }
                terminar(&mut actual, &mut subtrazos);
            }
        }
    }
    terminar(&mut actual, &mut subtrazos);
    subtrazos
}

fn distancia(a: Punto, b: Punto) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_capas_con_sus_nombres_colores_y_lineas() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="100" height="100">
            <g id="layer1" inkscape:label="cuerpo"><circle cx="50" cy="50" r="40" fill="#ff0000" opacity="0.5"/></g>
            <g id="boca" transform="translate(0 10)"><path d="M -5 0 Q 0 3 5 0" fill="none" stroke="#000" stroke-width="2" stroke-linecap="round"/></g>
            <rect id="oculto" width="5" height="5" style="display:none"/>
            <circle id="ancla-ojo" cx="3" cy="4" r="1" fill="none"/>
        </svg>"##;
        let elementos = leer(svg).unwrap();
        assert_eq!(elementos.len(), 3);
        // Lo que no tiene relleno ni borde no se pinta, pero está (marca un sitio).
        assert_eq!(elementos[2].nombres, vec!["ancla-ojo".to_string()]);
        assert_eq!(elementos[2].alfa, 0.0);
        let cuerpo = &elementos[0];
        assert!(cuerpo.nombres.contains(&"cuerpo".to_string()));
        assert_eq!(cuerpo.color, [255, 0, 0]);
        assert!((cuerpo.alfa - 0.5).abs() < 1e-3);
        assert!(cuerpo.subtrazos[0].cerrado);
        let boca = &elementos[1];
        assert_eq!(boca.nombres, vec!["boca".to_string()]);
        assert!(matches!(boca.pintura, Pintura::Linea { grosor, extremo: Extremo::Redondo, .. } if (grosor - 2.0).abs() < 1e-3));
        // La transformación del grupo ya está aplicada.
        assert!(boca.subtrazos[0].puntos.iter().all(|p| (p.1 - 10.0).abs() < 2.0));
        assert!(!boca.subtrazos[0].cerrado);
    }

    #[test]
    fn en_unidades_del_dibujo_el_origen_es_el_suyo_y_no_el_del_lienzo() {
        // El lienzo mide el doble (240) que el viewBox (120): el (0, 0) del dibujo es el centro.
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-60 -75 120 125" width="240" height="250">
            <rect id="a" x="-10" y="-20" width="20" height="40" fill="#f00"/>
            <path id="b" d="M 0 0 L 30 0" fill="none" stroke="#000" stroke-width="4"/>
        </svg>"##;
        let e = leer_en_unidades(svg).unwrap();
        let caja = |e: &Elemento| {
            let p: Vec<_> = e.subtrazos.iter().flat_map(|s| &s.puntos).collect();
            (p.iter().map(|p| p.0).fold(f32::MAX, f32::min), p.iter().map(|p| p.0).fold(f32::MIN, f32::max), p.iter().map(|p| p.1).fold(f32::MAX, f32::min))
        };
        let (x0, x1, y0) = caja(&e[0]);
        assert!((x0 + 10.0).abs() < 1e-3 && (x1 - 10.0).abs() < 1e-3 && (y0 + 20.0).abs() < 1e-3, "{:?}", (x0, x1, y0));
        assert!(matches!(e[1].pintura, Pintura::Linea { grosor, .. } if (grosor - 4.0).abs() < 1e-3), "{:?}", e[1].pintura);
        // Sin viewBox, las del lienzo.
        let sin = r##"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50"><rect x="5" y="5" width="10" height="10"/></svg>"##;
        assert!((leer_en_unidades(sin).unwrap()[0].subtrazos[0].puntos[0].0 - 5.0).abs() < 1e-3);
    }

    #[test]
    fn rechaza_lo_que_no_es_un_svg() {
        assert!(leer("no es un svg").is_err());
    }
}
