//! Dibujo de un cuadro del personaje en una imagen: el cuerpo de un color, lo pintado sobre él
//! (las capas y los coloretes del Limón), los rasgos (ojos y boca) de otro y el colmillo blanco,
//! recortados por el cuerpo (si un ojo llega al borde, no se sale), lo que sale de la cabeza (el
//! rulo de Nook, el arándano del Limón) y los accesorios que lleve, detrás o delante del
//! cuerpo. Las formas se rasterizan con `zeno`, el mismo que usa Slint para sus trazados, con los
//! bordes suavizados.

use super::accesorios::{self, Accesorio, Estilo};
use super::estados::{OJO_ALTO, OJO_ANCHO};
use super::forma::{self, Punto};
use super::motor::{COLMILLO, Cabeza, Cuadro, Figura, GROSOR_RULO, GROSOR_SONRISA, SONRISA};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use zeno::{Cap, Command, Fill, Join, Mask, PathBuilder, Stroke};

/// Cómo se ve el personaje: el color del cuerpo, el de los rasgos (ojos, sonrisa y pastilla de
/// aviso) y los accesorios que lleva puestos (se dibujan en el orden de `Tipo::orden`). Los
/// colores son los de Nook; el Limón tiene los suyos (ver `LIMON_*`).
#[derive(Clone, Debug, PartialEq)]
pub struct Aspecto {
    pub cuerpo: [u8; 3],
    pub rasgos: [u8; 3],
    /// Color de la pastilla de aviso; sin él, el de los rasgos.
    pub aviso: Option<[u8; 3]>,
    pub accesorios: Vec<Accesorio>,
}

/// Radio del cuerpo respecto al lado de la imagen (deja margen para el rulo y los saltos).
pub const RADIO: f32 = 0.34;
/// Cuánto queda el centro del cuerpo por debajo del centro de la imagen, en unidades del radio:
/// con el rulo encima, así el personaje entero queda centrado y al saltar el rulo no se sale.
pub const BAJADA: f32 = 0.28;

/// Color del colmillo.
const BLANCO: [u8; 3] = [255, 255, 255];

/// Colores del Limón, los de su peluche: el cuerpo crema, las capas amarillas, los rasgos
/// marrones, los coloretes melocotón y el arándano azul grisáceo.
const LIMON_CREMA: [u8; 3] = [255, 247, 214];
const LIMON_AMARILLO: [u8; 3] = [247, 232, 104];
const LIMON_RASGOS: [u8; 3] = [107, 75, 46];
const LIMON_COLORETE: [u8; 3] = [246, 166, 128];
const LIMON_ARANDANO: [u8; 3] = [143, 162, 201];
/// La sombra que hace el pelo sobre la franja crema.
const LIMON_SOMBRA: [u8; 3] = [222, 204, 150];
/// Ojos «> <» (tamaño en reposo, unidades de radio) y la boquita en «x» (media diagonal), con el
/// grueso de sus trazos.
const LIMON_OJO: (f32, f32) = (0.14, 0.17);
const LIMON_GROSOR_OJO: f32 = 0.055;
const LIMON_BOCA: f32 = 0.042;
const LIMON_GROSOR_BOCA: f32 = 0.04;

/// Constante para aproximar un cuarto de círculo con una curva cúbica.
const CUARTO: f32 = 0.552_284_8;

/// Elipse de `ancho` × `alto` centrada en el origen, con cada punto pasado por `p` (la matriz del
/// ojo y la escala de la imagen).
fn elipse(ancho: f32, alto: f32, p: impl Fn(f32, f32) -> [f32; 2]) -> Vec<Command> {
    let (w, h) = (ancho / 2.0, alto / 2.0);
    let (kx, ky) = (w * CUARTO, h * CUARTO);
    let mut trazo = Vec::new();
    trazo.move_to(p(-w, 0.0));
    trazo.curve_to(p(-w, -ky), p(-kx, -h), p(0.0, -h));
    trazo.curve_to(p(kx, -h), p(w, -ky), p(w, 0.0));
    trazo.curve_to(p(w, ky), p(kx, h), p(0.0, h));
    trazo.curve_to(p(-kx, h), p(-w, ky), p(-w, 0.0));
    trazo.close();
    trazo
}

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

