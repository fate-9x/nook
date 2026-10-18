//! Herramientas de personajes para el servidor MCP (`nook.exe --mcp`, ver `addons/mcp.rs`): con
//! ellas, la IA del usuario puede leer la guía para hacer personajes, comprobar el de una carpeta
//! (errores, avisos y un resumen de cómo lo entiende Nookly) y, más adelante, verlo dibujado y
//! ponerlo en la barra. Se atienden aquí mismo, sin pasar por la barra: el motor de personajes es
//! una librería.

use super::figura::{Figura, TipoCapa};
use crate::addons::traducido;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Lo que va delante del nombre de todas las herramientas (como los addons usan `id__`).
pub const PREFIJO: &str = "personaje__";

/// Las herramientas que se ofrecen, como las pide MCP (`tools/list`).
pub fn herramientas() -> Vec<Value> {
    vec![
        json!({
            "name": format!("{PREFIJO}guia"),
            "description": "La guía completa para hacer personajes de Nookly: léela antes de crear o cambiar un personaje.",
            "inputSchema": { "type": "object", "properties": {} },
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
    ]
}

/// Usa la herramienta `nombre` con sus `argumentos` y da el texto de su respuesta (o por qué no).
pub fn llamar(nombre: &str, argumentos: &Value) -> Result<String, String> {
    match nombre.strip_prefix(PREFIJO) {
        Some("guia") => Ok(guia()),
        Some("comprobar") => comprobar(argumentos["carpeta"].as_str().ok_or("falta la carpeta del personaje")?),
        _ => Err(format!("herramienta desconocida: {nombre}")),
    }
}

/// La guía para hacer personajes, la misma que va con el programa.
fn guia() -> String {
    include_str!("../../docs/guia-personajes.md").into()
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
        assert_eq!(herramientas.len(), 2);
        assert_eq!(herramientas[0]["name"], format!("{PREFIJO}guia"));
        assert_eq!(herramientas[1]["name"], format!("{PREFIJO}comprobar"));
        assert_eq!(herramientas[1]["inputSchema"]["required"][0], "carpeta");
    }

    #[test]
    fn la_guia_es_la_del_repositorio() {
        let texto = llamar(&format!("{PREFIJO}guia"), &json!({})).unwrap();
        assert!(texto.contains("# Guía: crea tu propio personaje para Nookly"), "{texto}");
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
        assert!(llamar(&format!("{PREFIJO}comprobar"), &json!({})).unwrap_err().contains("falta"));
        assert!(llamar("otra", &json!({})).unwrap_err().contains("desconocida"));
    }
}
