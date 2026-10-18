//! Dibujo de un cuadro del personaje en una imagen, con la figura que tenga (ver `figura.rs`): lo
//! que va detrás del cuerpo, el cuerpo con lo pintado en él (manchas, franjas, ojos y boca),
//! recortado por él, y encima lo que va delante (el pelo, lo que sale de la cabeza y los
//! accesorios) y la pastilla de aviso. En las figuras de imágenes, el cuerpo es el fotograma que
//! toque, girado, estirado y movido como el cuerpo. Las formas se rasterizan con `zeno`, el mismo
//! que usa Slint para sus trazados, con los bordes suavizados.

use super::accesorios::{self, Accesorio, Estilo};
use super::estados::{OJO_ALTO, OJO_ANCHO};
use super::figura::{Apendice, Capa, Delante, Dibujo, Figura, Forma, Fotograma, Imagenes, Orientacion, TipoCapa};
use super::forma::Punto;
use super::motor::{Cabeza, Cuadro, Ojo};
use super::svg::{Extremo, Pintura, Subtrazo, Union};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use zeno::{Cap, Command, Fill, Join, Mask, PathBuilder, Stroke};

/// Cómo se ve el personaje: el color del cuerpo y el de los rasgos (que sustituyen a los colores
/// del dibujo de la figura que los siguen, ver `figura::Colores`), el de la pastilla de aviso y
/// los accesorios que lleva puestos (se dibujan en el orden de `Tipo::orden`).
#[derive(Clone, Debug, PartialEq)]
pub struct Aspecto {
    pub cuerpo: [u8; 3],
    pub rasgos: [u8; 3],
    /// Color de la pastilla de aviso; sin él, el de la figura (o el de los rasgos).
    pub aviso: Option<[u8; 3]>,
    pub accesorios: Vec<Accesorio>,
}

/// Radio del cuerpo respecto al lado de la imagen (deja margen para lo que sale de él y los
/// saltos).
pub const RADIO: f32 = 0.34;
/// Cuánto queda el centro del cuerpo por debajo del centro de la imagen, en unidades del radio:
/// con el rulo encima, así el personaje entero queda centrado y al saltar el rulo no se sale.
pub const BAJADA: f32 = 0.28;

/// Hasta dónde se rellenan las franjas por fuera del cuerpo (unidades de radio).
const FUERA_FRANJA: f32 = 1.8;

/// Constante para aproximar un cuarto de círculo con una curva cúbica.
const CUARTO: f32 = 0.552_284_8;

/// Cápsula (rectángulo con los extremos redondos) de `ancho` × `alto` centrada en el origen, con
/// cada punto pasado por `p`.
pub fn capsula(ancho: f32, alto: f32, p: impl Fn(f32, f32) -> [f32; 2]) -> Vec<Command> {
    let (w, h) = (ancho / 2.0, alto / 2.0);
    let r = w.min(h);
    let k = r * CUARTO;
    let (ix, iy) = (w - r, h - r);
    let mut trazo = Vec::new();
    trazo.move_to(p(-w, -iy));
    trazo.curve_to(p(-w, -iy - k), p(-ix - k, -h), p(-ix, -h));
    trazo.line_to(p(ix, -h));
    trazo.curve_to(p(ix + k, -h), p(w, -iy - k), p(w, -iy));
    trazo.line_to(p(w, iy));
    trazo.curve_to(p(w, iy + k), p(ix + k, h), p(ix, h));
    trazo.line_to(p(-ix, h));
    trazo.curve_to(p(-ix - k, h), p(-w, iy + k), p(-w, iy));
    trazo.close();
    trazo
}

/// Cobertura de cada píxel (0–1).
type Mascara = Vec<f32>;

fn a_mascara(cobertura: Vec<u8>, alfa: f32) -> Mascara {
    cobertura.into_iter().map(|v| v as f32 / 255.0 * alfa).collect()
}

/// Cobertura de un trazo ya en píxeles pintado de una manera; los grosores, en unidades de radio.
fn cobertura(trazo: &[Command], pintura: Pintura, escala: f32, lado: u32) -> Vec<u8> {
    let mut mascara = Mask::new(trazo);
    match pintura {
        Pintura::Relleno { par_impar } => {
            mascara.style(if par_impar { Fill::EvenOdd } else { Fill::NonZero });
        }
        Pintura::Linea { grosor, extremo, union } => {
            let cap = match extremo {
                Extremo::Plano => Cap::Butt,
                Extremo::Redondo => Cap::Round,
                Extremo::Cuadrado => Cap::Square,
            };
            let join = match union {
                Union::Inglete => Join::Miter,
                Union::Redonda => Join::Round,
                Union::Bisel => Join::Bevel,
            };
            mascara.style(*Stroke::new((grosor * escala).max(0.5)).cap(cap).join(join));
        }
    }
    mascara.size(lado, lado).render().0
}

/// Trazo de unos subtrazos con cada punto llevado a la pantalla. Lo relleno se cierra siempre; las
/// líneas, solo si estaban cerradas.
fn trazo(subtrazos: &[Subtrazo], relleno: bool, a_pantalla: &dyn Fn(Punto) -> [f32; 2]) -> Vec<Command> {
    let mut trazo = Vec::new();
    for s in subtrazos {
        for (i, &p) in s.puntos.iter().enumerate() {
            if i == 0 { trazo.move_to(a_pantalla(p)) } else { trazo.line_to(a_pantalla(p)) };
        }
        if relleno || s.cerrado {
            trazo.close();
        }
    }
    trazo
}

