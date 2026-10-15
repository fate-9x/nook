//! Accesorios que se le pueden poner al personaje. Cada uno se coloca con la cabeza del cuadro
//! (`Cabeza`), así acompaña al personaje en todas sus animaciones: gira hacia donde mira, se
//! estira y se aplasta con él, salta y baila. Hay tres maneras de llevarlos:
//! - alrededor de la cabeza (audífonos, cuernos, corona): lo que queda del lado que no se ve
//!   pasa por detrás del cuerpo;
//! - pintados en la cara, como los ojos (gafas, monóculo, antifaz, mascarilla): giran sobre la
//!   esfera de la cara y no se salen del cuerpo;
//! - como un cartel que siempre mira al frente (gorro, capa, corbatín): se inclinan, se estiran
//!   y saltan con el cuerpo, pero no se estrechan ni se deforman al girar la cabeza.
//!
//! Las medidas, como las del personaje, salen de su dibujo en SVG, donde el cuerpo mide 98 de
//! ancho y 78 de alto, con el centro en (0, 0): las de Nook. Con otra figura, lo que va en los
//! ojos o en la boca se coloca en los suyos, y lo que va encima o debajo del cuerpo sube o baja
//! lo que mida de más (ver `Medidas`).

use super::dibujo::capsula;
use super::forma::{Punto, cubica};
use super::motor::{Cabeza, UNIDAD, svg};
use super::cara::Vec3;
use zeno::{Command, PathBuilder};

/// Los accesorios que hay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    Audifonos,
    Corona,
    GorroDeLana,
    Cuernos,
    GafasDeSol,
    Monoculo,
    Antifaz,
    Mascarilla,
    Corbatin,
    CapaDeVampiro,
}

impl Tipo {
    /// En qué orden se dibujan, de abajo arriba: la capa al fondo, luego lo que va en la cara (el
    /// corbatín, sobre la mascarilla) y encima lo que va en la cabeza (los audífonos, sobre el
    /// gorro).
    pub fn orden(self) -> u8 {
        match self {
            Tipo::CapaDeVampiro => 0,
            Tipo::Mascarilla => 1,
            Tipo::Corbatin => 2,
            Tipo::Antifaz => 3,
            Tipo::GafasDeSol => 4,
            Tipo::Monoculo => 5,
            Tipo::GorroDeLana => 6,
            Tipo::Cuernos => 7,
            Tipo::Corona => 8,
            Tipo::Audifonos => 9,
        }
    }

    /// Si tapa el rulo (que entonces no se dibuja).
    pub fn tapa_el_rulo(self) -> bool {
        self == Tipo::GorroDeLana
    }

    /// Si tapa los ojos (que entonces no se dibujan, para que no asomen por los bordes de los
    /// cristales ni se transparenten si son claros).
    pub fn tapa_los_ojos(self) -> bool {
        self == Tipo::GafasDeSol
    }

    /// Si tapa la boca (la sonrisa y el colmillo, que entonces no se dibujan).
    pub fn tapa_la_boca(self) -> bool {
        self == Tipo::Mascarilla
    }
}

/// Un accesorio puesto, con su color (RGB).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Accesorio {
    pub tipo: Tipo,
    pub color: [u8; 3],
}

/// Cómo se rasteriza una pieza: rellena, rellena con agujeros (lo que queda dentro de otro
/// contorno es un agujero) o como un trazo de tantos píxeles de grueso.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Estilo {
    Relleno,
    ConAgujeros,
    Trazo(f32),
}

/// Una pieza de un accesorio lista para rasterizar, ya en píxeles. Se dibuja delante del cuerpo
/// con la opacidad `delante` (0 = no se ve delante) y, si `detras`, también detrás del cuerpo:
/// así lo que rodea la cabeza pasa de un lado al otro sin saltos. Con `recortar`, lo de delante
/// no se sale del cuerpo (lo pintado en la cara).
pub struct Pieza {
    pub trazo: Vec<Command>,
    pub estilo: Estilo,
    pub color: [u8; 3],
    pub delante: f32,
    pub detras: bool,
    pub recortar: bool,
}

