//! Servidor MCP de los addons: `nook.exe --mcp`. Lo abre la IA del usuario (Claude Desktop,
//! Claude Code…) y habla con ella por la entrada y la salida estándar (JSON-RPC, un mensaje por
//! línea). Las herramientas son las de los addons instalados y, además, las de personajes (ver
//! `personaje/mcp.rs`, que se atienden aquí mismo); las de los addons se pasan a la barra por la
//! tubería (`tuberia.rs`), que es quien los tiene cargados.

use super::tuberia;
use crate::personaje::mcp as personaje;
use serde_json::{Value, json};
use std::io::{BufRead, Write};

/// Argumento con el que se abre el servidor.
pub const ARG: &str = "--mcp";

/// Versiones del protocolo que se entienden (la primera, si el cliente pide otra).
const VERSIONES: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCCIONES: &str = "Nookly es la barra de tareas del usuario. Estas herramientas son de sus \
addons (minijuegos y otros paneles de la barra): lo que hagas con ellas se ve en su barra. Con las \
herramientas personaje__ puedes también hacer personajes para su barra: la guía del personaje lo \
explica y las demás te dejan comprobar cómo va.";

/// Atiende al cliente hasta que cierra la entrada.
pub fn servir() {
    let entrada = std::io::stdin().lock();
    let mut salida = std::io::stdout().lock();
    for linea in entrada.lines() {
        let Ok(linea) = linea else { break };
        if linea.trim().is_empty() {
            continue;
        }
        let respuesta = match serde_json::from_str::<Value>(&linea) {
            Ok(mensaje) => responder(&mensaje, &mut |peticion| tuberia::pedir(&peticion)),
            Err(e) => Some(fallo(Value::Null, -32700, &e.to_string())),
        };
        if let Some(respuesta) = respuesta {
            if writeln!(salida, "{respuesta}").and_then(|_| salida.flush()).is_err() {
                break;
            }
        }
    }
}

