//! Portapapeles de Windows: copiar y leer texto. Lo usan la bóveda de textos y los campos de
//! texto de la interfaz (Ctrl+C / Ctrl+V, ver `plataforma.rs`). En nook Pro la bóveda también
//! copia archivos e imágenes (`poner_datos`).

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;

/// Pone `texto` en el portapapeles. Devuelve si se pudo (otro programa puede tenerlo abierto).
pub fn poner(texto: &str) -> bool {
    let utf16: Vec<u8> = texto.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
    poner_datos(&[(CF_UNICODETEXT.0 as u32, &utf16)])
}

/// Pone en el portapapeles estos datos, cada uno en su formato. Devuelve si se pudo poner el
/// primero (los demás son otras formas de lo mismo, por si el programa donde se pega no lo
/// entiende).
pub fn poner_datos(datos: &[(u32, &[u8])]) -> bool {
    unsafe {
        if OpenClipboard(None).is_err() {
            return false;
        }
        let ok = EmptyClipboard().is_ok() && datos.first().is_some_and(|&(formato, datos)| poner_formato(formato, datos));
        if ok {
            for &(formato, datos) in &datos[1..] {
                poner_formato(formato, datos);
            }
        }
        let _ = CloseClipboard();
        ok
    }
}

/// Con el portapapeles abierto, pone `datos` en `formato`.
unsafe fn poner_formato(formato: u32, datos: &[u8]) -> bool {
    unsafe {
        let Ok(memoria) = GlobalAlloc(GMEM_MOVEABLE, datos.len()) else { return false };
        let destino = GlobalLock(memoria) as *mut u8;
        if destino.is_null() {
            let _ = GlobalFree(Some(memoria));
            return false;
        }
        std::ptr::copy_nonoverlapping(datos.as_ptr(), destino, datos.len());
        let _ = GlobalUnlock(memoria);
        // Si Windows acepta los datos, la memoria pasa a ser suya; si no, hay que liberarla.
        if SetClipboardData(formato, Some(HANDLE(memoria.0))).is_err() {
            let _ = GlobalFree(Some(memoria));
            return false;
        }
        true
    }
}

/// Texto que hay en el portapapeles, si hay alguno.
pub fn leer() -> Option<String> {
    unsafe {
        OpenClipboard(None).ok()?;
        let texto = (|| {
            let datos = GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?;
            let memoria = HGLOBAL(datos.0);
            let origen = GlobalLock(memoria) as *const u16;
            if origen.is_null() {
                return None;
            }
            let mut largo = 0;
            while *origen.add(largo) != 0 {
                largo += 1;
            }
            let texto = String::from_utf16_lossy(std::slice::from_raw_parts(origen, largo));
            let _ = GlobalUnlock(memoria);
            Some(texto)
        })();
        let _ = CloseClipboard();
        texto
    }
}
