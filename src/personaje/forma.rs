//! Silueta del personaje como perfil radial: el radio en 64 direcciones fijas alrededor del
//! centro. Al compartir todas las formas las mismas direcciones, sus puntos se corresponden uno a
//! uno y pasar de una a otra es interpolar radios (técnica de bloub, ver `mod.rs`).

use std::f32::consts::TAU;

/// Direcciones en las que se mide el radio.
pub const MUESTRAS: usize = 64;

/// Un punto en unidades del radio del cuerpo (x a la derecha, y hacia abajo).
pub type Punto = (f32, f32);

/// Forma (perfil radial) más su pose: giro, desplazamiento del centro y estirado.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Silueta {
    pub radios: [f32; MUESTRAS],
    /// Giro del perfil, en radianes.
    pub giro: f32,
    /// Desplazamiento del centro, en unidades de radio.
    pub cx: f32,
    pub cy: f32,
    /// Estirado horizontal y vertical, en el plano de la pantalla (después del giro).
    pub sx: f32,
    pub sy: f32,
}

impl Silueta {
    pub fn new(radios: [f32; MUESTRAS]) -> Self {
        Silueta { radios, giro: 0.0, cx: 0.0, cy: 0.0, sx: 1.0, sy: 1.0 }
    }

    /// Contorno en pantalla: los 64 puntos, ya girados, estirados y desplazados.
    pub fn puntos(&self) -> [Punto; MUESTRAS] {
        let (s, c) = self.giro.sin_cos();
        std::array::from_fn(|i| {
            let (sa, ca) = angulo(i).sin_cos();
            let (x, y) = (self.radios[i] * ca, self.radios[i] * sa);
            let (rx, ry) = (x * c - y * s, x * s + y * c);
            (rx * self.sx + self.cx, ry * self.sy + self.cy)
        })
    }
}

fn angulo(i: usize) -> f32 {
    i as f32 / MUESTRAS as f32 * TAU
}

/// Punto de una curva cúbica en `t` (0–1).
pub fn cubica(p0: Punto, c1: Punto, c2: Punto, p3: Punto, t: f32) -> Punto {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (a * p0.0 + b * c1.0 + c * c2.0 + d * p3.0, a * p0.1 + b * c1.1 + c * c2.1 + d * p3.1)
}

/// Perfil de un contorno cerrado de curvas cúbicas que rodea al centro (como un trazado SVG que
/// empieza en `inicio`): se muestrea muy fino y el radio de cada dirección se interpola entre los
/// dos puntos que la rodean.
pub fn perfil(inicio: Punto, tramos: &[(Punto, Punto, Punto)]) -> [f32; MUESTRAS] {
    const PASOS: usize = 200;
    let mut polar = Vec::with_capacity(tramos.len() * PASOS);
    let mut p0 = inicio;
    for &(c1, c2, p3) in tramos {
        for k in 0..PASOS {
            let (x, y) = cubica(p0, c1, c2, p3, k as f32 / PASOS as f32);
            polar.push((y.atan2(x).rem_euclid(TAU), x.hypot(y)));
        }
        p0 = p3;
    }
    polar.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (primero, ultimo) = (polar[0], polar[polar.len() - 1]);
    std::array::from_fn(|i| {
        let a = angulo(i);
        let j = polar.partition_point(|p| p.0 < a);
        let (a0, r0) = if j == 0 { (ultimo.0 - TAU, ultimo.1) } else { polar[j - 1] };
        let (a1, r1) = if j == polar.len() { (primero.0 + TAU, primero.1) } else { polar[j] };
        r0 + (r1 - r0) * (a - a0) / (a1 - a0).max(1e-6)
    })
}

/// Radio del perfil en una dirección cualquiera, interpolando entre las dos muestras vecinas.
/// Sirve para colocar lo que va "sobre" el cuerpo (los ojos) cuando no es un círculo.
pub fn radio_en(radios: &[f32; MUESTRAS], angulo: f32) -> f32 {
    let t = (angulo / TAU).rem_euclid(1.0) * MUESTRAS as f32;
    let i = t.floor() as usize % MUESTRAS;
    let f = t - t.floor();
    radios[i] + (radios[(i + 1) % MUESTRAS] - radios[i]) * f
}

/// Contorno cerrado y suave que pasa por todos los puntos: cada tramo es una curva cúbica con
/// las tangentes de Catmull-Rom. Devuelve, por tramo, (control 1, control 2, punto final); el
/// contorno empieza en el último punto final.
pub fn curvas(puntos: &[Punto]) -> Vec<(Punto, Punto, Punto)> {
    const TENSION: f32 = 1.0 / 6.0;
    let n = puntos.len();
    (0..n)
        .map(|i| {
            let p0 = puntos[(i + n - 1) % n];
            let p1 = puntos[i];
            let p2 = puntos[(i + 1) % n];
            let p3 = puntos[(i + 2) % n];
            let c1 = (p1.0 + (p2.0 - p0.0) * TENSION, p1.1 + (p2.1 - p0.1) * TENSION);
            let c2 = (p2.0 - (p3.0 - p1.0) * TENSION, p2.1 - (p3.1 - p1.1) * TENSION);
            (c1, c2, p2)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_perfil_de_un_circulo_de_curvas_es_constante() {
        // Círculo de radio 1 con cuatro cuartos de curva cúbica.
        let k = 0.552_284_8;
        let tramos = [
            ((1.0, k), (k, 1.0), (0.0, 1.0)),
            ((-k, 1.0), (-1.0, k), (-1.0, 0.0)),
            ((-1.0, -k), (-k, -1.0), (0.0, -1.0)),
            ((k, -1.0), (1.0, -k), (1.0, 0.0)),
        ];
        assert!(perfil((1.0, 0.0), &tramos).iter().all(|r| (r - 1.0).abs() < 1e-3));
    }

    #[test]
    fn el_radio_se_interpola_entre_muestras() {
        let mut radios = [1.0; MUESTRAS];
        radios[1] = 2.0;
        let medio = TAU / MUESTRAS as f32 / 2.0;
        assert!((radio_en(&radios, medio) - 1.5).abs() < 1e-4);
        // Da la vuelta: un ángulo negativo es el mismo que sumándole una vuelta.
        assert!((radio_en(&radios, -TAU + medio) - 1.5).abs() < 1e-3);
    }

    #[test]
    fn las_curvas_cierran_el_contorno() {
        let puntos = Silueta::new([1.0; MUESTRAS]).puntos();
        let tramos = curvas(&puntos);
        assert_eq!(tramos.len(), MUESTRAS);
        // El último tramo termina donde empieza el primero.
        assert_eq!(tramos[MUESTRAS - 1].2, puntos[0]);
    }

    #[test]
    fn la_pose_desplaza_y_estira() {
        let mut s = Silueta::new([1.0; MUESTRAS]);
        s.cx = 0.5;
        s.sy = 0.5;
        let p = s.puntos();
        assert!((p[0].0 - 1.5).abs() < 1e-5);
        assert!((p[MUESTRAS / 4].1 - 0.5).abs() < 1e-5);
    }
}