/// Respuesta a un mensaje del cliente (ninguna a las notificaciones). `barra` manda una petición
/// a la barra y devuelve su respuesta.
pub fn responder(mensaje: &Value, barra: &mut dyn FnMut(Value) -> Result<Value, String>) -> Option<Value> {
    let id = mensaje.get("id")?.clone();
    let metodo = mensaje["method"].as_str().unwrap_or_default();
    let parametros = &mensaje["params"];
    let resultado = match metodo {
        "initialize" => {
            let pedida = parametros["protocolVersion"].as_str().unwrap_or_default();
            let version = VERSIONES.iter().find(|v| **v == pedida).unwrap_or(&VERSIONES[0]);
            json!({
                "protocolVersion": version,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "nookly", "version": crate::actualizacion::VERSION },
                "instructions": INSTRUCCIONES,
            })
        }
        "ping" => json!({}),
        "tools/list" => {
            // Las de personajes, propias del motor, van primero; si la barra no da las suyas, se
            // ofrecen solo las de personajes.
            let mut herramientas = personaje::herramientas();
            match barra(json!({ "orden": "herramientas" })) {
                Ok(respuesta) if respuesta.get("error").is_none() => herramientas.extend(respuesta["herramientas"].as_array().cloned().unwrap_or_default()),
                Ok(respuesta) => eprintln!("la barra no pudo dar sus herramientas: {}", respuesta["error"].as_str().unwrap_or_default()),
                Err(e) => eprintln!("sin barra, se ofrecen solo las herramientas de personajes: {e}"),
            }
            json!({ "tools": herramientas })
        }
        "tools/call" => {
            let nombre = parametros["name"].as_str().unwrap_or_default();
            let argumentos = parametros.get("arguments").cloned().unwrap_or(json!({}));
            // Las de personajes se atienden aquí mismo: sin barra hace falta.
            if nombre.starts_with(personaje::PREFIJO) {
                let (texto, es_error) = match personaje::llamar(nombre, &argumentos) {
                    Ok(texto) => (texto, false),
                    Err(e) => (e, true),
                };
                json!({ "content": [{ "type": "text", "text": texto }], "isError": es_error })
            } else {
                let peticion = json!({
                    "orden": "llamar",
                    "nombre": nombre,
                    "argumentos": argumentos,
                });
                let (texto, es_error) = match barra(peticion) {
                    Ok(respuesta) => match respuesta.get("error").and_then(Value::as_str) {
                        Some(error) => (error.to_string(), true),
                        None => (respuesta["texto"].as_str().unwrap_or_default().to_string(), false),
                    },
                    Err(e) => (e, true),
                };
                json!({ "content": [{ "type": "text", "text": texto }], "isError": es_error })
            }
        }
        _ => return Some(fallo(id, -32601, &format!("método desconocido: {metodo}"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": resultado }))
}

fn fallo(id: Value, codigo: i64, mensaje: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": codigo, "message": mensaje } })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sin_barra(_: Value) -> Result<Value, String> {
        Err("sin barra".into())
    }

    #[test]
    fn inicia_con_la_version_pedida_o_la_ultima() {
        let r = responder(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}), &mut sin_barra).unwrap();
        assert_eq!(r["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(r["result"]["serverInfo"]["name"], "nookly");
        let r = responder(&json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"1999-01-01"}}), &mut sin_barra).unwrap();
        assert_eq!(r["result"]["protocolVersion"], VERSIONES[0]);
    }

    #[test]
    fn no_responde_a_las_notificaciones() {
        assert!(responder(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}), &mut sin_barra).is_none());
    }

    #[test]
    fn lista_las_de_personajes_y_las_de_la_barra() {
        let mut barra = |peticion: Value| -> Result<Value, String> {
            Ok(match peticion["orden"].as_str().unwrap() {
                "herramientas" => json!({ "herramientas": [{ "name": "dado__tirar" }] }),
                _ => panic!("no se esperaba esta petición: {peticion}"),
            })
        };
        let r = responder(&json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}), &mut barra).unwrap();
        let nombres: Vec<&str> = r["result"]["tools"].as_array().unwrap().iter().map(|h| h["name"].as_str().unwrap()).collect();
        assert_eq!(nombres.iter().filter(|n| n.starts_with(personaje::PREFIJO)).count(), 2, "{nombres:?}");
        assert!(nombres.contains(&"dado__tirar"), "{nombres:?}");
    }

    #[test]
    fn sin_barra_se_ofrecen_las_de_personajes() {
        let r = responder(&json!({"jsonrpc":"2.0","id":4,"method":"tools/list"}), &mut sin_barra).unwrap();
        let nombres: Vec<&str> = r["result"]["tools"].as_array().unwrap().iter().map(|h| h["name"].as_str().unwrap()).collect();
        assert!(nombres.iter().all(|n| n.starts_with(personaje::PREFIJO)), "{nombres:?}");
    }

    #[test]
    fn llama_a_las_de_personajes_sin_tocar_la_barra() {
        let mut barra = |_: Value| -> Result<Value, String> { panic!("no hay que ir a la barra") };
        let r = responder(&json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"personaje__guia"}}), &mut barra).unwrap();
        assert_eq!(r["result"]["content"][0]["type"], "text");
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("Guía"), "{r}");
        assert_eq!(r["result"]["isError"], false);
        // Los errores de las de personajes llegan como error también.
        let r = responder(&json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"personaje__comprobar"}}), &mut barra).unwrap();
        assert_eq!(r["result"]["isError"], true);
    }

    #[test]
    fn las_desconocidas_sigan_yendo_a_la_barra() {
        let mut barra = |peticion: Value| -> Result<Value, String> {
            Ok(match peticion["orden"].as_str().unwrap() {
                "llamar" => json!({ "texto": "Ha salido un 4." }),
                _ => panic!("no se esperaba esta petición: {peticion}"),
            })
        };
        let r = responder(&json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"dado__tirar"}}), &mut barra).unwrap();
        assert_eq!(r["result"]["content"][0]["text"], "Ha salido un 4.");
        assert_eq!(r["result"]["isError"], false);
        let r = responder(&json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"dado__tirar"}}), &mut sin_barra).unwrap();
        assert_eq!(r["result"]["isError"], true);
        let r = responder(&json!({"jsonrpc":"2.0","id":9,"method":"nada"}), &mut sin_barra).unwrap();
        assert_eq!(r["error"]["code"], -32601);
    }
}
