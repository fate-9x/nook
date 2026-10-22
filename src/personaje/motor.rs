//! Motor del personaje: `muestrear(t)` da el cuadro del instante `t` sin cambiar nada, así que
//! dibujar dos veces el mismo instante da la misma imagen. Lo de fuera (hacia dónde mira, qué
//! está haciendo, el ritmo de la música) entra con setters fechados, nunca leyendo variables
//! mientras se muestrea.

use super::cara::{self, Mirada, Vec3};
use super::estados::{self, Estado, Golpe, Pose};
use super::figura::{Animacion, Dibujo, Figura, Medidas};
use super::forma::Punto;
use std::sync::Arc;

/// Curva de las transiciones: frena al final sin pasarse (como en bloub).
fn suavizar(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(5)
}

fn mezclar(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Unidades del dibujo de los accesorios: las del de Nook, en el que el cuerpo mide 98 de ancho
/// (49 de radio).
pub const UNIDAD: f32 = 49.0;

/// Un punto del dibujo SVG de los accesorios (centrado en el cuerpo) en unidades del radio del
/// cuerpo.
pub const fn svg(x: f32, y: f32) -> Punto {
    (x / UNIDAD, y / UNIDAD)
}

/// Ángulos (en grados) sobre la esfera de la cara de un rasgo que en reposo está en `p`: hacia
/// la derecha y hacia abajo del centro de la cara.
fn angulos(figura: &Figura, (x, y): Punto) -> (f32, f32) {
    let Medidas { ancho, abajo: alto, .. } = figura.medidas;
    let vertical = (y / alto).clamp(-1.0, 1.0).asin();
    let lateral = (x / ancho / vertical.cos().max(1e-3)).clamp(-1.0, 1.0).asin();
    (lateral.to_degrees(), vertical.to_degrees())
}
/// Hacia dónde mira cuando nada lo guía: de frente.
const MIRADA_REPOSO: Mirada = Mirada { guinada: 0.0, cabeceo: 0.0, alabeo: 0.0 };
/// Hacia dónde mira la pastilla de aviso (arriba a la derecha) y dónde va sobre el contorno.
const MIRADA_PASTILLA: Mirada = Mirada { guinada: 17.0, cabeceo: 15.0, alabeo: 0.0 };
/// Grados de mirada (a un lado y hacia arriba o abajo) con los que las pupilas llegan al borde de
/// lo que pueden moverse dentro del ojo: lo que gira la cabeza como mucho.
const PUPILA_GUINADA: f32 = 35.0;
const PUPILA_CABECEO: f32 = 26.0;
const ANGULO_PASTILLA: f32 = -0.6; // radianes, unos 34° por encima de la horizontal (lejos del rulo)
const RADIO_PASTILLA: f32 = 0.22;
/// Hueco que la pastilla abre en el cuerpo alrededor de ella.
const MUESCA: f32 = 0.07;
/// Cuánto se inclina el cuerpo hacia donde mira (en unidades de radio, mirando de lado del todo).
const INCLINACION: f32 = 0.06;
/// Lo que tarda en pasar de un estado a otro.
const TRANSICION: f64 = 0.35;
/// El pelo (la capa de arriba del Limón) se mueve como un muelle amortiguado que cuelga de lo
/// alto de la cabeza: se queda atrás cuando ella se mueve, se pasa al frenar y se asienta. Se
/// calcula con lo que hizo la cabeza en el último medio segundo, a `PASO_PELO` entre muestras:
/// cuánto tarda en calmarse, a qué ritmo oscila, cuánto se mueve por cada unidad que se mueve la
/// cabeza (en vertical, menos) y hasta dónde llega.
const PASO_PELO: f64 = 1.0 / 60.0;
const MUESTRAS_PELO: usize = 30;
const CALMA_PELO: f32 = 0.15;
const RITMO_PELO: f32 = 2.2;
const FUERZA_PELO: f32 = 2.2;
const FUERZA_PELO_VERTICAL: f32 = 0.5;
const TOPE_PELO: f32 = 0.15;

/// Hacia dónde mira cuando algo de fuera lo guía (el cursor).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mirar {
    /// Dirección absoluta en grados; sustituye a la de reposo a medida que sube `mezcla`.
    pub guinada: f32,
    pub cabeceo: f32,
    /// Cuánto manda esta dirección (0 = nada, se queda la de reposo).
    pub mezcla: f32,
    /// Cuánto se mueve la mirada por su cuenta además (0 = nada).
    pub deriva: f32,
}

impl Mirar {
    /// Sin guía: mirada de reposo con su deriva.
    pub const LIBRE: Mirar = Mirar { guinada: 0.0, cabeceo: 0.0, mezcla: 0.0, deriva: 1.0 };

    fn hacia(self, otro: Mirar, t: f32) -> Mirar {
        Mirar {
            guinada: mezclar(self.guinada, otro.guinada, t),
            cabeceo: mezclar(self.cabeceo, otro.cabeceo, t),
            mezcla: mezclar(self.mezcla, otro.mezcla, t),
            deriva: mezclar(self.deriva, otro.deriva, t),
        }
    }

    fn es_finito(self) -> bool {
        (self.guinada + self.cabeceo + self.mezcla + self.deriva).is_finite()
    }
}

