//! Conectar los addons con la IA del usuario: añadir nook (`nook.exe --mcp`) a los servidores MCP
//! de Claude Desktop, o el comando para hacerlo en Claude Code.
//!
//! Desde el paquete de la Microsoft Store (Nookly Pro) hay dos cosas distintas: el ejecutable está
//! en WindowsApps, desde donde otro programa no lo puede abrir, así que se usa su alias
//! (`%LOCALAPPDATA%\Microsoft\WindowsApps\nook.exe`, en msix/AppxManifest.xml); y Windows lleva
//! los archivos que crea la app en %APPDATA% a una carpeta privada del paquete, así que la
//! configuración de Claude solo se cambia si ya existe, y escribiendo en el mismo archivo.

use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

/// Nombre con el que nook sale en la lista de servidores MCP.
const SERVIDOR: &str = "nookly";
/// Nombre que tenía antes, cuando la app todavía se llamaba nook: se quita al añadir el nuevo.
const SERVIDOR_ANTIGUO: &str = "nook";
const ARCHIVO: &str = "claude_desktop_config.json";

/// Carpetas de la configuración de Claude Desktop que hay en este equipo: la del instalador
/// normal (%APPDATA%\Claude) y la de la versión de la Microsoft Store, que Windows guarda en la
/// carpeta de su paquete.
fn carpetas_de_claude() -> Vec<PathBuf> {
    let mut carpetas = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        carpetas.push(PathBuf::from(appdata).join("Claude"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        if let Ok(paquetes) = std::fs::read_dir(PathBuf::from(local).join("Packages")) {
            for paquete in paquetes.flatten() {
                if paquete.file_name().to_string_lossy().starts_with("Claude_") {
                    carpetas.push(paquete.path().join("LocalCache").join("Roaming").join("Claude"));
                }
            }
        }
    }
    carpetas.into_iter().filter(|c| c.is_dir()).collect()
}

/// Añade nook a Claude Desktop (en todas sus configuraciones). Devuelve cuántas se cambiaron; si
/// Claude Desktop no está instalado, un error.
pub fn con_claude_desktop() -> Result<usize, String> {
    let exe = ejecutable()?;
    let carpetas = carpetas_de_claude();
    if carpetas.is_empty() {
        return Err(crate::idioma::tr("Claude Desktop no está instalado").into());
    }
    for carpeta in &carpetas {
        anadir(&carpeta.join(ARCHIVO), &exe)?;
    }
    Ok(carpetas.len())
}

/// Si nook ya está en Claude Desktop.
pub fn conectado_con_claude_desktop() -> bool {
    carpetas_de_claude().iter().any(|carpeta| {
        std::fs::read_to_string(carpeta.join(ARCHIVO))
            .ok()
            .and_then(|texto| serde_json::from_str::<Value>(&texto).ok())
            .is_some_and(|config| config["mcpServers"].get(SERVIDOR).is_some())
    })
}

/// Pone a nook en los servidores MCP de un archivo de configuración de Claude Desktop, sin tocar
/// nada más. Si el archivo no se puede leer como JSON, no se cambia.
fn anadir(ruta: &Path, exe: &Path) -> Result<(), String> {
    let paquete = empaquetado();
    let texto = match std::fs::read_to_string(ruta) {
        Ok(texto) => texto,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && paquete => {
            // Desde el paquete no se puede crear: Windows lo dejaría en una carpeta privada.
            return Err(crate::idioma::tr("Para conectarlo, abre en Claude Desktop Configuración → Desarrollador → Editar configuración y vuelve a pulsar este botón.").into());
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => "{}".into(),
        Err(e) => return Err(e.to_string()),
    };
    let nuevo = con_nook(&texto, exe)?;
    if paquete {
        std::fs::write(ruta, nuevo.as_bytes()).map_err(|e| e.to_string())
    } else {
        crate::config::escribir_seguro(ruta, nuevo.as_bytes()).map_err(|e| e.to_string())
    }
}

/// Si Nookly está corriendo desde el paquete de la Microsoft Store.
fn empaquetado() -> bool {
    std::env::current_exe().is_ok_and(|exe| en_windowsapps(&exe))
}

fn en_windowsapps(exe: &Path) -> bool {
    exe.components().any(|c| c.as_os_str().eq_ignore_ascii_case("WindowsApps"))
}

/// Lo que tiene que abrir la IA: este ejecutable o, desde el paquete de la Store, su alias.
fn ejecutable() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if !en_windowsapps(&exe) {
        return Ok(exe);
    }
    let local = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA")?;
    Ok(PathBuf::from(local).join("Microsoft").join("WindowsApps").join(ALIAS))
}

/// El alias del ejecutable que declara el paquete de la Store (msix/AppxManifest.xml).
const ALIAS: &str = "nook.exe";

fn con_nook(texto: &str, exe: &Path) -> Result<String, String> {
    let mut config: Value = if texto.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(texto).map_err(|e| format!("{ARCHIVO} no es válido: {e}"))?
    };
    let Some(raiz) = config.as_object_mut() else { return Err(format!("{ARCHIVO} no es válido")) };
    let servidores = raiz.entry("mcpServers").or_insert_with(|| Value::Object(Map::new()));
    let Some(servidores) = servidores.as_object_mut() else { return Err(format!("{ARCHIVO}: mcpServers no es válido")) };
    // La entrada de antes apunta a esta misma app: si se quedara, las herramientas saldrían dos veces.
    if servidores.get(SERVIDOR_ANTIGUO).is_some_and(|s| s["args"][0] == super::mcp::ARG) {
        servidores.remove(SERVIDOR_ANTIGUO);
    }
    servidores.insert(
        SERVIDOR.into(),
        json!({ "command": exe.to_string_lossy(), "args": [super::mcp::ARG] }),
    );
    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
}

