// En modo release no se abre la ventana negra de consola.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actualizacion;
mod addons;
mod atajos;
mod avisos;
mod azar;
mod bandeja;
mod barra_de_tareas;
mod boveda;
mod buscador;
mod config;
mod cuenta;
mod dibujo;
mod fecha;
mod icono;
mod idioma;
mod musica;
mod personaje;
mod plataforma;
mod red;
mod portapapeles;
mod segundo_plano;
mod sistema;
mod ventana;

// Lo de nook Pro. Cada uno de estos módulos tiene dos versiones: la de verdad en src/pro/ (que
// solo está en el repositorio de nook Pro) y en src/gratis/ otra con lo mismo que no hace nada,
// para que el resto del código sea igual en las dos (ver build.rs).
#[cfg_attr(pro, path = "pro/abiertas.rs")]
#[cfg_attr(not(pro), path = "gratis/abiertas.rs")]
mod abiertas;
#[cfg_attr(pro, path = "pro/claude.rs")]
#[cfg_attr(not(pro), path = "gratis/claude.rs")]
mod claude;
#[cfg_attr(pro, path = "pro/energia.rs")]
#[cfg_attr(not(pro), path = "gratis/energia.rs")]
mod energia;
#[cfg_attr(pro, path = "pro/notificacion.rs")]
#[cfg_attr(not(pro), path = "gratis/notificacion.rs")]
mod notificacion;
#[cfg_attr(pro, path = "pro/pantalla_completa.rs")]
#[cfg_attr(not(pro), path = "gratis/pantalla_completa.rs")]
mod pantalla_completa;
#[cfg_attr(pro, path = "pro/reproductor.rs")]
#[cfg_attr(not(pro), path = "gratis/reproductor.rs")]
mod reproductor;
#[cfg_attr(pro, path = "pro/sonido.rs")]
#[cfg_attr(not(pro), path = "gratis/sonido.rs")]
mod sonido;
#[cfg_attr(pro, path = "pro/tecla_windows.rs")]
#[cfg_attr(not(pro), path = "gratis/tecla_windows.rs")]
mod tecla_windows;

/// Lo que hace la barra, por partes. Cada una añade a `Ctx` los métodos de lo suyo y conecta sus
/// eventos de la interfaz (`conectar`).
mod barra {
    pub mod abiertas;
    pub mod addons;
    pub mod atajos;
    pub mod claude;
    pub mod boveda;
    pub mod buscador;
    pub mod colocacion;
    pub mod configuracion;
    pub mod energia;
    pub mod menus;
    pub mod notificacion;
    pub mod personaje;
    pub mod reproductor;
    pub mod sonido;
    pub mod tira;
}

use atajos::Atajo;
use barra::abiertas::*;
use barra::atajos::*;
use barra::boveda::*;
use barra::buscador::*;
use barra::colocacion::*;
use barra::configuracion::*;
use barra::menus::*;
use barra::personaje::*;
use barra::tira::*;
use chrono::{DateTime, Local};
use config::{Ajustes, App, COLORES_ACCESORIOS, Config, Elemento, FONDOS, FormatoFecha, Grupo, Posicion, TipoAccesorio};
use idioma::{tr, tr_con};
use musica::Orden;
use plataforma::{Arrastre, Aviso, Rect};
use slint::platform::Key;
use slint::{Color, ComponentHandle, Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, Timer, TimerMode, VecModel};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

// Genera los tipos `Barra`, `Emergente`, `Tema` y los de las listas a partir de ui/barra.slint.
slint::include_modules!();

/// Si esto es nook Pro (se compila con la carpeta src/pro, ver build.rs). En la versión gratuita
/// sus funciones salen en la configuración con la marca "Pro" y sin poder activarse.
const PRO: bool = cfg!(pro);

#[derive(Clone, Copy, PartialEq, Debug)]
enum Modo {
    /// Solo una línea fina en el borde.
    Oculta,
    /// Tira de iconos (grupos y apps sueltas).
    Reposo,
    /// Grupo abierto con sus apps.
    Grupo,
    /// Configuración (botón de la cabeza del personaje).
    Config,
    /// Buscador de aplicaciones instaladas (botón "+").
    Buscar,
    /// Bóveda de textos guardados (botón del rombo).
    Boveda,
    /// Reproductor de lo que suena (clic en el personaje o en la portada de la tira).
    Musica,
    /// Volumen general de Windows (botón del altavoz).
    Sonido,
    /// Panel de un addon (ver src/addons/).
    Addon,
    /// Conversación con Claude Code (botón de la tira, solo en nook Pro).
    Claude,
}