/// Un ojo listo para dibujar: un óvalo de `ancho` × `alto` (unidades de radio) transformado por
/// `matriz` = [a, b, c, d, e, f] (x' = a·x + c·y + e, y' = b·x + d·y + f).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ojo {
    /// De qué lado de la cara está: -1 el de la izquierda (en la pantalla), 1 el de la derecha.
    pub lado: f32,
    pub ancho: f32,
    pub alto: f32,
    /// Cuánto es un arco «∩» en lugar de un óvalo (0–1).
    pub arco: f32,
    pub matriz: [f32; 6],
    /// Opacidad: se desvanece al llegar al borde de la esfera.
    pub alfa: f32,
    /// Hacia dónde miran sus pupilas dentro del ojo: de -1 a 1 en cada eje (x a la derecha, y
    /// hacia abajo), hasta el borde de lo que pueden moverse.
    pub pupila: Punto,
    /// Cuánto lo tapa el párpado por arriba (0–1), en las figuras con párpados (si no, 0).
    pub parpado: f32,
}

/// Boca lista para dibujar: `SONRISA` y `COLMILLO` transformados por `matriz` (como un ojo).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boca {
    pub matriz: [f32; 6],
    /// Opacidad: se desvanece al llegar al borde de la esfera.
    pub alfa: f32,
}

/// Pastilla de aviso: un círculo de radio `radio` centrado en (x, y), que abre en el cuerpo un
/// hueco de radio `muesca` para separarse de él.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pastilla {
    pub x: f32,
    pub y: f32,
    pub radio: f32,
    pub muesca: f32,
}

/// La cabeza en un instante: sus ejes (hacia dónde mira) y cómo la colocan el estado y la vida
/// (giro, estirado y desplazamiento). Sirve para poner encima lo que no va pintado en la cara,
/// como los accesorios: un punto en 3D respecto a la cabeza va a su sitio en la pantalla.
#[derive(Clone, Debug, PartialEq)]
pub struct Cabeza {
    pub figura: Arc<Figura>,
    pub ejes: cara::Ejes,
    giro: f32,
    sx: f32,
    sy: f32,
    dx: f32,
    dy: f32,
}

impl Cabeza {
    /// Una dirección respecto a la cabeza (x a su derecha, y hacia su abajo, z hacia donde mira;
    /// en unidades de radio) vista en la pantalla, sin el desplazamiento.
    pub fn direccion(&self, [x, y, z]: Vec3) -> Punto {
        let cara::Ejes { frente, derecha, abajo } = self.ejes;
        let px = x * derecha[0] + y * abajo[0] + z * frente[0];
        let py = x * derecha[1] + y * abajo[1] + z * frente[1];
        let (s, c) = self.giro.sin_cos();
        ((px * c - py * s) * self.sx, (px * s + py * c) * self.sy)
    }

    /// Un punto respecto al centro de la cabeza, en la pantalla.
    pub fn punto(&self, v: Vec3) -> Punto {
        let (x, y) = self.direccion(v);
        (x + self.dx, y + self.dy)
    }

    /// Un desplazamiento en el plano del cuerpo (unidades de radio) girado y estirado con él.
    pub fn al_cuerpo(&self, (x, y): Punto) -> Punto {
        let (s, c) = self.giro.sin_cos();
        ((x * c - y * s) * self.sx, (x * s + y * c) * self.sy)
    }

    /// Un punto del cuerpo en reposo (unidades de radio respecto a su centro), en la pantalla:
    /// girado, estirado y movido con él.
    pub fn cuerpo(&self, p: Punto) -> Punto {
        let (x, y) = self.al_cuerpo(p);
        (x + self.dx, y + self.dy)
    }

    /// Lo contrario de `cuerpo`: dónde estaba en reposo un punto de la pantalla.
    pub fn desde_cuerpo(&self, (x, y): Punto) -> Punto {
        let (x, y) = ((x - self.dx) / self.sx.max(1e-3), (y - self.dy) / self.sy.max(1e-3));
        let (s, c) = self.giro.sin_cos();
        (x * c + y * s, -x * s + y * c)
    }

    /// Un punto de algo pegado a la cabeza en `ancla` que se gira y se estira con el cuerpo, sin
    /// inclinarse con la cabeza: `p` es su posición respecto al ancla en reposo.
    pub fn en_cuerpo(&self, ancla: Vec3, p: Punto) -> Punto {
        let (ox, oy) = self.punto(ancla);
        let (x, y) = self.al_cuerpo(p);
        (ox + x, oy + y)
    }

    /// Cuánto se acerca al espectador una dirección respecto a la cabeza (> 0 es hacia él).
    pub fn profundidad(&self, [x, y, z]: Vec3) -> f32 {
        let cara::Ejes { frente, derecha, abajo } = self.ejes;
        x * derecha[2] + y * abajo[2] + z * frente[2]
    }

