//! Herramientas de personajes para el servidor MCP (`nook.exe --mcp`, ver `addons/mcp.rs`): con
//! ellas, la IA del usuario puede leer lo esencial para hacer personajes con un ejemplo del tipo
//! que quiera, guardarlo en la carpeta de personajes, comprobar el de una carpeta (errores, avisos
//! y un resumen de cómo lo entiende Nookly) y verlo dibujado. Se atienden aquí
//! mismo, sin pasar por la barra: el motor de personajes es una librería. Para poner el personaje
//! en la barra, en cambio, se manda la elección por la tubería (`tuberia.rs`).

use super::cara;
use super::figura::{Figura, TipoCapa};
use super::{Aspecto, Cuadro, Estado, Mirar, Motor, dibujar};
use crate::addons::{traducido, tuberia};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Lo que va delante del nombre de todas las herramientas (como los addons usan `id__`).
pub const PREFIJO: &str = "personaje__";

/// La respuesta de una herramienta: texto, una imagen (PNG) o las dos cosas.
#[derive(Debug)]
pub enum Resultado {
    Texto(String),
    Imagen { datos: Vec<u8>, tipo: &'static str },
    TextoEImagen { texto: String, datos: Vec<u8>, tipo: &'static str },
}

impl Resultado {
    fn texto(texto: String) -> Resultado {
        Resultado::Texto(texto)
    }
}

/// Lado de cada vista del personaje, en píxeles.
const LADO_VISTA: u32 = 256;
/// Las vistas del mosaico, así se van apilando (4 por fila).
const VISTAS: [&str; 8] = ["frente", "derecha", "izquierda", "arriba", "abajo", "aviso", "clic", "musica"];

/// Los ejemplos que da la guía, por tipo de personaje: para qué sirve cada uno, sus notas y sus
/// archivos (el de imágenes no tiene SVG).
const EJEMPLOS: [(&str, &str, &str, &str, &str); 8] = [
    ("simple", "mascota redonda sin nada que sobresalga", include_str!("../../docs/ejemplos/simple/notas.md"), include_str!("../../docs/ejemplos/simple/personaje.svg"), include_str!("../../docs/ejemplos/simple/personaje.json")),
    ("animal", "orejas o cuernos a los lados", include_str!("../../docs/ejemplos/animal/notas.md"), include_str!("../../docs/ejemplos/animal/personaje.svg"), include_str!("../../docs/ejemplos/animal/personaje.json")),
    ("pelo", "persona con flequillo, nuca y moño", include_str!("../../docs/ejemplos/pelo/notas.md"), include_str!("../../docs/ejemplos/pelo/personaje.svg"), include_str!("../../docs/ejemplos/pelo/personaje.json")),
    ("completo", "orejas, pelo en punta, cejas y un ojo de cada color", include_str!("../../docs/ejemplos/completo/notas.md"), include_str!("../../docs/ejemplos/completo/personaje.svg"), include_str!("../../docs/ejemplos/completo/personaje.json")),
    ("caricatura", "personas o personajes de series con contorno, orejas y párpados", include_str!("../../docs/ejemplos/caricatura/notas.md"), include_str!("../../docs/ejemplos/caricatura/personaje.svg"), include_str!("../../docs/ejemplos/caricatura/personaje.json")),
    ("antena", "algo encima que se balancea, como un robot", include_str!("../../docs/ejemplos/antena/notas.md"), include_str!("../../docs/ejemplos/antena/personaje.svg"), include_str!("../../docs/ejemplos/antena/personaje.json")),
    ("fruta", "comida u objetos con forma, hojas y algo que asoma", include_str!("../../docs/ejemplos/fruta/notas.md"), include_str!("../../docs/ejemplos/fruta/personaje.svg"), include_str!("../../docs/ejemplos/fruta/personaje.json")),
    ("imagenes", "pixel art o dibujos en PNG", include_str!("../../docs/ejemplos/fantasma/notas.md"), "", include_str!("../../docs/ejemplos/fantasma/personaje.json")),
];

/// Las herramientas que se ofrecen, como las pide MCP (`tools/list`).
pub fn herramientas() -> Vec<Value> {
    let tipos: Vec<String> = EJEMPLOS.iter().map(|(tipo, para, ..)| format!("{tipo} ({para})")).collect();
    vec![
        json!({
            "name": format!("{PREFIJO}guia"),
            "description": format!("Empieza aquí para crear o cambiar un personaje de Nookly: da lo esencial y, con «tipo», un ejemplo completo (su SVG y su json) listo para copiar y adaptar. Pide el tipo más parecido a lo que quieres hacer: {}.", tipos.join(", ")),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tipo": { "type": "string", "enum": EJEMPLOS.iter().map(|e| e.0).collect::<Vec<_>>(), "description": "El tipo de personaje del que quieres el ejemplo" },
                },
            },
        }),
        json!({
            "name": format!("{PREFIJO}guardar"),
            "description": "Guarda el personaje en una carpeta dentro de la de personajes de Nookly (la crea, o reemplaza su personaje.svg y su personaje.json) y devuelve su comprobación y una imagen con 8 vistas. Úsala para cada versión hasta que no haya errores ni avisos y se vea bien en todas las vistas.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "carpeta": { "type": "string", "description": "El nombre de la carpeta del personaje, sin barras (por ejemplo «zorro»). Una nueva para cada personaje nuevo" },
                    "svg": { "type": "string", "description": "El contenido de personaje.svg" },
                    "json": { "type": "string", "description": "El contenido de personaje.json" },
                },
                "required": ["carpeta"],
            },
        }),
        json!({
            "name": format!("{PREFIJO}comprobar"),
            "description": "Comprueba el personaje de una carpeta: si se puede leer, qué avisos tiene y cómo lo entiende Nookly (capas, ojos, boca, medidas). Itera con ella hasta que no haya errores ni avisos.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "carpeta": { "type": "string", "description": "La carpeta del personaje: una ruta entera, o el nombre de la carpeta dentro de la carpeta de personajes de Nookly" },
                },
                "required": ["carpeta"],
            },
        }),
        json!({
            "name": format!("{PREFIJO}ver"),
            "description": "Dibuja el personaje de una carpeta y devuelve una imagen PNG, para ver cómo queda. Con «mosaico» (por defecto), una con 8 vistas en dos filas de 4: frente, mirando a la derecha, a la izquierda y hacia arriba; luego abajo, con una notificación, al hacerle clic y con música. Con «vista» devuelve una sola de esas vistas. Con «guinada» y «cabeceo» (grados; cabeceo positivo es hacia arriba) lo dibuja mirando donde digas.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "carpeta": { "type": "string", "description": "La carpeta del personaje: una ruta entera, o el nombre de la carpeta dentro de la carpeta de personajes de Nookly" },
                    "vista": { "type": "string", "enum": VISTAS.to_vec(), "description": "Una sola vista, en vez del mosaico" },
                    "guinada": { "type": "number", "description": "Hacia dónde mira: grados a la derecha del de frente (una vista sola)" },
                    "cabeceo": { "type": "number", "description": "Hacia dónde mira: grados hacia arriba (una vista sola)" },
                    "acento": { "type": "string", "description": "Color de acento con el que dibujarlo («#rrggbb»); sin él, como lo dibujó su autor" },
                },
                "required": ["carpeta"],
            },
        }),
        json!({
            "name": format!("{PREFIJO}poner"),
            "description": "Pone el personaje en la barra del usuario: pasa a estar elegido en su lista y lo ve enseguida. Para que pille uno que estés editando, ha de estar en la carpeta de personajes de Nookly.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "carpeta": { "type": "string", "description": "La carpeta del personaje: el nombre de la carpeta dentro de la de personajes de Nookly (o una ruta entera), o «nook» o «limon»" },
                },
                "required": ["carpeta"],
            },
        }),
    ]
}