/// Contorno cerrado que pasa por los puntos (ya en píxeles).
fn poligono(puntos: &[[f32; 2]]) -> Vec<Command> {
    let mut trazo = Vec::new();
    for (i, &p) in puntos.iter().enumerate() {
        if i == 0 { trazo.move_to(p) } else { trazo.line_to(p) };
    }
    if !puntos.is_empty() {
        trazo.close();
    }
    trazo
}

/// Contorno de una franja (unidades de radio) a partir de su borde: el borde, pintado en la cara
/// para que se mueva con ella (solo la parte que se ve: la que queda al otro lado de la cabeza se
/// doblaría sobre sí misma), sigue en horizontal hasta fuera del cuerpo por los dos lados y se
/// cierra por fuera, arriba o abajo hasta `fuera`, como un cartel. Luego se recorta por el cuerpo.
fn franja(cabeza: &Cabeza, borde: &[Punto], fuera: f32) -> Vec<Punto> {
    // De izquierda a derecha, lo dibuje como lo dibuje.
    let al_reves = borde.first().zip(borde.last()).is_some_and(|(a, b)| a.0 > b.0);
    let borde: Vec<Punto> = if al_reves { borde.iter().rev().copied().collect() } else { borde.to_vec() };
    let puntos: Vec<Punto> = borde
        .iter()
        .filter_map(|&p| {
            let (q, profundidad) = cabeza.en_cara(p);
            (profundidad > 0.01).then_some(q)
        })
        .collect();
    let (Some(&primero), Some(&ultimo)) = (puntos.first(), puntos.last()) else { return Vec::new() };
    let centro = cabeza.cartel([0.0; 3], (0.0, 0.0));
    let hacia = |p: Punto, d: Punto| {
        let q = cabeza.cartel([0.0; 3], d);
        (p.0 + q.0 - centro.0, p.1 + q.1 - centro.1)
    };
    let mut contorno = vec![hacia(primero, (-1.0, 0.0))];
    contorno.extend(puntos);
    contorno.push(hacia(ultimo, (1.0, 0.0)));
    contorno.push(cabeza.cartel([0.0; 3], (FUERA_FRANJA, fuera)));
    contorno.push(cabeza.cartel([0.0; 3], (-FUERA_FRANJA, fuera)));
    contorno
}

/// Lo que queda de unos subtrazos por encima de la altura `corte` (y ≤ corte): de lo relleno, el
/// polígono recortado; de las líneas, los tramos que quedan.
fn por_encima(subtrazos: &[Subtrazo], relleno: bool, corte: f32) -> Vec<Subtrazo> {
    let cruce = |a: Punto, b: Punto| {
        let t = (corte - a.1) / (b.1 - a.1);
        (a.0 + (b.0 - a.0) * t, corte)
    };
    let dentro = |p: Punto| p.1 <= corte;
    let mut resultado = Vec::new();
    for s in subtrazos {
        let n = s.puntos.len();
        if relleno {
            // Sutherland-Hodgman contra un semiplano.
            let mut puntos = Vec::new();
            for i in 0..n {
                let (a, b) = (s.puntos[i], s.puntos[(i + 1) % n]);
                match (dentro(a), dentro(b)) {
                    (true, true) => puntos.push(b),
                    (true, false) => puntos.push(cruce(a, b)),
                    (false, true) => {
                        puntos.push(cruce(a, b));
                        puntos.push(b);
                    }
                    (false, false) => {}
                }
            }
            if puntos.len() >= 3 {
                resultado.push(Subtrazo { puntos, cerrado: true });
            }
        } else {
            let tramos = if s.cerrado { n } else { n.saturating_sub(1) };
            let mut actual: Vec<Punto> = Vec::new();
            for i in 0..tramos {
                let (a, b) = (s.puntos[i], s.puntos[(i + 1) % n]);
                match (dentro(a), dentro(b)) {
                    (true, true) => {
                        if actual.is_empty() {
                            actual.push(a);
                        }
                        actual.push(b);
                    }
                    (true, false) => {
                        if actual.is_empty() {
                            actual.push(a);
                        }
                        actual.push(cruce(a, b));
                        resultado.push(Subtrazo { puntos: std::mem::take(&mut actual), cerrado: false });
                    }
                    (false, true) => actual = vec![cruce(a, b), b],
                    (false, false) => {}
                }
            }
            if actual.len() >= 2 {
                resultado.push(Subtrazo { puntos: actual, cerrado: false });
            }
        }
    }
    resultado
}

