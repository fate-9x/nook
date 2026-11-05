//! Números pseudoaleatorios (mulberry32): una secuencia fija a partir de una semilla, y otra que
//! empieza en la hora de arranque para lo que tiene que cambiar cada vez (los addons).

use std::cell::Cell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Generador determinista: da siempre la misma secuencia de números entre 0 y 1.
pub fn aleatorio(semilla: u32) -> impl FnMut() -> f32 {
    let mut a = semilla;
    move || paso(&mut a) as f32 / 4_294_967_296.0
}

fn paso(a: &mut u32) -> u32 {
    *a = a.wrapping_add(0x6D2B_79F5);
    let mut t = (*a ^ (*a >> 15)).wrapping_mul(1 | *a);
    t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
    t ^ (t >> 14)
}

/// Para que dos hilos que empiezan a la vez no tengan la misma secuencia (las pruebas, que van en
/// paralelo, sacan de ella los nombres de sus carpetas temporales).
static HILOS: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static ESTADO: Cell<u32> = Cell::new(
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0x5EED, |d| d.as_nanos() as u32)
            ^ HILOS.fetch_add(0x9E37_79B9, Ordering::Relaxed),
    );
}

/// Siguiente número entre 0 (incluido) y 1 (sin incluir) de la secuencia que cambia cada vez.
pub fn siguiente() -> f64 {
    ESTADO.with(|estado| {
        let mut a = estado.get();
        let n = paso(&mut a);
        estado.set(a);
        n as f64 / 4_294_967_296.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_secuencia_fija_se_repite() {
        let (mut a, mut b) = (aleatorio(7), aleatorio(7));
        for _ in 0..10 {
            assert_eq!(a(), b());
        }
    }

    #[test]
    fn siguiente_queda_entre_0_y_1() {
        for _ in 0..1000 {
            let n = siguiente();
            assert!((0.0..1.0).contains(&n));
        }
    }

    #[test]
    fn dos_hilos_no_sacan_la_misma_secuencia() {
        let hilos: Vec<_> = (0..8).map(|_| std::thread::spawn(siguiente)).collect();
        let valores: std::collections::HashSet<u64> = hilos.into_iter().map(|h| h.join().unwrap().to_bits()).collect();
        assert_eq!(valores.len(), 8);
    }
}
