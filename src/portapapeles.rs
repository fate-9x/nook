//! Portapapeles de Windows: copiar y leer texto. Lo usan la bóveda de textos y los campos de
//! texto de la interfaz (Ctrl+C / Ctrl+V, ver `plataforma.rs`).

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;

/// Pone `texto` en el portapapeles. Devuelve si se pudo (otro programa puede tenerlo abierto).
pub fn poner(texto: &str) -> bool {
    let mut utf16: Vec<u16> = texto.encode_utf16().collect();
    utf16.push(0);
    unsafe {
        if OpenClipboard(None).is_err() {
            return false;
        }
        let ok = (|| {
            EmptyClipboard().ok()?;
            let memoria = GlobalAlloc(GMEM_MOVEABLE, utf16.len() * 2).ok()?;
            let destino = GlobalLock(memoria) as *mut u16;
            if destino.is_null() {
                let _ = GlobalFree(Some(memoria));
                return None;
            }
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), destino, utf16.len());
            let _ = GlobalUnlock(memoria);
            // Si Windows acepta los datos, la memoria pasa a ser suya; si no, hay que liberarla.
            if SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(memoria.0))).is_err() {
                let _ = GlobalFree(Some(memoria));
                return None;
            }
            Some(())
        })()
        .is_some();
        let _ = CloseClipboard();
        ok
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
