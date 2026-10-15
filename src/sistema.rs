//! Lanzar aplicaciones, listar las instaladas y obtener sus iconos usando el shell de Windows.

use crate::abiertas;
use crate::buscador;
use crate::config::App;
use crate::dibujo;
use std::collections::HashSet;
use std::ffi::c_void;
use std::mem::size_of;
use std::path::Path;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDIBits, GetObjectW,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::System::Com::{CoTaskMemFree, IBindCtx};
use windows::Win32::UI::Controls::{HIMAGELIST, ILD_TRANSPARENT, IImageList, ImageList_GetIcon};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, ILFree, IShellItem, IShellItem2, KF_FLAG_DEFAULT,
    SHFILEINFOW, SHGFI_FLAGS, SHGFI_LARGEICON, SHGFI_PIDL, SHGFI_SYSICONINDEX, SHGetFileInfoW,
    SHGetImageList, SHGetKnownFolderItem, SHIL_EXTRALARGE, SHIL_JUMBO, SHParseDisplayName, SIGDN, SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
    ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO, SW_SHOWNORMAL};
use windows::core::{HSTRING, Interface, PCWSTR, w};

/// Las apps instaladas se guardan como una ruta dentro de la carpeta virtual "Aplicaciones"
/// del menú Inicio (así se abren igual las de escritorio y las de la Tienda).
const CARPETA_APPS: &str = r"shell:AppsFolder\";

/// Abre la aplicación (o acceso directo/archivo) igual que si se hiciera doble clic en ella.
pub fn lanzar(ruta: &str) -> Result<(), String> {
    // Las rutas "shell:" y las direcciones (web o de Windows, como "ms-actioncenter:") no son
    // archivos: no tienen carpeta en la que arrancar.
    let carpeta = if es_ruta_shell(ruta) || es_direccion(ruta) {
        None
    } else {
        Path::new(ruta).parent().map(|p| HSTRING::from(p.as_os_str()))
    };
    let resultado = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &HSTRING::from(ruta),
            PCWSTR::null(),
            carpeta.as_ref().map_or(PCWSTR::null(), |c| PCWSTR(c.as_ptr())),
            SW_SHOWNORMAL,
        )
    };
    // Según la documentación de Windows, un valor mayor que 32 indica éxito.
    if resultado.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("no se pudo abrir {ruta} (código {})", resultado.0 as isize))
    }
}

fn es_ruta_shell(ruta: &str) -> bool {
    ruta.get(..6).is_some_and(|p| p.eq_ignore_ascii_case("shell:"))
}

/// Si empieza por un esquema ("https:", "ms-settings:"…): letras, cifras, "+", "-" o "." antes
/// de los dos puntos, más de una (con una es la unidad de una ruta, "C:").
fn es_direccion(ruta: &str) -> bool {
    ruta.split_once(':').is_some_and(|(esquema, _)| {
        esquema.len() > 1
            && esquema.starts_with(|c: char| c.is_ascii_alphabetic())
            && esquema.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    })
}

/// Aplicaciones instaladas, las mismas que lista el menú Inicio (de escritorio y de la Tienda),
/// sin documentos, enlaces web ni desinstaladores; cada una con sus claves (para reconocer sus
/// ventanas, ver `abiertas`).
pub fn apps_instaladas() -> Vec<(App, Vec<String>)> {
    let mut apps = Vec::new();
    let mut vistas = HashSet::new();
    unsafe {
        let Ok(carpeta) = SHGetKnownFolderItem::<IShellItem>(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None) else {
            return apps;
        };
        let Ok(lista) = carpeta.BindToHandler::<_, IEnumShellItems>(None::<&IBindCtx>, &BHID_EnumItems) else {
            return apps;
        };
        loop {
            let mut item = [None];
            let mut leidos = 0;
            if lista.Next(&mut item, Some(&mut leidos)).is_err() || leidos == 0 {
                break;
            }
            let Some(item) = item[0].take() else { break };
            let nombre = nombre_de(&item, SIGDN_NORMALDISPLAY);
            let id = nombre_de(&item, SIGDN_PARENTRELATIVEPARSING);
            let (Some(nombre), Some(id)) = (nombre, id) else { continue };
            let ruta = format!("{CARPETA_APPS}{id}");
            if buscador::es_aplicacion(&nombre, &ruta) && vistas.insert(ruta.clone()) {
                let claves = item.cast::<IShellItem2>().map_or(Vec::new(), |item| abiertas::claves_de_item(&item, Some(&id)));
                apps.push((App { nombre, ruta }, claves));
            }
        }
    }
    apps
}