impl Modo {
    fn numero(self) -> i32 {
        match self {
            Modo::Oculta => 0,
            Modo::Reposo => 1,
            Modo::Grupo => 2,
            Modo::Config => 3,
            Modo::Buscar => 4,
            Modo::Boveda => 5,
            Modo::Musica => 6,
            Modo::Sonido => 7,
            Modo::Addon => 8,
            Modo::Claude => 9,
        }
    }

    fn desplegado(self) -> bool {
        matches!(self, Modo::Grupo | Modo::Config | Modo::Buscar | Modo::Boveda | Modo::Musica | Modo::Sonido | Modo::Addon | Modo::Claude)
    }
}

/// Estado de la aplicación compartido por todos los eventos.
struct Estado {
    config: Config,
    modo: Modo,
    /// Grupo seleccionado (el que se abre con el atajo "mostrar").
    activo: usize,
    /// Iconos ya extraídos, por ruta y tamaño en píxeles (extraerlos cuesta, así se hace una sola
    /// vez por tamaño).
    iconos: HashMap<(String, u32), Option<Image>>,
    /// Iconos que se están extrayendo en segundo plano (para no pedirlos dos veces).
    iconos_pedidos: HashSet<(String, u32)>,
    /// Atajo que se está grabando en la configuración: -1 = mostrar/ocultar, n = elemento n.
    grabando: Option<i32>,
    /// Si Windows aceptó cada atajo de grupo (false = otro programa ya lo usa).
    atajos_ok: Vec<bool>,
    mostrar_ok: bool,
    boveda_ok: bool,
    microfono_ok: bool,
    buscar_ok: bool,
    claude_ok: bool,
    /// Cuántos atajos de grupo hay registrados (para quitarlos antes de volver a registrar).
    registrados: usize,
    inicia_con_windows: bool,
    /// Comprobaciones seguidas con el cursor fuera de la barra (para ocultarla).
    fuera: u32,
    /// Hay un juego o una app a pantalla completa delante en el monitor de la barra (y con la
    /// opción de ocultarse activa): la barra no se ve.
    pantalla_completa: bool,
    /// Nook ocultó la barra de tareas de Windows (para sustituirla).
    barra_de_windows_oculta: bool,
    /// Área de notificación: los iconos que Windows tiene a la vista, los cambios de los iconos que
    /// ya se pasaron a la interfaz y los que se ven (en la tira y los ocultos).
    a_la_vista: Vec<notificacion::AlaVista>,
    bandeja_vista: u64,
    bandeja: (Vec<notificacion::Icono>, Vec<notificacion::Icono>),
    /// Los iconos ocultos están abiertos (en la emergente), cuándo se cerraron por última vez y,
    /// mientras está abierto el menú de uno de ellos, el programa que lo abrió (la emergente sigue
    /// abierta mientras tanto; también el del menú, si es otro).
    ocultos_abiertos: bool,
    ocultos_cerrados: Option<Instant>,
    menu_de_icono: Option<(Vec<u32>, Instant)>,
    /// Apps con ventanas en la barra de tareas, en el orden en que se ven en la tira, lo que ya se
    /// sabe de cada ventana y la última ventana de otro programa que estuvo activa (la de antes
    /// de pulsar en la barra).
    abiertas: Vec<Abierta>,
    ventanas_vistas: abiertas::Vistas,
    ultima_ajena: Option<isize>,
    /// Claves de cada app por su ruta (las instaladas y las de la barra) para reconocer sus
    /// ventanas, y las que se están sacando.
    claves: HashMap<String, Vec<String>>,
    claves_pedidas: HashSet<String>,
    /// Qué hay en cada posición de la tira (ver `refrescar_tira`).
    tira: Vec<Ranura>,
    /// Buscador: apps instaladas (las de la última vez que se leyeron: se leen en segundo plano),
    /// resultados que se muestran y dónde se añade la elegida.
    instaladas: Vec<App>,
    resultados: Vec<App>,
    destino: Destino,
    /// Bóveda: grupo que se está viendo (None = la vista de arriba) y para qué es su campo.
    grupo_boveda: Option<usize>,
    entrada_boveda: EntradaBoveda,
    /// Cuenta de GitHub: si se está iniciando sesión, el código que hay que escribir en GitHub
    /// (cuando ya se tiene) y el último error.
    conectando_cuenta: bool,
    codigo_cuenta: Option<String>,
    error_cuenta: String,
    /// Addons instalados (solo lo que dicen sus manifiestos: se cargan al abrirlos) y lo de su
    /// sección de la configuración.
    addons: Vec<addons::Addon>,
    config_addons: barra::addons::ConfigAddons,
    /// Conversación con Claude Code.
    claude: barra::claude::Conversacion,
}