/// Usa la herramienta `nombre` con sus `argumentos` y da su respuesta (o por qué no).
pub fn llamar(nombre: &str, argumentos: &Value) -> Result<Resultado, String> {
    match nombre.strip_prefix(PREFIJO) {
        Some("guia") => Ok(Resultado::texto(guia(argumentos["tipo"].as_str())?)),
        Some("guardar") => guardar_en(&crate::config::carpeta().join("personajes"), argumentos),
        Some("comprobar") => Ok(Resultado::texto(comprobar(argumentos["carpeta"].as_str().ok_or("falta la carpeta del personaje")?)?)),
        Some("ver") => {
            let datos = ver(argumentos)?;
            Ok(Resultado::Imagen { datos, tipo: "image/png" })
        }
        Some("poner") => Ok(Resultado::texto(poner(argumentos["carpeta"].as_str().ok_or("falta la carpeta del personaje")?)?)),
        _ => Err(format!("herramienta desconocida: {nombre}")),
    }
}

/// Lo esencial para hacer personajes, con la carpeta de personajes de verdad, y, con `tipo`, su
/// ejemplo entero: sus notas y sus archivos.
fn guia(tipo: Option<&str>) -> Result<String, String> {
    let carpeta = crate::config::carpeta().join("personajes");
    let mut texto = include_str!("../../docs/personajes-ia.md").replace("{carpeta}", &carpeta.to_string_lossy());
    let Some(tipo) = tipo else { return Ok(texto) };
    let (_, _, notas, svg, json) = EJEMPLOS.iter().find(|e| e.0 == tipo).ok_or_else(|| format!("no hay ningún ejemplo de tipo «{tipo}»"))?;
    texto.push_str(&format!("\n# Ejemplo «{tipo}»\n\n{notas}"));
    if !svg.is_empty() {
        texto.push_str(&format!("\n## personaje.svg\n\n```svg\n{svg}```\n"));
    }
    texto.push_str(&format!("\n## personaje.json\n\n```json\n{json}```\n"));
    Ok(texto)
}

