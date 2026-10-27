//! Plataforma propia para Slint: una "ventana en capas" de Windows con transparencia por píxel.
//!
//! Slint dibuja por software (poca memoria, texto nítido) en un mapa de bits con canal alfa, y
//! se entrega a Windows con `UpdateLayeredWindow`. Así la ventana puede tener un tamaño fijo y
//! ser transparente fuera de la barra: la forma (con esquinas suavizadas) y sus animaciones las
//! dibuja Slint, y los clics sobre los píxeles transparentes pasan solos a lo que hay debajo.
//!
//! Este módulo sustituye a winit: crea las ventanas, lleva el bucle de mensajes y traduce el
//! ratón, el teclado y el arrastre de archivos a eventos de Slint.
//!
//! La primera ventana que pide Slint es la barra; las siguientes son emergentes (menús y el
//! selector de color), que se muestran junto al cursor y se ocultan al perder el foco.

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType, TargetPixel,
};
use slint::platform::{
    Key, Platform, PlatformError, PointerEventButton, WindowAdapter, WindowEvent,
};
use slint::{LogicalPosition, LogicalSize, PhysicalSize, SharedString};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::mem::size_of;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use crate::bandeja::WM_BANDEJA;
use windows::Win32::Foundation::{
    COLORREF, CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, POINTL, RECT, SIZE, WPARAM,
};
use windows::System::DispatcherQueueController;
use windows::UI::Composition::Desktop::DesktopWindowTarget;
use windows::UI::Composition::{Compositor, RectangleClip, SpriteVisual};
use windows::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_USE_HOSTBACKDROPBRUSH, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    DWMWCP_ROUND, DwmFlush, DwmSetWindowAttribute,
};
use windows::Win32::System::WinRT::Composition::ICompositorDesktopInterop;
use windows::Win32::System::WinRT::{CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions};
use windows_core::Interface;
use windows_numerics::{Vector2, Vector3};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CombineRgn, CreateCompatibleDC, CreateDIBSection, CreateEllipticRgn, CreateRectRgn, HRGN, RGN_DIFF, RGN_OR, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetMonitorInfoW, HBITMAP, HDC,
    HGDIOBJ, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, ScreenToClient, SelectObject, SetWindowRgn,

};
use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Ole::{
    CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_NONE, IDropTarget,
    IDropTarget_Impl, OleInitialize, RegisterDragDrop, ReleaseStgMedium,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::System::Threading::{CreateMutexW, GetCurrentThreadId, INFINITE};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
    VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F12,
    VK_HOME, VK_INSERT, VK_LEFT, VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT,
    VK_RWIN, VK_SHIFT, VK_TAB, VK_UP,
};
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP, NINF_KEY, NIN_SELECT};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW,
    DispatchMessageW, FindWindowW, GetWindowThreadProcessId, HWND_TOPMOST, IDC_ARROW, LoadCursorW,
    MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, PM_NOREMOVE, PM_REMOVE, PeekMessageW,
    PostMessageW, PostQuitMessage, PostThreadMessageW, QS_ALLINPUT, RegisterClassW,
    RegisterWindowMessageW, SW_HIDE, SW_SHOWNOACTIVATE, SetForegroundWindow, HTTRANSPARENT,
    MA_NOACTIVATE, SWP_NOCOPYBITS, SWP_NOREDRAW, SWP_NOZORDER, SWP_SHOWWINDOW, WM_MOUSEACTIVATE, WM_NCHITTEST, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    ShowWindow, TranslateMessage, ULW_ALPHA, UPDATELAYEREDWINDOWINFO, UpdateLayeredWindowIndirect, WA_INACTIVE, WM_ACTIVATE,
    WM_APP, WM_CHAR, WM_CLOSE, WM_CONTEXTMENU, WM_DPICHANGED, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, s, w};
use windows_core::{Ref, implement};

/// Rectángulo en píxeles físicos de pantalla: (x, y, ancho, alto).
pub type Rect = (i32, i32, i32, i32);

/// Parte de la forma visible de una ventana: un rectángulo (relativo a la ventana, en píxeles
/// físicos) y el radio de cada esquina, desde la de arriba a la izquierda en el sentido de las
/// agujas del reloj. A los empalmes entre la tira y el panel les falta un cuarto de círculo (del
/// radio del lado) centrado en la esquina `hueco`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Pieza {
    pub rect: Rect,
    pub radios: [i32; 4],
    pub hueco: Option<usize>,
}

impl Pieza {
    fn vacia(&self) -> bool {
        self.rect.2 <= 0 || self.rect.3 <= 0
    }
}

/// Forma visible de una ventana: en la barra, la tira y el panel abierto (cada uno de su largo,
/// ver ui/barra.slint) y los empalmes entre ellos; en las emergentes, una sola pieza. Las piezas
/// sin tamaño no cuentan.
pub type Forma = [Pieza; 4];

/// El ratón salió de la ventana (lo pide `TrackMouseEvent`).
const WM_MOUSELEAVE: u32 = 0x02A3;
const WM_DISPLAYCHANGE: u32 = 0x007E;
const WM_POWERBROADCAST: u32 = 0x0218;
/// Avisos de WM_POWERBROADCAST de que el equipo volvió de la suspensión (el primero llega
/// siempre; el segundo, si fue el usuario quien lo despertó).
const PBT_APMRESUMESUSPEND: usize = 0x0007;
const PBT_APMRESUMEAUTOMATIC: usize = 0x0012;

/// Mensaje para despertar el bucle cuando otro hilo pide ejecutar algo en él.
const WM_DESPERTAR: u32 = WM_APP + 1;

/// Mensaje que envía una segunda copia del programa a la primera antes de cerrarse.
const WM_OTRA_INSTANCIA: u32 = WM_APP + 3;

/// Clic en el icono del área de notificación hecho con el teclado.
const NIN_KEYSELECT: u32 = NIN_SELECT | NINF_KEY;

/// Clase de las ventanas del programa y título de la barra (con ellos otra copia la encuentra).
const CLASE: PCWSTR = w!("NookVentana");
const TITULO_BARRA: PCWSTR = w!("Nookly");
/// Clase de los "cristales" que desenfocan lo de detrás (ver `Interna::actualizar_cristal`).
const CLASE_CRISTAL: PCWSTR = w!("NookCristal");

/// Hilo del bucle de eventos (para `quit_event_loop` / `invoke_from_event_loop`).
static HILO: AtomicU32 = AtomicU32::new(0);
/// Mensaje que envía el Explorador al reiniciarse (hay que volver a poner el icono).
static BARRA_DE_TAREAS_CREADA: AtomicU32 = AtomicU32::new(0);
/// Tareas enviadas desde otros hilos con `slint::invoke_from_event_loop`.
static PENDIENTES: Mutex<Vec<Box<dyn FnOnce() + Send>>> = Mutex::new(Vec::new());

thread_local! {
    /// Ventanas creadas, en orden: la primera es la barra.
    static VENTANAS: RefCell<Vec<Rc<Interna>>> = const { RefCell::new(Vec::new()) };
    /// Si lo que hay detrás de las ventanas se ve desenfocado.
    static DESENFOQUE: Cell<bool> = const { Cell::new(false) };
    /// Tamaño de la interfaz elegido por el usuario (1 = normal); se suma al zoom de la pantalla.
    static TAMANO: Cell<f32> = const { Cell::new(1.0) };
    /// Composición de Windows con la que se dibuja el desenfoque (ver `Vidrio`), si está disponible.
    static ESCENA: RefCell<Option<Escena>> = const { RefCell::new(None) };
}

/// El compositor de Windows y la cola de mensajes que necesita en este hilo.
struct Escena {
    _cola: DispatcherQueueController,
    compositor: Compositor,
}

/// Desenfoque dibujado con la composición de Windows dentro del cristal: un pincel con lo que hay
/// detrás, ya desenfocado, recortado con las mismas esquinas redondeadas que la barra. Sin él (en
/// Windows 10), el cristal usa el desenfoque antiguo, que solo sabe redondear 8 px.
struct Vidrio {
    _destino: DesktopWindowTarget,
    /// Uno por pieza de la forma, cada uno con su recorte.
    piezas: [(SpriteVisual, RectangleClip); 4],
}

/// Cuánto se mete el cristal hacia dentro de la forma visible con el desenfoque antiguo, en
/// píxeles lógicos (ver `Interna::actualizar_cristal`): así no asoma por las esquinas, que Windows
/// redondea menos que la barra.
const MARGEN_CRISTAL: f32 = 2.0;