/// Lo que necesitan los eventos: la ventana, el estado y los temporizadores.
#[derive(Clone)]
struct Ctx {
    ui: slint::Weak<Barra>,
    estado: Rc<RefCell<Estado>>,
    /// Oculta la barra cuando el cursor se aleja (si "Ocultar en reposo" está activo).
    vigia: Rc<Timer>,
    /// Ventana real ("lienzo") en píxeles de pantalla, junto con el borde para el que se calculó.
    lienzo: Rc<Cell<Option<(i32, Rect)>>>,
    /// Caja visible (donde queda la barra cuando termina la animación).
    caja: Rc<Cell<Option<Rect>>>,
    /// Arrastre de la barra por la pantalla en curso, y el temporizador que lo sigue.
    movimiento: Rc<Cell<Option<Movimiento>>>,
    seguidor: Rc<Timer>,
    /// Deslizamiento de la barra hacia un borde nuevo, y el temporizador que lo anima.
    deslizamiento: Rc<Cell<Option<Deslizamiento>>>,
    animador: Rc<Timer>,
    /// Ventana emergente (menús y selector de color), lo que hará la opción que se elija en el
    /// menú abierto, y para qué es el color del selector.
    emergente: slint::Weak<Emergente>,
    accion_menu: Rc<RefCell<Option<AccionMenu>>>,
    destino_color: Rc<RefCell<DestinoColor>>,
    /// Ventana que tenía el foco al abrir la bóveda: se le devuelve al copiar un texto, para
    /// pegarlo en ella.
    ventana_previa: Rc<Cell<Option<isize>>>,
    /// Icono puesto en el área de notificación: la figura y el aspecto del personaje con que se
    /// dibujó y su lado en píxeles.
    icono_puesto: Rc<RefCell<Option<(Arc<personaje::Figura>, personaje::Aspecto, u32)>>>,
    /// Personaje y el temporizador que lo anima.
    personaje: Rc<RefCell<Personaje>>,
    animador_personaje: Rc<Timer>,
    /// Si suena música, su nivel y lo que muestra el reproductor (lo actualiza otro hilo).
    musica: Arc<musica::Musica>,
    /// Cuenta de GitHub con la que se sincroniza la bóveda.
    cuenta: Rc<cuenta::Cuenta>,
    /// Mira de vez en cuando lo que cambia fuera del programa (ver `sondear`), y cuántas veces lo
    /// ha hecho.
    sondeo: Rc<Timer>,
    vueltas: Rc<Cell<u32>>,
    /// Addons cargados y lo que esperan de ellos (ver barra/addons.rs).
    addons: Rc<barra::addons::Addons>,
    /// Hasta cuándo los cambios de forma de la barra se aplican sin animación (ver `saltar`).
    saltar_hasta: Rc<Cell<Instant>>,
}

impl Ctx {
    fn guardar(&self) {
        let e = self.estado.borrow();
        if let Err(e) = config::guardar(&e.config) {
            eprintln!("No se pudo guardar la configuración: {e}");
        }
        // Con la cuenta conectada, la bóveda se sube si cambió.
        self.cuenta.subir(&e.config.boveda);
    }

    /// Pasa todo el estado a la interfaz.
    fn refrescar(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let mut guarda = self.estado.borrow_mut();
        let e = &mut *guarda;
        ui.set_modo(e.modo.numero());
        self.refrescar_ajustes(&ui, e);
        self.refrescar_tira(&ui, e);
        self.refrescar_boveda(&ui, e);
        self.refrescar_addons(&ui, e);
    }