/// Guarda el personaje que manda la IA en la carpeta `carpeta` de `personajes` (su SVG y su
/// json, los que mande) y da su comprobación y, si se lee, el mosaico de sus vistas.
fn guardar_en(personajes: &Path, argumentos: &Value) -> Result<Resultado, String> {
    let carpeta = argumentos["carpeta"].as_str().ok_or("falta la carpeta del personaje")?.trim();
    if carpeta.is_empty() || carpeta.starts_with('.') || carpeta.contains(['/', '\\', ':']) {
        return Err(format!("«{carpeta}» no vale como carpeta: pon solo un nombre, sin barras (por ejemplo «zorro»)"));
    }
    // El json puede llegar como texto o, si la IA lo manda tal cual, como objeto.
    let json = match &argumentos["json"] {
        Value::String(texto) => Some(texto.clone()),
        Value::Null => None,
        otro => Some(serde_json::to_string_pretty(otro).map_err(|e| e.to_string())?),
    };
    let svg = argumentos["svg"].as_str();
    if svg.is_none() && json.is_none() {
        return Err("falta el dibujo («svg») o el «json» del personaje".into());
    }
    let ruta = personajes.join(carpeta);
    std::fs::create_dir_all(&ruta).map_err(|e| format!("no se pudo crear la carpeta {}: {e}", ruta.display()))?;
    for (archivo, contenido) in [("personaje.svg", svg), ("personaje.json", json.as_deref())] {
        if let Some(contenido) = contenido {
            std::fs::write(ruta.join(archivo), contenido).map_err(|e| format!("no se pudo guardar {archivo}: {e}"))?;
        }
    }
    let texto = format!("Guardado en {}.\n{}", ruta.display(), comprobar(&ruta.to_string_lossy())?);
    Ok(match ver(&json!({ "carpeta": ruta.to_string_lossy() })) {
        Ok(datos) => Resultado::TextoEImagen { texto, datos, tipo: "image/png" },
        Err(_) => Resultado::Texto(texto),
    })
}

/// Comprueba el personaje de una carpeta: la lee entera (json y dibujo) y da qué errores tiene,
/// qué avisos y un resumen de cómo lo entiende el motor.
fn comprobar(carpeta: &str) -> Result<String, String> {
    let (ruta, id) = carpeta_de(carpeta)?;
    match Figura::leer(&ruta, id) {
        Err(e) => Ok(format!("El personaje no se puede leer:\n- {e}")),
        Ok(figura) => Ok(resumen(&figura)),
    }
}