/// Entre qué profundidades lo que sale de la cabeza con `Delante::Girar` pasa de detrás a delante
/// (como lo que rodea la cabeza en los accesorios).
fn al_frente(profundidad: f32) -> f32 {
    let k = ((profundidad + 0.35) / 0.2).clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

/// Color premultiplicado (RGB y opacidad, 0–255) al que se le van poniendo capas encima.
type Pixel = [f32; 4];

/// Pone encima de `pixel` un color con opacidad `alfa` (0–1).
fn encima(pixel: &mut Pixel, color: [u8; 3], alfa: f32) {
    for i in 0..3 {
        pixel[i] = color[i] as f32 * alfa + pixel[i] * (1.0 - alfa);
    }
    pixel[3] = 255.0 * alfa + pixel[3] * (1.0 - alfa);
}

/// El color del cuerpo antes de pintar nada en él: uno solo o, con imágenes, el de cada píxel.
enum Base {
    Color([u8; 3]),
    Imagen(Vec<[f32; 3]>),
}

/// Lo que se va juntando para componer la imagen: lo de detrás del cuerpo, el cuerpo (cuánto
/// cubre cada píxel y su color), lo pintado en él (que no se sale) y lo de delante (que se
/// recorta por el cuerpo si lleva `true`).
struct Capas {
    detras: Vec<([u8; 3], Mascara)>,
    cuerpo: Mascara,
    base: Base,
    pinturas: Vec<([u8; 3], Mascara)>,
    delante: Vec<([u8; 3], Mascara, bool)>,
}

/// Lo necesario para dibujar las formas de la figura: el cuadro, el tamaño de la imagen, cómo
/// pasar a píxeles y cómo se ve.
struct Pintor<'a> {
    cuadro: &'a Cuadro,
    figura: &'a Figura,
    aspecto: &'a Aspecto,
    lado: u32,
    escala: f32,
    pantalla: &'a dyn Fn(Punto) -> [f32; 2],
}

