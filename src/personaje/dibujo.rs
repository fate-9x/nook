//! Dibujo de un cuadro del personaje en una imagen, con la figura que tenga (ver `figura.rs`): lo
//! que va detrás del cuerpo, el cuerpo con lo pintado en él (manchas, franjas, ojos y boca),
//! recortado por él, y encima lo que va delante (el pelo, lo que sale de la cabeza y los
//! accesorios) y la pastilla de aviso. En las figuras de imágenes, el cuerpo es el fotograma que
//! toque, girado, estirado y movido como el cuerpo. Las formas se rasterizan con `zeno`, el mismo
//! que usa Slint para sus trazados, con los bordes suavizados.

use super::accesorios::{self, Estilo, Puesto};
use super::estados::{OJO_ALTO, OJO_ANCHO};
use super::figura::{Apendice, Capa, Delante, Dibujo, EstiloParpado, Figura, Forma, Fotograma, Hacia, Imagenes, Orientacion, TipoCapa};
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
    pub accesorios: Vec<Puesto>,
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
/// doblaría sobre sí misma), sigue a lo largo hasta fuera del cuerpo por sus dos extremos y se
/// cierra por fuera, hacia `hacia`, como un cartel. Luego se recorta por el cuerpo. Una franja
/// hacia arriba o abajo tiene el borde de lado a lado; una hacia un lado, de arriba abajo.
fn franja(cabeza: &Cabeza, borde: &[Punto], hacia: Hacia) -> Vec<Punto> {
    // `a` lleva una posición a lo largo del borde y otra a través de él a (x, y), y `fuera` es
    // hasta dónde se rellena a través de él.
    let (a, fuera): (fn(f32, f32) -> Punto, f32) = match hacia {
        Hacia::Arriba => (|l, t| (l, t), -FUERA_FRANJA),
        Hacia::Abajo => (|l, t| (l, t), FUERA_FRANJA),
        Hacia::Izquierda => (|l, t| (t, l), -FUERA_FRANJA),
        Hacia::Derecha => (|l, t| (t, l), FUERA_FRANJA),
    };
    let a_lo_largo = |p: &Punto| if matches!(hacia, Hacia::Arriba | Hacia::Abajo) { p.0 } else { p.1 };
    // De un extremo al otro (de izquierda a derecha, o de arriba abajo), lo dibuje como lo dibuje.
    let al_reves = borde.first().zip(borde.last()).is_some_and(|(p, q)| a_lo_largo(p) > a_lo_largo(q));
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
    let alargar = |p: Punto, d: Punto| {
        let q = cabeza.cartel([0.0; 3], d);
        (p.0 + q.0 - centro.0, p.1 + q.1 - centro.1)
    };
    let mut contorno = vec![alargar(primero, a(-1.0, 0.0))];
    contorno.extend(puntos);
    contorno.push(alargar(ultimo, a(1.0, 0.0)));
    contorno.push(cabeza.cartel([0.0; 3], a(FUERA_FRANJA, fuera)));
    contorno.push(cabeza.cartel([0.0; 3], a(-FUERA_FRANJA, fuera)));
    contorno
}

