//! Dibujo que se hace en Rust porque el renderizador por software de Slint no lo hace bien:
//! rellena los degradados sin respetar las esquinas redondeadas y no recorta con ellas. Aquí se
//! generan el cuadro de saturación y brillo y la franja de tonos del selector de color, y la
//! cobertura de formas redondeadas.

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

/// Cuánto cubre un rectángulo redondeado (x0, y0, x1, y1) al píxel centrado en (px, py), de 0 a
/// 1. Con valores intermedios en el contorno, los bordes quedan suavizados.
pub fn cobertura(px: f32, py: f32, (x0, y0, x1, y1): (f32, f32, f32, f32), radio: f32) -> f32 {
    (0.5 - distancia(px, py, (x0, y0, x1, y1), radio)).clamp(0.0, 1.0)
}

/// Distancia con signo al contorno de un rectángulo redondeado (negativa dentro).
fn distancia(px: f32, py: f32, (x0, y0, x1, y1): (f32, f32, f32, f32), radio: f32) -> f32 {
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let qx = (px - cx).abs() - ((x1 - x0) / 2.0 - radio);
    let qy = (py - cy).abs() - ((y1 - y0) / 2.0 - radio);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radio
}

fn imagen(ancho: u32, alto: u32, mut pixel: impl FnMut(f32, f32) -> ([f32; 3], f32)) -> Image {
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(ancho, alto);
    let datos = buffer.make_mut_slice();
    for y in 0..alto {
        for x in 0..ancho {
            let ([r, g, b], a) = pixel(x as f32 + 0.5, y as f32 + 0.5);
            let byte = |v: f32| v.round().clamp(0.0, 255.0) as u8;
            datos[(y * ancho + x) as usize] = Rgba8Pixel { r: byte(r), g: byte(g), b: byte(b), a: byte(a * 255.0) };
        }
    }
    Image::from_rgba8(buffer)
}

/// Cambia el tamaño de una imagen RGBA (alfa sin premultiplicar) a `lado` × `lado`. Cada píxel
/// nuevo mezcla los de alrededor con un filtro triangular (al reducir, tan ancho como lo que
/// abarca), así que no quedan bordes en sierra como al escalar píxel a píxel. Se mezcla con el
/// color multiplicado por el alfa, para que los bordes transparentes no se oscurezcan.
pub fn redimensionar(rgba: &[u8], ancho: u32, alto: u32, lado: u32) -> Vec<u8> {
    let (ancho, alto, lado) = (ancho as usize, alto as usize, lado as usize);
    let origen: Vec<[f32; 4]> = rgba
        .chunks_exact(4)
        .map(|p| {
            let a = p[3] as f32 / 255.0;
            [p[0] as f32 * a, p[1] as f32 * a, p[2] as f32 * a, a]
        })
        .collect();
    let mezclar = |pesos: &[(usize, f32)], muestra: &dyn Fn(usize) -> [f32; 4]| {
        let mut total = [0.0; 4];
        for &(i, w) in pesos {
            let v = muestra(i);
            for c in 0..4 {
                total[c] += v[c] * w;
            }
        }
        total
    };
    let (en_x, en_y) = (pesos(ancho, lado), pesos(alto, lado));
    // Primero a lo ancho (lado × alto) y luego a lo alto (lado × lado).
    let mut horizontal = vec![[0.0; 4]; lado * alto];
    for y in 0..alto {
        for (x, p) in en_x.iter().enumerate() {
            horizontal[y * lado + x] = mezclar(p, &|i| origen[y * ancho + i]);
        }
    }
    let mut resultado = Vec::with_capacity(lado * lado * 4);
    for p in &en_y {
        for x in 0..lado {
            let [r, g, b, a] = mezclar(p, &|i| horizontal[i * lado + x]);
            let color = |c: f32| if a > 1e-4 { (c / a).round().clamp(0.0, 255.0) as u8 } else { 0 };
            resultado.extend_from_slice(&[color(r), color(g), color(b), (a * 255.0).round().clamp(0.0, 255.0) as u8]);
        }
    }
    resultado
}

/// Para cada píxel de salida, los de entrada que mezcla y su peso (suman 1).
fn pesos(entrada: usize, salida: usize) -> Vec<Vec<(usize, f32)>> {
    let escala = entrada as f32 / salida as f32;
    let alcance = escala.max(1.0);
    (0..salida)
        .map(|o| {
            let centro = (o as f32 + 0.5) * escala - 0.5;
            let desde = (centro - alcance).floor().max(0.0) as usize;
            let hasta = ((centro + alcance).ceil() as usize).min(entrada - 1);
            let mut lista: Vec<(usize, f32)> =
                (desde..=hasta).map(|i| (i, (1.0 - (i as f32 - centro).abs() / alcance).max(0.0))).filter(|&(_, w)| w > 0.0).collect();
            if lista.is_empty() {
                lista.push((centro.round().clamp(0.0, (entrada - 1) as f32) as usize, 1.0));
            }
            let suma: f32 = lista.iter().map(|&(_, w)| w).sum();
            lista.iter_mut().for_each(|(_, w)| *w /= suma);
            lista
        })
        .collect()
}

