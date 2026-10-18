//! Iconos con el personaje, de frente y quieto, dibujado con el mismo código que el de la barra:
//! el del área de notificación (con la figura, los colores y los accesorios que se eligieron) y
//! el del programa (assets/nook.ico: Nook, rojo y sin accesorios). Este no se genera solo; para
//! volver a generarlo:
//!
//! ```text
//! cargo test genera_el_icono -- --ignored
//! ```
//!
//! Se dibuja una vez en grande, se recorta lo que ocupa el personaje y se reduce a cada tamaño
//! promediando los píxeles (así los bordes quedan suaves también en los pequeños). En el .ico,
//! los tamaños de hasta 64 px van como mapa de bits y los grandes como PNG, como hace Windows.

use crate::personaje::{self, Aspecto, Figura, Mirar, Motor};
use std::sync::Arc;

/// Margen alrededor del personaje, en proporción al lado del icono.
const MARGEN: f32 = 0.03;
/// Cuántas veces más grande que el icono se dibuja antes de reducirlo.
const AUMENTO: u32 = 8;

/// Imagen RGBA con alfa premultiplicado.
struct Imagen {
    lado: u32,
    pixeles: Vec<u8>,
}

/// Píxeles RGBA (alfa sin premultiplicar) de un icono de `lado` × `lado` con el personaje.
pub fn pixeles(figura: &Arc<Figura>, aspecto: &Aspecto, lado: u32) -> Vec<u8> {
    let grande = dibujar(figura, aspecto, (lado * AUMENTO).max(128));
    sin_premultiplicar(&reducir(&grande, caja(&grande), lado))
}

/// El personaje dibujado en grande, de frente.
fn dibujar(figura: &Arc<Figura>, aspecto: &Aspecto, lado: u32) -> Imagen {
    let dibujo = personaje::dibujar(&cuadro(figura), lado, aspecto).to_rgba8_premultiplied();
    let pixeles = dibujo.map_or_else(|| vec![0; (lado * lado * 4) as usize], |d| d.as_bytes().to_vec());
    Imagen { lado, pixeles }
}

/// El personaje de frente, con los ojos abiertos y quieto: de los primeros segundos de su vida
/// en reposo, el cuadro con los ojos más abiertos.
fn cuadro(figura: &Arc<Figura>) -> personaje::Cuadro {
    let mut motor = Motor::new();
    motor.poner_figura(figura.clone());
    motor.poner_mirada(Mirar { guinada: 0.0, cabeceo: 0.0, mezcla: 1.0, deriva: 0.0 }, 0.0, 0.001);
    let abiertos = |c: &personaje::Cuadro| c.ojos.iter().map(|o| o.matriz[3].abs() * o.alto).sum::<f32>();
    (1..400)
        .map(|i| motor.muestrear(i as f64 * 0.02))
        .max_by(|a, b| abiertos(a).total_cmp(&abiertos(b)))
        .unwrap()
}