/// El borde de una franja pintado en la cara (unidades de radio), en los tramos que se ven (lo que
/// queda al otro lado de la cabeza no se dibuja).
fn borde_visible(cabeza: &Cabeza, borde: &[Punto]) -> Vec<Subtrazo> {
    let mut tramos = Vec::new();
    let mut actual = Vec::new();
    for &p in borde {
        let (q, profundidad) = cabeza.en_cara(p);
        if profundidad > 0.01 {
            actual.push(q);
        } else if actual.len() >= 2 {
            tramos.push(Subtrazo { puntos: std::mem::take(&mut actual), cerrado: false });
        } else {
            actual.clear();
        }
    }
    if actual.len() >= 2 {
        tramos.push(Subtrazo { puntos: actual, cerrado: false });
    }
    tramos
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
            TipoCapa::Franja { hacia, encima, recorte } => {
                let recorte = encima.then(|| recorte.map(|r| self.silueta(r, capa.muelle))).flatten();
                // Lo relleno del trazado es la franja; su línea, el borde dibujado encima.
                for forma in &capa.formas {
                    for borde in &forma.subtrazos {
                        let mut cubre = match forma.pintura {
                            Pintura::Relleno { .. } => {
                                let contorno: Vec<[f32; 2]> =
                                    franja(cabeza, &borde.puntos, hacia).into_iter().map(|p| (self.pantalla)(muelle(p))).collect();
                                let pintura = Pintura::Relleno { par_impar: false };
                                a_mascara(cobertura(&poligono(&contorno), pintura, self.escala, self.lado), forma.alfa)
                            }
                            Pintura::Linea { .. } => {
                                let tramos = borde_visible(cabeza, &borde.puntos);
                                self.subtrazos(&tramos, forma.pintura, forma.alfa, &|p| muelle(p))
                            }
                        };
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
            let Ojo { lado, ancho, alto, arco, matriz: [a, b, c, d, e, f], alfa, pupila, parpado } = *ojo;
            let (normal, reflejo) = match izquierdo {
                Some(izquierdo) if lado < 0.0 => (izquierdo, 1.0),
                _ => (derecho, lado.signum()),
            };
            let (kx, ky) = (ancho / OJO_ANCHO, alto / OJO_ALTO);
            // Con párpados, dónde queda su borde en reposo (el de la expresión puesta, sin el
            // parpadeo, más medio grosor de su línea) y el fondo del ojo: las pupilas no suben de él.
            let alto_de = |f: &Forma| f.subtrazos.iter().flat_map(|s| &s.puntos).fold((f32::MAX, f32::MIN), |(a, b), &(_, y)| (a.min(y), b.max(y)));
            let (y0, y1) = normal.formas.iter().filter(|f| f.pupila.is_none()).map(alto_de).fold((f32::MAX, f32::MIN), |(a, b), (c, d)| (a.min(c), b.max(d)));
            let linea = normal.formas.iter().find_map(|f| match (f.pupila, f.pintura) {
                (None, Pintura::Linea { grosor, .. }) => Some(grosor / 2.0),
                _ => None,
            });
            let parpado_en_reposo = self.figura.parpado.filter(|_| y0 < y1).map(|cierre| (y0 + (y1 - y0) * cierre + linea.unwrap_or(0.0), y1));
            // `bajar` (unidades de radio, ya estirado) es para el recorte del arco; las pupilas se
            // apartan además hacia donde mira, después del reflejo (las dos miran al mismo lado; en
            // el ojo reflejado, su recorrido a cada lado también se cambia de lado).
            let colocar = |ensanchar: f32, bajar: f32, reflejo: f32, forma: &Forma| {
                let (px, py) = forma.pupila.map_or((0.0, 0.0), |[izquierda, arriba, derecha, abajo]| {
                    let (izquierda, derecha) = if reflejo < 0.0 { (derecha, izquierda) } else { (izquierda, derecha) };
                    let x = pupila.0 * if pupila.0 < 0.0 { izquierda } else { derecha };
                    let mut y = pupila.1 * if pupila.1 < 0.0 { arriba } else { abajo };
                    if let Some((borde, fondo)) = parpado_en_reposo {
                        let (encima, debajo) = alto_de(forma);
                        y = y.max(borde - encima).min(fondo - debajo);
                    }
                    (x * kx, y * ky)
                });
                move |(x, y): Punto| {
                    let (x, y) = (x * reflejo * kx * ensanchar + px, y * ky + bajar + py);
                    (a * x + c * y + e, b * x + d * y + f)
                }
            };
            let alegria = if alegre.is_some() { arco } else { 0.0 };
            // Si el párpado quita parte del contorno del ojo, el contorno lo dibuja el párpado.
            let abierto = parpado > 0.001 && self.figura.estilo_parpado != EstiloParpado::Cerrado;
            for forma in &normal.formas {
                if abierto && forma.pupila.is_none() && matches!(forma.pintura, Pintura::Linea { .. }) {
                    continue;
                }
                let mut cubre = self.forma(forma, alfa * (1.0 - alegria), &colocar(1.0, 0.0, reflejo, forma));
                // Sin forma alegre, lo relleno se recorta con otra igual algo más ancha y más
                // baja: queda solo la parte de arriba, «∩».
                if alegre.is_none() && arco > 0.01 && matches!(forma.pintura, Pintura::Relleno { .. }) {
                    let corte = self.subtrazos(&forma.subtrazos, forma.pintura, 1.0, &colocar(1.3, alto * (1.0 - 0.5 * arco), reflejo, forma));
                    cubre.iter_mut().zip(corte).for_each(|(v, q)| *v *= 1.0 - q.min(1.0));
                }
                capas.pinturas.push((self.color(forma.color), cubre));
            }
            if let Some(fija) = normal.formas.iter().find(|f| f.pupila.is_none()).filter(|_| parpado > 0.001) {
                self.parpado(normal, parpado, alfa * (1.0 - alegria), &colocar(1.0, 0.0, reflejo, fija), capas);
            }
            if let Some(alegre) = alegre.filter(|_| arco > 0.01) {
                for forma in &alegre.formas {
                    let cubre = self.forma(forma, alfa * arco, &colocar(1.0, 0.0, lado.signum(), forma));
                    capas.pinturas.push((self.color(forma.color), cubre));
                }
            }
        }
    }

    /// El párpado de un ojo: tapa su parte de arriba (`cierre`, 0–1, de su alto) con el color del
    /// cuerpo, recortado por la forma del ojo, y lleva una línea en su borde del color y el grosor
    /// del contorno del ojo, si lo tiene. `colocar` lleva el ojo a la pantalla.
    fn parpado(&self, ojo: &Capa, cierre: f32, alfa: f32, colocar: &dyn Fn(Punto) -> Punto, capas: &mut Capas) {
        let fijas: Vec<&Forma> = ojo.formas.iter().filter(|f| f.pupila.is_none()).collect();
        let puntos = || fijas.iter().flat_map(|f| f.subtrazos.iter().flat_map(|s| s.puntos.iter().copied()));
        let (x0, x1) = puntos().fold((f32::MAX, f32::MIN), |(a, b), (x, _)| (a.min(x), b.max(x)));
        let (y0, y1) = puntos().fold((f32::MAX, f32::MIN), |(a, b), (_, y)| (a.min(y), b.max(y)));
        if x0 > x1 {
            return;
        }
        // Hasta dónde llega el párpado: lo relleno del ojo (o todo, si solo tiene líneas); si el
        // estilo quita parte del contorno, también el contorno, que el párpado tapa por encima.
        let estilo = self.figura.estilo_parpado;
        let quita_contorno = estilo != EstiloParpado::Cerrado;
        let rellenas: Vec<&&Forma> = fijas.iter().filter(|f| matches!(f.pintura, Pintura::Relleno { .. })).collect();
        let dentro: Vec<&&Forma> = if rellenas.is_empty() || quita_contorno { fijas.iter().collect() } else { rellenas };
        let mut ojo_entero: Mascara = vec![0.0; (self.lado * self.lado) as usize];
        // El borde suavizado del ojo, tapado entero: si no, asoma como un arco tenue.
        for f in &dentro {
            for (t, v) in ojo_entero.iter_mut().zip(self.subtrazos(&f.subtrazos, f.pintura, 1.0, colocar)) {
                *t = t.max((v * 4.0).min(1.0));
            }
        }
        let borde = y0 + (y1 - y0) * cierre;
        let margen = (x1 - x0) * 0.1;
        let rectangulo = |arriba: f32, abajo: f32| Subtrazo { puntos: vec![(x0 - margen, arriba), (x1 + margen, arriba), (x1 + margen, abajo), (x0 - margen, abajo)], cerrado: true };
        let relleno = Pintura::Relleno { par_impar: false };
        let mut cubre = self.subtrazos(&[rectangulo(y0 - margen, borde)], relleno, alfa, colocar);
        cubre.iter_mut().zip(&ojo_entero).for_each(|(v, &o)| *v = v.min(o));
        let piel = self.figura.capas().iter().find(|c| c.tipo == TipoCapa::Cuerpo).and_then(|c| c.formas.first()).map_or([255; 3], |f| f.color);
        capas.pinturas.push((self.color(piel), cubre));
        // Su línea, del color y el grosor del contorno del ojo, y el contorno otra vez encima
        // (cerrado, entero; cortado, solo por debajo de la línea del párpado; abierto, hasta cerca
        // de lo alto del ojo).
        let lineas: Vec<&&Forma> = fijas.iter().filter(|f| matches!(f.pintura, Pintura::Linea { .. })).collect();
        if let Some(contorno) = lineas.first() {
            let linea = Subtrazo { puntos: vec![(x0 - margen, borde), (x1 + margen, borde)], cerrado: false };
            let mut cubre = self.subtrazos(&[linea], contorno.pintura, alfa, colocar);
            cubre.iter_mut().zip(&ojo_entero).for_each(|(v, &o)| *v = v.min(o));
            capas.pinturas.push((self.color(contorno.color), cubre));
        }
        let corte = match estilo {
            EstiloParpado::Cerrado => None,
            EstiloParpado::Cortado => Some(borde),
            EstiloParpado::Abierto => Some(y0 + (y1 - y0) * ABIERTO.min(cierre)),
        };
        let debajo = corte.map(|corte| self.subtrazos(&[rectangulo(corte, y1 + margen)], relleno, 1.0, colocar));
        for f in lineas {
            let mut cubre = self.subtrazos(&f.subtrazos, f.pintura, alfa * f.alfa, colocar);
            if let Some(debajo) = &debajo {
                cubre.iter_mut().zip(debajo).for_each(|(v, &d)| *v = v.min(d));
            }
            capas.pinturas.push((self.color(f.color), cubre));
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
        let delante = match apendice.delante {
            Delante::Siempre | Delante::Revelar => 1.0,
            Delante::Nunca => 0.0,
            // Lo fundido (una oreja) pasa detrás en cuanto la cabeza gira hacia su lado, y casi de
            // golpe: así el contorno de la cabeza pasa por delante de su base, y a medias se vería
            // transparente sobre la cabeza.
            Delante::Girar if apendice.fundir => {
                let k = ((cabeza.profundidad(ancla) + 0.08) / 0.04).clamp(0.0, 1.0);
                k * k * (3.0 - 2.0 * k)
            }
            Delante::Girar => al_frente(cabeza.profundidad(ancla)),
        };
        // Fundido, se suma al cuerpo y se pinta sobre él (también sobre su contorno) mientras se
        // ve; si va también detrás, lo que no se ve de él queda detrás del cuerpo.
        if apendice.fundir {
            for forma in &capa.formas {
                if apendice.detras {
                    capas.detras.push((self.color(forma.color), self.forma(forma, 1.0, &colocar)));
                }
                if delante > 0.0 {
                    let cubre = self.forma(forma, delante, &colocar);
                    for (t, &v) in capas.cuerpo.iter_mut().zip(&cubre) {
                        *t = t.max(v);
                    }
                    capas.pinturas.push((self.color(forma.color), cubre));
                }
            }
            return;
        }
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

/// Con párpados abiertos, a qué altura del ojo (desde arriba, 0–1) se corta su contorno.
const ABIERTO: f32 = 0.18;

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
        cabeza: aspecto.accesorios.iter().any(Puesto::tapa_la_cabeza),
        ojos: aspecto.accesorios.iter().any(Puesto::tapa_los_ojos),
        boca: aspecto.accesorios.iter().any(Puesto::tapa_la_boca),
    };
    match &figura.dibujo {
        Dibujo::Vectorial(lista) => lista.iter().for_each(|capa| pintor.capa(capa, &mut capas, tapado)),
        Dibujo::Imagenes(imagenes) => pintor.imagen(imagenes, &mut capas),
    }

    // Accesorios, de abajo arriba: cada pieza va delante del cuerpo con su opacidad (recortada
    // por él si va pintada en la cara) y, si le toca, también detrás.
    let mut puestos: Vec<&Puesto> = aspecto.accesorios.iter().collect();
    puestos.sort_by_key(|a| a.orden());
    for accesorio in puestos {
        for pieza in accesorios::piezas_de(accesorio, &cuadro.cabeza, &pantalla, escala) {
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
                capas.detras.push((pieza.color, a_mascara(cubre.clone(), pieza.alfa)));
            }
            if pieza.delante > 0.0 {
                capas.delante.push((pieza.color, a_mascara(cubre, pieza.delante * pieza.alfa), pieza.recortar));
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
        use super::super::accesorios::{Accesorio, Tipo};
        let lado = 128;
        let cuadro = Motor::new().muestrear(0.5);
        // Ojos de un color que no usa nada más: con las gafas (negras) no queda ningún píxel así.
        let ojos = [1, 2, 250];
        let con = |accesorios: Vec<Puesto>| {
            let imagen = dibujar(&cuadro, lado, &Aspecto { cuerpo: VERDE, rasgos: ojos, aviso: None, accesorios });
            pixeles(&imagen).iter().filter(|p| p.a == 255 && [p.r, p.g, p.b] == ojos).count()
        };
        let gafas = Accesorio { tipo: Tipo::GafasDeSol, color: [34, 37, 45] }.into();
        let sin_gafas = con(Vec::new());
        let con_gafas = con(vec![gafas]);
        assert!(sin_gafas > 0);
        // Solo queda la sonrisa.
        assert!(con_gafas < sin_gafas / 3, "{con_gafas} de {sin_gafas}");
    }

    #[test]
    fn con_gorro_no_se_dibuja_el_rulo() {
        use super::super::accesorios::{Accesorio, Tipo};
        let lado = 128;
        let cuadro = Motor::new().muestrear(0.5);
        let gorro = Accesorio { tipo: Tipo::GorroDeLana, color: [1, 2, 250] }.into();
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
    fn una_franja_de_un_lado_cubre_la_parte_de_atras_de_la_cabeza_al_girar() {
        // Una cabeza redonda con una franja a la izquierda, con el borde pegado al de la cabeza.
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-60 -60 120 120">
            <circle id="cuerpo" r="40" fill="#e0d0c0"/>
            <path id="nuca" d="M -36 -18 L -39 0 L -36 18" fill="#1020f0" stroke="#000000" stroke-width="2"/>
        </svg>"##;
        let json = r#"{"capas": [{"id": "cuerpo"}, {"id": "nuca", "tipo": "franja", "hacia": "izquierda"}]}"#;
        let figura = std::sync::Arc::new(Figura::desde("x".into(), Some(json), Some(svg), &|_| Err("no".into())).unwrap());
        let de_color = |guinada: f32, color: [u8; 3]| {
            let mut motor = Motor::new();
            motor.poner_figura(figura.clone());
            motor.poner_mirada(Mirar { guinada, cabeceo: 0.0, mezcla: 1.0, deriva: 0.0 }, 0.0, 0.001);
            let imagen = dibujar(&motor.muestrear(1.0), 128, &aspecto([1, 2, 3], [1, 2, 3]));
            pixeles(&imagen).iter().filter(|p| p.a == 255 && [p.r, p.g, p.b] == color).count()
        };
        let azules = |guinada: f32| de_color(guinada, [16, 32, 240]);
        let (de_frente, a_la_derecha, a_la_izquierda) = (azules(0.0), azules(35.0), azules(-35.0));
        // Su línea es el borde, dibujado encima: al girar, se ve dentro de la cabeza.
        assert!(de_color(35.0, [0, 0, 0]) > 20, "no se ve el borde");
        assert!(a_la_derecha > de_frente * 4 + 200, "de frente {de_frente}, a la derecha {a_la_derecha}");
        assert!(a_la_izquierda <= de_frente, "a la izquierda {a_la_izquierda}, de frente {de_frente}");
    }

    #[test]
    fn con_el_parpado_bajado_la_pupila_no_se_esconde_al_mirar_arriba() {
        // Un párpado que tapa el 60 % del ojo: la pupila, en el centro, quedaría debajo.
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-60 -60 120 120">
            <circle id="cuerpo" r="40" fill="#e0d0c0"/>
            <g id="ojo"><ellipse cx="15" cy="-4" rx="12" ry="10" fill="#ffffff" stroke="#000000" stroke-width="2"/><circle id="pupila" cx="15" cy="-4" r="1.6" fill="#f01010"/></g>
        </svg>"##;
        let json = r#"{"parpado": 0.6, "capas": [{"id": "cuerpo"}, {"id": "ojo"}]}"#;
        let figura = std::sync::Arc::new(Figura::desde("x".into(), Some(json), Some(svg), &|_| Err("no".into())).unwrap());
        let mut motor = Motor::new();
        motor.poner_figura(figura);
        motor.poner_mirada(Mirar { guinada: 0.0, cabeceo: 26.0, mezcla: 1.0, deriva: 0.0 }, 0.0, 0.001);
        // En algún momento sin parpadear se ven sus píxeles rojos.
        let rojos = (1..20)
            .map(|i| {
                let imagen = dibujar(&motor.muestrear(i as f64 * 0.25), 128, &aspecto([1, 2, 3], [1, 2, 3]));
                pixeles(&imagen).iter().filter(|p| p.a == 255 && p.r > 200 && p.g < 60 && p.b < 60).count()
            })
            .max()
            .unwrap();
        assert!(rojos > 2, "la pupila queda bajo el párpado ({rojos} píxeles)");
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