    fn poner_modo(&self, modo: Modo) {
        {
            let mut e = self.estado.borrow_mut();
            e.modo = modo;
            e.fuera = 0;
            if modo != Modo::Config {
                e.grabando = None;
            }
        }
        if let Some(ui) = self.ui.upgrade() {
            if modo != Modo::Grupo {
                ui.set_editando(false);
            }
        }
        self.refrescar();
        self.recolocar();
        // Con un juego a pantalla completa, la barra solo se ve mientras está desplegada.
        self.aplicar_visibilidad();
        if modo.desplegado() {
            self.enfocar();
        }
        self.vigilar();
    }

    /// Da el foco del teclado a la barra.
    fn enfocar(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        if let Some(hwnd) = plataforma::hwnd() {
            ventana::traer_al_frente(hwnd);
        }
        ui.invoke_enfocar();
    }

    /// Vuelve al reposo (u oculta la barra si esa opción está activa y el cursor no está encima).
    fn cerrar(&self) {
        let ocultar = self.estado.borrow().config.ajustes.ocultar_en_reposo;
        let modo = if ocultar && !self.cursor_encima() { Modo::Oculta } else { Modo::Reposo };
        self.poner_modo(modo);
    }

    fn revelar(&self) {
        if self.estado.borrow().modo == Modo::Oculta {
            self.poner_modo(Modo::Reposo);
        }
    }

    fn cursor_encima(&self) -> bool {
        let Some((x, y, ancho, alto)) = self.caja.get() else { return false };
        let (cx, cy) = ventana::posicion_cursor();
        let margen = 8;
        cx >= x - margen && cy >= y - margen && cx <= x + ancho + margen && cy <= y + alto + margen
    }

    /// En reposo con "Ocultar en reposo" activo, oculta la barra cuando el cursor se aleja.
    fn vigilar(&self) {
        let activo = {
            let e = self.estado.borrow();
            e.config.ajustes.ocultar_en_reposo && e.modo == Modo::Reposo
        };
        if !activo {
            self.vigia.stop();
            return;
        }
        let ctx = self.clone();
        self.vigia.start(TimerMode::Repeated, Duration::from_millis(300), move || {
            // Mientras se arrastra o se desliza, la barra no está en su sitio: no se oculta. Ni
            // con una emergente suya abierta (un menú, los iconos ocultos), que suele quedar fuera.
            let moviendose = ctx.movimiento.get().is_some() || ctx.deslizamiento.get().is_some();
            let emergente = ctx.emergente.upgrade().is_some_and(|em| plataforma::se_ve(em.window()));
            if moviendose || emergente || ctx.cursor_encima() {
                ctx.estado.borrow_mut().fuera = 0;
                return;
            }
            let fuera = {
                let mut e = ctx.estado.borrow_mut();
                e.fuera += 1;
                e.fuera
            };
            if fuera >= 2 {
                ctx.poner_modo(Modo::Oculta);
            }
        });
    }

    /// Mira lo que puede cambiar fuera del programa, cada `SONDEO` (lo que corre menos prisa,
    /// cada `VUELTAS_LENTAS` veces): la ventana activa, la hora, si hay algo a pantalla completa,
    /// las apps abiertas (salvo con algo a pantalla completa, que no se ven), el micrófono, la
    /// batería y el volumen general (este, cada vez con su panel abierto).
    fn sondear(&self) {
        let vuelta = self.vueltas.get().wrapping_add(1);
        self.vueltas.set(vuelta);
        let lenta = vuelta % VUELTAS_LENTAS == 0;
        let (modo, boton_sonido, boton_microfono, boton_bateria) = {
            let e = self.estado.borrow();
            let a = &e.config.ajustes;
            (e.modo, a.boton_sonido, a.boton_microfono, a.boton_bateria)
        };
        if ventana::primer_plano_es_de_otro_proceso() {
            self.estado.borrow_mut().ultima_ajena = Some(ventana::en_primer_plano());
        }
        if lenta {
            if let Some(ui) = self.ui.upgrade() {
                let formato = self.estado.borrow().config.ajustes.fecha;
                actualizar_reloj(&ui, Local::now(), formato);
            }
            // Windows a veces vuelve a mostrar su barra (al reiniciarse el Explorador, por ejemplo).
            self.aplicar_barra_de_windows();
            self.sondear_pantalla_completa();
            if !self.estado.borrow().pantalla_completa {
                self.sondear_abiertas();
            }
        }
        if modo == Modo::Sonido || (lenta && boton_sonido && modo != Modo::Oculta) {
            self.mostrar_sonido();
        }
        if lenta && boton_bateria && modo != Modo::Oculta {
            self.mostrar_bateria();
        }
        if lenta && boton_microfono && modo != Modo::Oculta {
            self.mostrar_microfono();
        }
        self.sondear_bandeja(lenta);
        self.vigilar_menu_de_icono();
    }

