//! Lo que hace el personaje en cada estado, como una pose que depende del tiempo transcurrido
//! en él. El motor mezcla la pose del estado anterior con la del nuevo para pasar de uno a otro.

use std::f32::consts::{PI, TAU};

/// Qué está haciendo el personaje.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    /// Quieto: mira, parpadea y respira.
    Reposo,
    /// Ha llegado una notificación: da un salto, pone ojos de sorpresa, le sale una pastilla y
    /// la mira.
    Aviso,
    /// Suena música: se balancea, entorna los ojos y salta con cada golpe del ritmo.
    Musica,
}

/// Último golpe del ritmo: cuándo (en el reloj del motor) y con qué fuerza (0–1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Golpe {
    pub cuando: f64,
    pub fuerza: f32,
}

impl Golpe {
    pub const NINGUNO: Golpe = Golpe { cuando: f64::NEG_INFINITY, fuerza: 0.0 };
}

/// Lo que un estado cambia respecto al personaje quieto.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// Desplazamiento del cuerpo, en unidades de radio (y hacia abajo).
    pub dx: f32,
    pub dy: f32,
    /// Estirado horizontal y vertical.
    pub sx: f32,
    pub sy: f32,
    /// Giro del cuerpo (con la cara), en radianes.
    pub giro: f32,
    /// Inclinación de la cabeza que se suma a la mirada, en grados.
    pub alabeo: f32,
    /// Tamaño de los ojos, en unidades de radio.
    pub ojo_ancho: f32,
    pub ojo_alto: f32,
    /// Cuánto se convierten los ojos en arcos de alegría «∩» (0 = óvalos).
    pub ojo_arco: f32,
    /// Tamaño de la pastilla de aviso (0 = no hay).
    pub pastilla: f32,
    /// Cuánto mira hacia la pastilla en lugar de hacia donde mire (0–1).
    pub mirar_pastilla: f32,
}

/// Tamaño de los ojos en reposo, en unidades del radio del cuerpo.
pub const OJO_ANCHO: f32 = 0.163;
pub const OJO_ALTO: f32 = 0.286;

impl Pose {
    pub const REPOSO: Pose = Pose {
        dx: 0.0,
        dy: 0.0,
        sx: 1.0,
        sy: 1.0,
        giro: 0.0,
        alabeo: 0.0,
        ojo_ancho: OJO_ANCHO,
        ojo_alto: OJO_ALTO,
        ojo_arco: 0.0,
        pastilla: 0.0,
        mirar_pastilla: 0.0,
    };

    pub fn hacia(self, otra: Pose, t: f32) -> Pose {
        let m = |a: f32, b: f32| a + (b - a) * t;
        Pose {
            dx: m(self.dx, otra.dx),
            dy: m(self.dy, otra.dy),
            sx: m(self.sx, otra.sx),
            sy: m(self.sy, otra.sy),
            giro: m(self.giro, otra.giro),
            alabeo: m(self.alabeo, otra.alabeo),
            ojo_ancho: m(self.ojo_ancho, otra.ojo_ancho),
            ojo_alto: m(self.ojo_alto, otra.ojo_alto),
            ojo_arco: m(self.ojo_arco, otra.ojo_arco),
            pastilla: m(self.pastilla, otra.pastilla),
            mirar_pastilla: m(self.mirar_pastilla, otra.mirar_pastilla),
        }
    }
}