/// Eventos de arrastre de archivos sobre la ventana (posiciones en píxeles de pantalla).
pub enum Arrastre {
    Entra,
    Mueve(i32, i32),
    Sale,
    Suelta(Vec<PathBuf>, i32, i32),
}

/// Avisos de Windows que no son de ratón ni de teclado.
pub enum Aviso {
    /// Clic en el icono del área de notificación.
    ClicIcono,
    /// Clic derecho en el icono del área de notificación.
    MenuIcono,
    /// El Explorador se reinició y el icono desapareció.
    IconoPerdido,
    /// Se intentó abrir otra barra: esta debe mostrarse.
    OtraInstancia,
    /// La barra pasó a un monitor con otro zoom: hay que recalcular su tamaño.
    EscalaCambiada,
    /// Llegó una notificación o un programa pide atención (ver `avisos.rs`).
    Notificacion,
    /// El equipo volvió de la suspensión.
    Reanudado,
    /// Cambió la resolución o la disposición de las pantallas.
    PantallasCambiadas,
}

struct Interna {
    hwnd: HWND,
    slint: Rc<MinimalSoftwareWindow>,
    /// La barra (la primera ventana); las demás son emergentes.
    es_barra: bool,
    /// Zoom de la pantalla en la que está (1,25 = 125 %) y escala con la que dibuja Slint: en la
    /// barra, el zoom por el tamaño de la interfaz que eligió el usuario; en las emergentes, solo
    /// el zoom.
    zoom: Cell<f32>,
    escala: Cell<f32>,
    /// Posición y tamaño de la ventana (el lienzo).
    lienzo: Cell<Rect>,
    lienzo_presentado: Cell<Rect>,
    mapa: RefCell<Option<MapaDeBits>>,
    visible: Cell<bool>,
    /// La ventana debe estar oculta (emergentes cerradas): no se dibuja ni se muestra.
    oculta: Cell<bool>,
    /// El próximo dibujo debe ser completo. Slint repinta solo lo que cambió sobre el dibujo
    /// anterior; tras un cambio de escala o de contenido grande, ese dibujo deja restos.
    repintar_todo: Cell<bool>,
    /// "Cristal": ventana normal, sin contenido y con el desenfoque de Windows, que va justo debajo
    /// de esta y con la forma de lo visible (ver `actualizar_cristal`). Es la dueña de esta
    /// ventana, así que Windows la mantiene siempre por debajo.
    cristal: HWND,
    vidrio: Option<Vidrio>,
    /// Forma visible (relativa a la ventana), dónde está puesto el cristal ahora, y lo que se
    /// llama antes de cada dibujo para actualizar la forma (cambia durante las animaciones).
    recorte: Cell<Option<Forma>>,
    cristal_puesto: Cell<Option<Forma>>,
    /// Dónde está la ventana del cristal (con la composición, cubre todo el lienzo).
    cristal_ventana: Cell<Option<Rect>>,
    al_presentar: RefCell<Option<Box<dyn Fn()>>>,
    /// Si el ratón está sobre la ventana (para avisar a Slint cuando sale).
    raton_dentro: Cell<bool>,
    /// Primera mitad de un carácter fuera del plano básico (emoji…) recibido en WM_CHAR.
    sustituto_alto: Cell<Option<u16>>,
    al_activarse: RefCell<Option<Box<dyn Fn(bool)>>>,
    al_arrastrar: RefCell<Option<Box<dyn Fn(Arrastre)>>>,
    al_atajo: RefCell<Option<Box<dyn Fn(i32)>>>,
    al_aviso: RefCell<Option<Box<dyn Fn(Aviso)>>>,
}

/// Píxel del mapa de bits de Windows: azul, verde, rojo y alfa, premultiplicados.
#[repr(C)]
#[derive(Clone, Copy)]
struct Bgra {
    b: u8,
    g: u8,
    r: u8,
    a: u8,
}

impl TargetPixel for Bgra {
    /// Pone `c` encima: destino · (1 − α) + origen (colores premultiplicados). Es lo que más se
    /// repite al dibujar el fondo translúcido, por eso se evita dividir.
    fn blend(&mut self, c: PremultipliedRgbaColor) {
        let resto = (u8::MAX - c.alpha) as u32;
        let encima = |debajo: u8, nuevo: u8| entre_255(debajo as u32 * resto).saturating_add(nuevo);
        self.r = encima(self.r, c.red);
        self.g = encima(self.g, c.green);
        self.b = encima(self.b, c.blue);
        self.a = encima(self.a, c.alpha);
    }

    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { b, g, r, a: 255 }
    }

    fn background() -> Self {
        Self { b: 0, g: 0, r: 0, a: 0 }
    }
}

/// `v / 255` redondeado, sin dividir (para `v` hasta 255 · 255).
fn entre_255(v: u32) -> u8 {
    let v = v + 128;
    ((v + (v >> 8)) >> 8) as u8
}

/// Mapa de bits en memoria donde dibuja Slint y que se entrega a Windows.
struct MapaDeBits {
    dc: HDC,
    bitmap: HBITMAP,
    anterior: HGDIOBJ,
    pixeles: *mut Bgra,
    ancho: i32,
    alto: i32,
}