impl Pintor<'_> {
    /// El color de una forma: el del cuerpo o los rasgos si es uno de los que los siguen.
    fn color(&self, color: [u8; 3]) -> [u8; 3] {
        let colores = &self.figura.colores;
        if colores.cuerpo == Some(color) {
            self.aspecto.cuerpo
        } else if colores.rasgos == Some(color) {
            self.aspecto.rasgos
        } else {
            color
        }
    }

    /// Cobertura de una forma con cada punto llevado a su sitio por `mover` (unidades de radio
    /// en la pantalla), multiplicada por `alfa` (además de la suya).
    fn forma(&self, forma: &Forma, alfa: f32, mover: &dyn Fn(Punto) -> Punto) -> Mascara {
        self.subtrazos(&forma.subtrazos, forma.pintura, forma.alfa * alfa, mover)
    }

    fn subtrazos(&self, subtrazos: &[Subtrazo], pintura: Pintura, alfa: f32, mover: &dyn Fn(Punto) -> Punto) -> Mascara {
        let relleno = matches!(pintura, Pintura::Relleno { .. });
        let trazo = trazo(subtrazos, relleno, &|p| (self.pantalla)(mover(p)));
        a_mascara(cobertura(&trazo, pintura, self.escala, self.lado), alfa)
    }

    /// Cuánto se aparta del sitio un punto (ya en la pantalla) de lo que se mueve como el pelo:
    /// nada donde empieza el pelo y del todo en lo alto, como un mechón que se dobla.
    fn con_muelle(&self, (x, y): Punto) -> Punto {
        let Some(raiz) = self.figura.muelle else { return (x, y) };
        let cabeza = &self.cuadro.cabeza;
        let (borde, alto) = (cabeza.cartel([0.0; 3], (0.0, raiz)).1, cabeza.cartel([0.0; 3], (0.0, -self.figura.medidas.arriba)).1);
        let k = ((borde - y) / (borde - alto).max(1e-3)).clamp(0.0, 1.3);
        (x + self.cuadro.pelo.0 * k, y + self.cuadro.pelo.1 * k)
    }

    /// Las formas del cuerpo como máscara, agrandadas `escala` veces desde su centro.
    fn silueta(&self, agrandar: f32, muelle: bool) -> Mascara {
        let cabeza = &self.cuadro.cabeza;
        let (cx, cy) = cabeza.punto([0.0; 3]);
        let mover = |p: Punto| {
            let (x, y) = cabeza.cuerpo(p);
            let q = (cx + (x - cx) * agrandar, cy + (y - cy) * agrandar);
            if muelle { self.con_muelle(q) } else { q }
        };
        let mut total: Mascara = vec![0.0; (self.lado * self.lado) as usize];
        for capa in self.figura.capas().iter().filter(|c| c.tipo == TipoCapa::Cuerpo) {
            for forma in &capa.formas {
                for (t, v) in total.iter_mut().zip(self.forma(forma, 1.0, &mover)) {
                    *t = (*t).max(v);
                }
            }
        }
        total
    }

    /// Dibuja una capa de la figura en lo que le toque.
    fn capa(&self, capa: &Capa, capas: &mut Capas, tapado: Tapado) {
        let cabeza = &self.cuadro.cabeza;
        let muelle = |p: Punto| if capa.muelle { self.con_muelle(p) } else { p };
        match capa.tipo {
            TipoCapa::Cuerpo => {
                for (i, forma) in capa.formas.iter().enumerate() {
                    let cubre = self.forma(forma, 1.0, &|p| muelle(cabeza.cuerpo(p)));
                    for (t, &v) in capas.cuerpo.iter_mut().zip(&cubre) {
                        *t = t.max(v);
                    }
                    // La primera da el color de base; las demás se pintan encima.
                    if i == 0 && matches!(capas.base, Base::Color(_)) && capas.pinturas.is_empty() {
                        capas.base = Base::Color(self.color(forma.color));
                    } else {
                        capas.pinturas.push((self.color(forma.color), cubre));
                    }
                }
            }
            TipoCapa::Pintado => {
                for forma in &capa.formas {
                    let visible = (cabeza.en_cara(forma.centro).1 / 0.12).clamp(0.0, 1.0);
                    if visible > 0.0 {
                        let cubre = self.forma(forma, visible, &|p| muelle(cabeza.en_cara(p).0));
                        capas.pinturas.push((self.color(forma.color), cubre));
                    }
                }
            }
            TipoCapa::Franja { arriba, encima, recorte } => {
                let fuera = if arriba { -FUERA_FRANJA } else { FUERA_FRANJA };
                let recorte = encima.then(|| recorte.map(|r| self.silueta(r, capa.muelle))).flatten();
                for forma in &capa.formas {
                    for borde in &forma.subtrazos {
                        let contorno: Vec<[f32; 2]> = franja(cabeza, &borde.puntos, fuera).into_iter().map(|p| (self.pantalla)(muelle(p))).collect();
                        let pintura = Pintura::Relleno { par_impar: false };
                        let mut cubre = a_mascara(cobertura(&poligono(&contorno), pintura, self.escala, self.lado), forma.alfa);
                        let color = self.color(forma.color);
                        match (&recorte, encima) {
                            (Some(recorte), _) => {
                                cubre.iter_mut().zip(recorte).for_each(|(v, &r)| *v = v.min(r));
                                capas.delante.push((color, cubre, false));
                            }
                            (None, true) => capas.delante.push((color, cubre, true)),
                            (None, false) => capas.pinturas.push((color, cubre)),
                        }
                    }
                }
            }
            TipoCapa::Ojo if !tapado.ojos => self.ojos(capas),
            TipoCapa::Boca if !tapado.boca => {
                if let Some(boca) = self.cuadro.boca {
                    let [a, b, c, d, e, f] = boca.matriz;
                    for forma in &capa.formas {
                        let cubre = self.forma(forma, boca.alfa, &|(x, y)| (a * x + c * y + e, b * x + d * y + f));
                        capas.pinturas.push((self.color(forma.color), cubre));
                    }
                }
            }
            TipoCapa::Cabeza(apendice) if !(apendice.tapable && tapado.cabeza) => self.apendice(capa, apendice, capas),
            _ => {}
        }
    }

    /// Los dos ojos: el derecho y su reflejo (o el izquierdo, si la figura lo tiene), estirados
    /// como diga el cuadro (sorpresa, parpadeo) y, contentos, en arco (con su forma alegre o
    /// recortados por abajo).
    fn ojos(&self, capas: &mut Capas) {
        let de = |tipo: TipoCapa| self.figura.capas().iter().find(|c| c.tipo == tipo);
        let (Some(derecho), izquierdo, alegre) = (de(TipoCapa::Ojo), de(TipoCapa::OjoIzquierdo), de(TipoCapa::OjoAlegre)) else { return };
        for ojo in &self.cuadro.ojos {
            let Ojo { lado, ancho, alto, arco, matriz: [a, b, c, d, e, f], alfa } = *ojo;
            let (normal, reflejo) = match izquierdo {
                Some(izquierdo) if lado < 0.0 => (izquierdo, 1.0),
                _ => (derecho, lado.signum()),
            };
            let (kx, ky) = (ancho / OJO_ANCHO, alto / OJO_ALTO);
            // `bajar` (unidades de radio, ya estirado) es para el recorte del arco.
            let colocar = |ensanchar: f32, bajar: f32, reflejo: f32| {
                move |(x, y): Punto| {
                    let (x, y) = (x * reflejo * kx * ensanchar, y * ky + bajar);
                    (a * x + c * y + e, b * x + d * y + f)
                }
            };
            let alegria = if alegre.is_some() { arco } else { 0.0 };
            for forma in &normal.formas {
                let mut cubre = self.forma(forma, alfa * (1.0 - alegria), &colocar(1.0, 0.0, reflejo));
                // Sin forma alegre, lo relleno se recorta con otra igual algo más ancha y más
                // baja: queda solo la parte de arriba, «∩».
                if alegre.is_none() && arco > 0.01 && matches!(forma.pintura, Pintura::Relleno { .. }) {
                    let corte = self.subtrazos(&forma.subtrazos, forma.pintura, 1.0, &colocar(1.3, alto * (1.0 - 0.5 * arco), reflejo));
                    cubre.iter_mut().zip(corte).for_each(|(v, q)| *v *= 1.0 - q.min(1.0));
                }
                capas.pinturas.push((self.color(forma.color), cubre));
            }
            if let Some(alegre) = alegre.filter(|_| arco > 0.01) {
                for forma in &alegre.formas {
                    let cubre = self.forma(forma, alfa * arco, &colocar(1.0, 0.0, lado.signum()));
                    capas.pinturas.push((self.color(forma.color), cubre));
                }
            }
        }
    }

    /// Lo que sale de la cabeza: detrás del cuerpo, sumado a él o delante (siempre, al girar
    /// hacia su lado o asomando al mirarlo).
    fn apendice(&self, capa: &Capa, apendice: Apendice, capas: &mut Capas) {
        let cabeza = &self.cuadro.cabeza;
        let ancla = apendice.ancla;
        let colocar = |p: Punto| {
            let q = match apendice.orientacion {
                Orientacion::Cabeza => cabeza.cartel(ancla, p),
                Orientacion::Cuerpo => cabeza.en_cuerpo(ancla, p),
            };
            if capa.muelle { self.con_muelle(q) } else { q }
        };
        if apendice.fundir {
            for forma in &capa.formas {
                let cubre = self.forma(forma, 1.0, &colocar);
                for (t, &v) in capas.cuerpo.iter_mut().zip(&cubre) {
                    *t = t.max(v);
                }
                capas.pinturas.push((self.color(forma.color), cubre));
            }
            return;
        }
        let delante = match apendice.delante {
            Delante::Siempre | Delante::Revelar => 1.0,
            Delante::Nunca => 0.0,
            Delante::Girar => al_frente(cabeza.profundidad(ancla)),
        };
        // Al asomar: de frente se ve lo de encima de su centro; mirándolo, entero.
        let corte = (apendice.delante == Delante::Revelar).then(|| {
            let largo = ancla[0].hypot(ancla[1]).max(1e-3);
            let visto = (cabeza.profundidad([ancla[0] / largo, ancla[1] / largo, 0.0]) / 0.3).clamp(0.0, 1.0);
            let ((_, arriba), (_, abajo)) = capa.caja;
            let centro = (arriba + abajo) / 2.0;
            centro + (abajo - centro) * visto
        });
        for forma in &capa.formas {
            let color = self.color(forma.color);
            if apendice.detras {
                capas.detras.push((color, self.forma(forma, 1.0, &colocar)));
            }
            if delante > 0.0 {
                let cubre = match corte {
                    Some(corte) => {
                        let relleno = matches!(forma.pintura, Pintura::Relleno { .. });
                        self.subtrazos(&por_encima(&forma.subtrazos, relleno, corte), forma.pintura, forma.alfa, &colocar)
                    }
                    None => self.forma(forma, delante, &colocar),
                };
                capas.delante.push((color, cubre, false));
            }
        }
    }

    /// El cuerpo de una figura de imágenes: el fotograma que toque, llevado a cada píxel.
    fn imagen(&self, imagenes: &Imagenes, capas: &mut Capas) {
        let Some((animacion, i)) = self.cuadro.fotograma else { return };
        let Some(fotograma) = imagenes.fotogramas(animacion).get(i) else { return };
        // La copia reducida en la que una unidad de radio mide a lo sumo el doble que en la
        // imagen que se dibuja.
        let (mut fotograma, mut unidad, mut centro) = (fotograma, imagenes.unidad, imagenes.centro);
        while unidad > self.escala * 2.0 {
            let Some(mitad) = &fotograma.mitad else { break };
            (fotograma, unidad, centro) = (mitad, unidad / 2.0, (centro.0 / 2.0, centro.1 / 2.0));
        }
        let mitad = self.lado as f32 / 2.0;
        let mut colores = vec![[0.0; 3]; (self.lado * self.lado) as usize];
        for (i, (cubre, color)) in capas.cuerpo.iter_mut().zip(colores.iter_mut()).enumerate() {
            let (px, py) = ((i as u32 % self.lado) as f32 + 0.5, (i as u32 / self.lado) as f32 + 0.5);
            let reposo = self.cuadro.cabeza.desde_cuerpo(((px - mitad) / self.escala, (py - mitad) / self.escala - BAJADA));
            let (x, y) = (centro.0 + reposo.0 * unidad, centro.1 + reposo.1 * unidad);
            let [r, g, b, a] = if imagenes.pixelado { muestra_nitida(fotograma, x, y) } else { muestra(fotograma, x - 0.5, y - 0.5) };
            *cubre = a / 255.0;
            if a > 0.0 {
                *color = [r * 255.0 / a, g * 255.0 / a, b * 255.0 / a];
            }
        }
        capas.base = Base::Imagen(colores);
    }
}