/// La carpeta de un personaje por su nombre (dentro de la de personajes) o por una ruta entera.
fn carpeta_de(carpeta: &str) -> Result<(PathBuf, String), String> {
    let dada = Path::new(carpeta);
    if dada.is_dir() {
        let nombre = dada.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        return Ok((dada.to_path_buf(), format!("usuario/{nombre}")));
    }
    // Un nombre sin nada raro: la carpeta de personajes de Nookly.
    if !carpeta.is_empty() && !carpeta.contains(['/', '\\']) && !carpeta.contains("..") {
        let de_personajes = crate::config::carpeta().join("personajes").join(carpeta);
        if de_personajes.is_dir() {
            return Ok((de_personajes, format!("usuario/{carpeta}")));
        }
    }
    Err(format!("no encuentro la carpeta del personaje: «{carpeta}» no existe ni hay una carpeta con ese nombre en la de personajes de Nookly"))
}

// ---- Verlo dibujado ----

/// Una vista del personaje: unos ángulos de mirada o uno de sus estados.
enum Vista {
    Frente,
    Derecha,
    Izquierda,
    Arriba,
    Abajo,
    Aviso,
    Clic,
    Musica,
    Una { guinada: f32, cabeceo: f32 },
}

impl Vista {
    fn de(nombre: &str) -> Option<Vista> {
        Some(match nombre {
            "frente" => Vista::Frente,
            "derecha" => Vista::Derecha,
            "izquierda" => Vista::Izquierda,
            "arriba" => Vista::Arriba,
            "abajo" => Vista::Abajo,
            "aviso" => Vista::Aviso,
            "clic" => Vista::Clic,
            "musica" => Vista::Musica,
            _ => return None,
        })
    }

    fn angulos(&self) -> (f32, f32) {
        match self {
            Vista::Frente => (0.0, 0.0),
            Vista::Derecha => (35.0, 0.0),
            Vista::Izquierda => (-35.0, 0.0),
            Vista::Arriba => (0.0, 26.0),
            Vista::Abajo => (0.0, -26.0),
            Vista::Aviso | Vista::Clic | Vista::Musica => (0.0, 0.0),
            Vista::Una { guinada, cabeceo } => (*guinada, *cabeceo),
        }
    }
}

/// Dibuja el personaje como pidan los `argumentos` (una vista o el mosaico) y lo codifica en PNG.
fn ver(argumentos: &Value) -> Result<Vec<u8>, String> {
    let (ruta, id) = carpeta_de(argumentos["carpeta"].as_str().ok_or("falta la carpeta del personaje")?)?;
    let figura = Arc::new(Figura::leer(&ruta, id).map_err(|e| format!("El personaje no se puede leer:\n- {e}"))?);
    let aspecto = aspecto_de(&figura, argumentos["acento"].as_str())?;
    let vistas: Vec<Vista> = if let Some(nombre) = argumentos["vista"].as_str() {
        vec![Vista::de(nombre).ok_or_else(|| format!("no hay ninguna vista llamada «{nombre}»"))?]
    } else if argumentos["guinada"].is_number() || argumentos["cabeceo"].is_number() {
        vec![Vista::Una { guinada: argumentos["guinada"].as_f64().unwrap_or(0.0) as f32, cabeceo: argumentos["cabeceo"].as_f64().unwrap_or(0.0) as f32 }]
    } else {
        VISTAS.iter().map(|n| Vista::de(n).unwrap()).collect()
    };
    let (columnas, filas) = if vistas.len() == 1 { (1, 1) } else { (4, 2) };
    let lado = LADO_VISTA as usize;
    let ancho = columnas as u32 * LADO_VISTA;
    let alto = filas as u32 * LADO_VISTA;
    let mut lienzo = vec![0u8; 4 * ancho as usize * alto as usize];
    for (i, vista) in vistas.iter().enumerate() {
        let imagen = dibujar(&cuadro_de(&figura, vista), LADO_VISTA, &aspecto);
        let pixeles = imagen.to_rgba8_premultiplied().ok_or("no se pudo dibujar")?;
        let (col, fila) = (i % columnas, i / columnas);
        for (v, destino) in pixeles.as_slice().iter().enumerate() {
            let (x, y) = (v % lado, v / lado);
            let j = 4 * ((fila * lado + y) * columnas * lado + col * lado + x);
            // Lo que se ve sin el alfa premultiplicado.
            let (r, g, b, a) = (destino.r as u32, destino.g as u32, destino.b as u32, destino.a as u32);
            let sin = |c: u32| if a > 0 { ((c * 255 + a / 2) / a) as u8 } else { 0 };
            lienzo[j..j + 4].copy_from_slice(&[sin(r), sin(g), sin(b), a as u8]);
        }
    }
    png_de(&lienzo, ancho, alto)
}