    /// Un punto pintado en la cara, como los ojos y la boca: `(x, y)` es dónde está con el
    /// personaje de frente y en reposo (unidades de radio, respecto al centro del cuerpo). La
    /// cara es un elipsoide tan ancho y tan alto como el cuerpo: el punto se lleva a él, se gira
    /// con la cabeza y se vuelve a la pantalla. Con radios fijos (y no el del cuerpo en cada
    /// dirección, que cerca del centro de la cara cambia de golpe) lo que se pinta no se
    /// deforma al girar. Devuelve el punto en la pantalla y su profundidad (> 0 mira hacia el
    /// espectador; lo que pasa al otro lado de la cabeza baja de 0).
    pub fn en_cara(&self, (x, y): Punto) -> (Punto, f32) {
        let Medidas { ancho, abajo: alto, .. } = self.figura.medidas;
        let abajo = (y / alto).clamp(-1.0, 1.0);
        let corte = (1.0 - abajo * abajo).sqrt();
        let lado = (x / ancho).clamp(-corte, corte);
        let v = [lado, abajo, (corte * corte - lado * lado).max(0.0).sqrt()];
        let cara::Ejes { frente, derecha, abajo } = self.ejes;
        let en = |i: usize| v[0] * derecha[i] + v[1] * abajo[i] + v[2] * frente[i];
        let (px, py) = (en(0) * ancho, en(1) * alto);
        let (s, c) = self.giro.sin_cos();
        (((px * c - py * s) * self.sx + self.dx, (px * s + py * c) * self.sy + self.dy), en(2))
    }

    /// Un punto de un dibujo plano pegado a la cabeza en `ancla` y siempre de frente, como un
    /// cartel: `(x, y)` es su posición respecto al ancla con el personaje en reposo (unidades de
    /// radio). Se inclina con la cabeza y se acorta en vertical al mirar arriba o abajo, pero no
    /// se estrecha al girarla: es para lo que se ve igual desde cualquier lado (un gorro, una
    /// corona, una capa).
    pub fn cartel(&self, ancla: Vec3, (x, y): Punto) -> Punto {
        let [ax, ay, _] = self.ejes.abajo;
        let largo = ax.hypot(ay).max(1e-3);
        // La vertical del cartel es la de la cabeza en la pantalla; la horizontal, perpendicular.
        let (px, py) = (x * ay / largo + y * ax, -x * ax / largo + y * ay);
        let (s, c) = self.giro.sin_cos();
        let (ox, oy) = self.punto(ancla);
        (ox + (px * c - py * s) * self.sx, oy + (px * s + py * c) * self.sy)
    }

    /// Dónde están los rasgos de su figura en reposo.
    pub fn medidas(&self) -> Medidas {
        self.figura.medidas
    }

    fn casi_igual(&self, otra: &Cabeza, tolerancia: f32) -> bool {
        let (a, b) = (self.ejes, otra.ejes);
        Arc::ptr_eq(&self.figura, &otra.figura)
            && [self.giro, self.sx, self.sy, self.dx, self.dy]
            .into_iter()
            .chain(a.frente.into_iter().chain(a.derecha).chain(a.abajo))
            .zip([otra.giro, otra.sx, otra.sy, otra.dx, otra.dy].into_iter().chain(b.frente.into_iter().chain(b.derecha).chain(b.abajo)))
            .all(|(x, y)| (x - y).abs() <= tolerancia)
    }
}

/// Lo que hay que dibujar en un instante, en unidades del radio del cuerpo y con el centro en 0:
/// la cabeza (con la figura, y cómo están colocados el cuerpo y lo que va pegado a él), los ojos,
/// la boca y la pastilla.
#[derive(Clone, Debug, PartialEq)]
pub struct Cuadro {
    pub ojos: Vec<Ojo>,
    pub boca: Option<Boca>,
    pub pastilla: Option<Pastilla>,
    pub cabeza: Cabeza,
    /// Cuánto se aparta el pelo de su sitio (x, y), en unidades de radio: lo que se mueve lo
    /// más alto de él (solo en las figuras con pelo; si no, 0).
    pub pelo: Punto,
    /// En las figuras de imágenes, qué animación se ve y en qué fotograma va.
    pub fotograma: Option<(Animacion, usize)>,
}

impl Cuadro {
    /// Si `otro` se vería igual: ningún punto se mueve más de `tolerancia` (unidades de radio).
    pub fn casi_igual(&self, otro: &Cuadro, tolerancia: f32) -> bool {
        let cerca = |a: f32, b: f32| (a - b).abs() <= tolerancia;
        let pastillas = match (self.pastilla, otro.pastilla) {
            (None, None) => true,
            (Some(a), Some(b)) => cerca(a.x, b.x) && cerca(a.y, b.y) && cerca(a.radio, b.radio),
            _ => false,
        };
        let bocas = match (self.boca, otro.boca) {
            (None, None) => true,
            (Some(a), Some(b)) => a.matriz.iter().zip(&b.matriz).all(|(x, y)| cerca(*x, *y)) && cerca(a.alfa, b.alfa),
            _ => false,
        };
        let puntos = |a: &[Punto], b: &[Punto]| a.iter().zip(b).all(|(a, b)| cerca(a.0, b.0) && cerca(a.1, b.1));
        pastillas
            && bocas
            && self.ojos.len() == otro.ojos.len()
            && self.fotograma == otro.fotograma
            && puntos(&[self.pelo], &[otro.pelo])
            && self.cabeza.casi_igual(&otro.cabeza, tolerancia)
            && self.ojos.iter().zip(&otro.ojos).all(|(a, b)| {
                a.matriz.iter().zip(&b.matriz).all(|(x, y)| cerca(*x, *y)) && cerca(a.alfa, b.alfa) && cerca(a.arco, b.arco)
            })
    }
}