/// El píxel del fotograma en el que cae (x, y), tal cual: el pixel art se amplía sin suavizar.
fn muestra_nitida(f: &Fotograma, x: f32, y: f32) -> [f32; 4] {
    if x < 0.0 || y < 0.0 || x >= f.ancho as f32 || y >= f.alto as f32 {
        return [0.0; 4];
    }
    f.pixeles[(y as u32 * f.ancho + x as u32) as usize].map(f32::from)
}

/// Un píxel del fotograma en (x, y), mezclando los cuatro que lo rodean (premultiplicado; fuera,
/// transparente).
fn muestra(f: &Fotograma, x: f32, y: f32) -> [f32; 4] {
    let (x0, y0) = (x.floor(), y.floor());
    let (tx, ty) = (x - x0, y - y0);
    let pixel = |x: f32, y: f32| -> [f32; 4] {
        if x < 0.0 || y < 0.0 || x >= f.ancho as f32 || y >= f.alto as f32 {
            return [0.0; 4];
        }
        f.pixeles[(y as u32 * f.ancho + x as u32) as usize].map(f32::from)
    };
    let (a, b, c, d) = (pixel(x0, y0), pixel(x0 + 1.0, y0), pixel(x0, y0 + 1.0), pixel(x0 + 1.0, y0 + 1.0));
    std::array::from_fn(|i| (a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty) + (c[i] * (1.0 - tx) + d[i] * tx) * ty)
}