/// Color a partir del tono (0–360), la saturación y el brillo (0–1), en 0–255.
pub fn hsv(tono: f32, saturacion: f32, valor: f32) -> [f32; 3] {
    let h = (tono.rem_euclid(360.0)) / 60.0;
    let c = valor * saturacion;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = valor - c;
    [(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0]
}

/// Escala a la que se generan las imágenes del selector (más nítidas en pantallas con zoom).
const NITIDEZ: f32 = 2.0;

/// Cuadro del selector de color para un tono: la saturación crece hacia la derecha y el brillo
/// hacia arriba. `ancho` y `alto` en píxeles lógicos.
pub fn cuadro_de_color(tono: f32, ancho: f32, alto: f32, radio: f32) -> Image {
    let (w, h) = (ancho * NITIDEZ, alto * NITIDEZ);
    imagen(w as u32, h as u32, |px, py| {
        let c = hsv(tono, px / w, 1.0 - py / h);
        (c, cobertura(px, py, (0.0, 0.0, w, h), radio * NITIDEZ))
    })
}

/// Franja con todos los tonos, de rojo a rojo.
pub fn franja_de_tonos(ancho: f32, alto: f32) -> Image {
    let (w, h) = (ancho * NITIDEZ, alto * NITIDEZ);
    imagen(w as u32, h as u32, |px, py| {
        (hsv(px / w * 360.0, 1.0, 1.0), cobertura(px, py, (0.0, 0.0, w, h), h / 2.0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_cobertura_suaviza_el_borde() {
        let r = (0.0, 0.0, 32.0, 32.0);
        assert_eq!(cobertura(0.5, 0.5, r, 8.0), 0.0, "la esquina queda fuera");
        assert_eq!(cobertura(16.0, 16.0, r, 8.0), 1.0, "el centro queda dentro");
        let borde = cobertura(16.0, 0.0, r, 8.0);
        assert!(borde > 0.0 && borde < 1.0, "el contorno se mezcla: {borde}");
    }

    #[test]
    fn colores_hsv_conocidos() {
        assert_eq!(hsv(0.0, 1.0, 1.0), [255.0, 0.0, 0.0]);
        assert_eq!(hsv(120.0, 1.0, 1.0), [0.0, 255.0, 0.0]);
        assert_eq!(hsv(240.0, 1.0, 1.0), [0.0, 0.0, 255.0]);
        assert_eq!(hsv(360.0, 1.0, 1.0), [255.0, 0.0, 0.0]);
        assert_eq!(hsv(200.0, 0.0, 1.0), [255.0, 255.0, 255.0]);
        assert_eq!(hsv(200.0, 1.0, 0.0), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn redimensionar_conserva_un_color_liso_y_promedia_los_detalles() {
        let liso = [10u8, 200, 30, 255].repeat(64 * 64);
        let reducido = redimensionar(&liso, 64, 64, 22);
        assert_eq!(reducido.len(), 22 * 22 * 4);
        assert!(reducido.chunks_exact(4).all(|p| p == [10, 200, 30, 255]));
        // Un tablero de ajedrez blanco y negro, reducido a la mitad, queda gris.
        let tablero: Vec<u8> = (0..16 * 16)
            .flat_map(|i| if (i % 16 + i / 16) % 2 == 0 { [255, 255, 255, 255] } else { [0, 0, 0, 255] })
            .collect();
        let gris = redimensionar(&tablero, 16, 16, 8);
        assert!(gris.chunks_exact(4).all(|p| (p[0] as i32 - 128).abs() < 20));
    }

    #[test]
    fn redimensionar_no_oscurece_los_bordes_transparentes() {
        // Mitad roja opaca, mitad transparente (con color negro): el borde mezclado sigue rojo.
        let imagen: Vec<u8> =
            (0..8 * 8).flat_map(|i| if i % 8 < 4 { [255, 0, 0, 255] } else { [0, 0, 0, 0] }).collect();
        let reducido = redimensionar(&imagen, 8, 8, 3);
        let borde = &reducido[4..8];
        assert!(borde[3] > 0 && borde[3] < 255);
        assert_eq!(&borde[..3], &[255, 0, 0]);
    }

    #[test]
    fn redimensionar_tambien_amplia() {
        let imagen = [0u8, 0, 255, 255].repeat(2 * 2);
        let ampliado = redimensionar(&imagen, 2, 2, 7);
        assert!(ampliado.chunks_exact(4).all(|p| p == [0, 0, 255, 255]));
    }
}
