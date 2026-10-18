//! Geometría básica del personaje: puntos en unidades del radio del cuerpo y curvas cúbicas.

/// Un punto en unidades del radio del cuerpo (x a la derecha, y hacia abajo).
pub type Punto = (f32, f32);

/// Punto de una curva cúbica en `t` (0–1).
pub fn cubica(p0: Punto, c1: Punto, c2: Punto, p3: Punto, t: f32) -> Punto {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (a * p0.0 + b * c1.0 + c * c2.0 + d * p3.0, a * p0.1 + b * c1.1 + c * c2.1 + d * p3.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_cubica_pasa_por_sus_extremos() {
        let (p0, p3) = ((0.0, 0.0), (1.0, 2.0));
        assert_eq!(cubica(p0, (0.3, 1.0), (0.7, 1.0), p3, 0.0), p0);
        assert_eq!(cubica(p0, (0.3, 1.0), (0.7, 1.0), p3, 1.0), p3);
    }
}
