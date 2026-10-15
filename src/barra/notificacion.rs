//! Área de notificación en la tira, cuando Nook sustituye la barra de tareas de Windows: los
//! iconos que Windows tiene a la vista van en la tira, y los demás en una ventanita que abre la
//! flecha, como en Windows (ver `notificacion` para cómo llegan).

use crate::*;

/// Lado de los iconos del área de notificación, en píxeles lógicos (los `Image` de ui/barra.slint
/// y ui/emergente.slint).
const LADO_ICONO_BANDEJA: f32 = 20.0;
/// Cada cuántas vueltas lentas del sondeo se vuelve a leer qué iconos tiene Windows a la vista.
const VUELTAS_A_LA_VISTA: u32 = 5;

/// Separación entre la flecha y la ventanita de los iconos ocultos, en píxeles de pantalla, y
/// tiempo tras cerrarse en el que un clic en la flecha no la vuelve a abrir (era ese clic el que
/// la cerró, al quitarle el foco).
const SEPARACION_OCULTOS: i32 = 6;
const TRAS_CERRAR_OCULTOS: Duration = Duration::from_millis(300);
/// Lo que puede tardar el programa de un icono en abrir su menú tras el clic derecho.
const ESPERA_MENU_DE_ICONO: Duration = Duration::from_secs(1);

impl Ctx {
    /// Empieza o deja de recibir los iconos (solo se muestran con la barra de Windows sustituida).
    pub(crate) fn aplicar_area_de_notificacion(&self) {
        let activa = self.estado.borrow().config.ajustes.sustituir_barra_de_windows;
        if activa {
            notificacion::iniciar(lado_icono(LADO_ICONO_BANDEJA));
            self.estado.borrow_mut().a_la_vista = notificacion::a_la_vista_en_windows();
        } else {
            notificacion::parar();
            if self.estado.borrow().ocultos_abiertos {
                self.cerrar_emergente(false);
            }
        }
        self.refrescar_bandeja();
    }

    /// Pasa a la interfaz los iconos si cambiaron y, de vez en cuando (en las vueltas `lenta`s),
    /// hace el mantenimiento de la bandeja y vuelve a leer cuáles tiene Windows a la vista.
    pub(crate) fn sondear_bandeja(&self, lenta: bool) {
        if !notificacion::activa() {
            return;
        }
        if lenta {
            notificacion::revisar();
            let vuelta = self.vueltas.get() / VUELTAS_LENTAS;
            if vuelta % VUELTAS_A_LA_VISTA == 0 {
                let a_la_vista = notificacion::a_la_vista_en_windows();
                let mut e = self.estado.borrow_mut();
                if e.a_la_vista != a_la_vista {
                    e.a_la_vista = a_la_vista;
                    e.bandeja_vista = u64::MAX;
                }
            }
        }
        if self.estado.borrow().bandeja_vista != notificacion::cambios() {
            self.refrescar_bandeja();
        }
    }

    /// Pasa a la interfaz los iconos del área de notificación: los que Windows tiene a la vista, a
    /// la tira; los demás, a la ventanita de los ocultos (también si está abierta).
    pub(crate) fn refrescar_bandeja(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let iconos = notificacion::iconos();
        let lado = lado_icono(LADO_ICONO_BANDEJA);
        let mut e = self.estado.borrow_mut();
        let (vista, ocultos): (Vec<_>, Vec<_>) = iconos.into_iter().partition(|icono| e.a_la_vista.iter().any(|v| v.es(icono)));
        let item = |icono: &notificacion::Icono| {
            let imagen = icono.pixeles.as_ref().filter(|_| icono.lado == lado).map(|p| sistema::imagen(p, lado));
            IconoBandejaItem {
                tiene_icono: imagen.is_some(),
                icono: imagen.unwrap_or_default(),
                // En una sola línea, como cabe en su cartel.
                texto: icono.texto.split_whitespace().collect::<Vec<_>>().join(" ").into(),
            }
        };
        ui.set_bandeja(modelo(vista.iter().map(item).collect()));
        ui.set_iconos_ocultos(ocultos.len() as i32);
        ui.set_area_notificacion(notificacion::activa());
        if e.ocultos_abiertos {
            if let Some(em) = self.emergente.upgrade() {
                em.set_iconos_ocultos(modelo(ocultos.iter().map(item).collect()));
            }
        }
        e.bandeja_vista = notificacion::cambios();
        e.bandeja = (vista, ocultos);
    }

    /// Clic en el icono `i` de la tira o (`oculto`) de los ocultos, con el botón izquierdo o el
    /// derecho: le llega a su programa. Si es el clic derecho en uno oculto, los ocultos siguen
    /// abiertos mientras su programa muestra el menú.
    pub(crate) fn pulsar_bandeja(&self, i: usize, derecho: bool, oculto: bool) {
        let icono = {
            let e = self.estado.borrow();
            let lista = if oculto { &e.bandeja.1 } else { &e.bandeja.0 };
            lista.get(i).cloned()
        };
        let Some(icono) = icono else { return };
        if oculto && derecho {
            self.estado.borrow_mut().menu_de_icono = Some((vec![ventana::proceso_de(icono.hwnd)], Instant::now()));
        }
        notificacion::pulsar(&icono, derecho, ventana::posicion_cursor());
    }