fn nombre_de(item: &IShellItem, tipo: SIGDN) -> Option<String> {
    unsafe {
        let texto = item.GetDisplayName(tipo).ok()?;
        let nombre = texto.to_string().ok();
        CoTaskMemFree(Some(texto.0 as *const c_void));
        nombre.filter(|n| !n.is_empty())
    }
}

/// Icono que Windows muestra para ese archivo o app, de `lado` × `lado` píxeles, listo para
/// Slint. Se dibuja a ese tamaño exacto porque el dibujo de Slint escala las imágenes píxel a
/// píxel y deja los bordes en sierra: se parte del icono más grande que haya (256 o 48 píxeles)
/// y se reduce promediando.
pub fn icono(ruta: &str, lado: u32) -> Option<slint::Image> {
    pixeles_icono(ruta, lado).map(|pixeles| imagen(&pixeles, lado))
}

/// Los píxeles RGBA de `icono`, que (a diferencia de la imagen) se pueden sacar en otro hilo.
pub fn pixeles_icono(ruta: &str, lado: u32) -> Option<Vec<u8>> {
    let (lista, indice) = indice_de_icono(ruta)?;
    let (pixeles, ancho, alto) = mejor_imagen(lista, indice)?;
    Some(dibujo::redimensionar(&pixeles, ancho, alto, lado))
}

/// Los píxeles RGBA de un icono de Windows (por ejemplo, el de otro programa en el área de
/// notificación de nook Pro), reducido a `lado` × `lado`.
#[cfg_attr(not(pro), allow(dead_code))]
pub fn pixeles_de_hicon(icono: HICON, lado: u32) -> Option<Vec<u8>> {
    let (pixeles, ancho, alto) = icono_a_pixeles(icono)?;
    Some(dibujo::redimensionar(&pixeles, ancho, alto, lado))
}

/// Imagen para Slint de los píxeles de un icono de `lado` × `lado`.
pub fn imagen(pixeles: &[u8], lado: u32) -> slint::Image {
    let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(pixeles, lado, lado);
    slint::Image::from_rgba8(buffer)
}

/// Posición del icono del archivo o app en las listas de iconos del sistema (y la lista de
/// 32 píxeles, por si no hay otra mejor).
fn indice_de_icono(ruta: &str) -> Option<(HIMAGELIST, i32)> {
    unsafe {
        if es_ruta_shell(ruta) {
            // Las apps de la carpeta "Aplicaciones" no son archivos: se piden por su PIDL.
            let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
            SHParseDisplayName(&HSTRING::from(ruta), None::<&IBindCtx>, &mut pidl, 0, None).ok()?;
            let indice = pedir_indice(PCWSTR(pidl as *const u16), SHGFI_PIDL);
            ILFree(Some(pidl));
            indice
        } else {
            let ruta = HSTRING::from(ruta);
            pedir_indice(PCWSTR(ruta.as_ptr()), SHGFI_FLAGS(0))
        }
    }
}

/// Se pide el índice en la lista de iconos del sistema, no el icono ya hecho: Windows guarda
/// aparte las capas que pinta encima (como la flecha de los accesos directos) y solo las añade
/// si se le piden; con el icono ya hecho (`SHGFI_ICON`) los accesos directos salían con la flecha.
unsafe fn pedir_indice(ruta: PCWSTR, extra: SHGFI_FLAGS) -> Option<(HIMAGELIST, i32)> {
    unsafe {
        let mut info = SHFILEINFOW::default();
        let lista = SHGetFileInfoW(
            ruta,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX | SHGFI_LARGEICON | extra,
        );
        (lista != 0).then_some((HIMAGELIST(lista as isize), info.iIcon))
    }
}

/// Píxeles RGBA del icono más grande disponible: el de 256 píxeles si el programa lo trae de
/// verdad (si no, Windows pone el pequeño en una esquina de un cuadro vacío), si no el de 48 y,
/// en último caso, el de 32.
fn mejor_imagen(lista_32: HIMAGELIST, indice: i32) -> Option<(Vec<u8>, u32, u32)> {
    for lista in [SHIL_JUMBO, SHIL_EXTRALARGE] {
        let Ok(lista) = (unsafe { SHGetImageList::<IImageList>(lista as i32) }) else { continue };
        let Ok(icono) = (unsafe { lista.GetIcon(indice, ILD_TRANSPARENT.0) }) else { continue };
        let imagen = icono_a_pixeles(icono);
        unsafe {
            let _ = DestroyIcon(icono);
        }
        if let Some((pixeles, ancho, alto)) = imagen {
            let (ocupado_ancho, ocupado_alto) = tamano_ocupado(&pixeles, ancho, alto);
            if ancho <= 48 || (ocupado_ancho > 48 || ocupado_alto > 48) {
                return Some((pixeles, ancho, alto));
            }
        }
    }
    let icono = unsafe { ImageList_GetIcon(lista_32, indice, ILD_TRANSPARENT) };
    if icono.is_invalid() {
        return None;
    }
    let imagen = icono_a_pixeles(icono);
    unsafe {
        let _ = DestroyIcon(icono);
    }
    imagen
}