/// Recorta el cuadrado que ocupa el personaje (con su margen) y lo reduce a `lado` píxeles.
fn reducir(grande: &Imagen, caja: (f32, f32, f32), lado: u32) -> Imagen {
    let (x0, y0, ancho) = caja;
    let paso = ancho / lado as f32;
    let mut pixeles = vec![0u8; (lado * lado * 4) as usize];
    for y in 0..lado {
        for x in 0..lado {
            // Media de los píxeles grandes que caen en este (con la parte de cada uno que cae).
            let (ax, ay) = (x0 + x as f32 * paso, y0 + y as f32 * paso);
            let (bx, by) = (ax + paso, ay + paso);
            let mut suma = [0f32; 4];
            for gy in ay.floor().max(0.0) as u32..(by.ceil() as u32).min(grande.lado) {
                let peso_y = (by.min(gy as f32 + 1.0) - ay.max(gy as f32)).max(0.0);
                for gx in ax.floor().max(0.0) as u32..(bx.ceil() as u32).min(grande.lado) {
                    let peso = peso_y * (bx.min(gx as f32 + 1.0) - ax.max(gx as f32)).max(0.0);
                    let i = ((gy * grande.lado + gx) * 4) as usize;
                    for c in 0..4 {
                        suma[c] += grande.pixeles[i + c] as f32 * peso;
                    }
                }
            }
            let i = ((y * lado + x) * 4) as usize;
            for c in 0..4 {
                pixeles[i + c] = (suma[c] / (paso * paso)).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Imagen { lado, pixeles }
}

/// Lo que ocupa el personaje en la imagen: esquina y lado del cuadrado que lo contiene, centrado
/// y con el margen.
fn caja(imagen: &Imagen) -> (f32, f32, f32) {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..imagen.lado {
        for x in 0..imagen.lado {
            if imagen.pixeles[((y * imagen.lado + x) * 4 + 3) as usize] > 0 {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
    }
    if x0 >= x1 {
        return (0.0, 0.0, imagen.lado as f32);
    }
    let ancho = (x1 - x0).max(y1 - y0) as f32 / (1.0 - 2.0 * MARGEN);
    let (cx, cy) = ((x0 + x1) as f32 / 2.0, (y0 + y1) as f32 / 2.0);
    (cx - ancho / 2.0, cy - ancho / 2.0, ancho)
}

/// RGBA sin premultiplicar.
fn sin_premultiplicar(imagen: &Imagen) -> Vec<u8> {
    let mut pixeles = imagen.pixeles.clone();
    for p in pixeles.chunks_exact_mut(4) {
        if p[3] > 0 {
            for c in 0..3 {
                p[c] = ((p[c] as f32 * 255.0 / p[3] as f32).round()).min(255.0) as u8;
            }
        }
    }
    pixeles
}

#[cfg(test)]
mod archivo {
    //! El archivo assets/nook.ico.

    use super::*;
    use std::path::Path;
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapEncoder, BitmapPixelFormat};
    use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

    /// Color del cuerpo y de los rasgos (ojos y sonrisa; el colmillo siempre es blanco).
    const CUERPO: [u8; 3] = [0xE5, 0x39, 0x35];
    const RASGOS: [u8; 3] = [0xFF, 0xFF, 0xFF];
    /// Tamaños del icono, en píxeles.
    const TAMANOS: [u32; 9] = [16, 20, 24, 32, 40, 48, 64, 128, 256];
    /// Lado al que se dibuja antes de reducirlo.
    const LADO_GRANDE: u32 = 2048;

    /// Mapa de bits de un icono: la cabecera (con el doble de alto, por la máscara), los píxeles en
    /// BGRA de abajo arriba y la máscara de 1 bit (vacía: manda el alfa).
    fn mapa_de_bits(imagen: &Imagen) -> Vec<u8> {
        let lado = imagen.lado;
        let rgba = sin_premultiplicar(imagen);
        let mut datos = Vec::new();
        for v in [40u32, lado, lado * 2] {
            datos.extend_from_slice(&v.to_le_bytes());
        }
        datos.extend_from_slice(&1u16.to_le_bytes()); // planos
        datos.extend_from_slice(&32u16.to_le_bytes()); // bits por píxel
        datos.extend_from_slice(&[0u8; 24]); // sin compresión y el resto a cero
        for y in (0..lado).rev() {
            for x in 0..lado {
                let i = ((y * lado + x) * 4) as usize;
                datos.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
            }
        }
        let fila_mascara = (lado.div_ceil(32) * 4) as usize;
        datos.extend(std::iter::repeat_n(0u8, fila_mascara * lado as usize));
        datos
    }

    fn png(imagen: &Imagen) -> windows_core::Result<Vec<u8>> {
        png_rgba(&sin_premultiplicar(imagen), imagen.lado, imagen.lado)
    }

    /// PNG de una imagen RGBA (alfa sin premultiplicar) de `ancho` × `alto`.
    fn png_rgba(rgba: &[u8], ancho: u32, alto: u32) -> windows_core::Result<Vec<u8>> {
        let flujo = InMemoryRandomAccessStream::new()?;
        let codificador = BitmapEncoder::CreateAsync(BitmapEncoder::PngEncoderId()?, &flujo)?.join()?;
        codificador.SetPixelData(BitmapPixelFormat::Rgba8, BitmapAlphaMode::Straight, ancho, alto, 96.0, 96.0, rgba)?;
        codificador.FlushAsync()?.join()?;
        let largo = flujo.Size()? as u32;
        let lector = DataReader::CreateDataReader(&flujo.GetInputStreamAt(0)?)?;
        lector.LoadAsync(largo)?.join()?;
        let mut datos = vec![0u8; largo as usize];
        lector.ReadBytes(&mut datos)?;
        Ok(datos)
    }

    /// Archivo .ico con las imágenes dadas (y sus datos ya codificados).
    fn ico(imagenes: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut archivo = Vec::new();
        for v in [0u16, 1, imagenes.len() as u16] {
            archivo.extend_from_slice(&v.to_le_bytes());
        }
        let mut desplazamiento = 6 + 16 * imagenes.len() as u32;
        for (lado, datos) in imagenes {
            // 256 se escribe como 0.
            let l = if *lado >= 256 { 0 } else { *lado as u8 };
            archivo.extend_from_slice(&[l, l, 0, 0]);
            archivo.extend_from_slice(&1u16.to_le_bytes());
            archivo.extend_from_slice(&32u16.to_le_bytes());
            archivo.extend_from_slice(&(datos.len() as u32).to_le_bytes());
            archivo.extend_from_slice(&desplazamiento.to_le_bytes());
            desplazamiento += datos.len() as u32;
        }
        for (_, datos) in imagenes {
            archivo.extend_from_slice(datos);
        }
        archivo
    }

    /// Imágenes del instalador (instalador/nook.iss), en cada escala de pantalla: la pequeña (la
    /// de arriba a la derecha, sobre el blanco de la cabecera) y la grande (la de la pantalla
    /// final, con el fondo inicial de Nook). Lados a escala 100 %: 58 × 58 y 164 × 314.
    const ESCALAS: [f32; 6] = [1.0, 1.25, 1.5, 1.75, 2.0, 2.5];
    const FONDO_PEQUENA: [u8; 3] = [0xFF, 0xFF, 0xFF];
    const FONDO_GRANDE: [u8; 3] = [0x11, 0x13, 0x18];

    /// Pone el personaje (de `lado` píxeles, con su esquina en `x`, `y`) sobre un fondo liso de
    /// `ancho` × `alto` y lo devuelve en RGB.
    fn sobre_fondo(personaje: &Imagen, x: u32, y: u32, ancho: u32, alto: u32, fondo: [u8; 3]) -> Vec<u8> {
        let mut rgb = fondo.repeat((ancho * alto) as usize);
        for py in 0..personaje.lado {
            for px in 0..personaje.lado {
                let (dx, dy) = (x + px, y + py);
                if dx >= ancho || dy >= alto {
                    continue;
                }
                let i = ((py * personaje.lado + px) * 4) as usize;
                let alfa = personaje.pixeles[i + 3] as f32 / 255.0;
                let j = ((dy * ancho + dx) * 3) as usize;
                for c in 0..3 {
                    // Alfa premultiplicado: el color ya va multiplicado por su opacidad.
                    let valor = personaje.pixeles[i + c] as f32 + fondo[c] as f32 * (1.0 - alfa);
                    rgb[j + c] = valor.round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        rgb
    }

    /// Archivo .bmp de 24 bits (filas de abajo arriba en BGR, cada una rellena hasta 4 bytes).
    fn bmp(rgb: &[u8], ancho: u32, alto: u32) -> Vec<u8> {
        let fila = (ancho * 3).div_ceil(4) * 4;
        let tamano = 54 + fila * alto;
        let mut datos = Vec::with_capacity(tamano as usize);
        datos.extend_from_slice(b"BM");
        for v in [tamano, 0, 54, 40, ancho, alto] {
            datos.extend_from_slice(&v.to_le_bytes());
        }
        datos.extend_from_slice(&1u16.to_le_bytes()); // planos
        datos.extend_from_slice(&24u16.to_le_bytes()); // bits por píxel
        datos.extend_from_slice(&[0u8; 24]); // sin compresión y el resto a cero
        for y in (0..alto).rev() {
            for x in 0..ancho {
                let i = ((y * ancho + x) * 3) as usize;
                datos.extend_from_slice(&[rgb[i + 2], rgb[i + 1], rgb[i]]);
            }
            datos.extend(std::iter::repeat_n(0u8, (fila - ancho * 3) as usize));
        }
        datos
    }

    #[test]
    #[ignore]
    fn genera_las_imagenes_del_instalador() {
        let aspecto = Aspecto { cuerpo: CUERPO, rasgos: RASGOS, aviso: None, accesorios: Vec::new() };
        let grande = dibujar(&Figura::nook(), &aspecto, LADO_GRANDE);
        let caja = caja(&grande);
        let carpeta = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/instalador");
        std::fs::create_dir_all(&carpeta).unwrap();
        for escala in ESCALAS {
            // La pequeña: el personaje casi a todo lo que mide.
            let lado = (58.0 * escala).round() as u32;
            let personaje = reducir(&grande, caja, (lado as f32 * 0.86).round() as u32);
            let margen = (lado - personaje.lado) / 2;
            let rgb = sobre_fondo(&personaje, margen, margen, lado, lado, FONDO_PEQUENA);
            std::fs::write(carpeta.join(format!("pequena-{lado}.bmp")), bmp(&rgb, lado, lado)).unwrap();

            // La grande: el personaje centrado, algo por encima de la mitad.
            let (ancho, alto) = ((164.0 * escala).round() as u32, (314.0 * escala).round() as u32);
            let personaje = reducir(&grande, caja, (ancho as f32 * 0.62).round() as u32);
            let (x, y) = ((ancho - personaje.lado) / 2, alto * 2 / 5 - personaje.lado / 2);
            let rgb = sobre_fondo(&personaje, x, y, ancho, alto, FONDO_GRANDE);
            std::fs::write(carpeta.join(format!("grande-{ancho}.bmp")), bmp(&rgb, ancho, alto)).unwrap();
        }
    }

    /// Imágenes de la ficha de la Microsoft Store, en target/imagenes-store (no se guardan en el
    /// repositorio: se suben a mano al Partner Center). El icono de 300 × 300 sin fondo, sobre el
    /// fondo oscuro de Nook y sobre uno claro, y el póster vertical (720 × 1080 y 1440 × 2160)
    /// sobre el fondo oscuro.
    #[test]
    #[ignore]
    fn genera_las_imagenes_de_la_store() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        const LADO_ICONO: u32 = 300;
        const CLARO: [u8; 3] = [0xF4, 0xF5, 0xFA];
        let aspecto = Aspecto { cuerpo: CUERPO, rasgos: RASGOS, aviso: None, accesorios: Vec::new() };
        let grande = dibujar(&Figura::nook(), &aspecto, LADO_GRANDE);
        let caja = caja(&grande);
        let carpeta = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/imagenes-store");
        std::fs::create_dir_all(&carpeta).unwrap();
        let rgba = |rgb: Vec<u8>| rgb.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect::<Vec<u8>>();
        let guardar = |nombre: &str, rgba: &[u8], ancho: u32, alto: u32| {
            std::fs::write(carpeta.join(nombre), png_rgba(rgba, ancho, alto).unwrap()).unwrap();
        };

        // El icono: el personaje ocupando el 78 % del lado, centrado.
        let personaje = reducir(&grande, caja, (LADO_ICONO as f32 * 0.78).round() as u32);
        let margen = (LADO_ICONO - personaje.lado) / 2;
        let mut transparente = vec![0u8; (LADO_ICONO * LADO_ICONO * 4) as usize];
        let suelto = sin_premultiplicar(&personaje);
        for y in 0..personaje.lado {
            let origen = (y * personaje.lado * 4) as usize;
            let destino = (((y + margen) * LADO_ICONO + margen) * 4) as usize;
            let largo = (personaje.lado * 4) as usize;
            transparente[destino..destino + largo].copy_from_slice(&suelto[origen..origen + largo]);
        }
        guardar("icono-300-transparente.png", &transparente, LADO_ICONO, LADO_ICONO);
        for (nombre, fondo) in [("icono-300-oscuro.png", FONDO_GRANDE), ("icono-300-claro.png", CLARO)] {
            let rgb = sobre_fondo(&personaje, margen, margen, LADO_ICONO, LADO_ICONO, fondo);
            guardar(nombre, &rgba(rgb), LADO_ICONO, LADO_ICONO);
        }

        // El póster: el personaje centrado, algo por encima de la mitad, sobre el fondo oscuro.
        for (ancho, alto) in [(720, 1080), (1440, 2160)] {
            let personaje = reducir(&grande, caja, (ancho as f32 * 0.6).round() as u32);
            let (x, y) = ((ancho - personaje.lado) / 2, alto * 2 / 5 - personaje.lado / 2);
            let rgb = sobre_fondo(&personaje, x, y, ancho, alto, FONDO_GRANDE);
            guardar(&format!("poster-{ancho}x{alto}.png"), &rgba(rgb), ancho, alto);
        }
    }

    #[test]
    #[ignore]
    fn genera_el_icono() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let aspecto = Aspecto { cuerpo: CUERPO, rasgos: RASGOS, aviso: None, accesorios: Vec::new() };
        let grande = dibujar(&Figura::nook(), &aspecto, LADO_GRANDE);
        let caja = caja(&grande);
        let imagenes: Vec<(u32, Vec<u8>)> = TAMANOS
            .iter()
            .map(|&lado| {
                let imagen = reducir(&grande, caja, lado);
                (lado, if lado > 64 { png(&imagen).unwrap() } else { mapa_de_bits(&imagen) })
            })
            .collect();
        let ruta = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/nook.ico");
        std::fs::write(&ruta, ico(&imagenes)).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_icono_de_la_bandeja_es_transparente_en_las_esquinas_y_opaco_en_el_centro() {
        let lado = 32;
        for figura in Figura::integradas() {
            let aspecto = Aspecto { cuerpo: [0xDE, 0x3B, 0x3B], rasgos: [0xFF, 0xFF, 0xFF], aviso: None, accesorios: Vec::new() };
            let pixeles = pixeles(&figura, &aspecto, lado);
            assert_eq!(pixeles.len(), (lado * lado * 4) as usize);
            let alfa = |x: u32, y: u32| pixeles[((y * lado + x) * 4 + 3) as usize];
            assert_eq!(alfa(0, 0), 0, "{}", figura.id);
            assert_eq!(alfa(lado - 1, lado - 1), 0, "{}", figura.id);
            assert_eq!(alfa(lado / 2, lado * 2 / 3), 255, "{}", figura.id);
        }
    }
}