pub struct Motor {
    mirar: Mirar,
    mirar_antes: Mirar,
    /// Cuándo empezó el cambio de mirada actual y cuánto dura.
    mirar_desde: f64,
    mirar_duracion: f32,
    /// Estado actual y desde cuándo; el anterior y desde cuándo estaba en él (para mezclarlos
    /// durante la transición).
    estado: Estado,
    estado_desde: f64,
    anterior: Estado,
    anterior_desde: f64,
    /// Pose de partida fija, cuando el estado cambió en mitad de otra transición.
    congelada: Option<Pose>,
    golpe: Golpe,
    /// Cuándo se hizo clic en el personaje por última vez (para el rebote).
    rebote: f64,
    figura: Arc<Figura>,
}

impl Motor {
    /// Lo que tarda la mirada en alcanzar al cursor: atenta, sin parecer pegajosa. Como la
    /// meta se renueva con cada movimiento, es lo que le da inercia al seguimiento.
    pub const SEGUIR: f32 = 0.24;

    pub fn new() -> Self {
        Motor {
            mirar: Mirar::LIBRE,
            mirar_antes: Mirar::LIBRE,
            mirar_desde: f64::NEG_INFINITY,
            mirar_duracion: Self::SEGUIR,
            estado: Estado::Reposo,
            estado_desde: f64::NEG_INFINITY,
            anterior: Estado::Reposo,
            anterior_desde: f64::NEG_INFINITY,
            congelada: None,
            golpe: Golpe::NINGUNO,
            rebote: f64::NEG_INFINITY,
            figura: Figura::nook(),
        }
    }

    /// Cambia la figura del personaje (se ve en el siguiente cuadro).
    pub fn poner_figura(&mut self, figura: Arc<Figura>) {
        if !Arc::ptr_eq(&self.figura, &figura) {
            self.figura = figura;
        }
    }

    /// Nueva meta de la mirada a partir del instante `ahora`, alcanzada en `duracion` segundos.
    /// Parte de la mirada que se ve en ese instante (no de la meta anterior): si no, con cada
    /// movimiento del cursor retrocedería un poco y temblaría. Una meta no finita se ignora.
    pub fn poner_mirada(&mut self, mirar: Mirar, ahora: f64, duracion: f32) {
        if !mirar.es_finito() || mirar == self.mirar {
            return;
        }
        self.mirar_antes = self.mirada_en(ahora);
        self.mirar = mirar;
        self.mirar_desde = ahora;
        self.mirar_duracion = duracion.max(0.001);
    }

    fn mirada_en(&self, ahora: f64) -> Mirar {
        let k = ((ahora - self.mirar_desde) / self.mirar_duracion as f64) as f32;
        if k >= 1.0 { self.mirar } else { self.mirar_antes.hacia(self.mirar, suavizar(k)) }
    }

    pub fn estado(&self) -> Estado {
        self.estado
    }

    /// Pasa al estado `estado` en el instante `ahora` (volver a pedir el mismo no hace nada;
    /// para repetir un aviso, se pasa antes por otro estado). Si llega en mitad de una
    /// transición, parte de la pose que se ve en ese momento, así no hay saltos.
    pub fn poner_estado(&mut self, estado: Estado, ahora: f64) {
        if estado == self.estado {
            return;
        }
        let en_transicion = ahora - self.estado_desde < TRANSICION;
        self.congelada = en_transicion.then(|| self.pose_del_estado(ahora));
        self.anterior = self.estado;
        self.anterior_desde = self.estado_desde;
        self.estado = estado;
        self.estado_desde = ahora;
    }

    /// Repite el estado actual desde el principio (otro aviso mientras se ve uno).
    pub fn reiniciar_estado(&mut self, ahora: f64) {
        self.congelada = Some(self.pose_del_estado(ahora));
        self.anterior = self.estado;
        self.anterior_desde = self.estado_desde;
        self.estado_desde = ahora;
    }

    /// Golpe del ritmo de la música en el instante `ahora`, con fuerza de 0 a 1.
    pub fn golpe(&mut self, ahora: f64, fuerza: f32) {
        if fuerza.is_finite() {
            self.golpe = Golpe { cuando: ahora, fuerza: fuerza.clamp(0.0, 1.0) };
        }
    }

    /// Clic en el personaje en el instante `ahora`: da un rebote, haga lo que haga. Mientras
    /// está en el aire no se vuelve a empezar (daría un salto brusco); ya asentándose, sí.
    pub fn rebotar(&mut self, ahora: f64) {
        if ahora - self.rebote >= estados::REBOTE_EN_EL_AIRE as f64 {
            self.rebote = ahora;
        }
    }

    fn pose_en(&self, ahora: f64) -> Pose {
        estados::con_rebote(self.pose_del_estado(ahora), (ahora - self.rebote) as f32)
    }

    fn pose_del_estado(&self, ahora: f64) -> Pose {
        let pose = |estado, desde: f64| estados::pose(estado, (ahora - desde).max(0.0) as f32, ahora, self.golpe);
        let actual = pose(self.estado, self.estado_desde);
        let k = ((ahora - self.estado_desde) / TRANSICION) as f32;
        if k >= 1.0 {
            return actual;
        }
        let origen = self.congelada.unwrap_or_else(|| pose(self.anterior, self.anterior_desde));
        origen.hacia(actual, suavizar(k))
    }

