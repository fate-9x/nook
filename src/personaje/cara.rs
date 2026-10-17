//! La cara (ojos, boca y el arranque del rulo) y la "vida" en reposo.
//!
//! La cara está pintada sobre una esfera, no puesta plana: cada rasgo toma el plano tangente de
//! la esfera en su punto y se proyecta de frente. Al girar la cabeza, el ojo que se acerca al
//! borde se estrecha e inclina solo, y eso da el volumen (técnica de bloub, ver `mod.rs`).

use std::sync::LazyLock;

pub type Vec3 = [f32; 3];

/// Hacia dónde mira la cabeza, en grados.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Mirada {
    /// Giro horizontal; positivo = mira a la derecha.
    pub guinada: f32,
    /// Giro vertical; positivo = mira hacia arriba.
    pub cabeceo: f32,
    /// Inclinación de la cabeza.
    pub alabeo: f32,
}

/// Un punto de la cara proyectado: su centro (en unidades de radio) y la matriz 2×2 del plano
/// tangente.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Proyeccion {
    pub x: f32,
    pub y: f32,
    /// Ejes del plano tangente en pantalla: (a, b) su horizontal y (c, d) su vertical.
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    /// Componente z de la normal: > 0 es que el punto mira hacia nosotros.
    pub profundidad: f32,
}

/// Gira dos vectores perpendiculares dentro de su plano.
fn girar(u: Vec3, v: Vec3, angulo: f32) -> (Vec3, Vec3) {
    let (s, c) = angulo.sin_cos();
    (
        [u[0] * c + v[0] * s, u[1] * c + v[1] * s, u[2] * c + v[2] * s],
        [v[0] * c - u[0] * s, v[1] * c - u[1] * s, v[2] * c - u[2] * s],
    )
}

/// Ejes de la cabeza orientada según `mirada`: hacia dónde mira la cara, su derecha y su abajo.
/// Ejes de pantalla: x a la derecha, y hacia abajo, z hacia el espectador.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ejes {
    pub frente: Vec3,
    pub derecha: Vec3,
    pub abajo: Vec3,
}

pub fn ejes(mirada: Mirada) -> Ejes {
    let (mut frente, mut derecha, mut abajo) = ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    (frente, derecha) = girar(frente, derecha, mirada.guinada.to_radians());
    (abajo, frente) = girar(abajo, frente, mirada.cabeceo.to_radians());
    (derecha, abajo) = girar(derecha, abajo, mirada.alabeo.to_radians());
    Ejes { frente, derecha, abajo }
}

/// Un punto de la cara, `lateral` grados a la derecha y `vertical` grados hacia abajo del centro
/// de la cara, con la cabeza orientada según `mirada`.
pub fn proyectar(mirada: Mirada, lateral: f32, vertical: f32) -> Proyeccion {
    let Ejes { frente, derecha, abajo } = ejes(mirada);
    let (f, d) = girar(frente, derecha, lateral.to_radians());
    let (v, f) = girar(abajo, f, -vertical.to_radians());
    Proyeccion { x: f[0], y: f[1], a: d[0], b: d[1], c: v[0], d: v[1], profundidad: f[2] }
}

/// Los dos ojos, separados `separacion` grados a cada lado del centro de la cara.
#[cfg(test)]
pub fn poses_ojos(mirada: Mirada, separacion: f32) -> [Proyeccion; 2] {
    [proyectar(mirada, -separacion, 0.0), proyectar(mirada, separacion, 0.0)]
}

/// Lo que la vida en reposo suma a la pose: deriva de la mirada, parpadeos y una respiración
/// mínima.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vida {
    pub mirada: Mirada,
    /// 1 = ojo abierto, 0 = cerrado.
    pub parpado: f32,
    pub deriva_x: f32,
    pub deriva_y: f32,
    /// Factor de alto del cuerpo (respira solo en vertical).
    pub respiro: f32,
}

/// Ruido periódico suave: se repite cada `periodo` sin costura.
fn ruido(t: f64, periodo: f64, semilla: f64) -> f32 {
    let p = t / periodo * std::f64::consts::TAU;
    (0.55 * (p + semilla).sin() + 0.3 * (2.0 * p + semilla * 1.7 + 1.1).sin() + 0.15 * (3.0 * p + semilla * 2.3 + 2.4).sin())
        as f32
}

/// Los parpadeos se reparten de antemano en un calendario que se repite cada `CICLO` segundos:
/// así la vida es una función pura del tiempo, sin estado.
const CICLO: f64 = 900.0;
const DURACION_PARPADEO: f64 = 0.18;
static PARPADEOS: LazyLock<Vec<f64>> = LazyLock::new(|| {
    let mut azar = crate::azar::aleatorio(0x5EED);
    let mut lista = Vec::new();
    let mut t = 1.4;
    while t < CICLO - 5.0 {
        lista.push(t);
        // De 1,9 a 4,6 s entre parpadeos, y a veces uno doble.
        t += 1.9 + azar() as f64 * 2.7;
        if azar() < 0.18 {
            lista.push(t);
            t += 0.24;
        }
    }
    lista
});

