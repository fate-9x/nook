//! La tecla Windows abre el buscador de Nook (solo en nook Pro). En la versión gratuita solo queda
//! quitar el ayudante, por si lo preparó una versión anterior: lo hace el desinstalador.

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{EVENT_MODIFY_STATE, OpenEventW, SYNCHRONIZATION_SYNCHRONIZE, SetEvent};
use windows::core::{PCWSTR, w};

pub const ARG_AYUDANTE: &str = "--tecla-windows";
pub const ARG_PREPARAR: &str = "--preparar-tecla-windows";
/// Argumento con el que el desinstalador abre Nook como administrador para quitar la tarea y el
/// ayudante.
pub const ARG_QUITAR: &str = "--quitar-tecla-windows";
pub const ID_PULSADA: i32 = 104;

/// La tarea programada, el ayudante dentro de su carpeta y el aviso con el que se le pide que se
/// vaya.
const TAREA: &str = "Nook - tecla Windows";
const CARPETA: &str = "Nook Ayudante";
const EJECUTABLE: &str = "nook-tecla-windows.exe";
const PARAR: PCWSTR = w!("Local\\NookTeclaWindowsParar");
/// Para que no se vea una consola al llamar a schtasks.
const SIN_VENTANA: u32 = 0x0800_0000;

pub fn al_pulsar(_f: impl Fn() + 'static) {}

pub fn pulsada() {}

pub fn activar(_activa: bool) {}

pub fn preparar_ayudante() {}

pub fn preparar() -> i32 {
    1
}

pub fn ayudante() {}

/// Nook abierto como administrador con `ARG_QUITAR` (al desinstalarlo): quita la tarea y el
/// ayudante.
pub fn quitar() -> i32 {
    pedir_que_se_vaya();
    let _ = Command::new("schtasks").args(["/delete", "/tn", TAREA, "/f"]).creation_flags(SIN_VENTANA).output();
    let Some(carpeta) = carpeta() else { return 1 };
    let borrado = reintentar(|| std::fs::remove_file(carpeta.join(EJECUTABLE)));
    let _ = std::fs::remove_dir(&carpeta);
    if borrado.is_ok() { 0 } else { 1 }
}

/// La carpeta del ayudante, que solo puede cambiar un administrador.
fn carpeta() -> Option<PathBuf> {
    std::env::var_os("ProgramFiles").map(|base| PathBuf::from(base).join(CARPETA))
}

/// Le pide al ayudante que se vaya (si está), para poder borrar su copia.
fn pedir_que_se_vaya() {
    if let Ok(parar) = unsafe { OpenEventW(SYNCHRONIZATION_SYNCHRONIZE | EVENT_MODIFY_STATE, false, PARAR) } {
        unsafe {
            let _ = SetEvent(parar);
            let _ = CloseHandle(parar);
        }
    }
}

/// Lo intenta durante unos segundos: la copia sigue en uso mientras el ayudante se va.
fn reintentar(mut f: impl FnMut() -> std::io::Result<()>) -> std::io::Result<()> {
    let mut resultado = f();
    for _ in 0..20 {
        if resultado.is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(150));
        resultado = f();
    }
    resultado
}