/// Cómo se dibuja el personaje: del color de su dibujo, o con el `acento` que pidan.
fn aspecto_de(figura: &Figura, acento: Option<&str>) -> Result<Aspecto, String> {
    let (cuerpo, rasgos) = match acento {
        Some(hex) => {
            let cuerpo = super::figura::color(hex)?;
            let claro = crate::barra::configuracion::es_claro(slint::Color::from_rgb_u8(cuerpo[0], cuerpo[1], cuerpo[2]));
            (cuerpo, if claro { [0x0B, 0x12, 0x30] } else { [0xFF, 0xFF, 0xFF] })
        }
        None => (figura.colores.cuerpo.unwrap_or([46, 139, 87]), figura.colores.rasgos.unwrap_or([0x0B, 0x12, 0x30])),
    };
    Ok(Aspecto { cuerpo, rasgos, aviso: figura.colores.aviso, accesorios: Vec::new() })
}

/// El cuadro del personaje en la vista `vista`: mirando como pida y con su estado.
fn cuadro_de(figura: &Arc<Figura>, vista: &Vista) -> Cuadro {
    let mut motor = Motor::new();
    motor.poner_figura(figura.clone());
    let (guinada, cabeceo) = vista.angulos();
    motor.poner_mirada(Mirar { guinada, cabeceo, mezcla: 1.0, deriva: 0.0 }, 0.0, 0.001);
    let (estado, t) = match vista {
        Vista::Aviso => (Some(Estado::Aviso), tiempo(0.4)),
        // El clic: en el aire, en mitad del salto.
        Vista::Clic => {
            motor.rebotar(0.0);
            (None, tiempo(0.2))
        }
        Vista::Musica => {
            motor.poner_estado(Estado::Musica, 0.0);
            motor.golpe(0.05, 1.0);
            (None, tiempo(0.2))
        }
        _ => (None, tiempo(0.05)),
    };
    if let Some(estado) = estado {
        motor.poner_estado(estado, 0.0);
    }
    motor.muestrear(t)
}

/// El primer tiempo a partir de `desde` en el que el personaje no va a medio parpadeo (así las
/// vistas enseñan el personaje con los ojos abiertos).
fn tiempo(desde: f64) -> f64 {
    (0..2000).map(|i| i as f64 * 0.01).find(|&t| t >= desde && cara::vida(t, 0.0).parpado > 0.95).unwrap_or(desde)
}

/// Un PNG RGBA de los píxeles de un lienzo ya sin premultiplicar.
fn png_de(lienzo: &[u8], ancho: u32, alto: u32) -> Result<Vec<u8>, String> {
    fn codificar(salida: &mut Vec<u8>, lienzo: &[u8], ancho: u32, alto: u32) -> Result<(), String> {
        let mut codificador = png::Encoder::new(salida, ancho, alto);
        codificador.set_color(png::ColorType::Rgba);
        codificador.set_depth(png::BitDepth::Eight);
        let mut escritor = codificador.write_header().map_err(|e| e.to_string())?;
        escritor.write_image_data(lienzo).map_err(|e| e.to_string())?;
        Ok(())
    }
    let mut salida = Vec::new();
    codificar(&mut salida, lienzo, ancho, alto)?;
    Ok(salida)
}

// ---- Ponerlo en la barra ----