/// De 0 a 1 entre `a` y `b`, con arranque y final suaves.
fn rampa(t: f32, a: f32, b: f32) -> f32 {
    let k = ((t - a) / (b - a)).clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

/// Pose del estado `estado` a los `t` segundos de empezar. `ahora` y `golpe` son para el ritmo
/// de la música.
pub fn pose(estado: Estado, t: f32, ahora: f64, golpe: Golpe) -> Pose {
    match estado {
        Estado::Reposo => Pose::REPOSO,
        Estado::Aviso => aviso(t),
        Estado::Musica => musica(t, ahora, golpe),
    }
}

/// Se agacha, salta, cae aplastándose y se recupera con un pequeño rebote. Mientras, abre mucho
/// los ojos, le sale la pastilla (crece pasándose un poco y vuelve) y la mira un momento.
fn aviso(t: f32) -> Pose {
    let mut p = Pose::REPOSO;
    if t < 0.1 {
        let k = t / 0.1;
        (p.sx, p.sy, p.dy) = (1.0 + 0.07 * k, 1.0 - 0.1 * k, 0.05 * k);
    } else if t < 0.5 {
        let u = (t - 0.1) / 0.4;
        let s = (PI * u).sin();
        let recupera = (u * 4.0).min(1.0);
        p.dy = 0.05 * (1.0 - recupera) - 0.22 * s;
        p.sx = 1.07 + (1.0 - 1.07) * recupera - 0.04 * s;
        p.sy = 0.9 + 0.1 * recupera + 0.05 * s;
    } else if t < 0.68 {
        let s = (PI * (t - 0.5) / 0.18).sin();
        (p.sx, p.sy, p.dy) = (1.0 + 0.06 * s, 1.0 - 0.09 * s, 0.03 * s);
    } else {
        let d = t - 0.68;
        let temblor = (-d * 5.0).exp() * (d * 18.0).sin();
        (p.sx, p.sy) = (1.0 - 0.02 * temblor, 1.0 + 0.025 * temblor);
    }

    let sorpresa = rampa(t, 0.05, 0.2) * (1.0 - rampa(t, 1.4, 1.8));
    p.ojo_ancho = OJO_ANCHO + (0.204 - OJO_ANCHO) * sorpresa;
    p.ojo_alto = OJO_ALTO + (0.367 - OJO_ALTO) * sorpresa;

    let q = t - 0.3;
    p.pastilla = if q < 0.0 {
        0.0
    } else if q < 0.18 {
        1.14 * (1.0 - (1.0 - q / 0.18).powi(3))
    } else {
        1.14 - 0.14 * rampa(q, 0.18, 0.36)
    };
    p.mirar_pastilla = rampa(t, 0.3, 0.5) * (1.0 - rampa(t, 1.6, 2.1));
    p
}

/// Lo que dura el rebote al hacer clic en el personaje.
pub const DURACION_REBOTE: f32 = 1.4;
/// Hasta cuándo está en el aire o rebotando (luego solo se asienta con un temblor).
pub const REBOTE_EN_EL_AIRE: f32 = 0.88;

/// Rebote al hacer clic en el personaje, a los `t` segundos del clic: se agacha, salta, cae
/// aplastándose, da un saltito más pequeño y se asienta con un temblor, con ojos de alegría.
/// Se suma a la pose del estado en el que esté (ver `con_rebote`).
fn rebote(t: f32) -> Pose {
    let mut p = Pose::REPOSO;
    if t < 0.08 {
        let k = t / 0.08;
        (p.sx, p.sy, p.dy) = (1.0 + 0.08 * k, 1.0 - 0.12 * k, 0.06 * k);
    } else if t < 0.43 {
        let u = (t - 0.08) / 0.35;
        let s = (PI * u).sin();
        let recupera = (u * 4.0).min(1.0);
        p.dy = 0.06 * (1.0 - recupera) - 0.28 * s;
        p.sx = 1.08 + (1.0 - 1.08) * recupera - 0.05 * s;
        p.sy = 0.88 + 0.12 * recupera + 0.07 * s;
    } else if t < 0.55 {
        let s = (PI * (t - 0.43) / 0.12).sin();
        (p.sx, p.sy, p.dy) = (1.0 + 0.1 * s, 1.0 - 0.12 * s, 0.05 * s);
    } else if t < 0.8 {
        let s = (PI * (t - 0.55) / 0.25).sin();
        (p.sx, p.sy, p.dy) = (1.0 - 0.02 * s, 1.0 + 0.03 * s, -0.09 * s);
    } else if t < 0.88 {
        let s = (PI * (t - 0.8) / 0.08).sin();
        (p.sx, p.sy, p.dy) = (1.0 + 0.04 * s, 1.0 - 0.05 * s, 0.02 * s);
    } else {
        let d = t - 0.88;
        let temblor = (-d * 7.0).exp() * (d * 20.0).sin();
        (p.sx, p.sy) = (1.0 - 0.015 * temblor, 1.0 + 0.02 * temblor);
    }
    p.ojo_arco = rampa(t, 0.06, 0.18) * (1.0 - rampa(t, 0.95, DURACION_REBOTE));
    p
}

/// Suma el rebote de un clic de hace `t` segundos a la pose de un estado.
pub fn con_rebote(mut pose: Pose, t: f32) -> Pose {
    if !(0.0..DURACION_REBOTE).contains(&t) {
        return pose;
    }
    let r = rebote(t);
    pose.dy += r.dy;
    pose.sx *= r.sx;
    pose.sy *= r.sy;
    pose.ojo_arco = pose.ojo_arco.max(r.ojo_arco);
    pose
}

/// Se balancea a un compás por segundo con la cabeza inclinándose y ojos de alegría, y salta
/// con cada golpe del ritmo. Si no llegan golpes (música muy suave), rebota solo a dos por
/// segundo.
fn musica(t: f32, ahora: f64, golpe: Golpe) -> Pose {
    let mut p = Pose::REPOSO;
    let vaiven = (t * TAU).sin();
    p.giro = 0.06 * vaiven;
    p.dx = 0.03 * vaiven;
    p.alabeo = 5.0 * vaiven;
    p.ojo_ancho = 0.19;
    p.ojo_alto = 0.19;
    p.ojo_arco = 1.0;

    let desde_golpe = (ahora - golpe.cuando) as f32;
    if (0.0..1.5).contains(&desde_golpe) {
        if desde_golpe < 0.3 {
            let s = (PI * desde_golpe / 0.3).sin() * golpe.fuerza;
            p.dy = -0.10 * s;
            p.sy = 1.0 + 0.05 * s;
            p.sx = 1.0 - 0.035 * s;
        }
    } else {
        let s = (t * TAU).sin().abs();
        p.dy = -0.05 * s;
        p.sy = 1.0 + 0.02 * s;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_aviso_empieza_y_termina_en_reposo_salvo_la_pastilla() {
        let inicio = aviso(0.0);
        assert_eq!(inicio.dy, 0.0);
        assert_eq!(inicio.pastilla, 0.0);
        let fin = aviso(10.0);
        assert!((fin.sx - 1.0).abs() < 1e-3 && (fin.sy - 1.0).abs() < 1e-3);
        assert_eq!((fin.ojo_ancho, fin.ojo_alto), (OJO_ANCHO, OJO_ALTO));
        assert_eq!(fin.pastilla, 1.0);
        assert_eq!(fin.mirar_pastilla, 0.0);
    }

    #[test]
    fn en_el_aviso_salta_hacia_arriba_y_la_pastilla_se_pasa_un_poco() {
        assert!(aviso(0.3).dy < -0.15);
        let mayor = (0..100).map(|i| aviso(i as f32 * 0.01).pastilla).fold(0.0, f32::max);
        assert!(mayor > 1.1 && mayor <= 1.14 + 1e-4);
    }

    #[test]
    fn el_aviso_no_tiene_saltos_bruscos() {
        let mut anterior = aviso(0.0);
        for i in 1..300 {
            let p = aviso(i as f32 / 100.0);
            assert!((p.dy - anterior.dy).abs() < 0.06, "salto en dy a {i}");
            assert!((p.sy - anterior.sy).abs() < 0.04, "salto en sy a {i}");
            anterior = p;
        }
    }

    #[test]
    fn el_rebote_salta_y_vuelve_a_la_pose_sin_saltos_bruscos() {
        assert!(rebote(0.25).dy < -0.2, "salta hacia arriba");
        assert!(rebote(0.6).dy < -0.03, "da un segundo saltito");
        assert_eq!(con_rebote(Pose::REPOSO, 0.0), Pose::REPOSO);
        assert_eq!(con_rebote(Pose::REPOSO, DURACION_REBOTE), Pose::REPOSO);
        let fin = rebote(DURACION_REBOTE - 0.001);
        assert!((fin.sx - 1.0).abs() < 2e-3 && (fin.sy - 1.0).abs() < 2e-3 && fin.ojo_arco < 1e-2);
        let mut anterior = rebote(0.0);
        for i in 1..140 {
            let p = rebote(i as f32 / 100.0);
            assert!((p.dy - anterior.dy).abs() < 0.06, "salto en dy a {i}");
            assert!((p.sy - anterior.sy).abs() < 0.05, "salto en sy a {i}");
            anterior = p;
        }
    }

    #[test]
    fn con_la_musica_salta_con_cada_golpe() {
        let golpe = Golpe { cuando: 10.0, fuerza: 1.0 };
        assert!(musica(3.0, 10.15, golpe).dy < -0.09);
        // Pasado el salto, queda en el suelo hasta el siguiente golpe.
        assert_eq!(musica(3.0, 10.5, golpe).dy, 0.0);
        // Sin golpes rebota solo.
        assert!(musica(0.25, 100.0, golpe).dy < -0.04);
    }
}