fn parpado(t: f64) -> f32 {
    let t = t.rem_euclid(CICLO);
    // El último parpadeo que ha empezado.
    let i = PARPADEOS.partition_point(|&inicio| inicio <= t);
    let Some(&inicio) = i.checked_sub(1).and_then(|i| PARPADEOS.get(i)) else { return 1.0 };
    let k = ((t - inicio) / DURACION_PARPADEO) as f32;
    match k {
        // Se cierra rápido y se abre algo más despacio.
        k if k < 0.45 => 1.0 - k / 0.45,
        k if k <= 1.0 => (k - 0.45) / 0.55,
        _ => 1.0,
    }
}

/// Vida en el instante `t` (en segundos). `deriva` (0–1) es cuánto se mueve la mirada por su
/// cuenta: cuando sigue al cursor no debe sumarse.
pub fn vida(t: f64, deriva: f32) -> Vida {
    // Periodos que no se dividen entre sí: la deriva no se repite a la vista.
    Vida {
        mirada: Mirada {
            guinada: (ruido(t, 11.3, 0.4) * 5.5 + ruido(t, 3.7, 2.1) * 1.6) * deriva,
            cabeceo: (ruido(t, 9.1, 1.3) * 4.2 + ruido(t, 4.3, 0.7) * 1.3) * deriva,
            alabeo: ruido(t, 13.7, 3.2) * 2.2 * deriva,
        },
        parpado: parpado(t),
        deriva_x: ruido(t, 7.9, 1.9) * 0.006,
        deriva_y: ruido(t, 5.3, 0.3) * 0.007,
        respiro: 1.0 + ((t / 3.4 * std::f64::consts::TAU).sin() * 0.008) as f32,
    }
}

/// El parpadeo aplasta el ojo en vertical (en pantalla, no a lo largo de su eje inclinado).
pub fn aplastado(parpado: f32) -> f32 {
    0.06 + 0.94 * parpado.clamp(0.0, 1.0)
}

/// Ángulo de pantalla (radianes) de un punto, para buscar el radio del cuerpo en esa dirección.
#[cfg(test)]
pub fn direccion(x: f32, y: f32) -> f32 {
    y.atan2(x).rem_euclid(std::f32::consts::TAU)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_frente_los_ojos_quedan_simetricos_y_visibles() {
        let [izq, der] = poses_ojos(Mirada::default(), 15.0);
        assert!((izq.x + der.x).abs() < 1e-6);
        assert!((izq.y - der.y).abs() < 1e-6);
        assert!(izq.profundidad > 0.9 && der.profundidad > 0.9);
    }

    #[test]
    fn al_mirar_a_la_derecha_los_ojos_se_mueven_y_el_lejano_se_estrecha() {
        let [izq, der] = poses_ojos(Mirada { guinada: 30.0, ..Default::default() }, 15.0);
        assert!(izq.x > 0.0 && der.x > izq.x);
        // El ojo más cercano al borde se ve más estrecho (su eje horizontal se acorta).
        assert!(der.a.hypot(der.b) < izq.a.hypot(izq.b));
    }

    #[test]
    fn un_punto_por_debajo_del_centro_baja_y_sigue_a_la_cabeza() {
        let boca = proyectar(Mirada::default(), 0.0, 20.0);
        assert!(boca.x.abs() < 1e-6 && (boca.y - 20f32.to_radians().sin()).abs() < 1e-5);
        // Su eje vertical se acorta al alejarse del centro de la cara.
        assert!(boca.d < 1.0 && boca.d > 0.9);
        let girada = proyectar(Mirada { guinada: 30.0, ..Default::default() }, 0.0, 20.0);
        assert!(girada.x > 0.2);
    }

    #[test]
    fn mirar_arriba_sube_los_ojos() {
        let [ojo, _] = poses_ojos(Mirada { cabeceo: 20.0, ..Default::default() }, 15.0);
        assert!(ojo.y < 0.0);
    }

    #[test]
    fn la_vida_es_una_funcion_pura_del_tiempo() {
        assert_eq!(vida(12.34, 1.0), vida(12.34, 1.0));
        // Sin deriva la mirada no se mueve.
        assert_eq!(vida(7.0, 0.0).mirada, Mirada::default());
    }

    #[test]
    fn hay_parpadeos_y_se_repiten_cada_ciclo() {
        let inicio = PARPADEOS[3];
        let medio = inicio + DURACION_PARPADEO * 0.45;
        assert!(parpado(medio) < 0.05);
        assert_eq!(parpado(medio), parpado(medio + CICLO));
        // Entre parpadeos el ojo está abierto.
        assert_eq!(parpado(inicio - 0.01), 1.0);
    }
}
