//! Detecta que ha llegado una notificación, para que el personaje reaccione. Hay dos señales:
//!
//! - Una notificación de Windows: al aparecer, el shell descubre la ventana que la muestra (de
//!   clase `Windows.UI.Core.CoreWindow`, del proceso ShellExperienceHost). El centro de
//!   notificaciones y los paneles rápidos usan el mismo tipo de ventana, pero esos los abre el
//!   usuario y quedan en primer plano; una notificación aparece sin quitarle el foco a nadie. Con
//!   "No molestar" activo no aparecen, así que el personaje tampoco reacciona.
//! - Un programa que pide atención haciendo parpadear su botón de la barra de tareas (los chats
//!   lo hacen al recibir un mensaje): el shell lo avisa con `HSHELL_FLASH`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows::Win32::UI::WindowsAndMessaging::{
    EVENT_OBJECT_UNCLOAKED, GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId, OBJID_WINDOW,
    RegisterShellHookWindow, RegisterWindowMessageW, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
};
use windows::core::{PWSTR, w};

/// Mensaje con el que el shell avisa a las ventanas registradas (`RegisterShellHookWindow`).
static MENSAJE_SHELL: AtomicU32 = AtomicU32::new(0);
/// `wparam` del mensaje del shell cuando una ventana parpadea en la barra de tareas.
const HSHELL_FLASH: usize = 0x8006;
/// Una misma ventana que parpadea varias veces seguidas cuenta como un solo aviso en este tiempo.
const ENTRE_DESTELLOS: Duration = Duration::from_secs(30);

thread_local! {
    static DESTELLOS: RefCell<HashMap<isize, Instant>> = RefCell::new(HashMap::new());
    static AL_NOTIFICAR: RefCell<Option<fn()>> = const { RefCell::new(None) };
}

/// Empieza a vigilar las notificaciones. `barra` recibirá los mensajes del shell; las
/// notificaciones de Windows llegan a `al_notificar` (en el hilo de la interfaz).
pub fn vigilar(barra: HWND, al_notificar: fn()) {
    AL_NOTIFICAR.with(|f| f.replace(Some(al_notificar)));
    unsafe {
        MENSAJE_SHELL.store(RegisterWindowMessageW(w!("SHELLHOOK")), Ordering::Relaxed);
        let _ = RegisterShellHookWindow(barra);
        SetWinEventHook(
            EVENT_OBJECT_UNCLOAKED,
            EVENT_OBJECT_UNCLOAKED,
            None,
            Some(ventana_descubierta),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
    }
}

/// Si `mensaje` es un aviso del shell (ver `vigilar`).
pub fn es_mensaje_del_shell(mensaje: u32) -> bool {
    mensaje != 0 && mensaje == MENSAJE_SHELL.load(Ordering::Relaxed)
}

/// Si el aviso del shell es una ventana de otro programa que pide atención (y no lo había hecho
/// hace poco).
pub fn pide_atencion(wparam: WPARAM, lparam: LPARAM) -> bool {
    if wparam.0 != HSHELL_FLASH {
        return false;
    }
    let hwnd = HWND(lparam.0 as *mut _);
    let mut proceso = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut proceso)) };
    if proceso == unsafe { GetCurrentProcessId() } {
        return false;
    }
    DESTELLOS.with(|d| destello_nuevo(&mut d.borrow_mut(), lparam.0, Instant::now()))
}

/// Anota el destello de `ventana` y dice si cuenta como aviso nuevo.
fn destello_nuevo(destellos: &mut HashMap<isize, Instant>, ventana: isize, ahora: Instant) -> bool {
    destellos.retain(|_, &mut cuando| ahora.duration_since(cuando) < ENTRE_DESTELLOS);
    if destellos.contains_key(&ventana) {
        return false;
    }
    destellos.insert(ventana, ahora);
    true
}

unsafe extern "system" fn ventana_descubierta(_: HWINEVENTHOOK, _: u32, hwnd: HWND, objeto: i32, _: i32, _: u32, _: u32) {
    if objeto != OBJID_WINDOW.0 || hwnd.is_invalid() {
        return;
    }
    let mut clase = [0u16; 64];
    let largo = unsafe { GetClassNameW(hwnd, &mut clase) }.max(0) as usize;
    let clase = String::from_utf16_lossy(&clase[..largo]);
    // La clase primero: es lo barato, y casi todas las ventanas se descartan ahí.
    if !parece_notificacion(&clase, "shellexperiencehost.exe", false) {
        return;
    }
    let en_primer_plano = unsafe { GetForegroundWindow() } == hwnd;
    if parece_notificacion(&clase, &programa(hwnd), en_primer_plano) {
        if let Some(f) = AL_NOTIFICAR.with(|f| *f.borrow()) {
            f();
        }
    }
}

/// Si una ventana que acaba de aparecer es una notificación de Windows.
fn parece_notificacion(clase: &str, programa: &str, en_primer_plano: bool) -> bool {
    clase == "Windows.UI.Core.CoreWindow" && !en_primer_plano && programa.to_lowercase().ends_with("shellexperiencehost.exe")
}

/// Ruta del programa al que pertenece la ventana (vacía si no se puede saber).
fn programa(hwnd: HWND) -> String {
    let mut proceso = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut proceso));
        let Ok(manejador) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, proceso) else { return String::new() };
        let mut ruta = [0u16; 260];
        let mut largo = ruta.len() as u32;
        let ok = QueryFullProcessImageNameW(manejador, PROCESS_NAME_WIN32, PWSTR(ruta.as_mut_ptr()), &mut largo).is_ok();
        let _ = CloseHandle(manejador);
        if ok { String::from_utf16_lossy(&ruta[..largo as usize]) } else { String::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_notificacion_es_una_ventana_del_shell_que_aparece_sin_foco() {
        let shell = r"C:\Windows\SystemApps\ShellExperienceHost_cw5n1h2txyewy\ShellExperienceHost.exe";
        assert!(parece_notificacion("Windows.UI.Core.CoreWindow", shell, false));
        // El centro de notificaciones lo abre el usuario: queda en primer plano.
        assert!(!parece_notificacion("Windows.UI.Core.CoreWindow", shell, true));
        // Otras ventanas del mismo tipo de otros programas (menú Inicio, apps de la Tienda).
        assert!(!parece_notificacion("Windows.UI.Core.CoreWindow", r"C:\x\StartMenuExperienceHost.exe", false));
        assert!(!parece_notificacion("IME", shell, false));
    }

    #[test]
    fn una_ventana_que_parpadea_seguido_cuenta_una_vez() {
        let mut destellos = HashMap::new();
        let ahora = Instant::now();
        assert!(destello_nuevo(&mut destellos, 1, ahora));
        assert!(!destello_nuevo(&mut destellos, 1, ahora + Duration::from_secs(2)));
        assert!(destello_nuevo(&mut destellos, 2, ahora + Duration::from_secs(2)));
        assert!(destello_nuevo(&mut destellos, 1, ahora + ENTRE_DESTELLOS + Duration::from_secs(1)));
    }
}