    /// Cómo está el personaje en el instante `ahora`: su pose, la vida en reposo, hacia dónde
    /// mira y su cabeza.
    fn colocar(&self, ahora: f64) -> (Pose, cara::Vida, Mirada, Cabeza) {
        let pose = self.pose_en(ahora);
        // Las figuras de imágenes no tienen cara que girar: miran siempre de frente.
        let mirar = if self.figura.mira() { self.mirada_en(ahora) } else { Mirar { deriva: 0.0, ..Mirar::LIBRE } };
        let vida = cara::vida(ahora, mirar.deriva);
        // La dirección guiada sustituye a la de reposo; la deriva se suma después, para que
        // siga viva con la cabeza girada. Al salir la pastilla, la mira un momento.
        let mut mirada = Mirada {
            guinada: mezclar(MIRADA_REPOSO.guinada, mirar.guinada, mirar.mezcla) + vida.mirada.guinada,
            cabeceo: mezclar(MIRADA_REPOSO.cabeceo, mirar.cabeceo, mirar.mezcla) + vida.mirada.cabeceo,
            alabeo: MIRADA_REPOSO.alabeo + vida.mirada.alabeo + pose.alabeo,
        };
        mirada.guinada = mezclar(mirada.guinada, MIRADA_PASTILLA.guinada, pose.mirar_pastilla);
        mirada.cabeceo = mezclar(mirada.cabeceo, MIRADA_PASTILLA.cabeceo, pose.mirar_pastilla);
        // La cabeza no gira más de lo que diga la figura (las pupilas siguen mirando, ver `muestrear`).
        let [giro, cabeceo] = self.figura.giro.map(|g| g.unwrap_or(f32::INFINITY));
        mirada.guinada = mirada.guinada.clamp(-giro, giro);
        mirada.cabeceo = mirada.cabeceo.clamp(-cabeceo, cabeceo);

        // El cuerpo se inclina un poco hacia donde mira, y el estado lo mueve, estira y gira.
        let dx = mirada.guinada.to_radians().sin() * INCLINACION + vida.deriva_x + pose.dx;
        let dy = -mirada.cabeceo.to_radians().sin() * INCLINACION * 0.6 + vida.deriva_y + pose.dy;
        let (sx, sy) = (pose.sx, pose.sy * vida.respiro);
        let cabeza = Cabeza { figura: self.figura.clone(), ejes: cara::ejes(mirada), giro: pose.giro, sx, sy, dx, dy };
        (pose, vida, mirada, cabeza)
    }

    /// Cuánto se aparta el pelo de su sitio en el instante `ahora` (ver `CALMA_PELO`): la
    /// respuesta de un muelle amortiguado a lo que se ha movido lo alto de la cabeza.
    fn pelo(&self, ahora: f64) -> Punto {
        let alto = -self.figura.medidas.arriba * 0.7;
        let donde: Vec<Punto> = (0..=MUESTRAS_PELO)
            .map(|k| {
                let (_, _, _, cabeza) = self.colocar(ahora - k as f64 * PASO_PELO);
                cabeza.en_cara((0.0, alto)).0
            })
            .collect();
        let (mut x, mut y) = (0.0, 0.0);
        for k in 0..MUESTRAS_PELO {
            let t = k as f32 * PASO_PELO as f32;
            let peso = (-t / CALMA_PELO).exp() * (std::f32::consts::TAU * RITMO_PELO * t).cos();
            // Lo que se movió la cabeza en ese paso, hacia atrás: el pelo va hacia el otro lado.
            x += (donde[k + 1].0 - donde[k].0) * peso;
            y += (donde[k + 1].1 - donde[k].1) * peso;
        }
        let tope = |v: f32| v.clamp(-TOPE_PELO, TOPE_PELO);
        // En vertical solo se levanta (al caer): si bajara, dejaría ver lo alto de la cabeza.
        (tope(x * FUERZA_PELO), tope(y * FUERZA_PELO * FUERZA_PELO_VERTICAL).min(0.0))
    }