/// Lo que tapan los accesorios puestos.
#[derive(Clone, Copy)]
struct Tapado {
    cabeza: bool,
    ojos: bool,
    boca: bool,
}

/// Dibuja el cuadro en una imagen cuadrada de `lado` píxeles.
pub fn dibujar(cuadro: &Cuadro, lado: u32, aspecto: &Aspecto) -> Image {
    let figura = &*cuadro.cabeza.figura;
    let escala = lado as f32 * RADIO;
    let centro = lado as f32 / 2.0;
    let pantalla = |(x, y): Punto| [centro + x * escala, centro + (y + BAJADA) * escala];
    let pintor = Pintor { cuadro, figura, aspecto, lado, escala, pantalla: &pantalla };
    let n = (lado * lado) as usize;
    let mut capas = Capas { detras: Vec::new(), cuerpo: vec![0.0; n], base: Base::Color(aspecto.cuerpo), pinturas: Vec::new(), delante: Vec::new() };
    let tapado = Tapado {
        cabeza: aspecto.accesorios.iter().any(|a| a.tipo.tapa_la_cabeza()),
        ojos: aspecto.accesorios.iter().any(|a| a.tipo.tapa_los_ojos()),
        boca: aspecto.accesorios.iter().any(|a| a.tipo.tapa_la_boca()),
    };
    match &figura.dibujo {
        Dibujo::Vectorial(lista) => lista.iter().for_each(|capa| pintor.capa(capa, &mut capas, tapado)),
        Dibujo::Imagenes(imagenes) => pintor.imagen(imagenes, &mut capas),
    }

    // Accesorios, de abajo arriba: cada pieza va delante del cuerpo con su opacidad (recortada
    // por él si va pintada en la cara) y, si le toca, también detrás.
    let mut puestos: Vec<&Accesorio> = aspecto.accesorios.iter().collect();
    puestos.sort_by_key(|a| a.tipo.orden());
    for accesorio in puestos {
        for pieza in accesorios::piezas(accesorio, &cuadro.cabeza, &pantalla, escala) {
            if pieza.delante <= 0.0 && !pieza.detras {
                continue;
            }
            let pintura = match pieza.estilo {
                Estilo::Relleno => Pintura::Relleno { par_impar: false },
                Estilo::ConAgujeros => Pintura::Relleno { par_impar: true },
                Estilo::Trazo(grosor) => Pintura::Linea { grosor: grosor / escala, extremo: Extremo::Redondo, union: Union::Redonda },
            };
            let cubre = cobertura(&pieza.trazo, pintura, escala, lado);
            if pieza.detras {
                capas.detras.push((pieza.color, a_mascara(cubre.clone(), 1.0)));
            }
            if pieza.delante > 0.0 {
                capas.delante.push((pieza.color, a_mascara(cubre, pieza.delante), pieza.recortar));
            }
        }
    }

    // Pastilla de aviso (del color elegido, el de la figura o el de los rasgos) y el hueco que
    // abre en lo que hay debajo, en píxeles.
    let color_aviso = aspecto.aviso.or(figura.colores.aviso).unwrap_or(aspecto.rasgos);
    let pastilla = cuadro.pastilla.map(|p| {
        let [x, y] = pantalla((p.x, p.y));
        (x, y, p.radio * escala, p.muesca * escala)
    });

    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(lado, lado);
    for (i, pixel) in buffer.make_mut_slice().iter_mut().enumerate() {
        // Premultiplicado: cada canal ya va multiplicado por la opacidad. De abajo arriba: lo de
        // detrás, el cuerpo con lo pintado en él, lo de delante y la pastilla.
        let mut color: Pixel = [0.0; 4];
        for (tono, cubre) in &capas.detras {
            encima(&mut color, *tono, cubre[i]);
        }
        let b = capas.cuerpo[i];
        if b > 0.0 {
            let mut base = match &capas.base {
                Base::Color(c) => c.map(f32::from),
                Base::Imagen(colores) => colores[i],
            };
            for (tono, cubre) in &capas.pinturas {
                let k = cubre[i].min(1.0);
                for c in 0..3 {
                    base[c] += (tono[c] as f32 - base[c]) * k;
                }
            }
            for c in 0..3 {
                color[c] = base[c] * b + color[c] * (1.0 - b);
            }
            color[3] = 255.0 * b + color[3] * (1.0 - b);
        }
        for (tono, cubre, recortar) in &capas.delante {
            encima(&mut color, *tono, if *recortar { cubre[i].min(b) } else { cubre[i] });
        }
        if let Some((x, y, radio, muesca)) = pastilla {
            let distancia = ((i as u32 % lado) as f32 + 0.5 - x).hypot((i as u32 / lado) as f32 + 0.5 - y);
            let hueco = 1.0 - circulo(distancia, muesca);
            color = color.map(|c| c * hueco);
            encima(&mut color, color_aviso, circulo(distancia, radio));
        }
        let [r, g, b, a] = color.map(|c| c.round().clamp(0.0, 255.0) as u8);
        *pixel = Rgba8Pixel { r, g, b, a };
    }
    Image::from_rgba8_premultiplied(buffer)
}

