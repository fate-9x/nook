//! Canal entre la barra y `nook.exe --mcp` (ver `mcp.rs`): una tubería con nombre de Windows
//! solo para este equipo. Cada petición es una conexión: se escribe una línea de JSON y se lee
//! otra con la respuesta.

use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{FlushFileBuffers, PIPE_ACCESS_DUPLEX};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_core::HSTRING;

const NOMBRE: &str = r"\\.\pipe\nook-addons";
const BUFER: u32 = 64 * 1024;

/// Lo más que se espera la respuesta de la barra (las herramientas que esperan al usuario
/// responden antes, ver `ESPERA_MAXIMA` en barra/addons.rs).
pub const LIMITE: Duration = Duration::from_secs(58);

/// En la barra: atiende las peticiones en un hilo aparte (bloqueado mientras no llega ninguna, sin
/// gastar nada). `pedir` recibe cada petición y devuelve por dónde llegará la respuesta; se llama
/// desde otro hilo.
pub fn servir(pedir: impl Fn(Value) -> mpsc::Receiver<Value> + Send + Sync + 'static) {
    let pedir = std::sync::Arc::new(pedir);
    let _ = std::thread::Builder::new().name("tuberia".into()).spawn(move || {
        loop {
            let tuberia = match crear() {
                Ok(tuberia) => tuberia,
                Err(e) => {
                    eprintln!("No se pudo abrir la tubería de los addons: {e}");
                    return;
                }
            };
            // SAFETY: la tubería es nuestra y sigue abierta.
            let conectada = unsafe { ConnectNamedPipe(tuberia, None) };
            if let Err(e) = conectada {
                if e.code() != ERROR_PIPE_CONNECTED.to_hresult() {
                    // SAFETY: el `File` se encarga de cerrarla.
                    drop(unsafe { std::fs::File::from_raw_handle(tuberia.0) });
                    continue;
                }
            }
            // SAFETY: la conexión es de este `File` desde aquí; él la cierra al terminar.
            let conexion = unsafe { std::fs::File::from_raw_handle(tuberia.0) };
            let pedir = pedir.clone();
            let _ = std::thread::Builder::new().name("tuberia-peticion".into()).spawn(move || atender(conexion, &*pedir));
        }
    });
}

fn crear() -> windows_core::Result<HANDLE> {
    // SAFETY: llamada normal a la API de Windows con un nombre válido.
    let tuberia = unsafe {
        CreateNamedPipeW(
            &HSTRING::from(NOMBRE),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            BUFER,
            BUFER,
            0,
            None,
        )
    };
    if tuberia == INVALID_HANDLE_VALUE {
        return Err(windows_core::Error::from_thread());
    }
    Ok(tuberia)
}

fn atender(archivo: std::fs::File, pedir: &(dyn Fn(Value) -> mpsc::Receiver<Value> + Send + Sync)) {
    let mut linea = String::new();
    if BufReader::new(&archivo).read_line(&mut linea).is_err() {
        return;
    }
    let respuesta = match serde_json::from_str::<Value>(&linea) {
        Ok(peticion) => pedir(peticion).recv_timeout(LIMITE).unwrap_or_else(|_| error("la barra no ha respondido")),
        Err(e) => error(&e.to_string()),
    };
    let mut archivo = archivo;
    let _ = writeln!(archivo, "{respuesta}");
    // Que lo lea entero antes de cerrarla.
    // SAFETY: sigue abierta (es la del `File`).
    let _ = unsafe { FlushFileBuffers(HANDLE(archivo.as_raw_handle())) };
}

/// Respuesta de error, con el mismo formato que las de la barra.
pub fn error(mensaje: &str) -> Value {
    serde_json::json!({ "error": mensaje })
}

/// En `nook.exe --mcp`: manda una petición a la barra y espera su respuesta. Si la barra no está
/// abierta, la abre y lo vuelve a intentar unos segundos.
pub fn pedir(peticion: &Value) -> Result<Value, String> {
    let mut abierta = false;
    let limite = Instant::now() + Duration::from_secs(15);
    let archivo = loop {
        match std::fs::OpenOptions::new().read(true).write(true).open(NOMBRE) {
            Ok(archivo) => break archivo,
            Err(e) if Instant::now() >= limite => return Err(format!("no se pudo hablar con nook: {e}")),
            Err(_) => {
                if !abierta {
                    abierta = true;
                    abrir_nook();
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    };
    let mut escritor = &archivo;
    writeln!(escritor, "{peticion}").map_err(|e| e.to_string())?;
    let mut linea = String::new();
    BufReader::new(&archivo).read_line(&mut linea).map_err(|e| e.to_string())?;
    serde_json::from_str(&linea).map_err(|e| e.to_string())
}

/// Abre la barra (este mismo ejecutable sin argumentos). Si ya hay una, no pasa nada: esa copia
/// se cierra sola.
fn abrir_nook() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
}