    /// El cuadro del instante `ahora` (en segundos).
    pub fn muestrear(&self, ahora: f64) -> Cuadro {
        let (pose, vida, mirada, cabeza) = self.colocar(ahora);
        let medidas = self.figura.medidas;
        // Un rasgo de la cara en pantalla: su centro es donde está en reposo llevado a la cara,
        // igual que lo que se pinta en ella (así el antifaz y las gafas van siempre con los
        // ojos), y sus ejes, los del plano tangente de la esfera en ese punto, girados y
        // estirados con el cuerpo. `aplastar` lo aplasta en vertical (el parpadeo).
        let sobre_cuerpo = |p: &cara::Proyeccion, reposo: Punto, aplastar: f32| {
            let (a, b) = cabeza.al_cuerpo((p.a, p.b * aplastar));
            let (cc, d) = cabeza.al_cuerpo((p.c, p.d * aplastar));
            let ((e, f), _) = cabeza.en_cara(reposo);
            [a, b, cc, d, e, f]
        };
        let alfa = |p: &cara::Proyeccion| (p.profundidad / 0.12).clamp(0.0, 1.0);

        let (ojos, boca) = if self.figura.mira() {
            let ojos = [-1.0f32, 1.0]
                .into_iter()
                .map(|lado| {
                    let reposo = (lado * medidas.ojo.0, medidas.ojo.1);
                    let (lateral, vertical) = angulos(&self.figura, reposo);
                    (lado, reposo, cara::proyectar(mirada, lateral, vertical))
                })
                .filter(|(_, _, ojo)| ojo.profundidad > 0.02)
                .map(|(lado, reposo, ojo)| Ojo {
                    lado,
                    ancho: pose.ojo_ancho,
                    alto: pose.ojo_alto,
                    arco: pose.ojo_arco,
                    // Con párpados, parpadea bajándolos; sin ellos, aplastando el ojo.
                    matriz: sobre_cuerpo(&ojo, reposo, if self.figura.parpado.is_some() { 1.0 } else { cara::aplastado(vida.parpado) }),
                    alfa: alfa(&ojo),
                    // Con la cabeza limitada, las pupilas llegan al borde justo en su límite.
                    pupila: (
                        (mirada.guinada / self.figura.giro[0].unwrap_or(PUPILA_GUINADA).max(1.0)).clamp(-1.0, 1.0),
                        (-mirada.cabeceo / self.figura.giro[1].unwrap_or(PUPILA_CABECEO).max(1.0)).clamp(-1.0, 1.0),
                    ),
                    parpado: self.figura.parpado.map_or(0.0, |cierre| cierre + (1.0 - cierre) * (1.0 - vida.parpado.clamp(0.0, 1.0))),
                })
                .collect();
            let (lateral, vertical) = angulos(&self.figura, medidas.boca);
            let boca = cara::proyectar(mirada, lateral, vertical);
            let boca = (boca.profundidad > 0.02).then(|| Boca { matriz: sobre_cuerpo(&boca, medidas.boca, 1.0), alfa: alfa(&boca) });
            (ojos, boca)
        } else {
            (Vec::new(), None)
        };

        let pastilla = (pose.pastilla > 0.01).then(|| {
            let r = self.figura.radio_en(ANGULO_PASTILLA) * 0.92;
            let (x, y) = cabeza.cuerpo((r * ANGULO_PASTILLA.cos(), r * ANGULO_PASTILLA.sin()));
            let radio = RADIO_PASTILLA * pose.pastilla;
            Pastilla { x, y, radio, muesca: radio + MUESCA * pose.pastilla.min(1.0) }
        });

        let pelo = if self.figura.muelle.is_some() { self.pelo(ahora) } else { (0.0, 0.0) };
        let fotograma = self.fotograma(ahora);
        Cuadro { ojos, boca, pastilla, cabeza, pelo, fotograma }
    }