fn cobertura(trazo: &[Command], lado: u32) -> Vec<u8> {
    Mask::new(trazo).size(lado, lado).render().0
}

/// Cobertura de un contorno con agujeros: lo que queda dentro de otro contorno no se rellena.
fn cobertura_con_agujeros(trazo: &[Command], lado: u32) -> Vec<u8> {
    Mask::new(trazo).style(Fill::EvenOdd).size(lado, lado).render().0
}

/// Cobertura de un trazo abierto de `grosor` píxeles con los extremos y las uniones redondeados.
fn cobertura_linea(trazo: &[Command], grosor: f32, lado: u32) -> Vec<u8> {
    Mask::new(trazo).style(Stroke::new(grosor).cap(Cap::Round).join(Join::Round)).size(lado, lado).render().0
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

/// Borde de arriba de la capa amarilla de arriba del Limón, ondulado como un glaseado, en
/// reposo: su altura en cada x (unidades de radio).
fn borde_de_la_capa(x: f32) -> f32 {
    -0.27 + 0.04 * (6.5 * x + 0.3).sin() + 0.02 * (13.0 * x + 1.0).sin()
}

/// Borde de arriba de la capa amarilla de abajo del Limón: una curva suave, más baja en medio.
fn borde_de_la_base(x: f32) -> f32 {
    0.48 + 0.08 * (1.0 - x * x)
}

/// Contorno de una capa del Limón (unidades de radio): el borde, pintado en la cara para que se
/// mueva con ella (solo la parte que se ve: la que queda al otro lado de la cabeza se doblaría
/// sobre sí misma), sigue en horizontal hasta fuera del cuerpo por los dos lados y se cierra por
/// fuera, arriba o abajo hasta `fuera`, como un cartel. Luego se recorta por el cuerpo.
fn capa(cabeza: &Cabeza, borde: fn(f32) -> f32, fuera: f32) -> Vec<Punto> {
    let puntos: Vec<Punto> = (0..=60)
        .filter_map(|k| {
            let x = -1.0 + 2.0 * k as f32 / 60.0;
            let (p, profundidad) = cabeza.en_cara((x, borde(x)));
            (profundidad > 0.01).then_some(p)
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
    contorno.push(cabeza.cartel([0.0; 3], (1.8, fuera)));
    contorno.push(cabeza.cartel([0.0; 3], (-1.8, fuera)));
    contorno
}

/// Contorno cerrado que pasa por los puntos (unidades de radio), en píxeles.
fn poligono(puntos: &[Punto], pantalla: &dyn Fn(Punto) -> [f32; 2]) -> Vec<Command> {
    let mut trazo = Vec::new();
    for (i, &p) in puntos.iter().enumerate() {
        if i == 0 { trazo.move_to(pantalla(p)) } else { trazo.line_to(pantalla(p)) };
    }
    if !puntos.is_empty() {
        trazo.close();
    }
    trazo
}

/// Dibuja el cuadro en una imagen cuadrada de `lado` píxeles.
pub fn dibujar(cuadro: &Cuadro, lado: u32, aspecto: &Aspecto) -> Image {
    let figura = cuadro.cabeza.figura;
    let (cuerpo, ojos) = match figura {
        Figura::Nook => (aspecto.cuerpo, aspecto.rasgos),
        Figura::Limon => (LIMON_CREMA, LIMON_RASGOS),
    };
    let escala = lado as f32 * RADIO;
    let centro = lado as f32 / 2.0;
    let pantalla = |(x, y): (f32, f32)| [centro + x * escala, centro + (y + BAJADA) * escala];

    let mut trazo = Vec::new();
    let tramos = forma::curvas(&cuadro.cuerpo);
    if let Some(&(_, _, inicio)) = tramos.last() {
        trazo.move_to(pantalla(inicio));
        for &(c1, c2, p) in &tramos {
            trazo.curve_to(pantalla(c1), pantalla(c2), pantalla(p));
        }
        trazo.close();
    }
    let mut cubre_cuerpo = cobertura(&trazo, lado);
    let cabeza = &cuadro.cabeza;
    let rulo_tapado = aspecto.accesorios.iter().any(|a| a.tipo.tapa_el_rulo());
    // Lo que va detrás del cuerpo, de abajo arriba (el arándano y lo de los accesorios), lo que
    // se pinta sobre él (la capa de abajo, la sombra del pelo y los coloretes del Limón) y el
    // pelo del Limón, que va encima y asoma por fuera del cuerpo.
    let mut detras: Vec<([u8; 3], Vec<u8>)> = Vec::new();
    let mut pinturas: Vec<([u8; 3], Vec<f32>)> = Vec::new();
    let mut cubre_pelo: Vec<f32> = Vec::new();
    // Lo que va encima del pelo (la parte del arándano que se ve), con su opacidad.
    let mut sobre_pelo: Vec<([u8; 3], Vec<f32>)> = Vec::new();

    // El rulo es del color del cuerpo: se suma a él (salvo que lo tape un accesorio).
    if figura == Figura::Nook && !rulo_tapado {
        let r = cuadro.rulo.map(pantalla);
        let mut rulo = Vec::new();
        rulo.move_to(r[0]);
        rulo.curve_to(r[1], r[2], r[3]);
        rulo.curve_to(r[4], r[5], r[6]);
        for (total, v) in cubre_cuerpo.iter_mut().zip(cobertura_linea(&rulo, GROSOR_RULO * escala, lado)) {
            *total = (*total).max(v);
        }
    }

    if figura == Figura::Limon {
        // El pelo (la capa de arriba) se aparta de su sitio lo que diga el cuadro: nada en su
        // borde y del todo en lo alto, como un mechón que se dobla.
        let arriba = cabeza.medidas().arriba;
        let (borde_y, alto_y) = (cabeza.cartel([0.0; 3], (0.0, -0.25)).1, cabeza.cartel([0.0; 3], (0.0, -arriba)).1);
        let mover_pelo = |(x, y): Punto| {
            let k = ((borde_y - y) / (borde_y - alto_y).max(1e-3)).clamp(0.0, 1.3);
            (x + cuadro.pelo.0 * k, y + cuadro.pelo.1 * k)
        };

        // El arándano, en lo alto del pelo. De frente, el pelo tapa su mitad de abajo; al mirar
        // hacia abajo, lo alto de la cabeza se inclina hacia el espectador y se ve cada vez más,
        // hasta entero. Va encima del pelo, cortado por donde empieza a taparlo, y también
        // detrás del cuerpo (para cuando el pelo se mueve).
        if !rulo_tapado {
            let ancla = [0.0, -arriba * 0.98, 0.2];
            let en_baya = |p: Punto| pantalla(mover_pelo(cabeza.cartel(ancla, p)));
            let visto = (cabeza.profundidad([0.0, -1.0, 0.0]) / 0.3).clamp(0.0, 1.0);
            // Un óvalo de 0,30 × 0,24 desde el ángulo `desde` hasta `hasta` (la parte de arriba
            // está en 3π/2), cerrado con una recta.
            let ovalo = |desde: f32, hasta: f32| {
                let mut baya = Vec::new();
                for k in 0..=24 {
                    let (s, c) = (desde + (hasta - desde) * k as f32 / 24.0).sin_cos();
                    let p = en_baya((0.15 * c, -0.09 + 0.12 * s));
                    if k == 0 { baya.move_to(p) } else { baya.line_to(p) };
                }
                baya.close();
                baya
            };
            let entera = cobertura(&ovalo(0.0, std::f32::consts::TAU), lado);
            let corte = visto.asin();
            let vista = cobertura(&ovalo(std::f32::consts::PI - corte, std::f32::consts::TAU + corte), lado);
            // El cáliz: una uve más oscura arriba.
            let mut caliz = Vec::new();
            caliz.move_to(en_baya((-0.045, -0.19)));
            caliz.line_to(en_baya((0.0, -0.155)));
            caliz.line_to(en_baya((0.045, -0.19)));
            let oscuro = LIMON_ARANDANO.map(|c| (c as f32 * 0.7) as u8);
            let cubre_caliz = cobertura_linea(&caliz, 0.025 * escala, lado);
            let a_f32 = |v: &[u8]| v.iter().map(|&v| v as f32 / 255.0).collect::<Vec<_>>();
            sobre_pelo.push((LIMON_ARANDANO, a_f32(&vista)));
            sobre_pelo.push((oscuro, a_f32(&cubre_caliz)));
            detras.push((LIMON_ARANDANO, entera));
            detras.push((oscuro, cubre_caliz));
        }

        // El pelo: la capa de arriba, recortada por el cuerpo algo agrandado (así asoma por los
        // lados y por arriba) y movida con él.
        let a_f32 = |v: Vec<u8>| v.into_iter().map(|v| v as f32 / 255.0).collect::<Vec<_>>();
        let pelo: Vec<Punto> = capa(cabeza, borde_de_la_capa, -1.8).into_iter().map(mover_pelo).collect();
        let (dx, dy) = cabeza.punto([0.0; 3]);
        let grande: Vec<Punto> = cuadro.cuerpo.iter().map(|&(x, y)| mover_pelo((dx + (x - dx) * 1.06, dy + (y - dy) * 1.06))).collect();
        let mut silueta = Vec::new();
        let tramos = forma::curvas(&grande);
        if let Some(&(_, _, inicio)) = tramos.last() {
            silueta.move_to(pantalla(inicio));
            for &(c1, c2, p) in &tramos {
                silueta.curve_to(pantalla(c1), pantalla(c2), pantalla(p));
            }
            silueta.close();
        }
        let (dentro, contorno) = (cobertura(&silueta, lado), cobertura(&poligono(&pelo, &pantalla), lado));
        cubre_pelo = dentro.iter().zip(&contorno).map(|(&a, &b)| a.min(b) as f32 / 255.0).collect();
        // Su sombra sobre la franja crema, justo bajo el borde.
        let sombra: Vec<Punto> = pelo.iter().map(|&(x, y)| (x, y + 0.04)).collect();
        let sombra = a_f32(cobertura(&poligono(&sombra, &pantalla), lado)).into_iter().map(|v| v * 0.5).collect();
        pinturas.push((LIMON_SOMBRA, sombra));

        // La capa de abajo, pintada sobre el cuerpo, y los coloretes.
        pinturas.push((LIMON_AMARILLO, a_f32(cobertura(&poligono(&capa(cabeza, borde_de_la_base, 1.8), &pantalla), lado))));
        for lado_cara in [-1.0f32, 1.0] {
            let centro = (lado_cara * 0.45, 0.25);
            let (_, profundidad) = cabeza.en_cara(centro);
            let mut colorete = Vec::new();
            for k in 0..20 {
                let (s, c) = (k as f32 / 20.0 * std::f32::consts::TAU).sin_cos();
                let (p, _) = cabeza.en_cara((centro.0 + 0.09 * c, centro.1 + 0.07 * s));
                if k == 0 { colorete.move_to(pantalla(p)) } else { colorete.line_to(pantalla(p)) };
            }
            colorete.close();
            let visible = (profundidad / 0.12).clamp(0.0, 1.0);
            let cubre = cobertura(&colorete, lado).into_iter().map(|v| v as f32 / 255.0 * visible).collect();
            pinturas.push((LIMON_COLORETE, cubre));
        }
    }

    // Cobertura de los rasgos (la mayor en cada píxel, con su opacidad). Un ojo en arco es la
    // elipse menos otra algo más ancha y más baja: queda solo la parte de arriba, «∩». Los del
    // Limón son «> <», que se agrandan y aplastan como los de Nook (sorpresa, parpadeo).
    let mut cubre_rasgos = vec![0.0f32; cubre_cuerpo.len()];
    let ojos_tapados = aspecto.accesorios.iter().any(|a| a.tipo.tapa_los_ojos());
    for ojo in cuadro.ojos.iter().filter(|_| !ojos_tapados && figura == Figura::Limon) {
        let [a, b, c, d, e, f] = ojo.matriz;
        let en_ojo = |(x, y): (f32, f32)| pantalla((a * x + c * y + e, b * x + d * y + f));
        let (w, h) = (LIMON_OJO.0 * ojo.ancho / OJO_ANCHO / 2.0, LIMON_OJO.1 * ojo.alto / OJO_ALTO / 2.0);
        // La punta mira hacia el centro de la cara: «>» el de la izquierda, «<» el de la derecha.
        let punta = -ojo.lado * w;
        let mut trazo = Vec::new();
        trazo.move_to(en_ojo((-punta, -h)));
        trazo.line_to(en_ojo((punta, 0.0)));
        trazo.line_to(en_ojo((-punta, h)));
        for (total, v) in cubre_rasgos.iter_mut().zip(cobertura_linea(&trazo, LIMON_GROSOR_OJO * escala, lado)) {
            *total = total.max(v as f32 / 255.0 * ojo.alfa);
        }
    }
    for ojo in cuadro.ojos.iter().filter(|_| !ojos_tapados && figura == Figura::Nook) {
        let [a, b, c, d, e, f] = ojo.matriz;
        let transformar = |dy: f32| move |x: f32, y: f32| pantalla((a * x + c * (y + dy) + e, b * x + d * (y + dy) + f));
        let cubre = cobertura(&elipse(ojo.ancho, ojo.alto, transformar(0.0)), lado);
        let corte = (ojo.arco > 0.01).then(|| {
            cobertura(&elipse(ojo.ancho * 1.3, ojo.alto, transformar(ojo.alto * (1.0 - 0.5 * ojo.arco))), lado)
        });
        for (i, (total, &v)) in cubre_rasgos.iter_mut().zip(&cubre).enumerate() {
            let quitado = corte.as_ref().map_or(0.0, |corte| corte[i] as f32 / 255.0);
            *total = total.max(v as f32 / 255.0 * (1.0 - quitado) * ojo.alfa);
        }
    }

    // La sonrisa va con los rasgos y el colmillo, colgando de ella, encima.
    let mut cubre_colmillo = vec![0.0f32; cubre_cuerpo.len()];
    let boca_tapada = aspecto.accesorios.iter().any(|a| a.tipo.tapa_la_boca());
    if let Some(boca) = cuadro.boca.filter(|_| !boca_tapada && figura == Figura::Limon) {
        // La boquita del Limón: una «x».
        let [a, b, c, d, e, f] = boca.matriz;
        let en_boca = |(x, y): (f32, f32)| pantalla((a * x + c * y + e, b * x + d * y + f));
        let s = LIMON_BOCA;
        let mut equis = Vec::new();
        equis.move_to(en_boca((-s, -s)));
        equis.line_to(en_boca((s, s)));
        equis.move_to(en_boca((-s, s)));
        equis.line_to(en_boca((s, -s)));
        for (total, v) in cubre_rasgos.iter_mut().zip(cobertura_linea(&equis, LIMON_GROSOR_BOCA * escala, lado)) {
            *total = total.max(v as f32 / 255.0 * boca.alfa);
        }
    }
    if let Some(boca) = cuadro.boca.filter(|_| !boca_tapada && figura == Figura::Nook) {
        let [a, b, c, d, e, f] = boca.matriz;
        let en_boca = |(x, y): (f32, f32)| pantalla((a * x + c * y + e, b * x + d * y + f));
        let mut sonrisa = Vec::new();
        sonrisa.move_to(en_boca(SONRISA[0]));
        sonrisa.quad_to(en_boca(SONRISA[1]), en_boca(SONRISA[2]));
        for (total, v) in cubre_rasgos.iter_mut().zip(cobertura_linea(&sonrisa, GROSOR_SONRISA * escala, lado)) {
            *total = total.max(v as f32 / 255.0 * boca.alfa);
        }
        let mut colmillo = Vec::new();
        colmillo.move_to(en_boca(COLMILLO[0]));
        colmillo.line_to(en_boca(COLMILLO[1]));
        colmillo.line_to(en_boca(COLMILLO[2]));
        colmillo.close();
        for (total, v) in cubre_colmillo.iter_mut().zip(cobertura(&colmillo, lado)) {
            *total = v as f32 / 255.0 * boca.alfa;
        }
    }

    // Accesorios, de abajo arriba: cada pieza va delante del cuerpo con su opacidad (recortada
    // por él si va pintada en la cara) y, si le toca, también detrás.
    let mut puestos: Vec<&Accesorio> = aspecto.accesorios.iter().collect();
    puestos.sort_by_key(|a| a.tipo.orden());
    let mut delante = Vec::new();
    for accesorio in puestos {
        for pieza in accesorios::piezas(accesorio, &cuadro.cabeza, &pantalla, escala) {
            if pieza.delante <= 0.0 && !pieza.detras {
                continue;
            }
            let cubre = match pieza.estilo {
                Estilo::Relleno => cobertura(&pieza.trazo, lado),
                Estilo::ConAgujeros => cobertura_con_agujeros(&pieza.trazo, lado),
                Estilo::Trazo(grosor) => cobertura_linea(&pieza.trazo, grosor, lado),
            };
            if pieza.delante > 0.0 {
                let cubre = cubre.iter().map(|&v| v as f32 / 255.0 * pieza.delante).collect::<Vec<_>>();
                delante.push((pieza.color, cubre, pieza.recortar));
            }
            if pieza.detras {
                detras.push((pieza.color, cubre));
            }
        }
    }

    // Pastilla de aviso (del color elegido o, si no, el de los ojos) y el hueco que abre en lo que hay debajo, en
    // píxeles.
    let pastilla = cuadro.pastilla.map(|p| {
        let [x, y] = pantalla((p.x, p.y));
        (x, y, p.radio * escala, p.muesca * escala)
    });

    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(lado, lado);
    let pixeles = cubre_cuerpo.iter().zip(&cubre_rasgos).zip(&cubre_colmillo);
    for (i, (pixel, ((&b, &o), &w))) in buffer.make_mut_slice().iter_mut().zip(pixeles).enumerate() {
        // Premultiplicado: cada canal ya va multiplicado por la opacidad. De abajo arriba: lo de
        // los accesorios que va detrás, el cuerpo con los rasgos y el colmillo, lo de los
        // accesorios que va delante y la pastilla.
        let mut color: Pixel = [0.0; 4];
        for (tono, cubre) in &detras {
            encima(&mut color, *tono, cubre[i] as f32 / 255.0);
        }
        let b = b as f32 / 255.0;
        let o = o.min(b);
        // Parte del cuerpo que tapa el colmillo (0–1).
        let w = if b > 0.0 { w.min(b) / b } else { 0.0 };
        // El color del cuerpo en este píxel, con lo que lleva pintado encima.
        let mut base = cuerpo.map(|c| c as f32);
        for (tono, cubre) in &pinturas {
            for c in 0..3 {
                base[c] += (tono[c] as f32 - base[c]) * cubre[i];
            }
        }
        for c in 0..3 {
            let cara = (base[c] * (b - o) + ojos[c] as f32 * o) * (1.0 - w) + BLANCO[c] as f32 * w * b;
            color[c] = cara + color[c] * (1.0 - b);
        }
        color[3] = 255.0 * b + color[3] * (1.0 - b);
        if let Some(&pelo) = cubre_pelo.get(i) {
            encima(&mut color, LIMON_AMARILLO, pelo);
        }
        for (tono, cubre) in &sobre_pelo {
            encima(&mut color, *tono, cubre[i]);
        }
        for (tono, cubre, recortar) in &delante {
            encima(&mut color, *tono, if *recortar { cubre[i].min(b) } else { cubre[i] });
        }
        if let Some((x, y, radio, muesca)) = pastilla {
            let distancia = ((i as u32 % lado) as f32 + 0.5 - x).hypot((i as u32 / lado) as f32 + 0.5 - y);
            let hueco = 1.0 - circulo(distancia, muesca);
            color = color.map(|c| c * hueco);
            encima(&mut color, aspecto.aviso.unwrap_or(ojos), circulo(distancia, radio));
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
    use super::super::motor::Motor;
    use super::*;

    #[test]
    fn dibuja_el_cuerpo_los_rasgos_y_el_colmillo_dentro() {
        let lado = 128;
        let cuadro = Motor::new().muestrear(0.5);
        let imagen = dibujar(&cuadro, lado, &Aspecto { cuerpo: [46, 139, 87], rasgos: [18, 32, 58], aviso: None, accesorios: Vec::new() });
        let buffer = imagen.to_rgba8_premultiplied().expect("imagen de píxeles");
        let pixeles = buffer.as_slice();
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
            let imagen = dibujar(&cuadro, lado, &Aspecto { cuerpo: [46, 139, 87], rasgos: ojos, aviso: None, accesorios });
            let buffer = imagen.to_rgba8_premultiplied().expect("imagen de píxeles");
            buffer.as_slice().iter().filter(|p| p.a == 255 && [p.r, p.g, p.b] == ojos).count()
        };
        let gafas = Accesorio { tipo: Tipo::GafasDeSol, color: [34, 37, 45] };
        let sin_gafas = con(Vec::new());
        let con_gafas = con(vec![gafas]);
        assert!(sin_gafas > 0);
        // Solo queda la sonrisa.
        assert!(con_gafas < sin_gafas / 3, "{con_gafas} de {sin_gafas}");
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
            let imagen = dibujar(&cuadro, lado, &Aspecto { cuerpo: [46, 139, 87], rasgos, aviso, accesorios: Vec::new() });
            let buffer = imagen.to_rgba8_premultiplied().expect("imagen de píxeles");
            buffer.as_slice().iter().filter(|p| p.a == 255 && [p.r, p.g, p.b] == color).count()
        };
        // Con color elegido, la pastilla (un círculo grande) sale de él y no de los rasgos.
        assert!(con(Some(rojo), rojo) > 100);
        // Sin elegirlo, sale del color de los rasgos: hay muchos más píxeles que solo con los ojos.
        assert_eq!(con(None, rojo), 0);
        assert!(con(None, rasgos) > con(Some(rojo), rasgos) + 100);
    }

    #[test]
    fn el_arandano_del_limon_se_ve_tambien_mirando_hacia_abajo() {
        use super::super::motor::{Figura, Mirar};
        let lado = 128;
        let arandanos = |cabeceo: f32| {
            let mut motor = Motor::new();
            motor.poner_figura(Figura::Limon);
            motor.poner_mirada(Mirar { guinada: 0.0, cabeceo, mezcla: 1.0, deriva: 0.0 }, 0.0, 0.001);
            let imagen = dibujar(&motor.muestrear(1.0), lado, &Aspecto { cuerpo: [0; 3], rasgos: [0; 3], aviso: None, accesorios: Vec::new() });
            let buffer = imagen.to_rgba8_premultiplied().expect("imagen de píxeles");
            buffer.as_slice().iter().filter(|p| p.a == 255 && [p.r, p.g, p.b] == LIMON_ARANDANO).count()
        };
        let de_frente = arandanos(0.0);
        assert!(de_frente > 10, "de frente: {de_frente}");
        // Mirando hacia abajo se ve entero, encima del pelo: más que de frente.
        let abajo = arandanos(-26.0);
        assert!(abajo > de_frente, "mirando abajo: {abajo} (de frente {de_frente})");
    }

    #[test]
    fn al_saltar_el_rulo_no_se_sale_de_la_imagen() {
        // Mitad de la imagen en unidades del radio, con el centro del cuerpo `BAJADA` más abajo.
        let mitad = 0.5 / RADIO;
        let dentro = |(x, y): (f32, f32)| {
            let margen = GROSOR_RULO / 2.0;
            x.abs() + margen < mitad && (y + BAJADA).abs() + margen < mitad
        };
        // Con el salto de un aviso y con el rebote de un clic.
        let mut aviso = Motor::new();
        aviso.poner_estado(Estado::Aviso, 0.0);
        let mut clic = Motor::new();
        clic.rebotar(0.0);
        for motor in [aviso, clic] {
            for i in 0..300 {
                let cuadro = motor.muestrear(i as f64 / 100.0);
                let r = cuadro.rulo;
                let rulo = (0..=20).flat_map(|k| {
                    let t = k as f32 / 20.0;
                    [forma::cubica(r[0], r[1], r[2], r[3], t), forma::cubica(r[3], r[4], r[5], r[6], t)]
                });
                assert!(rulo.into_iter().all(dentro), "rulo fuera a los {i} cs");
                assert!(cuadro.cuerpo.iter().all(|&p| dentro(p)), "cuerpo fuera a los {i} cs");
            }
        }
    }
}