/// El personaje de una carpeta pasa a ser el de la barra: se lo manda a ella, por la tubería (si
/// no está abierta, la abre).
fn poner(carpeta: &str) -> Result<String, String> {
    let nombre = nombre_de_carpeta(carpeta)?;
    de_la_barra(&tuberia::pedir(&json!({ "orden": "figura", "nombre": nombre }))?)
}

/// El nombre de la figura que la barra entenderá: el de la carpeta (o «nook» y «limon» tal
/// cual); de una ruta, el de su carpeta final. La ruta tiene que existir.
fn nombre_de_carpeta(carpeta: &str) -> Result<String, String> {
    let nombre = if carpeta.contains(['/', '\\']) || carpeta.contains("..") {
        let dada = Path::new(carpeta);
        if !dada.is_dir() {
            return Err(format!("no encuentro la carpeta «{carpeta}»"));
        }
        dada.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    } else {
        carpeta.trim().into()
    };
    if nombre.is_empty() {
        return Err("falta la carpeta del personaje".into());
    }
    Ok(nombre)
}

/// La respuesta de la barra a «figura»: su texto o su error.
fn de_la_barra(respuesta: &Value) -> Result<String, String> {
    match respuesta.get("error").and_then(Value::as_str) {
        Some(e) => Err(e.into()),
        None => Ok(respuesta["texto"].as_str().unwrap_or_default().into()),
    }
}

/// Cómo se ve un personaje para Nookly: su nombre, sus avisos, sus capas, dónde quedan los ojos
/// y la boca, cuánto mide y qué hace solo.
fn resumen(figura: &Figura) -> String {
    let mut texto = format!("El personaje se lee bien. Se llama «{}».", traducido(&figura.nombre));
    if figura.avisos.is_empty() {
        texto.push_str("\nSin avisos.");
    } else {
        for aviso in &figura.avisos {
            texto.push_str(&format!("\nAviso: {aviso}."));
        }
    }
    let capas: Vec<String> = figura.capas().iter().map(|c| format!("{} ({})", c.id, tipo_texto(&c.tipo))).collect();
    texto.push_str(&format!(
        "\nCapas, en el orden en que se pintan: {}.",
        if capas.is_empty() { "ninguna: es un personaje de imágenes".into() } else { capas.join(", ") }
    ));
    let m = figura.medidas;
    texto.push_str(&format!(
        "\nEn radios del cuerpo (y hacia abajo es positivo): los ojos van en ({:+.2}, {:+.2}) y la boca en ({:+.2}, {:+.2}); el cuerpo mide {:+.2} hacia arriba, {:+.2} hacia abajo y {:+.2} a cada lado.",
        m.ojo.0, m.ojo.1, m.boca.0, m.boca.1, m.arriba, m.abajo, m.ancho
    ));
    texto.push_str(if figura.mira() {
        "\nSigue al cursor con la mirada: parpadea, se sorprende con las notificaciones y sus ojos rellenos se vuelven arcos al estar contento."
    } else {
        "\nEs de imágenes: mira siempre de frente y no tiene ojos ni boca que animar, pero salta, baila y rebota igual."
    });
    if figura.muelle.is_some() {
        texto.push_str("\nTiene pelo con muelle.");
    }
    texto
}