/// Ancho y alto de lo que no es transparente, contando desde la esquina superior izquierda.
fn tamano_ocupado(pixeles: &[u8], ancho: u32, alto: u32) -> (u32, u32) {
    let (mut x_max, mut y_max) = (0, 0);
    for y in 0..alto {
        for x in 0..ancho {
            if pixeles[((y * ancho + x) * 4 + 3) as usize] > 0 {
                x_max = x_max.max(x + 1);
                y_max = y_max.max(y + 1);
            }
        }
    }
    (x_max, y_max)
}

/// Copia los píxeles de un HICON (RGBA, alfa sin premultiplicar).
fn icono_a_pixeles(icono: HICON) -> Option<(Vec<u8>, u32, u32)> {
    unsafe {
        let mut partes = ICONINFO::default();
        GetIconInfo(icono, &mut partes).ok()?;
        let color = partes.hbmColor;
        let _ = DeleteObject(partes.hbmMask.into());
        if color.is_invalid() {
            return None; // icono monocromo antiguo: se usará la inicial
        }

        let mut mapa = BITMAP::default();
        GetObjectW(
            color.into(),
            size_of::<BITMAP>() as i32,
            Some(&mut mapa as *mut _ as *mut c_void),
        );
        let (ancho, alto) = (mapa.bmWidth, mapa.bmHeight);

        let mut formato = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: ancho,
                biHeight: -alto, // negativo = filas de arriba hacia abajo
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixeles = vec![0u8; (ancho * alto * 4) as usize];
        let dc = CreateCompatibleDC(None);
        let filas = GetDIBits(
            dc,
            color,
            0,
            alto as u32,
            Some(pixeles.as_mut_ptr() as *mut c_void),
            &mut formato,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(dc);
        let _ = DeleteObject(color.into());
        if filas == 0 || ancho <= 0 || alto <= 0 {
            return None;
        }

        Some((bgra_a_rgba(pixeles), ancho as u32, alto as u32))
    }
}

/// Windows entrega los píxeles en orden BGRA; Slint los quiere en RGBA.
fn bgra_a_rgba(mut pixeles: Vec<u8>) -> Vec<u8> {
    // Los iconos antiguos no traen transparencia (alfa = 0 en todo): se hacen opacos.
    let sin_alfa = pixeles.chunks_exact(4).all(|p| p[3] == 0);
    for p in pixeles.chunks_exact_mut(4) {
        p.swap(0, 2);
        if sin_alfa {
            p[3] = 255;
        }
    }
    pixeles
}

const CLAVE_INICIO: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NOMBRE_INICIO: &str = "Nook";

/// Indica si la barra está configurada para arrancar al iniciar sesión en Windows.
pub fn inicia_con_windows() -> bool {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(CLAVE_INICIO),
            &HSTRING::from(NOMBRE_INICIO),
            RRF_RT_REG_SZ,
            None,
            None,
            None,
        )
        .is_ok()
    }
}

/// Activa o desactiva el arranque con Windows (clave Run del usuario actual).
pub fn iniciar_con_windows(activar: bool) -> Result<(), String> {
    use windows::Win32::System::Registry::{
        HKEY_CURRENT_USER, REG_SZ, RegDeleteKeyValueW, RegSetKeyValueW,
    };
    let resultado = unsafe {
        if activar {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            // Valor REG_SZ: texto UTF-16 terminado en cero, con la ruta entre comillas.
            let valor: Vec<u16> =
                format!("\"{}\"", exe.display()).encode_utf16().chain(std::iter::once(0)).collect();
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                &HSTRING::from(CLAVE_INICIO),
                &HSTRING::from(NOMBRE_INICIO),
                REG_SZ.0,
                Some(valor.as_ptr() as *const c_void),
                (valor.len() * 2) as u32,
            )
        } else {
            RegDeleteKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(CLAVE_INICIO), &HSTRING::from(NOMBRE_INICIO))
        }
    };
    resultado.ok().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distingue_direcciones_de_rutas() {
        assert!(es_direccion("ms-actioncenter:"));
        assert!(es_direccion("https://example.com"));
        assert!(!es_direccion(r"C:\Program Files\app.exe"));
        assert!(!es_direccion(r"\servidor\carpeta"));
        assert!(!es_direccion("notas.txt"));
    }
}