/// Las piezas de un accesorio. `pantalla` pasa de unidades de radio a píxeles y `escala` es
/// cuántos píxeles mide una unidad.
pub fn piezas(accesorio: &Accesorio, cabeza: &Cabeza, pantalla: &dyn Fn(Punto) -> [f32; 2], escala: f32) -> Vec<Pieza> {
    let mut l = Lienzo { cabeza, pantalla, escala, color: accesorio.color, piezas: Vec::new() };
    match accesorio.tipo {
        Tipo::Audifonos => audifonos(&mut l),
        Tipo::Corona => corona(&mut l),
        Tipo::GorroDeLana => gorro_de_lana(&mut l),
        Tipo::Cuernos => cuernos(&mut l),
        Tipo::GafasDeSol => gafas_de_sol(&mut l),
        Tipo::Monoculo => monoculo(&mut l),
        Tipo::Antifaz => antifaz(&mut l),
        Tipo::Mascarilla => mascarilla(&mut l),
        Tipo::Corbatin => corbatin(&mut l),
        Tipo::CapaDeVampiro => capa_de_vampiro(&mut l),
    }
    l.piezas
}

// ---- Herramientas para dibujar ----

/// Lo que necesita el dibujo de un accesorio: dónde está la cabeza, cómo pasar a píxeles, su
/// color y las piezas que lleva hechas.
struct Lienzo<'a> {
    cabeza: &'a Cabeza,
    pantalla: &'a dyn Fn(Punto) -> [f32; 2],
    escala: f32,
    color: [u8; 3],
    piezas: Vec<Pieza>,
}

/// Lo que mide Nook hacia arriba y hacia abajo en el dibujo SVG.
const NOOK_ARRIBA: f32 = 39.0;
const NOOK_ABAJO: f32 = 39.0;

/// Las medidas de la figura del personaje en las del dibujo SVG: dónde está el ojo derecho y la
/// boca, y cuánto más mide que Nook hacia arriba y hacia abajo.
struct Medidas {
    ojo: (f32, f32),
    boca_y: f32,
    subir: f32,
    bajar: f32,
}

impl Lienzo<'_> {
    fn medidas(&self) -> Medidas {
        let m = self.cabeza.medidas();
        Medidas {
            ojo: (m.ojo.0 * UNIDAD, m.ojo.1 * UNIDAD),
            boca_y: m.boca.1 * UNIDAD,
            subir: m.arriba * UNIDAD - NOOK_ARRIBA,
            bajar: m.abajo * UNIDAD - NOOK_ABAJO,
        }
    }

    /// Un grosor del dibujo SVG, en píxeles.
    fn grosor(&self, svg: f32) -> f32 {
        svg / UNIDAD * self.escala
    }

    /// Puntos pintados en la cara, en píxeles, y la opacidad con la que se ven (como los ojos:
    /// se desvanecen al llegar al borde de la esfera), según la profundidad del primero.
    fn en_cara(&self, puntos: &[Punto]) -> (Vec<[f32; 2]>, f32) {
        let mut profundidad = 1.0;
        let pixeles = puntos
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                let (q, z) = self.cabeza.en_cara(p);
                if i == 0 {
                    profundidad = z;
                }
                (self.pantalla)(q)
            })
            .collect();
        (pixeles, (profundidad / 0.12).clamp(0.0, 1.0))
    }

    /// Puntos de un cartel pegado a la cabeza en `ancla`, en píxeles.
    fn en_cartel(&self, ancla: Vec3, puntos: &[Punto]) -> Vec<[f32; 2]> {
        puntos.iter().map(|&p| (self.pantalla)(self.cabeza.cartel(ancla, p))).collect()
    }

    /// Una pieza pintada en la cara: delante (mientras se vea) y sin salirse del cuerpo.
    fn en_la_cara(&mut self, trazo: Vec<Command>, estilo: Estilo, color: [u8; 3], visible: f32) {
        self.piezas.push(Pieza { trazo, estilo, color, delante: visible, detras: false, recortar: true });
    }

    /// Una pieza alrededor de la cabeza, delante con la opacidad `delante` y siempre detrás.
    fn alrededor(&mut self, trazo: Vec<Command>, estilo: Estilo, color: [u8; 3], delante: f32) {
        self.piezas.push(Pieza { trazo, estilo, color, delante, detras: true, recortar: false });
    }
}

/// Entre qué profundidades lo que rodea la cabeza pasa de detrás a delante: no en 0, para que
/// con la deriva de la mirada (que gira la cabeza unos grados) no se vaya y vuelva.
const DETRAS: f32 = -0.35;
const DELANTE: f32 = -0.15;

