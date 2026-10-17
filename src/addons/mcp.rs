//! Servidor MCP de los addons: `nook.exe --mcp`. Lo abre la IA del usuario (Claude Desktop,
//! Claude Code…) y habla con ella por la entrada y la salida estándar (JSON-RPC, un mensaje por
//! línea). Las herramientas son las de los addons instalados; cada petición se pasa a la barra por
//! la tubería (ver `tuberia.rs`), que es quien tiene los addons cargados.

use super::tuberia;
use serde_json::{Value, json};
use std::io::{BufRead, Write};

/// Argumento con el que se abre el servidor.
pub const ARG: &str = "--mcp";

/// Versiones del protocolo que se entienden (la primera, si el cliente pide otra).
const VERSIONES: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCCIONES: &str = "nook es la barra de tareas del usuario. Estas herramientas son de sus \
addons (minijuegos y otros paneles de la barra): lo que hagas con ellas se ve en su barra.";

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
                "serverInfo": { "name": "nook", "version": crate::actualizacion::VERSION },
                "instructions": INSTRUCCIONES,
            })
        }
        "ping" => json!({}),
        "tools/list" => match barra(json!({ "orden": "herramientas" })) {
            Ok(respuesta) if respuesta.get("error").is_none() => json!({ "tools": respuesta["herramientas"] }),
            Ok(respuesta) => return Some(fallo(id, -32603, respuesta["error"].as_str().unwrap_or_default())),
            Err(e) => return Some(fallo(id, -32603, &e)),
        },
        "tools/call" => {
            let peticion = json!({
                "orden": "llamar",
                "nombre": parametros["name"],
                "argumentos": parametros.get("arguments").cloned().unwrap_or(json!({})),
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
        assert_eq!(r["result"]["serverInfo"]["name"], "nook");
        let r = responder(&json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"1999-01-01"}}), &mut sin_barra).unwrap();
        assert_eq!(r["result"]["protocolVersion"], VERSIONES[0]);
    }

    #[test]
    fn no_responde_a_las_notificaciones() {
        assert!(responder(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}), &mut sin_barra).is_none());
    }

    #[test]
    fn lista_y_llama_a_las_herramientas_de_la_barra() {
        let mut barra = |peticion: Value| -> Result<Value, String> {
            Ok(match peticion["orden"].as_str().unwrap() {
                "herramientas" => json!({ "herramientas": [{ "name": "dado__tirar" }] }),
                _ => {
                    assert_eq!(peticion["nombre"], "dado__tirar");
                    json!({ "texto": "Ha salido un 4." })
                }
            })
        };
        let r = responder(&json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}), &mut barra).unwrap();
        assert_eq!(r["result"]["tools"][0]["name"], "dado__tirar");
        let r = responder(&json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"dado__tirar"}}), &mut barra).unwrap();
        assert_eq!(r["result"]["content"][0]["text"], "Ha salido un 4.");
        assert_eq!(r["result"]["isError"], false);
    }

    #[test]
    fn los_errores_de_la_barra_llegan_como_error_de_la_herramienta() {
        let r = responder(&json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"x__y"}}), &mut sin_barra).unwrap();
        assert_eq!(r["result"]["isError"], true);
        let r = responder(&json!({"jsonrpc":"2.0","id":6,"method":"nada"}), &mut sin_barra).unwrap();
        assert_eq!(r["error"]["code"], -32601);
    }
}