    /// Teclas pulsadas con la barra enfocada. Devuelve true si se usaron.
    fn tecla(&self, texto: &str) -> bool {
        let Some(ui) = self.ui.upgrade() else { return false };
        let escape = SharedString::from(Key::Escape);
        let tab = SharedString::from(Key::Tab);
        let tab_atras = SharedString::from(Key::Backtab);
        let (modo, grabando) = {
            let e = self.estado.borrow();
            (e.modo, e.grabando)
        };

        if let Some(objetivo) = grabando {
            let borrar = [Key::Delete, Key::Backspace].map(SharedString::from);
            if texto == escape.as_str() {
                self.estado.borrow_mut().grabando = None;
            } else if borrar.iter().any(|b| b.as_str() == texto) {
                // Supr o Retroceso: deja el atajo sin definir.
                self.poner_atajo(objetivo, None);
            } else if let Some(atajo) = atajos::capturar() {
                self.poner_atajo(objetivo, Some(atajo));
            } else {
                // Solo modificadores (por ejemplo Ctrl sin soltar): se sigue esperando.
                return true;
            }
            self.refrescar();
            return true;
        }

        if texto == escape.as_str() && modo.desplegado() {
            self.cerrar();
            return true;
        }
        if modo == Modo::Musica {
            return self.tecla_reproductor(texto);
        }
        if modo == Modo::Sonido {
            return self.tecla_sonido(texto);
        }
        if modo != Modo::Grupo || ui.get_editando() {
            return false;
        }
        if texto == tab.as_str() || texto == tab_atras.as_str() {
            let siguiente = {
                let e = self.estado.borrow();
                let grupos: Vec<bool> = e.config.elementos.iter().map(|el| el.grupo().is_some()).collect();
                siguiente_grupo(&grupos, e.activo, texto == tab.as_str())
            };
            if let Some(siguiente) = siguiente {
                self.mostrar_grupo(siguiente, false);
            }
            return true;
        }
        if texto.eq_ignore_ascii_case("e") {
            ui.set_editando(true);
            return true;
        }
        if let Some(d) = texto.chars().next().and_then(|c| c.to_digit(10)).filter(|d| *d >= 1) {
            if texto.chars().count() == 1 {
                self.lanzar(d as usize - 1);
                return true;
            }
        }
        false
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // El ayudante de la tecla Windows, su preparación y su retirada (como administrador) no son
    // la barra.
    match std::env::args().nth(1).as_deref() {
        Some(tecla_windows::ARG_AYUDANTE) => {
            tecla_windows::ayudante();
            return Ok(());
        }
        Some(tecla_windows::ARG_PREPARAR) => std::process::exit(tecla_windows::preparar()),
        Some(tecla_windows::ARG_QUITAR) => std::process::exit(tecla_windows::quitar()),
        // El servidor MCP de los addons, para la IA del usuario (ver addons/mcp.rs).
        Some(addons::mcp::ARG) => {
            addons::mcp::servir();
            return Ok(());
        }
        _ => {}
    }
    // Solo puede haber una barra: si ya hay una abierta, se le pide que se muestre y se sale.
    // Tras actualizarse, la versión nueva espera a que se cierre la anterior.
    let tras_actualizar = std::env::args().any(|a| a == actualizacion::TRAS_ACTUALIZAR);
    if !plataforma::unica_instancia(tras_actualizar) {
        return Ok(());
    }
    actualizacion::limpiar();

    // Ventana transparente propia dibujada por software (ver plataforma.rs).
    plataforma::instalar()?;

    // La barra tiene que ser la primera ventana que se crea; la emergente, la segunda.
    let ui = Barra::new()?;
    let _ = ui.window();
    let emergente = Emergente::new()?;
    ui.set_version(format!("v{}", actualizacion::VERSION).into());
    // Hasta saber cuál se eligió, el idioma de Windows: en él se nombra el grupo de ejemplo que se
    // crea la primera vez.
    idioma::poner(config::Idioma::Sistema);
    let config = config::cargar();
    idioma::poner(config.ajustes.idioma);
    let modo_inicial = if config.ajustes.ocultar_en_reposo { Modo::Oculta } else { Modo::Reposo };
    let ctx = Ctx {
        ui: ui.as_weak(),
        estado: Rc::new(RefCell::new(Estado {
            config,
            modo: modo_inicial,
            activo: 0,
            iconos: HashMap::new(),
            iconos_pedidos: HashSet::new(),
            grabando: None,
            atajos_ok: Vec::new(),
            mostrar_ok: false,
            boveda_ok: false,
            microfono_ok: false,
            buscar_ok: false,
            claude_ok: false,
            registrados: 0,
            inicia_con_windows: sistema::inicia_con_windows(),
            fuera: 0,
            pantalla_completa: false,
            barra_de_windows_oculta: false,
            a_la_vista: Vec::new(),
            bandeja_vista: 0,
            bandeja: (Vec::new(), Vec::new()),
            ocultos_abiertos: false,
            ocultos_cerrados: None,
            menu_de_icono: None,
            abiertas: Vec::new(),
            ventanas_vistas: abiertas::Vistas::new(),
            ultima_ajena: None,
            claves: HashMap::new(),
            claves_pedidas: HashSet::new(),
            tira: Vec::new(),
            instaladas: Vec::new(),
            resultados: Vec::new(),
            destino: Destino::Barra,
            grupo_boveda: None,
            entrada_boveda: EntradaBoveda::Texto,
            conectando_cuenta: false,
            codigo_cuenta: None,
            error_cuenta: String::new(),
            addons: Vec::new(),
            config_addons: Default::default(),
            claude: Default::default(),
        })),
        vigia: Rc::new(Timer::default()),
        lienzo: Rc::new(Cell::new(None)),
        caja: Rc::new(Cell::new(None)),
        movimiento: Rc::new(Cell::new(None)),
        seguidor: Rc::new(Timer::default()),
        deslizamiento: Rc::new(Cell::new(None)),
        animador: Rc::new(Timer::default()),
        emergente: emergente.as_weak(),
        accion_menu: Rc::new(RefCell::new(None)),
        destino_color: Rc::new(RefCell::new(DestinoColor::Paleta { acento: false })),
        ventana_previa: Rc::new(Cell::new(None)),
        icono_puesto: Rc::new(RefCell::new(None)),
        personaje: Rc::new(RefCell::new(Personaje::new())),
        animador_personaje: Rc::new(Timer::default()),
        musica: musica::vigilar(),
        cuenta: Rc::new(cuenta::Cuenta::iniciar({
            let ui = ui.as_weak();
            move || {
                let _ = ui.upgrade_in_event_loop(|ui| ui.invoke_novedades_cuenta());
            }
        })),
        sondeo: Rc::new(Timer::default()),
        vueltas: Rc::new(Cell::new(0)),
        addons: Rc::default(),
        saltar_hasta: Rc::new(Cell::new(Instant::now())),
    };
    // Al arrancar, la barra cambia de forma varias veces mientras llegan los datos (los addons,
    // las apps abiertas, los iconos): sin animarlo, se ve ya en su sitio y no a medio camino.
    ctx.saltar_durante(SIN_ANIMAR_AL_ARRANCAR);
    ctx.leer_addons();
    ctx.refrescar();
    ctx.recolocar();
    // Lo primero, por si Nook arranca con Windows: así recibe los iconos de los programas que
    // arrancan después.
    ctx.aplicar_barra_de_windows();
    ctx.aplicar_area_de_notificacion();
    let c = ctx.clone();
    tecla_windows::al_pulsar(move || c.alternar_lanzador());
    tecla_windows::activar(ctx.estado.borrow().config.ajustes.tecla_windows_abre_buscador);

    // Reproductor: lo que suena se pasa a la interfaz cuando cambia.
    let c = ctx.clone();
    let mut visto = (None, None);
    c.mostrar_reproduccion(&mut visto);
    musica::al_cambiar(move || c.mostrar_reproduccion(&mut visto));

    // --- Eventos de la interfaz ---

    barra::abiertas::conectar(&ui, &ctx);
    barra::addons::conectar(&ui, &ctx);
    barra::claude::conectar(&ui, &ctx);
    barra::atajos::conectar(&ui, &ctx);
    barra::boveda::conectar(&ui, &ctx);
    barra::buscador::conectar(&ui, &ctx);
    barra::colocacion::conectar(&ui, &ctx);
    barra::configuracion::conectar(&ui, &ctx);
    barra::energia::conectar(&ui, &ctx);
    barra::menus::conectar(&ui, &ctx);
    barra::menus::conectar_emergente(&emergente, &ctx);
    barra::notificacion::conectar(&ui, &ctx);
    barra::notificacion::conectar_emergente(&emergente, &ctx);
    barra::personaje::conectar(&ui, &ctx);
    barra::reproductor::conectar(&ui, &ctx);
    barra::sonido::conectar(&ui, &ctx);
    barra::tira::conectar(&ui, &ctx);

    let c = ctx.clone();
    ui.on_revelar(move || c.revelar());
    let c = ctx.clone();
    ui.on_cerrar(move || c.cerrar());
    let c = ctx.clone();
    ui.on_tecla_pulsada(move |texto| c.tecla(&texto));

    // Clic en otro programa: se cierra el panel. Se comprueba un instante después porque el foco
    // puede pasar al selector de archivos, que es de este programa.
    // Y al ponerse delante con el buscador abierto, el foco va a su campo.
    let c = ctx.clone();
    plataforma::al_activarse(move |activa| {
        if activa {
            if let (Modo::Buscar, Some(ui)) = (c.estado.borrow().modo, c.ui.upgrade()) {
                ui.invoke_enfocar();
            }
            return;
        }
        let c = c.clone();
        Timer::single_shot(Duration::from_millis(120), move || {
            let desplegado = c.estado.borrow().modo.desplegado();
            if desplegado && ventana::primer_plano_es_de_otro_proceso() {
                c.cerrar();
            }
        });
    });
    let c = ctx.clone();
    plataforma::al_arrastrar(move |evento| c.arrastre(evento));
    let c = ctx.clone();
    plataforma::al_aviso(move |aviso| c.aviso(aviso));
    // Con transparencia, antes de cada dibujo se dice dónde está la forma visible de la barra (el
    // desenfoque va debajo): la forma animada.
    let barra = ui.as_weak();
    ui.global::<Tema>().set_escala(ui.window().scale_factor());
    plataforma::al_presentar(ui.window(), move || {
        let Some(ui) = barra.upgrade() else { return };
        // Para que los bordes de la forma caigan en píxeles enteros (ver `escala` en ui/barra.slint)
        // y los iconos midan un número entero de píxeles (ver `IconoApp` en ui/iconos.slint).
        ui.set_escala(ui.window().scale_factor());
        ui.global::<Tema>().set_escala(ui.window().scale_factor());
        let forma = [ui.get_forma_tira(), ui.get_forma_panel(), ui.get_forma_empalme_inicio(), ui.get_forma_empalme_final()];
        plataforma::recortar(ui.window(), Some(recorte(ui.window(), forma)));
    });

    ui.show()?;
    emergente.show()?;

    ctx.registrar_atajos();
    ctx.refrescar();
    ctx.vigilar();
    ctx.animar(true);
    ctx.poner_icono(true);
    ctx.mostrar_sonido();
    ctx.mostrar_microfono();
    ctx.mostrar_bateria();
    let c = ctx.clone();
    ctx.sondeo.start(TimerMode::Repeated, SONDEO, move || c.sondear());
    // Las apps instaladas, para que el buscador se abra ya con ellas.
    ctx.leer_instaladas();
    buscar_actualizaciones(ui.as_weak());

    let resultado = slint::run_event_loop_until_quit();
    if let Some(hwnd) = plataforma::hwnd() {
        bandeja::quitar(hwnd);
    }
    // Sin Nook, los iconos vuelven a ir solo a la barra de tareas de Windows, y esta vuelve como
    // estaba.
    notificacion::parar();
    ctx.devolver_barra_de_windows();
    resultado?;
    Ok(())
}

/// Cada cuánto se mira lo que cambia fuera del programa con algo abierto que lo muestra, y cada
/// cuántas de esas veces lo demás.
const SONDEO: Duration = Duration::from_millis(250);
const VUELTAS_LENTAS: u32 = 4;

/// Cada cuánto se pregunta a GitHub si hay una versión nueva.
const BUSCAR_ACTUALIZACION: Duration = Duration::from_secs(6 * 60 * 60);

/// Pregunta a GitHub por versiones nuevas al arrancar y luego de vez en cuando, en un hilo
/// aparte; si hay una, la interfaz lo avisa. No se hace en las compilaciones de desarrollo.
fn buscar_actualizaciones(ui: slint::Weak<Barra>) {
    if cfg!(debug_assertions) {
        return;
    }
    let _ = std::thread::Builder::new().name("actualizacion".into()).spawn(move || loop {
        if let Some(nueva) = actualizacion::buscar() {
            let _ = ui.upgrade_in_event_loop(move |ui| {
                if !ui.get_actualizando() {
                    ui.set_actualizacion(format!("v{}", nueva.version).into());
                }
            });
            return;
        }
        std::thread::sleep(BUSCAR_ACTUALIZACION);
    });
}

/// Descarga e instala la versión nueva (botón "Actualizar"), con la barra de progreso al día. Si
/// va bien, la versión nueva ya está abierta y esta se cierra.
fn actualizar(ui: slint::Weak<Barra>) {
    let (Some(barra), Some(nueva)) = (ui.upgrade(), actualizacion::disponible()) else { return };
    if barra.get_actualizando() {
        return;
    }
    barra.set_actualizando(true);
    barra.set_progreso_actualizacion(0.0);
    barra.set_error_actualizacion("".into());
    let _ = std::thread::Builder::new().name("actualizacion".into()).spawn(move || {
        let progreso = |p: f32| {
            let _ = ui.upgrade_in_event_loop(move |ui| ui.set_progreso_actualizacion(p));
        };
        let resultado = actualizacion::instalar(&nueva, progreso);
        let _ = ui.upgrade_in_event_loop(move |ui| match resultado {
            Ok(()) => {
                let _ = slint::quit_event_loop();
            }
            Err(e) => {
                eprintln!("No se pudo actualizar: {e}");
                ui.set_actualizando(false);
                ui.set_error_actualizacion(e.into());
            }
        });
    });
}

fn actualizar_reloj(ui: &Barra, ahora: DateTime<Local>, formato: FormatoFecha) {
    ui.set_hora(formatear_hora(&ahora).into());
    ui.set_hora_vertical(ahora.format("%H\n%M").to_string().into());
    let (fecha, vertical) = fecha::formatear(ahora.date_naive(), formato, idioma::codigo_actual());
    ui.set_fecha(fecha.into());
    ui.set_fecha_vertical(vertical.into());
}

fn formatear_hora(t: &DateTime<Local>) -> String {
    t.format("%H:%M").to_string()
}

fn modelo<T: Clone + 'static>(datos: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(datos))
}

