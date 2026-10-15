//! La barra de tareas de Windows, para que Nook la sustituya: se oculta en todos los monitores y
//! se pone en "ocultar automáticamente", que es lo que hace que deje de reservar su sitio (sin
//! eso, las ventanas maximizadas dejarían un hueco donde estaba). Al dejar de sustituirla vuelve
//! como estaba.

use std::mem::size_of;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::Shell::{ABM_GETSTATE, ABM_SETSTATE, ABS_ALWAYSONTOP, ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, IsWindowVisible, SW_HIDE, SW_SHOWNA, ShowWindow};
use windows::core::BOOL;

use crate::ventana;

/// La barra del monitor principal y las de los demás.
const CLASES: [&str; 2] = ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

/// Las barras de tareas de todos los monitores (la principal, la primera).
fn barras() -> Vec<HWND> {
    let mut todas: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(recoger), LPARAM(&mut todas as *mut Vec<HWND> as isize));
    }
    // Sin la de Nook, que también es de esa clase cuando muestra el área de notificación.
    let mut barras: Vec<HWND> = todas
        .into_iter()
        .filter(|&h| !ventana::es_de_este_proceso(h) && CLASES.contains(&ventana::clase(h).as_str()))
        .collect();
    barras.sort_by_key(|&h| ventana::clase(h) != CLASES[0]);
    barras
}

unsafe extern "system" fn recoger(hwnd: HWND, datos: LPARAM) -> BOOL {
    unsafe { (*(datos.0 as *mut Vec<HWND>)).push(hwnd) };
    true.into()
}

/// Si se ve alguna barra de tareas.
pub fn se_ve() -> bool {
    barras().into_iter().any(|h| unsafe { IsWindowVisible(h) }.as_bool())
}

/// Oculta las barras de tareas. Devuelve si hubo que activar "ocultar automáticamente" (estaba
/// desactivado): hay que recordarlo para dejarlo como estaba.
pub fn ocultar() -> bool {
    let activar = !se_oculta_sola();
    if activar {
        poner_ocultar_sola(true);
    }
    for barra in barras() {
        unsafe {
            let _ = ShowWindow(barra, SW_HIDE);
        }
    }
    activar
}

/// Vuelve a mostrar las barras de tareas y, si Nook lo activó (`quitar_ocultar_sola`), desactiva
/// "ocultar automáticamente".
pub fn mostrar(quitar_ocultar_sola: bool) {
    if quitar_ocultar_sola {
        poner_ocultar_sola(false);
    }
    for barra in barras() {
        unsafe {
            let _ = ShowWindow(barra, SW_SHOWNA);
        }
    }
}

fn datos() -> APPBARDATA {
    APPBARDATA { cbSize: size_of::<APPBARDATA>() as u32, ..Default::default() }
}

/// Si está activado "ocultar automáticamente la barra de tareas".
fn se_oculta_sola() -> bool {
    let mut datos = datos();
    unsafe { SHAppBarMessage(ABM_GETSTATE, &mut datos) as u32 & ABS_AUTOHIDE != 0 }
}

fn poner_ocultar_sola(si: bool) {
    let Some(&principal) = barras().first() else { return };
    let mut datos = datos();
    datos.hWnd = principal;
    datos.lParam = LPARAM((if si { ABS_AUTOHIDE } else { ABS_ALWAYSONTOP }) as isize);
    unsafe {
        SHAppBarMessage(ABM_SETSTATE, &mut datos);
    }
}