/// El nombre que se enseña de un tipo de capa.
fn tipo_texto(tipo: &TipoCapa) -> &'static str {
    match tipo {
        TipoCapa::Cuerpo => "cuerpo",
        TipoCapa::Pintado => "pintado",
        TipoCapa::Franja { .. } => "franja",
        TipoCapa::Ojo => "ojo",
        TipoCapa::OjoIzquierdo => "ojo-izquierdo",
        TipoCapa::OjoAlegre => "ojo-alegre",
        TipoCapa::Boca => "boca",
        TipoCapa::Cabeza(_) => "cabeza",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ejemplos() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/ejemplos")
    }

    #[test]
    fn lista_sus_herramientas() {
        let herramientas = herramientas();
        assert_eq!(herramientas.len(), 5);
        for (i, nombre) in ["guia", "guardar", "comprobar", "ver", "poner"].iter().enumerate() {
            assert_eq!(herramientas[i]["name"], format!("{PREFIJO}{nombre}"));
        }
        assert_eq!(herramientas[0]["inputSchema"]["properties"]["tipo"]["enum"].as_array().unwrap().len(), EJEMPLOS.len());
        assert_eq!(herramientas[2]["inputSchema"]["required"][0], "carpeta");
        let vistas: Vec<&str> = herramientas[3]["inputSchema"]["properties"]["vista"]["enum"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(vistas, VISTAS.to_vec());
    }

    #[test]
    fn ve_el_personaje_en_png() {
        let gato = ejemplos().join("gato");
        let mosaico = ver(&json!({ "carpeta": gato.to_string_lossy() })).unwrap();
        assert_eq!(&mosaico[..4], &[137, 80, 78, 71]);
        let decodificado = png::Decoder::new(std::io::Cursor::new(&mosaico)).read_info().unwrap();
        assert_eq!(
            (decodificado.info().width, decodificado.info().height),
            (4 * LADO_VISTA, 2 * LADO_VISTA),
            "el mosaico son las 8 vistas en dos filas de 4"
        );
        // Una vista sola, con otro acento y con ángulos a mano; y una figura de imágenes.
        let devuelve = |extra: Value| {
            let mut argumentos = json!({ "carpeta": gato.to_string_lossy() });
            let objeto = argumentos.as_object_mut().unwrap();
            for (clave, valor) in extra.as_object().unwrap() {
                objeto.insert(clave.clone(), valor.clone());
            }
            ver(&argumentos).unwrap()
        };
        for imagen in [
            devuelve(json!({ "vista": "derecha" })),
            devuelve(json!({ "vista": "frente", "acento": "#0b1230" })),
            devuelve(json!({ "guinada": 30.0, "cabeceo": -10.0 })),
        ] {
            let info = png::Decoder::new(std::io::Cursor::new(&imagen)).read_info().unwrap();
            assert_eq!((info.info().width, info.info().height), (LADO_VISTA, LADO_VISTA));
        }
    }

    #[test]
    fn explicar_lo_que_falla_al_verlo() {
        assert!(ver(&json!({})).unwrap_err().contains("falta la carpeta"));
        assert!(ver(&json!({ "carpeta": "no-esta" })).unwrap_err().contains("no encuentro"));
        let gato = ejemplos().join("gato");
        assert!(ver(&json!({ "carpeta": gato.to_string_lossy(), "vista": "de lado" })).unwrap_err().contains("vista"));
    }

    #[test]
    fn la_guia_da_lo_esencial_y_el_ejemplo_de_cada_tipo() {
        let texto = match llamar(&format!("{PREFIJO}guia"), &json!({})) {
            Ok(Resultado::Texto(texto)) => texto,
            otra => panic!("no es texto: {otra:?}"),
        };
        assert!(texto.contains("# Personajes de Nookly: lo esencial") && !texto.contains("{carpeta}"), "{texto}");
        assert!(!texto.contains("# Ejemplo"), "sin tipo no hay ejemplo");
        for (tipo, ..) in EJEMPLOS {
            let texto = guia(Some(tipo)).unwrap();
            assert!(texto.contains(&format!("# Ejemplo «{tipo}»")) && texto.contains("```json"), "{tipo}");
            assert!(texto.contains("```svg") != (tipo == "imagenes"), "{tipo}");
        }
        assert!(guia(Some("dinosaurio")).unwrap_err().contains("dinosaurio"));
    }

    #[test]
    fn los_ejemplos_se_leen_sin_avisos() {
        for carpeta in ["simple", "animal", "pelo", "completo", "caricatura", "antena", "fruta", "fantasma"] {
            let texto = comprobar(&ejemplos().join(carpeta).to_string_lossy()).unwrap();
            assert!(texto.contains("se lee bien") && texto.contains("Sin avisos"), "{carpeta}: {texto}");
        }
    }

    #[test]
    fn guarda_el_personaje_y_lo_comprueba() {
        let personajes = std::env::temp_dir().join(format!("nook-prueba-guardar-{}", crate::azar::siguiente().to_bits()));
        let (_, _, _, svg, json) = EJEMPLOS[1];
        match guardar_en(&personajes, &json!({ "carpeta": "zorro", "svg": svg, "json": json })).unwrap() {
            Resultado::TextoEImagen { texto, datos, .. } => {
                assert!(texto.contains("Guardado en") && texto.contains("«Zorro»") && texto.contains("Sin avisos"), "{texto}");
                assert_eq!(&datos[..4], &[137, 80, 78, 71]);
            }
            otra => panic!("esperaba texto e imagen: {otra:?}"),
        }
        assert_eq!(std::fs::read_to_string(personajes.join("zorro/personaje.svg")).unwrap(), svg);
        // El json también vale como objeto; uno que no se lee se explica, sin imagen.
        let objeto: Value = serde_json::from_str(json).unwrap();
        assert!(matches!(guardar_en(&personajes, &json!({ "carpeta": "zorro", "json": objeto })).unwrap(), Resultado::TextoEImagen { .. }));
        match guardar_en(&personajes, &json!({ "carpeta": "zorro", "json": "{" })).unwrap() {
            Resultado::Texto(texto) => assert!(texto.contains("no se puede leer"), "{texto}"),
            otra => panic!("esperaba solo texto: {otra:?}"),
        }
        for mala in ["", "../fuera", "a/b", "C:", ".oculta"] {
            assert!(guardar_en(&personajes, &json!({ "carpeta": mala, "svg": svg })).is_err(), "{mala}");
        }
        assert!(guardar_en(&personajes, &json!({ "carpeta": "zorro" })).unwrap_err().contains("falta"));
        let _ = std::fs::remove_dir_all(&personajes);
    }

    #[test]
    fn comprueba_el_ejemplo_de_la_guia() {
        let gato = ejemplos().join("gato");
        let texto = comprobar(&gato.to_string_lossy()).unwrap();
        assert!(texto.contains("se lee bien"), "{texto}");
        assert!(texto.contains("«Gato»"), "{texto}");
        assert!(texto.contains("oreja-izquierda (cabeza)"), "{texto}");
        assert!(texto.contains("Sin avisos"), "{texto}");
        let fantasma = ejemplos().join("fantasma");
        assert!(comprobar(&fantasma.to_string_lossy()).unwrap().contains("de imágenes"), "fantasma");
    }

    #[test]
    fn comprueba_un_carpetas_con_error_y_otra_que_no_esta() {
        let rota = std::env::temp_dir().join(format!("nook-prueba-mcp-{}", crate::azar::siguiente().to_bits()));
        std::fs::create_dir_all(&rota).unwrap();
        std::fs::write(rota.join("personaje.json"), "{").unwrap();
        let texto = comprobar(&rota.to_string_lossy()).unwrap();
        assert!(texto.contains("no se puede leer") && texto.contains("personaje.json"), "{texto}");
        let _ = std::fs::remove_dir_all(&rota);
        assert!(comprobar("carpeta-que-no-esta").is_err());
    }

    #[test]
    fn sin_carpeta_da_error_claro() {
        let texto = |llamada: Result<Resultado, String>| match llamada {
            Err(e) => e,
            otra => panic!("esperaba un error: {otra:?}"),
        };
        assert!(texto(llamar(&format!("{PREFIJO}comprobar"), &json!({}))).contains("falta"));
        assert!(texto(llamar("otra", &json!({}))).contains("desconocida"));
    }

    #[test]
    fn el_nombre_de_la_carpeta_que_va_a_la_barra() {
        assert_eq!(nombre_de_carpeta("nook").unwrap(), "nook");
        let gato = ejemplos().join("gato");
        assert_eq!(nombre_de_carpeta(&gato.to_string_lossy()).unwrap(), "gato");
        // Una ruta que no existe, error; un nombre suelto que no está pasa a la barra (ella avisa).
        assert!(nombre_de_carpeta("./no-esta").is_err());
        assert_eq!(nombre_de_carpeta("gato-no-esta").unwrap(), "gato-no-esta");
        assert_eq!(de_la_barra(&json!({ "texto": "listo" })).unwrap(), "listo");
        assert_eq!(de_la_barra(&json!({ "error": "no hay ninguno" })).unwrap_err(), "no hay ninguno");
    }
}