/// Cuánto se ve delante del cuerpo algo que rodea la cabeza con esta profundidad (0–1).
fn al_frente(profundidad: f32) -> f32 {
    let k = ((profundidad - DETRAS) / (DELANTE - DETRAS)).clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

/// Tramo de un contorno, como en un trazado SVG: una recta o una curva cúbica (control 1,
/// control 2 y final), en las medidas del dibujo.
#[derive(Clone, Copy)]
enum T {
    L(f32, f32),
    C(f32, f32, f32, f32, f32, f32),
}

/// Puntos de un contorno que empieza en (x, y) y sigue por `tramos` (las curvas, muestreadas),
/// en unidades de radio.
fn contorno(x: f32, y: f32, tramos: &[T]) -> Vec<Punto> {
    let mut p = svg(x, y);
    let mut puntos = vec![p];
    for &tramo in tramos {
        match tramo {
            T::L(x, y) => {
                p = svg(x, y);
                puntos.push(p);
            }
            T::C(a, b, c, d, x, y) => {
                let (c1, c2, fin) = (svg(a, b), svg(c, d), svg(x, y));
                puntos.extend((1..=10).map(|k| cubica(p, c1, c2, fin, k as f32 / 10.0)));
                p = fin;
            }
        }
    }
    puntos
}

/// Óvalo de semiejes `rx` × `ry` centrado en (cx, cy), que con `n` > 2 se va pareciendo a un
/// rectángulo de esquinas redondeadas (superelipse), en unidades de radio.
fn ovalo(cx: f32, cy: f32, rx: f32, ry: f32, n: f32) -> Vec<Punto> {
    (0..36)
        .map(|i| {
            let (s, c) = (i as f32 / 36.0 * std::f32::consts::TAU).sin_cos();
            let k = |v: f32| v.signum() * v.abs().powf(2.0 / n);
            svg(cx + rx * k(c), cy + ry * k(s))
        })
        .collect()
}

/// Los mismos puntos reflejados de izquierda a derecha (respecto a x = 0).
fn reflejar(puntos: &[Punto]) -> Vec<Punto> {
    puntos.iter().map(|&(x, y)| (-x, y)).collect()
}

/// Los mismos puntos desplazados (dx, dy), en las medidas del dibujo.
fn mover(puntos: &[Punto], dx: f32, dy: f32) -> Vec<Punto> {
    let (dx, dy) = svg(dx, dy);
    puntos.iter().map(|&(x, y)| (x + dx, y + dy)).collect()
}

/// Contorno cerrado que pasa por los puntos (en píxeles).
fn cerrado(puntos: &[[f32; 2]]) -> Vec<Command> {
    let mut trazo = abierto(puntos);
    trazo.close();
    trazo
}

/// Línea que pasa por los puntos (en píxeles).
fn abierto(puntos: &[[f32; 2]]) -> Vec<Command> {
    let mut trazo = Vec::new();
    for (i, &p) in puntos.iter().enumerate() {
        if i == 0 { trazo.move_to(p) } else { trazo.line_to(p) };
    }
    trazo
}

/// El color aclarado hacia el blanco (`k` > 0) u oscurecido hacia el negro (`k` < 0).
fn tono(color: [u8; 3], k: f32) -> [u8; 3] {
    color.map(|c| {
        let c = c as f32;
        (if k >= 0.0 { c + (255.0 - c) * k } else { c * (1.0 + k) }).round() as u8
    })
}

// ---- Audífonos ----
// Como en el dibujo del personaje escuchando música: la diadema es un arco de 4 de grueso que
// baja hasta los lados de la cabeza, y las almohadillas, cápsulas de 13 × 24 a los lados.

/// Diadema: dos curvas cuadráticas (inicio, control y final), de la izquierda a lo alto y de lo
/// alto a la derecha, en el plano de la cabeza que pasa por sus lados.
const DIADEMA: [[Punto; 3]; 2] = [
    [svg(-46.0, 0.0), svg(-46.0, -54.0), svg(0.0, -54.0)],
    [svg(0.0, -54.0), svg(46.0, -54.0), svg(46.0, 0.0)],
];
/// Almohadillas: a cada lado de la cabeza, de frente se ven de canto (estrechas) y, al girar la
/// cabeza, la que se acerca se ve más de cara (más ancha).
const ALMOHADILLA_X: f32 = 49.5 / UNIDAD;
const ALMOHADILLA_ANCHO: f32 = 13.0 / UNIDAD;
const ALMOHADILLA_ANCHO_DE_CARA: f32 = 22.0 / UNIDAD;
const ALMOHADILLA_ALTO: f32 = 24.0 / UNIDAD;

fn audifonos(l: &mut Lienzo) {
    for (lado, mitad) in [(-1.0f32, DIADEMA[0]), (1.0, DIADEMA[1])] {
        let z = l.cabeza.profundidad([lado, 0.0, 0.0]);
        let delante = al_frente(z);

        // Media diadema, como una línea de tramos cortos: cada punto del arco va a la pantalla.
        let [p0, c, p1] = mitad;
        // Una cuadrática es una cúbica con los controles a 2/3 del camino hacia el suyo.
        let c1 = (p0.0 + (c.0 - p0.0) * 2.0 / 3.0, p0.1 + (c.1 - p0.1) * 2.0 / 3.0);
        let c2 = (p1.0 + (c.0 - p1.0) * 2.0 / 3.0, p1.1 + (c.1 - p1.1) * 2.0 / 3.0);
        let diadema: Vec<_> = (0..=16)
            .map(|k| {
                let (x, y) = cubica(p0, c1, c2, p1, k as f32 / 16.0);
                (l.pantalla)(l.cabeza.punto([x, y, 0.0]))
            })
            .collect();
        l.alrededor(abierto(&diadema), Estilo::Trazo(l.grosor(4.0)), l.color, delante);

        // Almohadilla: una cápsula en su lado, con el alto a lo largo del eje vertical de la
        // cabeza (que se inclina y se acorta con ella) y el ancho perpendicular a él.
        let (cx, cy) = l.cabeza.punto([lado * ALMOHADILLA_X, 0.0, 0.0]);
        let (vx, vy) = l.cabeza.direccion([0.0, 1.0, 0.0]);
        let largo = vx.hypot(vy).max(1e-3);
        let (ux, uy) = (vy / largo, -vx / largo);
        let ancho = ALMOHADILLA_ANCHO + (ALMOHADILLA_ANCHO_DE_CARA - ALMOHADILLA_ANCHO) * z.abs().min(1.0);
        let pantalla = l.pantalla;
        let almohadilla = capsula(ancho, ALMOHADILLA_ALTO, |x, y| pantalla((cx + ux * x + vx * y, cy + uy * x + vy * y)));
        l.alrededor(almohadilla, Estilo::Relleno, l.color, delante);
    }
}

// ---- Corona ----
// Pequeña y ladeada, en lo alto de la cabeza a la izquierda del rulo: tres puntas con una bolita
// cada una, una banda algo más oscura y una gema roja en medio.

fn corona(l: &mut Lienzo) {
    let ancla = [-16.0 / UNIDAD, -(35.0 + l.medidas().subir) / UNIDAD, 0.3];
    let delante = al_frente(l.cabeza.profundidad(ancla));
    // Ladeada unos 14° hacia la izquierda.
    let (s, c) = (-0.24f32).sin_cos();
    let ladear = |puntos: Vec<Punto>| -> Vec<Punto> { puntos.into_iter().map(|(x, y)| (x * c - y * s, x * s + y * c)).collect() };
    let pieza = |puntos: Vec<Punto>| cerrado(&l.en_cartel(ancla, &ladear(puntos)));

    let cuerpo = contorno(-17.0, 2.0, &[T::L(-17.0, -16.0), T::L(-9.0, -7.0), T::L(0.0, -21.0), T::L(9.0, -7.0), T::L(17.0, -16.0), T::L(17.0, 2.0)]);
    let banda = contorno(-17.0, -4.0, &[T::L(17.0, -4.0), T::L(17.0, 2.0), T::L(-17.0, 2.0)]);
    let mut piezas = vec![(pieza(cuerpo), l.color), (pieza(banda), tono(l.color, -0.18))];
    for (x, y) in [(-17.0, -17.0), (0.0, -22.0), (17.0, -17.0)] {
        piezas.push((pieza(ovalo(x, y, 2.6, 2.6, 2.0)), tono(l.color, 0.4)));
    }
    piezas.push((pieza(ovalo(0.0, -1.0, 2.4, 2.4, 2.0)), [224, 69, 90]));
    for (trazo, color) in piezas {
        l.alrededor(trazo, Estilo::Relleno, color, delante);
    }
}

// ---- Gorro de lana ----
// Cubre lo alto de la cabeza (y el rulo): una cúpula, un doblez más oscuro con canalés y un
// pompón más claro arriba.

fn gorro_de_lana(l: &mut Lienzo) {
    let ancla = [0.0; 3];
    // Con una figura más alta, el doblez sube con lo alto de la cabeza; la cúpula llega casi al
    // mismo sitio (es más baja), para que el pompón no se salga de la imagen al saltar.
    let subir = l.medidas().subir;
    let base = -26.0 - subir;
    let cupula = contorno(-45.0, base, &[T::C(-45.0, -60.0, 45.0, -60.0, 45.0, base)]);
    let cupula = cerrado(&l.en_cartel(ancla, &cupula));
    l.alrededor(cupula, Estilo::Relleno, l.color, 1.0);
    let doblez = cerrado(&l.en_cartel(ancla, &ovalo(0.0, -25.0 - subir, 50.0, 7.0, 6.0)));
    l.alrededor(doblez, Estilo::Relleno, tono(l.color, -0.2), 1.0);
    for k in -4..=4 {
        let x = k as f32 * 10.0;
        let canale = abierto(&l.en_cartel(ancla, &[svg(x, -29.5 - subir), svg(x, -20.5 - subir)]));
        l.alrededor(canale, Estilo::Trazo(l.grosor(1.6)), tono(l.color, -0.35), 1.0);
    }
    // El pompón, un poco por encima de lo alto de la cúpula.
    let alto_cupula = base + 0.75 * (-60.0 - base);
    let pompon = cerrado(&l.en_cartel(ancla, &ovalo(0.0, alto_cupula - 3.5, 8.0, 8.0, 2.0)));
    l.alrededor(pompon, Estilo::Relleno, tono(l.color, 0.45), 1.0);
}

// ---- Cuernos ----
// Uno a cada lado del rulo, saliendo de lo alto de la cabeza y curvados hacia fuera, con un
// brillo a lo largo.

fn cuernos(l: &mut Lienzo) {
    let cuerno = contorno(-7.0, 0.0, &[T::C(-7.0, -12.0, -2.0, -20.0, 12.0, -25.0), T::C(6.0, -18.0, 5.0, -9.0, 7.0, 0.0)]);
    let brillo = contorno(-3.0, -4.0, &[T::C(-2.0, -11.0, 1.0, -16.0, 6.0, -20.0)]);
    for lado in [-1.0f32, 1.0] {
        let ancla = [lado * 33.0 / UNIDAD, -(31.0 + l.medidas().subir) / UNIDAD, 0.25];
        let delante = al_frente(l.cabeza.profundidad(ancla));
        let (cuerno, brillo) = if lado < 0.0 { (reflejar(&cuerno), reflejar(&brillo)) } else { (cuerno.clone(), brillo.clone()) };
        l.alrededor(cerrado(&l.en_cartel(ancla, &cuerno)), Estilo::Relleno, l.color, delante);
        l.alrededor(abierto(&l.en_cartel(ancla, &brillo)), Estilo::Trazo(l.grosor(1.6)), tono(l.color, 0.3), delante);
    }
}

// ---- Gafas de sol ----
// Dos cristales sobre los ojos (que tapan), con un brillo cada uno, el puente entre ellos y las
// patillas hasta los lados de la cabeza.

fn gafas_de_sol(l: &mut Lienzo) {
    let grueso = Estilo::Trazo(l.grosor(2.4));
    // Cristales de 22 × 16 centrados en los ojos, más pequeños si los ojos están más juntos
    // (que quede sitio para el puente).
    let (ox, oy) = l.medidas().ojo;
    let rx = (ox - 2.0).min(11.0);
    let ry = 8.0 * rx / 11.0;
    let y = oy - 1.0;
    for lado in [-1.0f32, 1.0] {
        let x = lado * ox;
        let (cristal, visible) = l.en_cara(&ovalo(x, y, rx, ry, 3.5));
        l.en_la_cara(cerrado(&cristal), Estilo::Relleno, l.color, visible);
        let (brillo, visible) = l.en_cara(&[svg(x - rx * 0.55, y), svg(x - rx * 0.18, y - ry * 0.5)]);
        l.en_la_cara(abierto(&brillo), Estilo::Trazo(l.grosor(1.8)), tono(l.color, 0.5), visible);
        let (patilla, _) = l.en_cara(&[svg(lado * (ox + rx - 1.0), y - ry * 0.5), svg(lado * 47.0, y - 6.0)]);
        let (_, visible) = l.en_cara(&[svg(lado * 40.0, y - 5.0)]);
        l.en_la_cara(abierto(&patilla), grueso, l.color, visible);
    }
    let g = ox - rx + 1.0;
    let (puente, visible) = l.en_cara(&contorno(-g, y - 4.0, &[T::C(-g / 3.0, y - 6.0, g / 3.0, y - 6.0, g, y - 4.0)]));
    l.en_la_cara(abierto(&puente), grueso, l.color, visible);
}

// ---- Monóculo ----
// Un aro alrededor del ojo derecho (el de la derecha en la pantalla), con un brillo, y una
// cadenita que cuelga hacia el lado.

fn monoculo(l: &mut Lienzo) {
    let (ox, oy) = l.medidas().ojo;
    let (aro, visible) = l.en_cara(&ovalo(ox, oy, 10.0, 10.0, 2.0));
    l.en_la_cara(cerrado(&aro), Estilo::Trazo(l.grosor(2.2)), l.color, visible);
    let brillo: Vec<Punto> = (0..=8)
        .map(|k| {
            let a = (200.0 + k as f32 * 6.0f32).to_radians();
            svg(ox + 7.0 * a.cos(), oy + 7.0 * a.sin())
        })
        .collect();
    let (brillo, visible) = l.en_cara(&brillo);
    l.en_la_cara(abierto(&brillo), Estilo::Trazo(l.grosor(1.4)), tono(l.color, 0.5), visible);
    let cadena = contorno(ox + 7.0, oy + 7.0, &[T::C(ox + 10.0, oy + 18.0, ox + 17.0, oy + 26.0, ox + 27.0, oy + 24.0)]);
    let (cadena, visible) = l.en_cara(&cadena);
    l.en_la_cara(abierto(&cadena), Estilo::Trazo(l.grosor(1.2)), l.color, visible);
}

// ---- Antifaz ----
// Una banda sobre los ojos, con un agujero para cada uno, y dos cintas asomando a cada lado.

fn antifaz(l: &mut Lienzo) {
    let banda = contorno(
        -36.0,
        -8.0,
        &[
            T::C(-30.0, -16.0, -8.0, -14.0, 0.0, -9.0),
            T::C(8.0, -14.0, 30.0, -16.0, 36.0, -8.0),
            T::C(38.0, 0.0, 34.0, 10.0, 26.0, 11.0),
            T::C(16.0, 12.0, 8.0, 6.0, 0.0, 4.0),
            T::C(-8.0, 6.0, -16.0, 12.0, -26.0, 11.0),
            T::C(-34.0, 10.0, -38.0, 0.0, -36.0, -8.0),
        ],
    );
    // A la altura de los ojos de la figura, con los agujeros sobre ellos.
    let (ox, oy) = l.medidas().ojo;
    let banda = mover(&banda, 0.0, oy);
    // El primer punto de la banda decide si se ve: se usa el centro de la cara.
    let (mut trazo, visible) = {
        let mut puntos = vec![svg(0.0, oy - 2.0)];
        puntos.extend(banda);
        let (pixeles, visible) = l.en_cara(&puntos);
        (cerrado(&pixeles[1..]), visible)
    };
    for lado in [-1.0f32, 1.0] {
        let (agujero, _) = l.en_cara(&ovalo(lado * ox, oy - 1.0, 6.5, 8.0, 2.0));
        trazo.extend(cerrado(&agujero));
    }
    l.en_la_cara(trazo, Estilo::ConAgujeros, l.color, visible);

    // Cintas: salen de cada borde de la banda hacia fuera de la cabeza, en el plano de la
    // pantalla; las de un lado pasan por detrás cuando la cabeza gira hacia el otro.
    let cabeza = l.cabeza;
    let fuera = |d: Punto| {
        let (a, b) = (cabeza.cartel([0.0; 3], d), cabeza.cartel([0.0; 3], (0.0, 0.0)));
        (a.0 - b.0, a.1 - b.1)
    };
    for lado in [-1.0f32, 1.0] {
        let delante = al_frente(cabeza.profundidad([lado, 0.0, 0.0]));
        for (inicio, hacia) in [((35.0, oy - 6.0), (13.0, -8.0)), ((35.0, oy - 1.0), (12.0, 6.0))] {
            let (desde, _) = cabeza.en_cara(svg(lado * inicio.0, inicio.1));
            let d = fuera(svg(lado * hacia.0, hacia.1));
            let cinta = [(l.pantalla)(desde), (l.pantalla)((desde.0 + d.0, desde.1 + d.1))];
            l.alrededor(abierto(&cinta), Estilo::Trazo(l.grosor(3.0)), l.color, delante);
        }
    }
}

// ---- Mascarilla ----
// Tapa la boca: una mascarilla con tres pliegues más oscuros y una goma a cada lado que va de
// sus esquinas hacia los lados de la cabeza.

fn mascarilla(l: &mut Lienzo) {
    // Bajo la boca de Nook; en una figura con la boca entre los ojos, justo bajo los ojos.
    let m = l.medidas();
    let dy = (m.boca_y - 6.0).max(m.ojo.1 + 9.0) - 9.0;
    let tela = contorno(
        -25.0,
        9.0,
        &[
            T::C(-12.0, 5.0, 12.0, 5.0, 25.0, 9.0),
            T::C(27.0, 16.0, 25.0, 22.0, 19.0, 25.0),
            T::C(8.0, 28.5, -8.0, 28.5, -19.0, 25.0),
            T::C(-25.0, 22.0, -27.0, 16.0, -25.0, 9.0),
        ],
    );
    let tela = mover(&tela, 0.0, dy);
    // El primer punto decide si se ve: se usa el centro de la mascarilla.
    let mut puntos = vec![svg(0.0, 17.0 + dy)];
    puntos.extend(tela);
    let (pixeles, visible) = l.en_cara(&puntos);
    l.en_la_cara(cerrado(&pixeles[1..]), Estilo::Relleno, l.color, visible);
    for (y, ancho) in [(13.0 + dy, 22.0), (17.5 + dy, 23.0), (22.0 + dy, 20.0)] {
        let (pliegue, visible) = l.en_cara(&contorno(-ancho, y, &[T::C(-ancho / 3.0, y - 2.0, ancho / 3.0, y - 2.0, ancho, y)]));
        l.en_la_cara(abierto(&pliegue), Estilo::Trazo(l.grosor(1.3)), tono(l.color, -0.2), visible);
    }
    for lado in [-1.0f32, 1.0] {
        for (x, y) in [(25.0, 11.0), (20.0, 24.0)] {
            let (goma, visible) = l.en_cara(&[svg(lado * x, y + dy), svg(lado * 47.0, 2.0 + dy)]);
            l.en_la_cara(abierto(&goma), Estilo::Trazo(l.grosor(1.4)), tono(l.color, -0.1), visible);
        }
    }
}

// ---- Corbatín ----
// En la parte de abajo del cuerpo, bajo la sonrisa: dos alas y un nudo más oscuro. Va pegado
// al cuerpo como un cartel (pintado en la cara, tan abajo, se estiraría al girar la cabeza) y
// no se sale de él.

fn corbatin(l: &mut Lienzo) {
    // Por delante del cuerpo, abajo, de modo que en reposo quede 8 por encima de su borde.
    let ancla = [0.0, (31.0 + l.medidas().bajar) / UNIDAD, 0.45];
    let visible = (l.cabeza.profundidad(ancla) / 0.12).clamp(0.0, 1.0);
    let ala = contorno(
        -3.0,
        -2.0,
        &[T::C(-8.0, -6.0, -12.0, -8.0, -14.0, -5.0), T::C(-15.0, -2.0, -15.0, 2.0, -14.0, 5.0), T::C(-12.0, 8.0, -8.0, 6.0, -3.0, 2.0)],
    );
    for ala in [reflejar(&ala), ala.clone()] {
        l.en_la_cara(cerrado(&l.en_cartel(ancla, &ala)), Estilo::Relleno, l.color, visible);
    }
    let nudo = cerrado(&l.en_cartel(ancla, &ovalo(0.0, 0.0, 3.6, 3.6, 4.0)));
    l.en_la_cara(nudo, Estilo::Relleno, tono(l.color, -0.25), visible);
}

// ---- Capa de vampiro ----
// Detrás del cuerpo: un cuello alto en abanico a cada lado de la cabeza, rojo por dentro con el
// borde del color de la capa, y la capa, que asoma por los lados y por debajo con el borde en
// picos.

fn capa_de_vampiro(l: &mut Lienzo) {
    let ancla = [0.0; 3];
    // Con una figura más alta, lo de arriba sube y lo de abajo baja lo que mide de más (poco a
    // poco desde el centro), para que asome lo mismo.
    let m = l.medidas();
    let (subir, bajar) = (m.subir / UNIDAD, m.bajar / UNIDAD);
    let estirar = |puntos: Vec<Punto>| -> Vec<Punto> {
        let k = |y: f32, mide: f32| (y.abs() * UNIDAD / mide).min(1.0);
        puntos.into_iter().map(|(x, y)| (x, if y < 0.0 { y - subir * k(y, NOOK_ARRIBA) } else { y + bajar * k(y, NOOK_ABAJO) })).collect()
    };
    let capa = contorno(
        -30.0,
        -18.0,
        &[
            T::L(-62.0, -52.0),
            T::L(-56.0, -2.0),
            T::C(-61.0, 14.0, -60.0, 30.0, -56.0, 44.0),
            T::L(-42.0, 39.0),
            T::L(-28.0, 46.0),
            T::L(-14.0, 40.0),
            T::L(0.0, 47.0),
            T::L(14.0, 40.0),
            T::L(28.0, 46.0),
            T::L(42.0, 39.0),
            T::L(56.0, 44.0),
            T::C(60.0, 30.0, 61.0, 14.0, 56.0, -2.0),
            T::L(62.0, -52.0),
            T::L(30.0, -18.0),
        ],
    );
    l.alrededor(cerrado(&l.en_cartel(ancla, &estirar(capa))), Estilo::Relleno, l.color, 0.0);
    // El forro del cuello, metido unos 3 por dentro de su borde.
    let forro = estirar(contorno(-33.0, -20.0, &[T::L(-58.0, -47.0), T::L(-53.0, -5.0)]));
    for forro in [forro.clone(), reflejar(&forro)] {
        l.alrededor(cerrado(&l.en_cartel(ancla, &forro)), Estilo::Relleno, [179, 18, 46], 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::super::dibujo::{BAJADA, RADIO};
    use super::super::estados::Estado;
    use super::super::motor::{Figura, Mirar, Motor};
    use super::*;

    const TODOS: [Tipo; 10] = [
        Tipo::Audifonos,
        Tipo::Corona,
        Tipo::GorroDeLana,
        Tipo::Cuernos,
        Tipo::GafasDeSol,
        Tipo::Monoculo,
        Tipo::Antifaz,
        Tipo::Mascarilla,
        Tipo::Corbatin,
        Tipo::CapaDeVampiro,
    ];

    fn cabeza(guinada: f32) -> Cabeza {
        let mut motor = Motor::new();
        motor.poner_mirada(Mirar { guinada, cabeceo: 0.0, mezcla: 1.0, deriva: 0.0 }, 0.0, 0.001);
        motor.muestrear(1.0).cabeza
    }

    fn de(tipo: Tipo, cabeza: &Cabeza) -> Vec<Pieza> {
        piezas(&Accesorio { tipo, color: [10, 20, 30] }, cabeza, &|(x, y)| [x, y], 1.0)
    }

    /// Los puntos de un trazo (en unidades de radio, con `pantalla` sin cambios).
    fn puntos(trazo: &[Command]) -> Vec<[f32; 2]> {
        trazo
            .iter()
            .filter_map(|c| match *c {
                Command::MoveTo(p) | Command::LineTo(p) => Some([p.x, p.y]),
                Command::CurveTo(_, _, p) | Command::QuadTo(_, p) => Some([p.x, p.y]),
                Command::Close => None,
            })
            .collect()
    }

    #[test]
    fn de_frente_los_audifonos_van_delante_y_al_girar_el_lado_lejano_pasa_detras() {
        let piezas = |guinada| de(Tipo::Audifonos, &cabeza(guinada));
        // Diadema y almohadilla de cada lado.
        assert_eq!(piezas(0.0).len(), 4);
        assert!(piezas(0.0).iter().all(|p| p.delante == 1.0));
        // Mirando a la derecha, el lado derecho (las dos últimas piezas) se va detrás.
        let girado = piezas(35.0);
        assert!(girado[..2].iter().all(|p| p.delante == 1.0));
        assert!(girado[2..].iter().all(|p| p.delante == 0.0));
    }

    #[test]
    fn cada_accesorio_se_ve_de_frente_y_la_capa_solo_por_detras() {
        let cabeza = cabeza(0.0);
        for tipo in TODOS {
            let piezas = de(tipo, &cabeza);
            assert!(!piezas.is_empty() && piezas.iter().all(|p| !p.trazo.is_empty()), "{tipo:?} sin piezas");
            if tipo == Tipo::CapaDeVampiro {
                assert!(piezas.iter().all(|p| p.detras && p.delante == 0.0));
            } else {
                assert!(piezas.iter().any(|p| p.delante > 0.99), "{tipo:?} no se ve delante");
            }
        }
    }

    #[test]
    fn lo_pintado_en_la_cara_sigue_a_la_cabeza() {
        let centro = |guinada| {
            let p = puntos(&de(Tipo::GafasDeSol, &cabeza(guinada))[0].trazo);
            p.iter().map(|q| q[0]).sum::<f32>() / p.len() as f32
        };
        // El cristal izquierdo está a la izquierda y, al mirar a la derecha, se va hacia allí.
        assert!(centro(0.0) < -0.2);
        assert!(centro(30.0) > centro(0.0) + 0.2);
    }

    #[test]
    fn al_saltar_ningun_accesorio_se_sale_de_la_imagen() {
        let mitad = 0.5 / RADIO;
        let mut motores = Vec::new();
        for figura in [Figura::Nook, Figura::Limon] {
            let con = |f: &dyn Fn(&mut Motor)| {
                let mut m = Motor::new();
                m.poner_figura(figura);
                f(&mut m);
                m
            };
            motores.push(con(&|m| m.poner_estado(Estado::Aviso, 0.0)));
            motores.push(con(&|m| m.rebotar(0.0)));
            motores.push(con(&|m| m.poner_estado(Estado::Musica, 0.0)));
        }
        for motor in motores {
            for i in 0..300 {
                let cabeza = motor.muestrear(i as f64 / 100.0).cabeza;
                for tipo in TODOS {
                    for pieza in de(tipo, &cabeza) {
                        for [x, y] in puntos(&pieza.trazo) {
                            assert!(x.abs() < mitad && (y + BAJADA).abs() < mitad, "{tipo:?} fuera a los {i} cs: ({x}, {y})");
                        }
                    }
                }
            }
        }
    }
}