/// Radio de las esquinas de las emergentes (el de su `Fondo` en ui/tema.slint).
const RADIO_EMERGENTE: f32 = 18.0;

/// Forma visible de una ventana (para el desenfoque de detrás), de píxeles lógicos a físicos.
fn recorte(ventana: &slint::Window, piezas: [Pieza; 4]) -> plataforma::Forma {
    let escala = ventana.scale_factor();
    let px = |v: f32| (v * escala).round() as i32;
    piezas.map(|p| {
        let (x, y) = (px(p.x), px(p.y));
        plataforma::Pieza {
            // Con los bordes redondeados como los dibuja Slint: así dos piezas que se tocan no
            // dejan un hueco entre ellas.
            rect: (x, y, px(p.x + p.ancho) - x, px(p.y + p.alto) - y),
            radios: [px(p.arriba_izquierda), px(p.arriba_derecha), px(p.abajo_derecha), px(p.abajo_izquierda)],
            hueco: (1..=4).contains(&p.hueco).then(|| p.hueco as usize - 1),
        }
    })
}

/// Pieza rectangular con todas las esquinas del mismo radio (en píxeles lógicos).
fn pieza_redondeada(ancho: f32, alto: f32, radio: f32) -> Pieza {
    Pieza {
        x: 0.0,
        y: 0.0,
        ancho,
        alto,
        arriba_izquierda: radio,
        arriba_derecha: radio,
        abajo_derecha: radio,
        abajo_izquierda: radio,
        hueco: 0,
    }
}

// Tests de la lógica que no es visual. Se ejecutan con `cargo test` (los de cada parte, en su
// módulo).
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn hora_con_dos_digitos() {
        let t = Local.with_ymd_and_hms(2026, 1, 5, 7, 3, 0).unwrap();
        assert_eq!(formatear_hora(&t), "07:03");
    }
}