/// Cuánto cubre un círculo de `radio` píxeles a un píxel cuyo centro está a `distancia` del suyo
/// (con el borde suavizado).
fn circulo(distancia: f32, radio: f32) -> f32 {
    (radio - distancia + 0.5).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::super::estados::Estado;
    use super::super::motor::{Mirar, Motor};
    use super::*;

    const VERDE: [u8; 3] = [46, 139, 87];
    const ARANDANO: [u8; 3] = [143, 162, 201];

    fn aspecto(cuerpo: [u8; 3], rasgos: [u8; 3]) -> Aspecto {
        Aspecto { cuerpo, rasgos, aviso: None, accesorios: Vec::new() }
    }

    fn pixeles(imagen: &Image) -> Vec<Rgba8Pixel> {
        imagen.to_rgba8_premultiplied().expect("imagen de píxeles").as_slice().to_vec()
    }

    #[test]
    fn dibuja_el_cuerpo_los_rasgos_y_el_colmillo_dentro() {
        let lado = 128;
        let cuadro = Motor::new().muestrear(0.5);
        let pixeles = pixeles(&dibujar(&cuadro, lado, &aspecto(VERDE, [18, 32, 58])));
        let en = |x: u32, y: u32| pixeles[(y * lado + x) as usize];
        // Las esquinas quedan transparentes.
        assert_eq!(en(0, 0).a, 0);
        assert_eq!(en(lado - 1, lado - 1).a, 0);
        // Hay píxeles del color del cuerpo, de los rasgos y del colmillo.
        assert!(pixeles.iter().any(|p| p.a == 255 && (p.r, p.g, p.b) == (46, 139, 87)));
        assert!(pixeles.iter().any(|p| p.a == 255 && (p.r, p.g, p.b) == (18, 32, 58)));
        assert!(pixeles.iter().any(|p| p.a == 255 && p.r > 200 && p.g > 200 && p.b > 200));
        // El rulo asoma por encima del cuerpo: hay cuerpo en la columna central muy arriba.
        let arriba = (0..lado / 4).any(|y| (lado / 2..lado / 2 + lado / 8).any(|x| en(x, y).a == 255));
        assert!(arriba, "no se ve el rulo");
    }

    #[test]
    fn con_gafas_de_sol_no_se_dibujan_los_ojos() {
        use super::super::accesorios::Tipo;
        let lado = 128;
        let cuadro = Motor::new().muestrear(0.5);
        // Ojos de un color que no usa nada más: con las gafas (negras) no queda ningún píxel así.
        let ojos = [1, 2, 250];
        let con = |accesorios: Vec<Accesorio>| {
            let imagen = dibujar(&cuadro, lado, &Aspecto { cuerpo: VERDE, rasgos: ojos, aviso: None, accesorios });
            pixeles(&imagen).iter().filter(|p| p.a == 255 && [p.r, p.g, p.b] == ojos).count()
        };
        let gafas = Accesorio { tipo: Tipo::GafasDeSol, color: [34, 37, 45] };
        let sin_gafas = con(Vec::new());
        let con_gafas = con(vec![gafas]);
        assert!(sin_gafas > 0);
        // Solo queda la sonrisa.
        assert!(con_gafas < sin_gafas / 3, "{con_gafas} de {sin_gafas}");
    }

    #[test]
    fn con_gorro_no_se_dibuja_el_rulo() {
        use super::super::accesorios::Tipo;
        let lado = 128;
        let cuadro = Motor::new().muestrear(0.5);
        let gorro = Accesorio { tipo: Tipo::GorroDeLana, color: [1, 2, 250] };
        let imagen = dibujar(&cuadro, lado, &Aspecto { accesorios: vec![gorro], ..aspecto(VERDE, [18, 32, 58]) });
        // Por encima del gorro (que llega a lo alto del rulo) no asoma nada del color del cuerpo.
        let verdes_arriba = pixeles(&imagen)[..(lado * lado / 5) as usize].iter().filter(|p| p.a == 255 && [p.r, p.g, p.b] == VERDE).count();
        assert_eq!(verdes_arriba, 0);
    }

    #[test]
    fn la_pastilla_de_aviso_es_del_color_elegido_o_del_de_los_ojos() {
        let lado = 128;
        let mut motor = Motor::new();
        motor.poner_estado(Estado::Aviso, 0.0);
        let cuadro = motor.muestrear(1.0);
        assert!(cuadro.pastilla.is_some());
        // Colores que no usa nada más, para contar solo la pastilla (los ojos, aparte, son del
        // color de los rasgos).
        let (rasgos, rojo) = ([1, 2, 250], [250, 3, 2]);
        let con = |aviso: Option<[u8; 3]>, color: [u8; 3]| {
            let imagen = dibujar(&cuadro, lado, &Aspecto { aviso, ..aspecto(VERDE, rasgos) });
            pixeles(&imagen).iter().filter(|p| p.a == 255 && [p.r, p.g, p.b] == color).count()
        };
        // Con color elegido, la pastilla (un círculo grande) sale de él y no de los rasgos.
        assert!(con(Some(rojo), rojo) > 100);
        // Sin elegirlo, sale del color de los rasgos: hay muchos más píxeles que solo con los ojos.
        assert_eq!(con(None, rojo), 0);
        assert!(con(None, rasgos) > con(Some(rojo), rasgos) + 100);
    }

    #[test]
    fn el_limon_tiene_sus_colores_y_no_los_del_tema() {
        let lado = 128;
        let mut motor = Motor::new();
        motor.poner_figura(Figura::limon());
        let pixeles = pixeles(&dibujar(&motor.muestrear(0.5), lado, &aspecto(VERDE, [1, 2, 250])));
        let de = |color: [u8; 3]| pixeles.iter().filter(|p| p.a == 255 && [p.r, p.g, p.b] == color).count();
        assert_eq!(de(VERDE), 0);
        assert!(de([255, 247, 214]) > 100, "crema");
        assert!(de([247, 232, 104]) > 100, "amarillo");
        assert!(de([107, 75, 46]) > 5, "rasgos");
    }

    #[test]
    fn el_arandano_del_limon_se_ve_tambien_mirando_hacia_abajo() {
        let lado = 128;
        let arandanos = |cabeceo: f32| {
            let mut motor = Motor::new();
            motor.poner_figura(Figura::limon());
            motor.poner_mirada(Mirar { guinada: 0.0, cabeceo, mezcla: 1.0, deriva: 0.0 }, 0.0, 0.001);
            let imagen = dibujar(&motor.muestrear(1.0), lado, &aspecto([0; 3], [0; 3]));
            pixeles(&imagen).iter().filter(|p| p.a == 255 && [p.r, p.g, p.b] == ARANDANO).count()
        };
        let de_frente = arandanos(0.0);
        assert!(de_frente > 10, "de frente: {de_frente}");
        // Mirando hacia abajo se ve entero, encima del pelo: más que de frente.
        let abajo = arandanos(-26.0);
        assert!(abajo > de_frente, "mirando abajo: {abajo} (de frente {de_frente})");
    }

    #[test]
    fn al_saltar_nada_se_sale_de_la_imagen() {
        // Con el salto de un aviso y con el rebote de un clic, el borde de la imagen queda
        // transparente con las dos figuras.
        let lado = 96;
        for figura in Figura::integradas() {
            let mut aviso = Motor::new();
            aviso.poner_estado(Estado::Aviso, 0.0);
            let mut clic = Motor::new();
            clic.rebotar(0.0);
            for mut motor in [aviso, clic] {
                motor.poner_figura(figura.clone());
                for i in (0..300).step_by(5) {
                    let p = pixeles(&dibujar(&motor.muestrear(i as f64 / 100.0), lado, &aspecto(VERDE, [0; 3])));
                    let borde = (0..lado).flat_map(|k| [(k, 0), (k, lado - 1), (0, k), (lado - 1, k)]);
                    let fuera = borde.filter(|&(x, y)| p[(y * lado + x) as usize].a > 0).count();
                    assert_eq!(fuera, 0, "{} se sale a los {i} cs", figura.id);
                }
            }
        }
    }

    #[test]
    fn recorta_por_encima_de_una_altura() {
        let cuadrado = vec![Subtrazo { puntos: vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], cerrado: true }];
        let mitad = por_encima(&cuadrado, true, 0.5);
        assert_eq!(mitad.len(), 1);
        assert!(mitad[0].puntos.iter().all(|p| p.1 <= 0.5 + 1e-6));
        let linea = vec![Subtrazo { puntos: vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)], cerrado: false }];
        let tramos = por_encima(&linea, false, 0.5);
        assert_eq!(tramos.len(), 2);
    }

    #[test]
    fn dibuja_una_figura_de_imagenes() {
        // Un círculo rojo de 64 px como única imagen: se ve rojo y sin seguir al cursor.
        let mut png = Vec::new();
        {
            let mut codificador = png::Encoder::new(&mut png, 64, 64);
            codificador.set_color(png::ColorType::Rgba);
            let mut escritor = codificador.write_header().unwrap();
            let datos: Vec<u8> = (0..64 * 64)
                .flat_map(|i| {
                    let (x, y) = ((i % 64) as f32 - 31.5, (i / 64) as f32 - 31.5);
                    if x.hypot(y) < 30.0 { [220, 20, 20, 255] } else { [0, 0, 0, 0] }
                })
                .collect();
            escritor.write_image_data(&datos).unwrap();
        }
        let json = r#"{"imagenes": {"reposo": ["a.png"]}}"#;
        let figura = Figura::desde("x".into(), Some(json), None, &|_| Ok(png.clone())).unwrap();
        let mut motor = Motor::new();
        motor.poner_figura(std::sync::Arc::new(figura));
        motor.poner_mirada(Mirar { guinada: 30.0, cabeceo: 0.0, mezcla: 1.0, deriva: 0.0 }, 0.0, 0.001);
        let cuadro = motor.muestrear(1.0);
        assert!(cuadro.ojos.is_empty() && cuadro.fotograma.is_some());
        let lado = 100;
        let p = pixeles(&dibujar(&cuadro, lado, &aspecto(VERDE, [0; 3])));
        let rojos = p.iter().filter(|p| p.a == 255 && p.r > 200 && p.g < 40).count();
        assert!(rojos > 1000, "{rojos}");
        // De frente: tanto rojo a la izquierda como a la derecha del centro.
        let mitad = |izquierda: bool| {
            p.iter().enumerate().filter(|(i, q)| q.a > 128 && ((*i as u32 % lado) < lado / 2) == izquierda).count() as i32
        };
        assert!((mitad(true) - mitad(false)).abs() < 150, "{} / {}", mitad(true), mitad(false));
    }
}