    /// Abre o cierra la ventanita de los iconos ocultos, debajo de la flecha (encima con la barra
    /// abajo, o a su lado con la barra en un lateral).
    pub(crate) fn alternar_ocultos(&self) {
        let (abiertos, recien_cerrados) = {
            let e = self.estado.borrow();
            (e.ocultos_abiertos, e.ocultos_cerrados.is_some_and(|t| t.elapsed() < TRAS_CERRAR_OCULTOS))
        };
        if abiertos {
            self.cerrar_emergente(true);
            return;
        }
        if recien_cerrados {
            return;
        }
        let (Some(em), Some((x, y, ancho, alto))) = (self.emergente.upgrade(), self.caja.get()) else { return };
        let lado = lado_icono(LADO_ICONO_BANDEJA);
        let items: Vec<IconoBandejaItem> = self
            .estado
            .borrow()
            .bandeja
            .1
            .iter()
            .map(|icono| {
                let imagen = icono.pixeles.as_ref().filter(|_| icono.lado == lado).map(|p| sistema::imagen(p, lado));
                IconoBandejaItem { tiene_icono: imagen.is_some(), icono: imagen.unwrap_or_default(), texto: icono.texto.as_str().into() }
            })
            .collect();
        self.accion_menu.borrow_mut().take();
        em.set_tipo(2);
        em.set_iconos_ocultos(modelo(items));
        aplicar_tema(&self.estado.borrow().config.ajustes, em.global::<Tema>());
        let (cx, cy) = ventana::posicion_cursor();
        let (punto, junto) = match self.estado.borrow().config.ajustes.borde() {
            Posicion::Arriba => ((cx, y + alto + SEPARACION_OCULTOS), plataforma::Junto::Debajo),
            Posicion::Abajo => ((cx, y - SEPARACION_OCULTOS), plataforma::Junto::Encima),
            Posicion::Izquierda => ((x + ancho + SEPARACION_OCULTOS, cy), plataforma::Junto::Derecha),
            Posicion::Derecha => ((x - SEPARACION_OCULTOS, cy), plataforma::Junto::Izquierda),
        };
        plataforma::mostrar_emergente_junto(em.window(), punto, junto, (em.get_ancho(), em.get_alto()));
        em.invoke_enfocar();
        self.estado.borrow_mut().ocultos_abiertos = true;
        if let Some(ui) = self.ui.upgrade() {
            ui.set_ocultos_abiertos(true);
        }
    }

    /// La emergente se cerró: si eran los iconos ocultos, se anota.
    pub(crate) fn ocultos_cerrados(&self) {
        let mut e = self.estado.borrow_mut();
        if !std::mem::replace(&mut e.ocultos_abiertos, false) {
            return;
        }
        e.ocultos_cerrados = Some(Instant::now());
        e.menu_de_icono = None;
        drop(e);
        if let Some(ui) = self.ui.upgrade() {
            ui.set_ocultos_abiertos(false);
        }
    }

    /// Mientras está abierto el menú de un icono oculto, los ocultos siguen abiertos; se cierran
    /// cuando se pasa a otro programa (un clic fuera). Si se vuelve a ellos, todo sigue como
    /// siempre. El menú puede ser de otro proceso del mismo programa (el de Steam lo dibuja su
    /// navegador): cuenta también el que se pone delante justo después del clic.
    pub(crate) fn vigilar_menu_de_icono(&self) {
        let Some((programas, desde)) = self.estado.borrow().menu_de_icono.clone() else { return };
        if !ventana::primer_plano_es_de_otro_proceso() {
            // Al principio aún no ha abierto su menú.
            if desde.elapsed() > ESPERA_MENU_DE_ICONO {
                self.estado.borrow_mut().menu_de_icono = None;
            }
            return;
        }
        let delante = ventana::proceso_de(ventana::en_primer_plano());
        if programas.contains(&delante) {
            return;
        }
        let mut e = self.estado.borrow_mut();
        if desde.elapsed() <= ESPERA_MENU_DE_ICONO {
            if let Some((programas, _)) = e.menu_de_icono.as_mut() {
                programas.push(delante);
            }
            return;
        }
        e.menu_de_icono = None;
        drop(e);
        self.cerrar_emergente(false);
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_pulsar_bandeja(move |i, derecho| c.pulsar_bandeja(i.max(0) as usize, derecho, false));
    let c = ctx.clone();
    ui.on_alternar_ocultos(move || {
        // Se difiere: abre la emergente y no debe hacerse desde dentro del evento de ratón.
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.alternar_ocultos());
    });
    // Las notificaciones y el calendario de Windows, como al pulsar la hora en su barra (en nook
    // Pro).
    ui.on_abrir_notificaciones(|| {
        if !PRO {
            return;
        }
        if let Err(e) = sistema::lanzar("ms-actioncenter:") {
            eprintln!("{e}");
        }
    });
}

/// Conecta los eventos de los iconos ocultos (en la emergente).
pub(crate) fn conectar_emergente(emergente: &Emergente, ctx: &Ctx) {
    let c = ctx.clone();
    emergente.on_pulsar_icono(move |i, derecho| c.pulsar_bandeja(i.max(0) as usize, derecho, true));
}