impl MapaDeBits {
    fn nuevo(ancho: i32, alto: i32) -> Option<Self> {
        let formato = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: ancho,
                biHeight: -alto, // negativo = filas de arriba abajo
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        unsafe {
            let dc = CreateCompatibleDC(None);
            let mut bits: *mut c_void = std::ptr::null_mut();
            let bitmap = CreateDIBSection(Some(dc), &formato, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
            let anterior = SelectObject(dc, bitmap.into());
            Some(Self { dc, bitmap, anterior, pixeles: bits as *mut Bgra, ancho, alto })
        }
    }

    fn pixeles(&mut self) -> &mut [Bgra] {
        unsafe { std::slice::from_raw_parts_mut(self.pixeles, (self.ancho * self.alto) as usize) }
    }
}

impl Drop for MapaDeBits {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.anterior);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

impl Interna {
    /// Dibuja lo que haya cambiado y lo entrega a Windows.
    fn presentar(&self) {
        let (x, y, ancho, alto) = self.lienzo.get();
        if ancho <= 0 || alto <= 0 || self.oculta.get() {
            return;
        }
        if let Some(f) = &*self.al_presentar.borrow() {
            f();
        }
        let mut mapa = self.mapa.borrow_mut();
        let mapa_nuevo = mapa.as_ref().is_none_or(|m| m.ancho != ancho || m.alto != alto);
        if mapa_nuevo {
            *mapa = MapaDeBits::nuevo(ancho, alto);
        }
        // Slint tiene que dibujar exactamente al tamaño del mapa de bits (si no, el renderizador
        // se detiene). Al cambiar la escala, por ejemplo al pasar a un monitor con otro zoom,
        // Slint recalcula su tamaño por su cuenta, así que se iguala aquí antes de cada dibujo.
        let tamano = PhysicalSize::new(ancho as u32, alto as u32);
        if mapa_nuevo || self.slint.window().size() != tamano {
            self.slint.set_size(tamano);
            self.repintar_todo.set(true);
            self.slint.request_redraw();
        }
        let Some(mapa) = mapa.as_mut() else { return };

        let dibujado_antes = self.lienzo_presentado.get();
        let mut repintado = None;
        let dibujado = self.slint.draw_if_needed(|renderizador| {
            // Un cuadro como "búfer nuevo" se pinta entero (y limpio); cambiar de tipo además
            // olvida lo que Slint recordaba del dibujo anterior. Luego se vuelve a repintar solo
            // lo que cambia.
            let completo = self.repintar_todo.take();
            if completo {
                // El fondo es transparente: dibujarlo no borra lo anterior. Sin vaciar el mapa,
                // al encoger la interfaz (por ejemplo, al bajar el tamaño) quedaban restos de lo
                // que se veía antes alrededor.
                mapa.pixeles().fill(Bgra { b: 0, g: 0, r: 0, a: 0 });
                renderizador.set_repaint_buffer_type(RepaintBufferType::NewBuffer);
            }
            let region = renderizador.render(mapa.pixeles(), ancho as usize);
            repintado = Some((region.bounding_box_origin(), region.bounding_box_size()));
            if completo {
                renderizador.set_repaint_buffer_type(RepaintBufferType::ReusedBuffer);
            }
        });
        self.actualizar_cristal();
        if !dibujado && dibujado_antes == self.lienzo.get() {
            return;
        }

        let mezcla = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        // Si la ventana sigue en su sitio y con su tamaño, Windows solo copia lo que se repintó
        // (el personaje, por ejemplo), no el lienzo entero, que puede medir varios megas.
        let quieta = self.visible.get() && dibujado_antes == self.lienzo.get();
        let sucio = repintado.filter(|_| quieta).map(|(o, t)| RECT {
            left: o.x,
            top: o.y,
            right: o.x + t.width as i32,
            bottom: o.y + t.height as i32,
        });
        if sucio.is_some_and(|r| r.right <= r.left || r.bottom <= r.top) {
            return;
        }
        let (destino, tamano, origen) = (POINT { x, y }, SIZE { cx: ancho, cy: alto }, POINT { x: 0, y: 0 });
        let info = UPDATELAYEREDWINDOWINFO {
            cbSize: size_of::<UPDATELAYEREDWINDOWINFO>() as u32,
            hdcDst: HDC::default(),
            pptDst: &destino,
            psize: &tamano,
            hdcSrc: mapa.dc,
            pptSrc: &origen,
            crKey: COLORREF(0),
            pblend: &mezcla,
            dwFlags: ULW_ALPHA,
            prcDirty: sucio.as_ref().map_or(std::ptr::null(), |r| r),
        };
        unsafe {
            let _ = UpdateLayeredWindowIndirect(self.hwnd, &info);
            if !self.visible.replace(true) {
                let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
        self.lienzo_presentado.set(self.lienzo.get());
    }

    fn evento(&self, evento: WindowEvent) {
        self.slint.window().dispatch_event(evento);
    }

    fn avisar(&self, aviso: Aviso) {
        if let Some(f) = &*self.al_aviso.borrow() {
            f(aviso);
        }
    }

    /// Pone el cristal debajo de la forma visible, o lo oculta si no hay desenfoque o la ventana
    /// no se ve. Windows desenfoca todo el rectángulo de una ventana en capas, así que el
    /// desenfoque va en esta otra ventana, del tamaño de la forma.
    ///
    /// Con la composición (`Vidrio`) el desenfoque se recorta con el mismo radio que la barra y
    /// llega hasta su borde. La ventana del cristal cubre todo el lienzo y no se mueve con las
    /// animaciones: lo que cambia es dónde va el desenfoque dentro de ella. Windows aplica los
    /// cambios de la composición un poco después que los de las ventanas; si se moviera la
    /// ventana en cada cuadro (como al desplegar la barra en el borde derecho, que crece hacia la
    /// izquierda), el desenfoque se vería desplazado mientras tanto.
    ///
    /// Una región del tamaño de la forma visible limita la ventana: fuera de ella no existe para
    /// el ratón, y los clics llegan a lo que haya detrás (el escritorio y otros programas; sin
    /// ella, toda la zona del lienzo se los quedaba). La región se aplica al momento, así que al
    /// encoger también recorta el desenfoque que aún no se ha actualizado.
    ///
    /// Con el desenfoque antiguo, Windows redondea las esquinas con un radio menor: va metido
    /// `MARGEN_CRISTAL` hacia dentro para no asomar por ellas (y entonces se ve un borde fino sin
    /// desenfocar), y la ventana sí se ajusta a la forma.
    fn actualizar_cristal(&self) {
        let visible = DESENFOQUE.with(Cell::get) && !self.oculta.get() && self.visible.get();
        let (x, y, _, _) = self.lienzo.get();
        let (margen, minimo) = if self.vidrio.is_some() {
            (0, 1)
        } else {
            // Las esquinas de Windows miden 8 px (con el zoom de la pantalla): más pequeño no cabe.
            ((MARGEN_CRISTAL * self.escala.get()).ceil() as i32, (2.0 * 8.0 * self.zoom.get()) as i32)
        };
        // En la pantalla y sin las piezas demasiado pequeñas.
        let destino = self
            .recorte
            .get()
            .filter(|_| visible)
            .map(|forma| {
                forma.map(|Pieza { rect: (rx, ry, ancho, alto), radios, hueco }| {
                    // Los empalmes, solo con la composición: el desenfoque antiguo no llega a ellos.
                    if hueco.is_some() && self.vidrio.is_none() {
                        return Pieza::default();
                    }
                    let rect = (x + rx + margen, y + ry + margen, ancho - 2 * margen, alto - 2 * margen);
                    if rect.2 >= minimo && rect.3 >= minimo { Pieza { rect, radios, hueco } } else { Pieza::default() }
                })
            })
            .filter(|forma| forma.iter().any(|p| !p.vacia()));
        if self.cristal_puesto.replace(destino) == destino {
            return;
        }
        unsafe {
            let Some(forma) = destino else {
                self.cristal_ventana.set(None);
                let _ = ShowWindow(self.cristal, SW_HIDE);
                return;
            };
            if let Some(vidrio) = &self.vidrio {
                let lienzo = self.lienzo.get();
                // Windows se queda la región (la libera él): las piezas juntas.
                let region = CreateRectRgn(0, 0, 0, 0);
                for (pieza, (visual, recorte)) in forma.iter().zip(&vidrio.piezas) {
                    if pieza.vacia() {
                        let _ = visual.SetIsVisible(false);
                        continue;
                    }
                    let (px, py, ancho, alto) = pieza.rect;
                    let (rx, ry) = (px - lienzo.0, py - lienzo.1);
                    let tope = ancho.min(alto) / 2;
                    let esquina = |r: i32| {
                        let r = r.clamp(0, tope) as f32;
                        Vector2::new(r, r)
                    };
                    let _ = visual.SetOffset(Vector3::new(rx as f32, ry as f32, 0.0));
                    let _ = visual.SetSize(Vector2::new(ancho as f32, alto as f32));
                    let _ = recorte.SetRight(ancho as f32);
                    let _ = recorte.SetBottom(alto as f32);
                    let _ = recorte.SetTopLeftRadius(esquina(pieza.radios[0]));
                    let _ = recorte.SetTopRightRadius(esquina(pieza.radios[1]));
                    let _ = recorte.SetBottomRightRadius(esquina(pieza.radios[2]));
                    let _ = recorte.SetBottomLeftRadius(esquina(pieza.radios[3]));
                    let _ = visual.SetIsVisible(true);
                    match pieza.hueco {
                        // El cuarto de círculo que le falta al empalme lo quita la región (con el
                        // borde sin suavizar, pero el empalme es pequeño y el fondo lo cubre).
                        Some(esquina) => unir_empalme(region, (rx, ry, ancho, alto), esquina),
                        None => unir_rectangulo(region, (rx, ry, ancho, alto)),
                    }
                }
                let _ = SetWindowRgn(self.cristal, Some(region), false);
                if self.cristal_ventana.replace(Some(lienzo)) != Some(lienzo) {
                    let (lx, ly, lw, lh) = lienzo;
                    let _ = SetWindowPos(
                        self.cristal,
                        None,
                        lx,
                        ly,
                        lw,
                        lh,
                        SWP_NOACTIVATE | SWP_NOZORDER | SWP_SHOWWINDOW | SWP_NOREDRAW | SWP_NOCOPYBITS,
                    );
                }
                return;
            }
            // Con el desenfoque antiguo, la ventana abarca las piezas y, si hay dos, una región
            // la limita a ellas.
            let piezas: Vec<Rect> = forma.iter().filter(|p| !p.vacia()).map(|p| p.rect).collect();
            let x = piezas.iter().map(|r| r.0).min().unwrap_or(0);
            let y = piezas.iter().map(|r| r.1).min().unwrap_or(0);
            let ancho = piezas.iter().map(|r| r.0 + r.2).max().unwrap_or(0) - x;
            let alto = piezas.iter().map(|r| r.1 + r.3).max().unwrap_or(0) - y;
            let region = (piezas.len() > 1).then(|| {
                let region = CreateRectRgn(0, 0, 0, 0);
                for &(px, py, w, h) in &piezas {
                    unir_rectangulo(region, (px - x, py - y, w, h));
                }
                region
            });
            let _ = SetWindowRgn(self.cristal, region, false);
            let _ = SetWindowPos(
                self.cristal,
                None,
                x,
                y,
                ancho,
                alto,
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_SHOWWINDOW | SWP_NOREDRAW | SWP_NOCOPYBITS,
            );
        }
    }

    /// Nuevo zoom de la pantalla (o el mismo, tras cambiar el tamaño de la interfaz).
    fn cambiar_zoom(&self, zoom: f32) {
        self.zoom.set(zoom);
        let escala = zoom * tamano_de(self.es_barra);
        if self.escala.replace(escala) != escala {
            self.evento(WindowEvent::ScaleFactorChanged { scale_factor: escala });
            // Slint guarda el tamaño de la ventana en píxeles lógicos, calculados con la escala
            // anterior; si el tamaño físico no cambia, no los recalcula. Sin esto, al cambiar el
            // tamaño de la interfaz, la barra se centraba en una ventana más estrecha o más ancha
            // de lo que es y se veía desplazada hacia un lado.
            let (_, _, ancho, alto) = self.lienzo.get();
            if ancho > 0 && alto > 0 {
                let size = LogicalSize::new(ancho as f32 / escala, alto as f32 / escala);
                self.evento(WindowEvent::Resized { size });
            }
            self.repintar_todo.set(true);
            self.slint.request_redraw();
        }
    }

    fn posicion(&self, lparam: LPARAM) -> LogicalPosition {
        let x = (lparam.0 & 0xFFFF) as i16 as f32;
        let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
        let escala = self.escala.get();
        LogicalPosition::new(x / escala, y / escala)
    }
}

// ---- API para main.rs ----

/// Comprueba que no haya otra barra abierta. Si la hay, le pide que se muestre y devuelve
/// `false`: esta copia debe terminar sin crear nada. Con `esperar` (al abrirse tras una
/// actualización) antes se espera unos segundos a que la otra, la versión anterior, se cierre.
pub fn unica_instancia(esperar: bool) -> bool {
    unsafe {
        let limite = Instant::now() + Duration::from_secs(if esperar { 10 } else { 0 });
        loop {
            // El mutex queda abierto mientras dure el proceso; Windows lo libera al salir.
            let mutex = CreateMutexW(None, false, w!("Local\\NookBarra"));
            if GetLastError() != ERROR_ALREADY_EXISTS {
                return true;
            }
            if Instant::now() >= limite {
                break;
            }
            // Se suelta para que, al cerrarse la otra copia, el mutex deje de existir.
            if let Ok(mutex) = mutex {
                let _ = CloseHandle(mutex);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if let Ok(otra) = FindWindowW(CLASE, TITULO_BARRA) {
            // Esta copia es la que el usuario acaba de abrir: puede ceder el primer plano.
            let mut pid = 0u32;
            GetWindowThreadProcessId(otra, Some(&mut pid));
            let _ = AllowSetForegroundWindow(pid);
            let _ = PostMessageW(Some(otra), WM_OTRA_INSTANCIA, WPARAM(0), LPARAM(0));
        }
        false
    }
}

/// Registra esta plataforma en Slint. Hay que llamarla antes de crear la interfaz; la primera
/// ventana que se cree será la barra.
pub fn instalar() -> Result<(), PlatformError> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // OLE es necesario para recibir archivos arrastrados.
        let _ = OleInitialize(None);
    }
    preparar_composicion();
    HILO.store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);
    BARRA_DE_TAREAS_CREADA.store(unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) }, Ordering::Relaxed);
    registrar_clase().map_err(|e| PlatformError::Other(e.to_string()))?;
    slint::platform::set_platform(Box::new(PlataformaBarra)).map_err(|e| PlatformError::Other(e.to_string()))
}