    /// En una figura de imágenes, la animación que toca y su fotograma: la del clic mientras
    /// rebota (si tiene), si no la de lo que esté haciendo. El aviso empieza desde su primer
    /// fotograma; las demás van con el reloj.
    fn fotograma(&self, ahora: f64) -> Option<(Animacion, usize)> {
        let Dibujo::Imagenes(imagenes) = &self.figura.dibujo else { return None };
        let desde_clic = ahora - self.rebote;
        let (animacion, t) = if imagenes.tiene(Animacion::Clic) && (0.0..estados::DURACION_REBOTE as f64).contains(&desde_clic) {
            (Animacion::Clic, desde_clic)
        } else {
            match self.estado {
                Estado::Reposo => (Animacion::Reposo, ahora),
                Estado::Aviso => (Animacion::Aviso, ahora - self.estado_desde),
                Estado::Musica => (Animacion::Musica, ahora),
            }
        };
        let total = imagenes.fotogramas(animacion).len().max(1);
        Some((animacion, (t.max(0.0) * imagenes.fps as f64) as usize % total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::personaje::figura::TipoCapa;

    fn seguir(guinada: f32, cabeceo: f32) -> Mirar {
        Mirar { guinada, cabeceo, mezcla: 1.0, deriva: 0.0 }
    }

    #[test]
    fn muestrear_no_cambia_nada() {
        let mut motor = Motor::new();
        motor.poner_mirada(seguir(20.0, -5.0), 1.0, Motor::SEGUIR);
        motor.poner_estado(Estado::Aviso, 1.0);
        let a = motor.muestrear(1.1);
        let _ = motor.muestrear(5.0);
        assert_eq!(a, motor.muestrear(1.1));
    }

    #[test]
    fn la_mirada_llega_a_la_meta_sin_saltos() {
        let mut motor = Motor::new();
        motor.poner_mirada(seguir(30.0, 0.0), 0.0, 0.24);
        assert_eq!(motor.mirada_en(0.0), Mirar::LIBRE);
        assert_eq!(motor.mirada_en(0.3), seguir(30.0, 0.0));
        // Un cambio a medio camino parte de lo que se ve, no de la meta anterior.
        let visto = motor.mirada_en(0.1);
        motor.poner_mirada(seguir(-30.0, 0.0), 0.1, 0.24);
        assert_eq!(motor.mirada_en(0.1), visto);
    }

    #[test]
    fn una_meta_no_finita_se_ignora() {
        let mut motor = Motor::new();
        motor.poner_mirada(seguir(f32::NAN, 0.0), 0.0, 0.24);
        assert_eq!(motor.mirada_en(1.0), Mirar::LIBRE);
    }

    #[test]
    fn los_ojos_siguen_hacia_donde_mira() {
        let mut motor = Motor::new();
        motor.poner_mirada(seguir(30.0, 0.0), 0.0, 0.24);
        let derecha = motor.muestrear(1.0);
        motor.poner_mirada(seguir(-30.0, 0.0), 1.0, 0.24);
        let izquierda = motor.muestrear(2.0);
        let centro = |c: &Cuadro| c.ojos.iter().map(|o| o.matriz[4]).sum::<f32>() / c.ojos.len() as f32;
        assert!(centro(&derecha) > 0.2 && centro(&izquierda) < -0.2);
    }

    #[test]
    fn la_cabeza_no_gira_mas_de_su_limite_pero_las_pupilas_siguen() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><circle id="cuerpo" r="40"/><circle id="ojo" cx="15" r="5"/></svg>"##;
        let figura = Figura::desde("x".into(), Some(r#"{"giro": 20, "cabeceo": 10}"#), Some(svg), &|_| Err(String::new())).unwrap();
        assert_eq!(figura.giro, [Some(20.0), Some(10.0)]);
        let figura = Arc::new(figura);
        // Dos iguales, en el mismo instante: uno mirando justo al límite y otro más allá.
        let mirando = |guinada: f32, cabeceo: f32| {
            let mut motor = Motor::new();
            motor.poner_figura(figura.clone());
            motor.poner_mirada(seguir(guinada, cabeceo), 0.0, 0.001);
            motor.muestrear(1.0)
        };
        let centro = |c: &Cuadro| c.ojos.iter().map(|o| o.matriz[4]).sum::<f32>() / c.ojos.len() as f32;
        let (en_el_limite, mas_alla) = (mirando(20.0, 10.0), mirando(35.0, 26.0));
        assert!((centro(&mas_alla) - centro(&en_el_limite)).abs() < 1e-3, "la cara se queda en el límite");
        assert!(mas_alla.ojos.iter().all(|o| o.pupila.0 > 0.99 && o.pupila.1 < -0.99), "las pupilas, al borde");
    }

    #[test]
    fn las_pupilas_miran_hacia_donde_mira() {
        let mut motor = Motor::new();
        motor.poner_mirada(seguir(35.0, 26.0), 0.0, 0.001);
        assert!(motor.muestrear(1.0).ojos.iter().all(|o| o.pupila.0 > 0.9 && o.pupila.1 < -0.9));
        motor.poner_mirada(seguir(-17.5, 0.0), 1.0, 0.001);
        assert!(motor.muestrear(2.0).ojos.iter().all(|o| (o.pupila.0 + 0.5).abs() < 0.1 && o.pupila.1.abs() < 0.1));
    }

    #[test]
    fn mirando_a_los_extremos_los_ojos_quedan_dentro_del_cuerpo() {
        for figura in Figura::integradas() {
            let mut motor = Motor::new();
            motor.poner_figura(figura.clone());
            for (g, c) in [(35.0, 0.0), (-35.0, 0.0), (0.0, 28.0), (0.0, -28.0), (30.0, 24.0), (-30.0, -24.0)] {
                motor.poner_mirada(seguir(g, c), 0.0, 0.001);
                for ojo in motor.muestrear(10.0).ojos {
                    let [a, b, cc, d, e, f] = ojo.matriz;
                    // Esquinas del óvalo (por fuera), comprobadas contra el radio en su dirección.
                    for (x, y) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                        let (x, y) = (x * ojo.ancho / 2.0, y * ojo.alto / 2.0);
                        let (px, py) = (a * x + cc * y + e, b * x + d * y + f);
                        let limite = figura.radio_en(cara::direccion(px, py)) * 1.04;
                        assert!(px.hypot(py) < limite, "ojo de {} fuera mirando a ({g}, {c})", figura.id);
                    }
                }
            }
        }
    }

    #[test]
    fn en_reposo_los_rasgos_de_cada_figura_quedan_donde_sus_medidas() {
        for figura in Figura::integradas() {
            let mut motor = Motor::new();
            motor.poner_figura(figura.clone());
            motor.poner_mirada(seguir(0.0, 0.0), 0.0, 0.001);
            let cuadro = motor.muestrear(0.5);
            let m = figura.medidas;
            for ojo in &cuadro.ojos {
                assert!((ojo.matriz[4] - ojo.lado * m.ojo.0).abs() < 0.02 && (ojo.matriz[5] - m.ojo.1).abs() < 0.02, "{}", figura.id);
            }
            let boca = cuadro.boca.expect("boca");
            assert!((boca.matriz[4] - m.boca.0).abs() < 0.02 && (boca.matriz[5] - m.boca.1).abs() < 0.02, "{}", figura.id);
            assert!(Arc::ptr_eq(&cuadro.cabeza.figura, &figura));
        }
    }

    /// Dónde arranca el rulo de Nook en un cuadro.
    fn arranque_del_rulo(cuadro: &Cuadro) -> Punto {
        let rulo = cuadro.cabeza.figura.capas().iter().find_map(|c| match c.tipo {
            TipoCapa::Cabeza(a) => Some(a),
            _ => None,
        });
        cuadro.cabeza.en_cuerpo(rulo.expect("rulo").ancla, (0.0, 0.0))
    }

    #[test]
    fn en_reposo_la_cara_y_el_rulo_quedan_como_en_el_dibujo() {
        let mut motor = Motor::new();
        motor.poner_mirada(seguir(0.0, 0.0), 0.0, 0.001);
        let cuadro = motor.muestrear(0.5);
        let (izq, der) = (cuadro.ojos[0].matriz, cuadro.ojos[1].matriz);
        assert!((izq[4] + 13.0 / UNIDAD).abs() < 0.02 && (der[4] - 13.0 / UNIDAD).abs() < 0.02);
        let boca = cuadro.boca.expect("boca");
        assert!(boca.matriz[4].abs() < 0.02 && (boca.matriz[5] - 15.0 / UNIDAD).abs() < 0.02);
        // El rulo arranca dentro del cuerpo, en lo alto.
        let (x, y) = arranque_del_rulo(&cuadro);
        assert!(x.abs() < 0.03 && (y + 36.0 / UNIDAD).abs() < 0.03, "({x}, {y})");
    }

    #[test]
    fn al_girar_la_cabeza_el_rulo_y_la_boca_van_hacia_ese_lado() {
        let mut motor = Motor::new();
        let quieto = motor.muestrear(0.0);
        motor.poner_mirada(seguir(30.0, 0.0), 0.0, 0.001);
        let girado = motor.muestrear(1.0);
        assert!(arranque_del_rulo(&girado).0 > arranque_del_rulo(&quieto).0 + 0.1);
        assert!(girado.boca.expect("boca").matriz[4] > 0.2);
    }

    #[test]
    fn los_ojos_van_donde_lo_pintado_en_la_cara_los_espera() {
        // El antifaz y las gafas se colocan con `en_cara`: los ojos tienen que caer en el mismo
        // sitio mire donde mire, o asomarían por los bordes.
        for figura in Figura::integradas() {
            let mut motor = Motor::new();
            motor.poner_figura(figura.clone());
            let ojo = figura.medidas.ojo;
            for (g, c) in [(0.0, 0.0), (0.0, -26.0), (0.0, 26.0), (30.0, -20.0), (-35.0, 15.0)] {
                motor.poner_mirada(seguir(g, c), 0.0, 0.001);
                let cuadro = motor.muestrear(10.0);
                for o in &cuadro.ojos {
                    let ((ex, ey), _) = cuadro.cabeza.en_cara((o.lado * ojo.0, ojo.1));
                    assert!((o.matriz[4] - ex).abs() < 1e-5 && (o.matriz[5] - ey).abs() < 1e-5, "ojo de {} fuera de sitio mirando a ({g}, {c})", figura.id);
                }
            }
        }
    }

    #[test]
    fn el_pelo_se_queda_atras_al_moverse_y_luego_se_asienta() {
        let mut motor = Motor::new();
        motor.poner_figura(Figura::limon());
        motor.poner_mirada(seguir(0.0, 0.0), 0.0, 0.001);
        assert!(motor.muestrear(5.0).pelo.0.abs() < 1e-3, "quieto, el pelo está en su sitio");
        // Gira la cabeza a la derecha: el pelo se queda atrás (a la izquierda)…
        motor.poner_mirada(seguir(35.0, 0.0), 5.0, 0.24);
        assert!(motor.muestrear(5.15).pelo.0 < -0.03);
        // …se pasa al otro lado al frenar y se asienta.
        let mayor = (0..40).map(|i| motor.muestrear(5.3 + i as f64 * 0.01).pelo.0).fold(f32::MIN, f32::max);
        assert!(mayor > 0.005, "no se pasa: {mayor}");
        assert!(motor.muestrear(7.0).pelo.0.abs() < 1e-3);
        // Nook no tiene pelo.
        let mut nook = Motor::new();
        nook.poner_mirada(seguir(35.0, 0.0), 5.0, 0.24);
        assert_eq!(nook.muestrear(5.15).pelo, (0.0, 0.0));
    }

    #[test]
    fn el_aviso_saca_la_pastilla_y_al_volver_al_reposo_se_va() {
        let mut motor = Motor::new();
        assert!(motor.muestrear(0.5).pastilla.is_none());
        motor.poner_estado(Estado::Aviso, 1.0);
        let pastilla = motor.muestrear(3.0).pastilla.expect("pastilla");
        // Arriba a la derecha, sobre el contorno.
        assert!(pastilla.x > 0.3 && pastilla.y < -0.3);
        assert!(pastilla.muesca > pastilla.radio);
        motor.poner_estado(Estado::Reposo, 5.0);
        assert!(motor.muestrear(6.0).pastilla.is_none());
    }

    #[test]
    fn cambiar_de_estado_a_mitad_de_transicion_no_salta() {
        let mut motor = Motor::new();
        motor.poner_estado(Estado::Musica, 0.0);
        motor.golpe(0.05, 1.0);
        let antes = motor.muestrear(0.15);
        motor.poner_estado(Estado::Aviso, 0.15);
        let despues = motor.muestrear(0.15);
        assert!(antes.casi_igual(&despues, 1e-4));
    }
}