/// Comando para añadir nook a Claude Code.
pub fn comando_claude_code() -> String {
    let exe = ejecutable().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|_| ALIAS.into());
    format!("claude mcp add {SERVIDOR} -- \"{exe}\" {}", super::mcp::ARG)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconoce_el_paquete_de_la_store() {
        assert!(en_windowsapps(Path::new(r"C:\Program Files\WindowsApps\NookLabs.Nookly_1.2.0.0_x64__abc\nook.exe")));
        assert!(!en_windowsapps(Path::new(r"C:\Users\lucas\AppData\Local\Programs\nook\nook.exe")));
    }

    #[test]
    fn anade_nook_sin_tocar_lo_demas() {
        let antes = r#"{"mcpServers":{"otro":{"command":"otro.exe"}},"preferencias":{"tema":"oscuro"}}"#;
        let despues: Value = serde_json::from_str(&con_nook(antes, Path::new(r"C:\nook\nook.exe")).unwrap()).unwrap();
        assert_eq!(despues["mcpServers"]["otro"]["command"], "otro.exe");
        assert_eq!(despues["preferencias"]["tema"], "oscuro");
        assert_eq!(despues["mcpServers"]["nookly"]["command"], r"C:\nook\nook.exe");
        assert_eq!(despues["mcpServers"]["nookly"]["args"][0], "--mcp");
    }

    #[test]
    fn crea_la_configuracion_si_no_hay() {
        let despues: Value = serde_json::from_str(&con_nook("", Path::new("nook.exe")).unwrap()).unwrap();
        assert_eq!(despues["mcpServers"]["nookly"]["command"], "nook.exe");
    }

    #[test]
    fn quita_la_entrada_de_antes_si_es_esta_app() {
        let antes = r#"{"mcpServers":{"nook":{"command":"nook.exe","args":["--mcp"]},"otro":{"command":"otro.exe"}}}"#;
        let despues: Value = serde_json::from_str(&con_nook(antes, Path::new("nook.exe")).unwrap()).unwrap();
        assert!(despues["mcpServers"].get("nook").is_none());
        assert_eq!(despues["mcpServers"]["nookly"]["command"], "nook.exe");
        assert_eq!(despues["mcpServers"]["otro"]["command"], "otro.exe");
    }

    #[test]
    fn no_quita_un_servidor_nook_ajeno() {
        let antes = r#"{"mcpServers":{"nook":{"command":"ajeno.exe","args":["--otra"]}}}"#;
        let despues: Value = serde_json::from_str(&con_nook(antes, Path::new("nook.exe")).unwrap()).unwrap();
        assert_eq!(despues["mcpServers"]["nook"]["command"], "ajeno.exe");
    }

    #[test]
    fn no_cambia_un_archivo_danado() {
        assert!(con_nook("{ esto no es json", Path::new("nook.exe")).is_err());
        assert!(con_nook("[1, 2]", Path::new("nook.exe")).is_err());
    }
}