/// Crea una ventana de Windows con su ventana de Slint. La primera (la barra) acepta archivos
/// arrastrados; las demás empiezan ocultas.
fn crear_interna() -> Result<Rc<Interna>, PlatformError> {
    let es_barra = VENTANAS.with(|v| v.borrow().is_empty());
    let titulo = if es_barra { TITULO_BARRA } else { w!("Nookly (emergente)") };
    let (cristal, vidrio) = crear_cristal().map_err(|e| PlatformError::Other(e.to_string()))?;
    let hwnd = crear_ventana(titulo, cristal).map_err(|e| PlatformError::Other(e.to_string()))?;
    let slint = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    let zoom = unsafe { GetDpiForWindow(hwnd) } as f32 / 96.0;
    let escala = zoom * tamano_de(es_barra);
    slint.window().dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: escala });

    let interna = Rc::new(Interna {
        hwnd,
        slint,
        es_barra,
        zoom: Cell::new(zoom),
        escala: Cell::new(escala),
        lienzo: Cell::new((0, 0, 0, 0)),
        lienzo_presentado: Cell::new((0, 0, 0, 0)),
        mapa: RefCell::new(None),
        visible: Cell::new(false),
        oculta: Cell::new(!es_barra),
        repintar_todo: Cell::new(true),
        cristal,
        vidrio,
        recorte: Cell::new(None),
        cristal_puesto: Cell::new(None),
        cristal_ventana: Cell::new(None),
        al_presentar: RefCell::new(None),
        raton_dentro: Cell::new(false),
        sustituto_alto: Cell::new(None),
        al_activarse: RefCell::new(None),
        al_arrastrar: RefCell::new(None),
        al_atajo: RefCell::new(None),
        al_aviso: RefCell::new(None),
    });
    VENTANAS.with(|v| v.borrow_mut().push(interna.clone()));

    if es_barra {
        let destino: IDropTarget = DestinoArrastre.into();
        unsafe {
            let _ = RegisterDragDrop(hwnd, &destino);
        }
        crate::avisos::vigilar(hwnd, || {
            if let Some(barra) = barra() {
                barra.avisar(Aviso::Notificacion);
            }
        });
    }
    Ok(interna)
}

/// La barra (la primera ventana).
fn barra() -> Option<Rc<Interna>> {
    VENTANAS.with(|v| v.borrow().first().cloned())
}

fn con_interna<R>(f: impl FnOnce(&Rc<Interna>) -> R) -> Option<R> {
    barra().map(|i| f(&i))
}

fn de_hwnd(hwnd: HWND) -> Option<Rc<Interna>> {
    VENTANAS.with(|v| v.borrow().iter().find(|i| i.hwnd == hwnd).cloned())
}

fn de_slint(ventana: &slint::Window) -> Option<Rc<Interna>> {
    VENTANAS.with(|v| v.borrow().iter().find(|i| std::ptr::eq(i.slint.window(), ventana)).cloned())
}

/// Identificador de la ventana de Windows.
pub fn hwnd() -> Option<HWND> {
    con_interna(|i| i.hwnd)
}

/// Escala con la que se dibuja la barra: el zoom de la pantalla (1,25 = 125 %) por el tamaño
/// de la interfaz.
pub fn escala() -> f32 {
    con_interna(|i| i.escala.get()).unwrap_or(1.0)
}

/// Coloca y dimensiona la ventana (el lienzo), en píxeles físicos de pantalla. Si solo cambia
/// la posición (arrastre, deslizamiento) no hace falta volver a dibujar: basta con entregar a
/// Windows lo ya dibujado en el nuevo sitio.
pub fn colocar(lienzo: Rect) {
    con_interna(|i| {
        let anterior = i.lienzo.replace(lienzo);
        if (anterior.2, anterior.3) != (lienzo.2, lienzo.3) {
            if lienzo.2 > 0 && lienzo.3 > 0 {
                i.slint.set_size(PhysicalSize::new(lienzo.2 as u32, lienzo.3 as u32));
            }
            i.slint.request_redraw();
        }
    });
}

