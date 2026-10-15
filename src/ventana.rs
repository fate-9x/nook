//! Utilidades de Win32: pantalla, cursor y foco.

use crate::plataforma::Rect;
use std::mem::size_of;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::core::BOOL;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
};

/// Da el foco de teclado a la ventana. Windows solo deja ponerse delante al programa que recibió
/// la última pulsación (si no, la ventana se ve pero el teclado sigue en el otro programa): para
/// eso, mientras tanto, se comparte la entrada del teclado con el programa que está delante.
pub fn traer_al_frente(hwnd: HWND) {
    unsafe {
        let delante = GetForegroundWindow();
        if delante == hwnd {
            return;
        }
        let propio = GetCurrentThreadId();
        let suyo = GetWindowThreadProcessId(delante, None);
        let unidos = suyo != 0 && suyo != propio && AttachThreadInput(propio, suyo, true).as_bool();
        let _ = SetForegroundWindow(hwnd);
        let _ = BringWindowToTop(hwnd);
        if unidos {
            let _ = AttachThreadInput(propio, suyo, false);
        }
    }
}

unsafe extern "system" fn recoger_monitor(monitor: HMONITOR, _: HDC, _: *mut RECT, datos: LPARAM) -> BOOL {
    let lista = unsafe { &mut *(datos.0 as *mut Vec<Rect>) };
    let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        let r = info.rcMonitor;
        lista.push((r.left, r.top, r.right - r.left, r.bottom - r.top));
    }
    true.into()
}

/// Monitores conectados, en píxeles físicos (siempre hay al menos uno).
pub fn monitores() -> Vec<Rect> {
    let mut lista: Vec<Rect> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(recoger_monitor), LPARAM(&mut lista as *mut _ as isize));
    }
    if lista.is_empty() {
        lista.push((0, 0, 1920, 1080));
    }
    lista
}

/// Monitor con esquina superior izquierda `origen`; si ya no existe, el principal.
pub fn monitor_en(origen: Option<(i32, i32)>) -> Rect {
    let todos = monitores();
    origen
        .and_then(|o| todos.iter().copied().find(|m| (m.0, m.1) == o))
        .or_else(|| todos.iter().copied().find(|m| (m.0, m.1) == (0, 0)))
        .unwrap_or(todos[0])
}

/// Monitor más cercano al punto (el que lo contiene, o el más próximo si está fuera de todos).
pub fn monitor_cercano((x, y): (i32, i32)) -> Rect {
    let distancia = |m: &Rect| {
        let dx = (m.0 - x).max(x - (m.0 + m.2)).max(0) as i64;
        let dy = (m.1 - y).max(y - (m.1 + m.3)).max(0) as i64;
        dx * dx + dy * dy
    };
    let todos = monitores();
    todos.iter().copied().min_by_key(distancia).unwrap_or(todos[0])
}

/// Si el botón izquierdo del ratón está pulsado ahora mismo.
pub fn boton_izquierdo_pulsado() -> bool {
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000 != 0 }
}

/// Posición del cursor en píxeles físicos de pantalla.
pub fn posicion_cursor() -> (i32, i32) {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    (p.x, p.y)
}

/// Ventana que tiene el foco ahora (para devolvérselo luego con `activar`).
pub fn en_primer_plano() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}

/// Devuelve el foco a una ventana guardada con `en_primer_plano`.
pub fn activar(ventana: isize) {
    traer_al_frente(HWND(ventana as *mut _));
}

/// Indica si la ventana activa pertenece a otro programa (el usuario hizo clic fuera).
pub fn primer_plano_es_de_otro_proceso() -> bool {
    let activa = unsafe { GetForegroundWindow() };
    !activa.is_invalid() && !es_de_este_proceso(activa)
}

/// El proceso de una ventana.
pub fn proceso_de(ventana: isize) -> u32 {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(HWND(ventana as *mut std::ffi::c_void), Some(&mut pid));
    }
    pid
}

/// Si la ventana es de este programa.
pub fn es_de_este_proceso(ventana: HWND) -> bool {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(ventana, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}

/// Clase de la ventana (con ella se reconocen el escritorio y la barra de tareas).
pub fn clase(ventana: HWND) -> String {
    let mut texto = [0u16; 64];
    let largo = unsafe { GetClassNameW(ventana, &mut texto) }.max(0) as usize;
    String::from_utf16_lossy(&texto[..largo])
}
