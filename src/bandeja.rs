//! Icono en el área de notificación de Windows (junto al reloj o en la flecha de iconos ocultos).
//!
//! Windows avisa de los clics en el icono con el mensaje `WM_BANDEJA`, que recibe la ventana de
//! la barra (ver `plataforma.rs`).

use std::ffi::c_void;
use std::mem::size_of;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS, DeleteObject,
};
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, HICON, ICONINFO, SM_CXSMICON, WM_APP,
};

/// Mensaje con el que Windows avisa de lo que pasa en el icono.
pub const WM_BANDEJA: u32 = WM_APP + 2;

/// Icono que se muestra ahora (para destruirlo al cambiarlo o al salir).
static ICONO: AtomicUsize = AtomicUsize::new(0);

/// Lado en píxeles del icono del área de notificación (depende del zoom de la pantalla).
pub fn lado(hwnd: HWND) -> u32 {
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    unsafe { GetSystemMetricsForDpi(SM_CXSMICON, dpi) }.max(16) as u32
}

/// Añade el icono al área de notificación con esa imagen (RGBA de `lado` × `lado`, alfa sin
/// premultiplicar). Si ya estaba, lo actualiza (por ejemplo, al cambiar el personaje).
pub fn poner(hwnd: HWND, rgba: &[u8], lado: u32) {
    let Some(icono) = crear_icono(rgba, lado as i32) else { return };

    let mut datos = datos(hwnd);
    datos.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP;
    datos.uCallbackMessage = WM_BANDEJA;
    datos.hIcon = icono;
    for (destino, c) in datos.szTip.iter_mut().zip("Nook".encode_utf16()) {
        *destino = c;
    }
    unsafe {
        // Si ya existe se modifica; si no (primera vez o el Explorador se reinició), se añade.
        if !Shell_NotifyIconW(NIM_MODIFY, &datos).as_bool() {
            let _ = Shell_NotifyIconW(NIM_ADD, &datos);
        }
        datos.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        let _ = Shell_NotifyIconW(NIM_SETVERSION, &datos);
    }
    destruir_icono(ICONO.swap(icono.0 as usize, Ordering::Relaxed));
}

/// Quita el icono (al cerrar la barra).
pub fn quitar(hwnd: HWND) {
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &datos(hwnd));
    }
    destruir_icono(ICONO.swap(0, Ordering::Relaxed));
}

fn datos(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW { cbSize: size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, ..Default::default() }
}

fn destruir_icono(valor: usize) {
    if valor != 0 {
        unsafe {
            let _ = DestroyIcon(HICON(valor as *mut c_void));
        }
    }
}

/// Icono de Windows con esa imagen RGBA.
fn crear_icono(rgba: &[u8], lado: i32) -> Option<HICON> {
    if rgba.len() != (lado * lado * 4) as usize {
        return None;
    }
    // Windows quiere los píxeles en BGRA.
    let pixeles: Vec<u8> = rgba.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
    unsafe {
        let formato = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: lado,
                biHeight: -lado, // negativo = filas de arriba abajo
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let color = CreateDIBSection(None, &formato, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        std::ptr::copy_nonoverlapping(pixeles.as_ptr(), bits as *mut u8, pixeles.len());
        // Máscara a cero: la transparencia la da el canal alfa del color.
        let fila = ((lado + 15) / 16 * 2) as usize;
        let ceros = vec![0u8; fila * lado as usize];
        let mascara = CreateBitmap(lado, lado, 1, 1, Some(ceros.as_ptr() as *const c_void));
        let info = ICONINFO { fIcon: true.into(), xHotspot: 0, yHotspot: 0, hbmMask: mascara, hbmColor: color };
        let icono = CreateIconIndirect(&info);
        let _ = DeleteObject(color.into());
        let _ = DeleteObject(mascara.into());
        icono.ok()
    }
}