/// Vuelve a dibujar la barra entera, a entregársela entera a Windows y a ponerla encima de todo,
/// con su cristal: al volver de la suspensión o cambiar las pantallas, Windows puede haber perdido
/// lo que tenía de ella, y repintar solo lo que cambia la dejaba a trozos.
pub fn repintar() {
    con_interna(|i| {
        i.repintar_todo.set(true);
        i.lienzo_presentado.set((0, 0, 0, 0));
        i.cristal_puesto.set(None);
        i.cristal_ventana.set(None);
        if i.visible.get() {
            unsafe {
                let _ = SetWindowPos(i.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
        i.slint.request_redraw();
    });
}

/// Muestra u oculta la barra entera, con el personaje (por ejemplo, mientras hay un juego a
/// pantalla completa). Al volver a mostrarla se dibuja de nuevo y se pone otra vez encima.
pub fn mostrar_barra(visible: bool) {
    con_interna(|i| {
        if i.oculta.replace(!visible) == !visible {
            return;
        }
        if visible {
            i.repintar_todo.set(true);
            i.slint.request_redraw();
        } else {
            i.visible.set(false);
            i.raton_dentro.set(false);
            unsafe {
                let _ = ShowWindow(i.hwnd, SW_HIDE);
            }
            i.actualizar_cristal();
        }
    });
}

/// Posición y tamaño actuales de la ventana (el lienzo).
pub fn lienzo() -> Rect {
    con_interna(|i| i.lienzo.get()).unwrap_or((0, 0, 0, 0))
}

/// Suelta el ratón para Slint como si hubiera terminado fuera de la barra: así ningún botón
/// recibe un clic cuando el usuario empieza a arrastrar la barra desde él.
pub fn cancelar_puntero() {
    con_interna(|i| {
        unsafe {
            let _ = ReleaseCapture();
        }
        let fuera = LogicalPosition::new(-10000.0, -10000.0);
        i.evento(WindowEvent::PointerReleased { position: fuera, button: PointerEventButton::Left });
        i.evento(WindowEvent::PointerExited);
    });
}

/// Se llama cuando la ventana gana (true) o pierde (false) el foco.
pub fn al_activarse(f: impl Fn(bool) + 'static) {
    con_interna(|i| *i.al_activarse.borrow_mut() = Some(Box::new(f)));
}

/// Se llama durante el arrastre de archivos desde el Explorador.
pub fn al_arrastrar(f: impl Fn(Arrastre) + 'static) {
    con_interna(|i| *i.al_arrastrar.borrow_mut() = Some(Box::new(f)));
}

/// Se llama cuando Windows avisa de un atajo global registrado con `RegisterHotKey`.
pub fn al_atajo(f: impl Fn(i32) + 'static) {
    con_interna(|i| *i.al_atajo.borrow_mut() = Some(Box::new(f)));
}

/// Se llama con los avisos del icono del área de notificación y de otras copias del programa.
pub fn al_aviso(f: impl Fn(Aviso) + 'static) {
    con_interna(|i| *i.al_aviso.borrow_mut() = Some(Box::new(f)));
}

// ---- Ventanas emergentes ----

/// Dónde va una ventana emergente respecto a su punto: con la esquina en él (junto al cursor),
/// debajo y centrada, o a un lado y centrada.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Junto {
    Esquina,
    Debajo,
    Encima,
    Derecha,
    Izquierda,
}

/// Muestra una ventana emergente de `tamano` (en píxeles lógicos) junto al punto `cerca` de la
/// pantalla, sin salirse del área de trabajo de ese monitor, y le da el foco del teclado.
pub fn mostrar_emergente(ventana: &slint::Window, cerca: (i32, i32), tamano: (f32, f32)) {
    mostrar_emergente_junto(ventana, cerca, Junto::Esquina, tamano);
}

/// Lo mismo, colocándola respecto a `punto` según `junto`.
pub fn mostrar_emergente_junto(ventana: &slint::Window, punto: (i32, i32), junto: Junto, tamano: (f32, f32)) {
    let Some(i) = de_slint(ventana) else { return };
    let cerca = punto;
    unsafe {
        let monitor = MonitorFromPoint(POINT { x: cerca.0, y: cerca.1 }, MONITOR_DEFAULTTONEAREST);
        let (mut dpi, mut _dpi_y) = (96u32, 96u32);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut _dpi_y);
        i.cambiar_zoom(dpi as f32 / 96.0);
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let r = info.rcWork;
        let escala = i.escala.get();
        let tamano = ((tamano.0 * escala).ceil() as i32, (tamano.1 * escala).ceil() as i32);
        let cerca = match junto {
            Junto::Esquina => cerca,
            Junto::Debajo => (cerca.0 - tamano.0 / 2, cerca.1),
            Junto::Encima => (cerca.0 - tamano.0 / 2, cerca.1 - tamano.1),
            Junto::Derecha => (cerca.0, cerca.1 - tamano.1 / 2),
            Junto::Izquierda => (cerca.0 - tamano.0, cerca.1 - tamano.1 / 2),
        };
        let (x, y) = junto_a(cerca, tamano, (r.left, r.top, r.right - r.left, r.bottom - r.top));

        i.oculta.set(false);
        // Puede conservar restos del menú anterior.
        i.repintar_todo.set(true);
        i.lienzo.set((x, y, tamano.0, tamano.1));
        i.slint.set_size(PhysicalSize::new(tamano.0 as u32, tamano.1 as u32));
        i.slint.request_redraw();
        i.presentar();
        let _ = SetForegroundWindow(i.hwnd);
    }
}

/// Si la ventana (una emergente) se ve.
pub fn se_ve(ventana: &slint::Window) -> bool {
    de_slint(ventana).is_some_and(|i| !i.oculta.get())
}

/// Si la ventana se ve y tiene el cursor encima.
pub fn cursor_dentro(ventana: &slint::Window) -> bool {
    de_slint(ventana).is_some_and(|i| !i.oculta.get() && i.raton_dentro.get())
}

/// Oculta una ventana emergente.
pub fn ocultar(ventana: &slint::Window) {
    let Some(i) = de_slint(ventana) else { return };
    if i.oculta.replace(true) {
        return;
    }
    i.visible.set(false);
    i.raton_dentro.set(false);
    unsafe {
        let _ = ShowWindow(i.hwnd, SW_HIDE);
    }
    i.actualizar_cristal();
}

/// Se llama cuando esa ventana gana (true) o pierde (false) el foco.
pub fn al_activarse_ventana(ventana: &slint::Window, f: impl Fn(bool) + 'static) {
    if let Some(i) = de_slint(ventana) {
        *i.al_activarse.borrow_mut() = Some(Box::new(f));
    }
}

/// Tamaño de la barra (1 = normal): multiplica el zoom de la pantalla, así que agranda o encoge
/// por igual iconos, textos y espacios. Las emergentes (menús y selector de color) no lo siguen:
/// se ven del tamaño de los menús de los demás programas.
pub fn poner_tamano(tamano: f32) {
    if TAMANO.with(|t| t.replace(tamano)) == tamano {
        return;
    }
    for i in VENTANAS.with(|v| v.borrow().clone()) {
        i.cambiar_zoom(i.zoom.get());
    }
}

/// Tamaño de la interfaz con que se dibuja una ventana: el elegido en la barra, el normal en las
/// emergentes.
fn tamano_de(es_barra: bool) -> f32 {
    if es_barra { TAMANO.with(Cell::get) } else { 1.0 }
}

// ---- Desenfoque de lo que hay detrás ----

/// Activa o quita el desenfoque de lo que hay detrás de todas las ventanas. Se ve a través de
/// los píxeles semitransparentes, y solo dentro del recorte de cada ventana (ver `recortar`).
pub fn poner_desenfoque(activo: bool) {
    if DESENFOQUE.with(|d| d.replace(activo)) == activo {
        return;
    }
    for i in VENTANAS.with(|v| v.borrow().clone()) {
        i.actualizar_cristal();
    }
}

/// Forma visible de la ventana: el desenfoque de detrás solo se ve ahí.
pub fn recortar(ventana: &slint::Window, recorte: Option<Forma>) {
    if let Some(i) = de_slint(ventana) {
        i.recorte.set(recorte);
    }
}

/// Se llama antes de cada dibujo de esa ventana (para seguir su forma con `recortar`).
pub fn al_presentar(ventana: &slint::Window, f: impl Fn() + 'static) {
    if let Some(i) = de_slint(ventana) {
        *i.al_presentar.borrow_mut() = Some(Box::new(f));
    }
}

/// Efecto de composición de Windows ("blur behind") de una ventana. No está documentado: se
/// busca en user32.dll al usarlo y, si no existe, la ventana queda sin desenfoque.
fn poner_acento(hwnd: HWND, desenfoque: bool) {
    #[repr(C)]
    struct Acento {
        estado: u32,
        banderas: u32,
        color: u32,
        animacion: u32,
    }
    #[repr(C)]
    struct Composicion {
        atributo: u32,
        datos: *mut c_void,
        tamano: usize,
    }
    type Funcion = unsafe extern "system" fn(HWND, *mut Composicion) -> i32;
    const WCA_ACCENT_POLICY: u32 = 19;
    const ACCENT_DISABLED: u32 = 0;
    // Desenfoque sencillo: el acrílico (4) hace que mover la ventana vaya a tirones.
    const ACCENT_ENABLE_BLURBEHIND: u32 = 3;

    let estado = if desenfoque { ACCENT_ENABLE_BLURBEHIND } else { ACCENT_DISABLED };
    let mut acento = Acento { estado, banderas: 0, color: 0, animacion: 0 };
    let mut datos =
        Composicion { atributo: WCA_ACCENT_POLICY, datos: &mut acento as *mut _ as *mut c_void, tamano: size_of::<Acento>() };
    unsafe {
        let Ok(user32) = GetModuleHandleW(w!("user32.dll")) else { return };
        let Some(funcion) = GetProcAddress(user32, s!("SetWindowCompositionAttribute")) else { return };
        let funcion: Funcion = std::mem::transmute(funcion);
        funcion(hwnd, &mut datos);
    }
}

/// Esquina superior izquierda para una ventana de `tamano` junto al punto `cerca`: hacia abajo
/// y a la derecha si cabe; si no, hacia arriba o a la izquierda, siempre dentro de `area`.
fn junto_a(cerca: (i32, i32), (ancho, alto): (i32, i32), (ax, ay, aw, ah): Rect) -> (i32, i32) {
    let x = if cerca.0 + ancho > ax + aw { cerca.0 - ancho } else { cerca.0 };
    let y = if cerca.1 + alto > ay + ah { cerca.1 - alto } else { cerca.1 };
    (x.clamp(ax, (ax + aw - ancho).max(ax)), y.clamp(ay, (ay + ah - alto).max(ay)))
}

// ---- Plataforma de Slint ----

struct PlataformaBarra;

impl Platform for PlataformaBarra {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(crear_interna()?.slint.clone())
    }

    fn duration_since_start(&self) -> Duration {
        static INICIO: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        INICIO.get_or_init(Instant::now).elapsed()
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn slint::platform::EventLoopProxy>> {
        Some(Box::new(Despertador))
    }

    fn set_clipboard_text(&self, texto: &str, portapapeles: slint::platform::Clipboard) {
        if matches!(portapapeles, slint::platform::Clipboard::DefaultClipboard) {
            crate::portapapeles::poner(texto);
        }
    }

    fn clipboard_text(&self, portapapeles: slint::platform::Clipboard) -> Option<String> {
        matches!(portapapeles, slint::platform::Clipboard::DefaultClipboard).then(crate::portapapeles::leer).flatten()
    }

    fn run_event_loop(&self) -> Result<(), PlatformError> {
        loop {
            slint::platform::update_timers_and_animations();
            let tareas: Vec<_> = std::mem::take(&mut *PENDIENTES.lock().unwrap());
            for tarea in tareas {
                tarea();
            }
            let ventanas = VENTANAS.with(|v| v.borrow().clone());
            for ventana in &ventanas {
                ventana.presentar();
            }

            // Mientras algo se mueve (animaciones de Slint o temporizadores rápidos, como el
            // deslizamiento y el arrastre) se espera al siguiente refresco de la pantalla: cada
            // cuadro coincide con uno del monitor y no hay tirones (dormir un tiempo fijo no
            // sirve: el reloj de Windows solo despierta cada ~15,6 ms). Si no, se duerme hasta el
            // próximo temporizador o hasta que llegue un mensaje (consumo cero en reposo). Un
            // temporizador de animación algo más lento (como el del personaje tranquilo) se
            // despierta antes de tiempo para alinearse igual con el refresco: si no, el reloj de
            // Windows lo retrasa hasta 15 ms y sus cuadros salen desiguales.
            let animando = ventanas.iter().any(|v| !v.oculta.get() && v.slint.window().has_active_animations());
            let proximo = slint::platform::duration_until_next_timer_update();
            let cerca = Duration::from_millis(20);
            let moviendose = animando || proximo.is_some_and(|d| d <= cerca);
            let espera = if moviendose {
                // Sin composición de escritorio (no debería pasar) se vuelve a los ~60 por segundo.
                if unsafe { DwmFlush() }.is_ok() { 0 } else { 16 }
            } else {
                match proximo {
                    Some(d) if d < Duration::from_millis(50) => (d - cerca).as_millis() as u32,
                    Some(d) => d.as_millis().min(INFINITE as u128 - 1) as u32,
                    None => INFINITE,
                }
            };
            unsafe {
                MsgWaitForMultipleObjectsEx(None, espera, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
            }

            let mut mensaje = MSG::default();
            while unsafe { PeekMessageW(&mut mensaje, None, 0, 0, PM_REMOVE) }.as_bool() {
                match mensaje.message {
                    WM_QUIT => return Ok(()),
                    WM_DESPERTAR => {}
                    WM_HOTKEY if mensaje.hwnd.is_invalid() => {
                        let id = mensaje.wParam.0 as i32;
                        con_interna(|i| {
                            if let Some(f) = &*i.al_atajo.borrow() {
                                f(id);
                            }
                        });
                    }
                    _ => unsafe {
                        let _ = TranslateMessage(&mensaje);
                        DispatchMessageW(&mensaje);
                    },
                }
            }
        }
    }
}

struct Despertador;

impl slint::platform::EventLoopProxy for Despertador {
    fn quit_event_loop(&self) -> Result<(), slint::EventLoopError> {
        unsafe {
            let _ = PostThreadMessageW(HILO.load(Ordering::Relaxed), WM_QUIT, WPARAM(0), LPARAM(0));
        }
        Ok(())
    }

    fn invoke_from_event_loop(&self, tarea: Box<dyn FnOnce() + Send>) -> Result<(), slint::EventLoopError> {
        PENDIENTES.lock().unwrap().push(tarea);
        unsafe {
            let _ = PostThreadMessageW(HILO.load(Ordering::Relaxed), WM_DESPERTAR, WPARAM(0), LPARAM(0));
        }
        Ok(())
    }
}

// ---- Ventana de Windows ----

fn registrar_clase() -> windows::core::Result<()> {
    unsafe {
        let instancia = GetModuleHandleW(None)?;
        let registro = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(procedimiento),
            hInstance: instancia.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: CLASE,
            ..Default::default()
        };
        RegisterClassW(&registro);
        let cristal = WNDCLASSW {
            lpfnWndProc: Some(procedimiento_cristal),
            hInstance: instancia.into(),
            lpszClassName: CLASE_CRISTAL,
            ..Default::default()
        };
        RegisterClassW(&cristal);
    }
    Ok(())
}

/// Prepara la composición de Windows en este hilo (para el desenfoque con esquinas a medida). Si
/// no se puede, los cristales usan el desenfoque antiguo.
fn preparar_composicion() {
    let opciones = DispatcherQueueOptions {
        dwSize: size_of::<DispatcherQueueOptions>() as u32,
        threadType: DQTYPE_THREAD_CURRENT,
        apartmentType: DQTAT_COM_NONE,
    };
    let Ok(cola) = (unsafe { CreateDispatcherQueueController(opciones) }) else { return };
    let Ok(compositor) = Compositor::new() else { return };
    ESCENA.with(|e| *e.borrow_mut() = Some(Escena { _cola: cola, compositor }));
}

/// Pone en el cristal el desenfoque de la composición: un visual del tamaño del cristal con el
/// pincel de lo que hay detrás y un recorte redondeado. `None` si Windows no lo permite (necesita
/// Windows 11).
fn crear_vidrio(cristal: HWND) -> Option<Vidrio> {
    ESCENA.with(|e| {
        let escena = e.borrow();
        let compositor = &escena.as_ref()?.compositor;
        let si = windows_core::BOOL::from(true);
        unsafe {
            DwmSetWindowAttribute(cristal, DWMWA_USE_HOSTBACKDROPBRUSH, &si as *const _ as *const c_void, size_of_val(&si) as u32)
                .ok()?;
        }
        let destino = unsafe { compositor.cast::<ICompositorDesktopInterop>().ok()?.CreateDesktopWindowTarget(cristal, false) }.ok()?;
        let raiz = compositor.CreateContainerVisual().ok()?;
        let pincel = compositor.CreateHostBackdropBrush().ok()?;
        let pieza = || -> Option<(SpriteVisual, RectangleClip)> {
            let visual = compositor.CreateSpriteVisual().ok()?;
            visual.SetBrush(&pincel).ok()?;
            let recorte = compositor.CreateRectangleClip().ok()?;
            visual.SetClip(&recorte).ok()?;
            raiz.Children().ok()?.InsertAtTop(&visual).ok()?;
            Some((visual, recorte))
        };
        let piezas = [pieza()?, pieza()?, pieza()?, pieza()?];
        destino.SetRoot(&raiz).ok()?;
        Some(Vidrio { _destino: destino, piezas })
    })
}

/// Añade a `region` el rectángulo `(x, y, ancho, alto)` sin el cuarto de círculo del radio de su
/// lado centrado en la esquina `esquina` (0 la de arriba a la izquierda, y en el sentido de las
/// agujas del reloj).
unsafe fn unir_empalme(region: HRGN, (x, y, ancho, alto): Rect, esquina: usize) {
    let r = ancho.max(alto);
    let cx = if esquina == 1 || esquina == 2 { x + ancho } else { x };
    let cy = if esquina >= 2 { y + alto } else { y };
    unsafe {
        let parte = CreateRectRgn(x, y, x + ancho, y + alto);
        let circulo = CreateEllipticRgn(cx - r, cy - r, cx + r, cy + r);
        CombineRgn(Some(parte), Some(parte), Some(circulo), RGN_DIFF);
        CombineRgn(Some(region), Some(region), Some(parte), RGN_OR);
        let _ = DeleteObject(circulo.into());
        let _ = DeleteObject(parte.into());
    }
}

/// Añade a `region` el rectángulo `(x, y, ancho, alto)`.
unsafe fn unir_rectangulo(region: HRGN, (x, y, ancho, alto): Rect) {
    unsafe {
        let parte = CreateRectRgn(x, y, x + ancho, y + alto);
        CombineRgn(Some(region), Some(region), Some(parte), RGN_OR);
        let _ = DeleteObject(parte.into());
    }
}

/// Crea el cristal de una ventana (empieza oculto) con el desenfoque de Windows.
fn crear_cristal() -> windows::core::Result<(HWND, Option<Vidrio>)> {
    unsafe {
        let cristal = CreateWindowExW(
            // Sin superficie de dibujo propia (no tiene nada que pintar): solo se ve el desenfoque,
            // y moverla y redimensionarla en cada cuadro de las animaciones es mucho más barato.
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP,
            CLASE_CRISTAL,
            w!("Nookly (cristal)"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(GetModuleHandleW(None)?.into()),
            None,
        )?;
        let vidrio = crear_vidrio(cristal);
        // Con el desenfoque antiguo, Windows desenfoca todo el rectángulo de la ventana (no respeta
        // su región), pero sí sus esquinas redondeadas, que dibuja él mismo. Con la composición
        // las esquinas las pone el recorte del visual. Sin borde: debe verse solo el desenfoque.
        if vidrio.is_none() {
            poner_acento(cristal, true);
        }
        let redondas = if vidrio.is_some() { DWMWCP_DONOTROUND } else { DWMWCP_ROUND };
        let sin_borde = DWMWA_COLOR_NONE;
        let _ = DwmSetWindowAttribute(
            cristal,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &redondas as *const _ as *const c_void,
            size_of_val(&redondas) as u32,
        );
        let _ = DwmSetWindowAttribute(cristal, DWMWA_BORDER_COLOR, &sin_borde as *const _ as *const c_void, 4);
        Ok((cristal, vidrio))
    }
}

/// El cristal no recibe el ratón ni el foco: todo pasa a la ventana de encima o a lo de detrás.
unsafe extern "system" fn procedimiento_cristal(hwnd: HWND, mensaje: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match mensaje {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        _ => unsafe { DefWindowProcW(hwnd, mensaje, wparam, lparam) },
    }
}

fn crear_ventana(titulo: PCWSTR, dueno: HWND) -> windows::core::Result<HWND> {
    unsafe {
        let instancia = GetModuleHandleW(None)?;
        // WS_EX_LAYERED: transparencia por píxel. WS_EX_TOOLWINDOW: fuera de Alt+Tab y de la barra
        // de tareas (el programa se ve en el área de notificación). WS_EX_TOPMOST: siempre encima.
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            CLASE,
            titulo,
            WS_POPUP,
            0,
            0,
            1,
            1,
            // Con el cristal como dueña, Windows mantiene esta ventana siempre por encima de él.
            Some(dueno),
            None,
            Some(instancia.into()),
            None,
        )
    }
}

unsafe extern "system" fn procedimiento(hwnd: HWND, mensaje: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let Some(interna) = de_hwnd(hwnd) else {
        return unsafe { DefWindowProcW(hwnd, mensaje, wparam, lparam) };
    };
    // Los avisos (icono, otra copia, Explorador reiniciado) son siempre para la barra.
    let avisar = |aviso: Aviso| {
        if let Some(barra) = barra() {
            barra.avisar(aviso);
        }
    };
    match mensaje {
        WM_MOUSEMOVE => {
            if !interna.raton_dentro.replace(true) {
                let mut seguimiento = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe {
                    let _ = TrackMouseEvent(&mut seguimiento);
                }
            }
            interna.evento(WindowEvent::PointerMoved { position: interna.posicion(lparam) });
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            interna.raton_dentro.set(false);
            interna.evento(WindowEvent::PointerExited);
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            unsafe {
                SetCapture(hwnd);
            }
            interna.evento(WindowEvent::PointerPressed { position: interna.posicion(lparam), button: boton(mensaje) });
            LRESULT(0)
        }
        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => {
            unsafe {
                let _ = ReleaseCapture();
            }
            interna.evento(WindowEvent::PointerReleased { position: interna.posicion(lparam), button: boton(mensaje) });
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            // La posición llega en coordenadas de pantalla.
            let mut punto = POINT {
                x: (lparam.0 & 0xFFFF) as i16 as i32,
                y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
            };
            unsafe {
                let _ = ScreenToClient(hwnd, &mut punto);
            }
            let giro = ((wparam.0 >> 16) & 0xFFFF) as i16 as f32 / 120.0;
            let escala = interna.escala.get();
            interna.evento(WindowEvent::PointerScrolled {
                position: LogicalPosition::new(punto.x as f32 / escala, punto.y as f32 / escala),
                delta_x: 0.0,
                delta_y: giro * 50.0,
            });
            LRESULT(0)
        }
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => {
            let pulsada = matches!(mensaje, WM_KEYDOWN | WM_SYSKEYDOWN);
            if let Some(texto) = texto_de_tecla(VIRTUAL_KEY(wparam.0 as u16), lparam, pulsada) {
                let repetida = pulsada && (lparam.0 >> 30) & 1 == 1;
                interna.evento(match (pulsada, repetida) {
                    (true, false) => WindowEvent::KeyPressed { text: texto },
                    (true, true) => WindowEvent::KeyPressRepeated { text: texto },
                    (false, _) => WindowEvent::KeyReleased { text: texto },
                });
            }
            // Alt+F4 y similares los resuelve Windows.
            if matches!(mensaje, WM_SYSKEYDOWN | WM_SYSKEYUP) {
                return unsafe { DefWindowProcW(hwnd, mensaje, wparam, lparam) };
            }
            LRESULT(0)
        }
        WM_CHAR => {
            let unidad = wparam.0 as u16;
            // Los caracteres de control (Tab, Intro, Escape, Ctrl+letra…) ya llegan por WM_KEYDOWN.
            if unidad < 0x20 || unidad == 0x7F {
                return LRESULT(0);
            }
            let texto = if (0xD800..0xDC00).contains(&unidad) {
                interna.sustituto_alto.set(Some(unidad));
                return LRESULT(0);
            } else if (0xDC00..0xE000).contains(&unidad) {
                let Some(alto) = interna.sustituto_alto.take() else { return LRESULT(0) };
                String::from_utf16_lossy(&[alto, unidad])
            } else {
                String::from_utf16_lossy(&[unidad])
            };
            let texto = SharedString::from(texto.as_str());
            interna.evento(WindowEvent::KeyPressed { text: texto.clone() });
            interna.evento(WindowEvent::KeyReleased { text: texto });
            LRESULT(0)
        }
        WM_ACTIVATE => {
            let activa = (wparam.0 & 0xFFFF) as u32 != WA_INACTIVE;
            interna.evento(WindowEvent::WindowActiveChanged(activa));
            if let Some(f) = &*interna.al_activarse.borrow() {
                f(activa);
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            interna.cambiar_zoom((wparam.0 & 0xFFFF) as f32 / 96.0);
            if barra().is_some_and(|b| b.hwnd == hwnd) {
                avisar(Aviso::EscalaCambiada);
            }
            LRESULT(0)
        }
        // Llegan a todas las ventanas; los atiende solo la barra.
        WM_POWERBROADCAST => {
            if barra().is_some_and(|b| b.hwnd == hwnd) && matches!(wparam.0, PBT_APMRESUMESUSPEND | PBT_APMRESUMEAUTOMATIC) {
                avisar(Aviso::Reanudado);
            }
            LRESULT(1)
        }
        WM_DISPLAYCHANGE => {
            if barra().is_some_and(|b| b.hwnd == hwnd) {
                avisar(Aviso::PantallasCambiadas);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        WM_BANDEJA => {
            // Con NOTIFYICON_VERSION_4 el suceso llega en la parte baja de lparam.
            match (lparam.0 & 0xFFFF) as u32 {
                // NIN_SELECT | NINF_KEY: el icono se eligió con el teclado.
                NIN_SELECT | NIN_KEYSELECT => avisar(Aviso::ClicIcono),
                WM_CONTEXTMENU => avisar(Aviso::MenuIcono),
                _ => {}
            }
            LRESULT(0)
        }
        WM_OTRA_INSTANCIA => {
            avisar(Aviso::OtraInstancia);
            LRESULT(0)
        }
        m if m != 0 && m == BARRA_DE_TAREAS_CREADA.load(Ordering::Relaxed) => {
            avisar(Aviso::IconoPerdido);
            LRESULT(0)
        }
        m if crate::avisos::es_mensaje_del_shell(m) => {
            if crate::avisos::pide_atencion(wparam, lparam) {
                avisar(Aviso::Notificacion);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, mensaje, wparam, lparam) },
    }
}

fn boton(mensaje: u32) -> PointerEventButton {
    match mensaje {
        WM_RBUTTONDOWN | WM_RBUTTONUP => PointerEventButton::Right,
        WM_MBUTTONDOWN | WM_MBUTTONUP => PointerEventButton::Middle,
        _ => PointerEventButton::Left,
    }
}

fn tecla_pulsada(vk: VIRTUAL_KEY) -> bool {
    (unsafe { GetKeyState(vk.0 as i32) }) < 0
}

/// Texto que Slint espera para una tecla especial (flechas, Escape, modificadores…) o para
/// Ctrl+letra. Las teclas que escriben texto llegan por WM_CHAR.
fn texto_de_tecla(vk: VIRTUAL_KEY, lparam: LPARAM, pulsada: bool) -> Option<SharedString> {
    let extendida = (lparam.0 >> 24) & 1 == 1;
    let tecla = match vk {
        VK_BACK => Key::Backspace,
        VK_TAB if tecla_pulsada(VK_SHIFT) => Key::Backtab,
        VK_TAB => Key::Tab,
        VK_RETURN => Key::Return,
        VK_ESCAPE => Key::Escape,
        VK_DELETE => Key::Delete,
        VK_INSERT => Key::Insert,
        VK_LEFT => Key::LeftArrow,
        VK_RIGHT => Key::RightArrow,
        VK_UP => Key::UpArrow,
        VK_DOWN => Key::DownArrow,
        VK_HOME => Key::Home,
        VK_END => Key::End,
        VK_PRIOR => Key::PageUp,
        VK_NEXT => Key::PageDown,
        VK_SHIFT => Key::Shift,
        // AltGr (Alt derecha) llega como Ctrl izquierda + Alt derecha: no son modificadores
        // para Slint, el carácter que producen llega por WM_CHAR.
        VK_MENU if extendida => return None,
        VK_MENU => Key::Alt,
        VK_CONTROL if pulsada && viene_altgr() => return None,
        VK_CONTROL => Key::Control,
        VK_LWIN | VK_RWIN => Key::Meta,
        v if (VK_F1.0..=VK_F12.0).contains(&v.0) => {
            const F: [Key; 12] = [
                Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6,
                Key::F7, Key::F8, Key::F9, Key::F10, Key::F11, Key::F12,
            ];
            F[(v.0 - VK_F1.0) as usize]
        }
        // Ctrl+letra o Ctrl+número (atajos de los campos de texto: copiar, pegar, deshacer…).
        // Con Alt pulsada a la vez es AltGr y el carácter llega por WM_CHAR.
        v if tecla_pulsada(VK_CONTROL) && !tecla_pulsada(VK_MENU) => {
            let c = char::from_u32(v.0 as u32).filter(|c| c.is_ascii_alphanumeric())?;
            return Some(c.to_ascii_lowercase().to_string().into());
        }
        _ => return None,
    };
    Some(tecla.into())
}

/// Windows envía un Ctrl izquierdo falso justo antes de Alt derecha (AltGr).
fn viene_altgr() -> bool {
    let mut siguiente = MSG::default();
    unsafe {
        PeekMessageW(&mut siguiente, None, 0, 0, PM_NOREMOVE).as_bool()
            && matches!(siguiente.message, WM_KEYDOWN | WM_SYSKEYDOWN)
            && siguiente.wParam.0 as u16 == VK_MENU.0
            && (siguiente.lParam.0 >> 24) & 1 == 1
    }
}

// ---- Arrastrar archivos (OLE) ----

#[implement(IDropTarget)]
struct DestinoArrastre;

fn avisar_arrastre(evento: Arrastre) {
    con_interna(|i| {
        if let Some(f) = &*i.al_arrastrar.borrow() {
            f(evento);
        }
    });
}

impl IDropTarget_Impl for DestinoArrastre_Impl {
    fn DragEnter(
        &self,
        _datos: Ref<IDataObject>,
        _teclas: MODIFIERKEYS_FLAGS,
        punto: &POINTL,
        efecto: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        avisar_arrastre(Arrastre::Entra);
        avisar_arrastre(Arrastre::Mueve(punto.x, punto.y));
        unsafe { *efecto = DROPEFFECT_COPY | DROPEFFECT_LINK };
        Ok(())
    }

    fn DragOver(&self, _teclas: MODIFIERKEYS_FLAGS, punto: &POINTL, efecto: *mut DROPEFFECT) -> windows_core::Result<()> {
        avisar_arrastre(Arrastre::Mueve(punto.x, punto.y));
        unsafe { *efecto = DROPEFFECT_COPY | DROPEFFECT_LINK };
        Ok(())
    }

    fn DragLeave(&self) -> windows_core::Result<()> {
        avisar_arrastre(Arrastre::Sale);
        Ok(())
    }

    fn Drop(
        &self,
        datos: Ref<IDataObject>,
        _teclas: MODIFIERKEYS_FLAGS,
        punto: &POINTL,
        efecto: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        let rutas = datos.ok().map(rutas_de).unwrap_or_default();
        unsafe { *efecto = if rutas.is_empty() { DROPEFFECT_NONE } else { DROPEFFECT_COPY } };
        avisar_arrastre(Arrastre::Suelta(rutas, punto.x, punto.y));
        Ok(())
    }
}

/// Rutas de los archivos que trae un arrastre del Explorador.
fn rutas_de(datos: &IDataObject) -> Vec<PathBuf> {
    let formato = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    let mut rutas = Vec::new();
    unsafe {
        let Ok(mut medio) = datos.GetData(&formato) else { return rutas };
        let soltado = HDROP(medio.u.hGlobal.0);
        let cantidad = DragQueryFileW(soltado, u32::MAX, None);
        for i in 0..cantidad {
            let largo = DragQueryFileW(soltado, i, None) as usize;
            let mut texto = vec![0u16; largo + 1];
            DragQueryFileW(soltado, i, Some(&mut texto));
            rutas.push(PathBuf::from(String::from_utf16_lossy(&texto[..largo])));
        }
        ReleaseStgMedium(&mut medio);
    }
    rutas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dividir_entre_255_sin_dividir() {
        for v in 0..=255 * 255 {
            assert_eq!(entre_255(v) as u32, (v + 127) / 255, "{v}");
        }
    }

    #[test]
    fn la_emergente_no_se_sale_de_la_pantalla() {
        let area = (0, 0, 1920, 1040);
        // Con sitio: abajo a la derecha del cursor.
        assert_eq!(junto_a((100, 100), (200, 150), area), (100, 100));
        // Junto a la barra de tareas (abajo a la derecha): hacia arriba y a la izquierda.
        assert_eq!(junto_a((1900, 1030), (200, 150), area), (1700, 880));
        // Segundo monitor a la izquierda.
        assert_eq!(junto_a((-10, 500), (200, 150), (-1920, 0, 1920, 1080)), (-210, 500));
    }
}
