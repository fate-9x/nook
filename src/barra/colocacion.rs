//! Dónde va la barra: la ventana y la caja visible en la pantalla, el arrastre de la barra con el
//! ratón y el deslizamiento hasta el borde en el que se suelta.

use crate::*;

/// Arrastre de la barra: dónde estaba el cursor y la ventana al empezar.
#[derive(Clone, Copy)]
pub(crate) struct Movimiento {
    pub(crate) cursor: (i32, i32),
    pub(crate) lienzo: Rect,
}

/// Al cambiar de borde, la ventana parte desplazada `desde` (píxeles respecto a su sitio final)
/// y se acerca a él.
#[derive(Clone, Copy)]
pub(crate) struct Deslizamiento {
    pub(crate) desde: (i32, i32),
    pub(crate) inicio: Instant,
}

/// Duración del deslizamiento al cambiar de borde.
pub(crate) const DURACION_DESLIZAMIENTO: Duration = Duration::from_millis(420);

/// Margen del lienzo, en píxeles lógicos, para el pequeño rebote de la animación al abrir.
pub(crate) const MARGEN_REBOTE: f32 = 24.0;

/// Lienzo (ventana real) y caja visible en píxeles de pantalla.
/// Arriba (o abajo): ambos centrados en horizontal y pegados a ese borde. Laterales: pegados al borde; la caja
/// empieza a la altura de la barra en reposo (centrada) y el lienzo la contiene en todos los modos.
/// La línea fina del modo oculto va centrada.
pub(crate) fn geometria(
    monitor: Rect,
    posicion: i32,
    oculta: bool,
    caja: (i32, i32),
    lienzo: (i32, i32),
    alto_reposo: i32,
) -> (Rect, Rect) {
    let (ox, oy, ..) = monitor;
    let (l, c) = geometria_local((monitor.2, monitor.3), posicion, oculta, caja, lienzo, alto_reposo);
    ((l.0 + ox, l.1 + oy, l.2, l.3), (c.0 + ox, c.1 + oy, c.2, c.3))
}

/// Parte del ancho del monitor, a cada lado, en la que soltar la barra la lleva a ese lateral.
pub(crate) const ZONA_LATERAL: f32 = 0.3;

/// Borde del monitor que corresponde al punto: el tercio izquierdo lleva a la izquierda, el
/// derecho a la derecha y el centro arriba o, si se puede (`abajo`), abajo en su mitad inferior.
pub(crate) fn borde_cercano((x, y, ancho, alto): Rect, (cx, cy): (i32, i32), abajo: bool) -> Posicion {
    let zona = (ancho as f32 * ZONA_LATERAL) as i32;
    if cx - x < zona {
        Posicion::Izquierda
    } else if x + ancho - cx < zona {
        Posicion::Derecha
    } else if abajo && cy - y > alto / 2 {
        Posicion::Abajo
    } else {
        Posicion::Arriba
    }
}

/// Curva del deslizamiento: rápida al principio y frenando al llegar (de 0 a 1).
pub(crate) fn suavizar(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// Lo mismo que `geometria`, con el monitor en el origen.
pub(crate) fn geometria_local(
    (pantalla_w, pantalla_h): (i32, i32),
    posicion: i32,
    oculta: bool,
    (w, h): (i32, i32),
    (lw, lh): (i32, i32),
    alto_reposo: i32,
) -> (Rect, Rect) {
    let ancla = ((pantalla_h - alto_reposo) / 2).max(0);
    let caja_y = if oculta { (pantalla_h - h) / 2 } else { y_desplegado(pantalla_h, alto_reposo, h) };
    let lienzo_y = ancla.min(pantalla_h - lh).max(0).min(caja_y);
    match posicion {
        1 => ((0, lienzo_y, lw, lh), (0, caja_y, w, h)),
        2 => ((pantalla_w - lw, lienzo_y, lw, lh), (pantalla_w - w, caja_y, w, h)),
        3 => (((pantalla_w - lw) / 2, pantalla_h - lh, lw, lh), ((pantalla_w - w) / 2, pantalla_h - h, w, h)),
        _ => (((pantalla_w - lw) / 2, 0, lw, lh), ((pantalla_w - w) / 2, 0, w, h)),
    }
}

/// Posición vertical de la barra lateral: el borde superior de la barra en reposo
/// (centrada), ajustado para que una ventana de alto `alto` no se salga por abajo.
pub(crate) fn y_desplegado(alto_pantalla: i32, alto_reposo: i32, alto: i32) -> i32 {
    let arriba = (alto_pantalla - alto_reposo) / 2;
    arriba.min(alto_pantalla - alto).max(0)
}

impl Ctx {
    /// Oculta la barra de tareas de Windows si Nook la sustituye (y si se volvió a mostrar), o la
    /// devuelve si dejó de sustituirla (o si se quedó a medias de una vez anterior, por haberse
    /// cerrado Nook de golpe).
    pub(crate) fn aplicar_barra_de_windows(&self) {
        let (sustituir, oculta, ocultandose) = {
            let e = self.estado.borrow();
            let a = &e.config.ajustes;
            (a.sustituir_barra_de_windows, e.barra_de_windows_oculta, a.barra_de_windows_ocultandose)
        };
        if sustituir {
            if oculta && !barra_de_tareas::se_ve() {
                return;
            }
            let activada = barra_de_tareas::ocultar();
            let mut e = self.estado.borrow_mut();
            e.barra_de_windows_oculta = true;
            if activada && !ocultandose {
                e.config.ajustes.barra_de_windows_ocultandose = true;
                drop(e);
                self.guardar();
            }
        } else if oculta || ocultandose {
            self.devolver_barra_de_windows();
        }
    }

    /// Vuelve a mostrar la barra de tareas de Windows como estaba, si Nook la ocultó.
    pub(crate) fn devolver_barra_de_windows(&self) {
        let (oculta, ocultandose) = {
            let e = self.estado.borrow();
            (e.barra_de_windows_oculta, e.config.ajustes.barra_de_windows_ocultandose)
        };
        if !oculta && !ocultandose {
            return;
        }
        barra_de_tareas::mostrar(ocultandose);
        let mut e = self.estado.borrow_mut();
        e.barra_de_windows_oculta = false;
        if ocultandose {
            e.config.ajustes.barra_de_windows_ocultandose = false;
            drop(e);
            self.guardar();
        }
    }

    /// Mira si hay un juego o una app a pantalla completa delante en el monitor de la barra (con
    /// la opción de ocultarse activa) y, si cambió, oculta o vuelve a mostrar la barra.
    pub(crate) fn sondear_pantalla_completa(&self) {
        let (ocultarse, monitor) = {
            let a = &self.estado.borrow().config.ajustes;
            (a.ocultar_en_pantalla_completa, a.monitor)
        };
        let ahora = ocultarse && pantalla_completa::hay(ventana::monitor_en(monitor));
        if std::mem::replace(&mut self.estado.borrow_mut().pantalla_completa, ahora) != ahora {
            self.aplicar_visibilidad();
        }
    }

    /// Con un juego o una app a pantalla completa delante no se ven ni la barra ni el personaje
    /// (que deja de animarse), salvo mientras está desplegada: con su atajo o desde el icono del
    /// área de notificación se puede abrir igual.
    pub(crate) fn aplicar_visibilidad(&self) {
        let visible = {
            let e = self.estado.borrow();
            !e.pantalla_completa || e.modo.desplegado()
        };
        plataforma::mostrar_barra(visible);
        self.animar(visible);
    }

    /// Coloca el lienzo (la ventana transparente) y dice a la interfaz dónde va la caja visible.
    /// La animación del contorno la hace Slint, y los clics fuera de la barra pasan solos a lo
    /// que hay debajo porque ahí la ventana es transparente.
    pub(crate) fn recolocar(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        // Mientras se arrastra, la ventana la mueve el arrastre.
        if self.movimiento.get().is_some() {
            return;
        }
        let escala = plataforma::escala();
        let px = |v: f32| (v * escala).round() as i32;
        let posicion = ui.get_posicion();
        let (w, h) = (px(ui.get_ancho()), px(ui.get_alto()));

        // Espacio extra para el rebote: a ambos lados si está arriba o abajo (crece desde el
        // centro), hacia el interior de la pantalla en los laterales, y a lo alto hacia abajo (o,
        // abajo, hacia arriba: el lienzo va pegado al borde inferior).
        let margen = px(MARGEN_REBOTE);
        let horizontal = posicion == 0 || posicion == 3;
        let extra_ancho = if horizontal { 2 * margen } else { margen };
        let (mut lw, mut lh) =
            (px(ui.get_lienzo_ancho()).max(w) + extra_ancho, px(ui.get_lienzo_alto()).max(h) + margen);
        // El lienzo solo crece (para no tener que mover la ventana), salvo al cambiar de borde.
        if let Some((p, anterior)) = self.lienzo.get() {
            if p == posicion {
                lw = lw.max(anterior.2);
                lh = lh.max(anterior.3);
            }
        }
        let pantalla = ventana::monitor_en(self.estado.borrow().config.ajustes.monitor);
        let oculta = ui.get_modo() == 0;
        let (lienzo, mut caja) =
            geometria(pantalla, posicion, oculta, (w, h), (lw, lh), px(ui.get_largo_reposo()));
        // Arriba (o abajo), con el hueco del "+" abierto la caja se corre a la derecha (ver
        // `desvio-caja`).
        caja.0 += px(ui.get_desvio_caja());
        ui.set_caja_y((caja.1 - lienzo.1) as f32 / escala);
        self.caja.set(Some(caja));
        if self.lienzo.get() != Some((posicion, lienzo)) {
            self.lienzo.set(Some((posicion, lienzo)));
            // Durante un deslizamiento la ventana la mueve la animación, hacia este destino.
            if self.deslizamiento.get().is_none() {
                plataforma::colocar(lienzo);
            }
        }
    }

    /// Cambia el tamaño de la interfaz. La forma de la barra pasa directamente al nuevo
    /// tamaño: con otra escala cambia su posición dentro de la ventana (en píxeles lógicos) y,
    /// si se animara, se la vería dar un salto hasta su sitio.
    pub(crate) fn cambiar_tamano(&self, tamano: u8) {
        if self.estado.borrow().config.ajustes.tamano == tamano {
            return;
        }
        self.saltar();
        self.modificar_ajustes(|a| a.tamano = tamano);
    }

    /// Hace que los cambios de forma de la barra de este instante se apliquen sin animación.
    pub(crate) fn saltar(&self) {
        if let Some(ui) = self.ui.upgrade() {
            ui.set_saltar(true);
        }
        let ui = self.ui.clone();
        Timer::single_shot(Duration::from_millis(80), move || {
            if let Some(ui) = ui.upgrade() {
                ui.set_saltar(false);
            }
        });
    }

    /// Centro de la barra tal como se ve ahora en la pantalla (la ventana puede estar fuera de
    /// su sitio por un arrastre o un deslizamiento).
    pub(crate) fn centro_visible(&self) -> Option<(i32, i32)> {
        let ((_, destino), caja) = (self.lienzo.get()?, self.caja.get()?);
        let real = plataforma::lienzo();
        Some((real.0 + caja.0 - destino.0 + caja.2 / 2, real.1 + caja.1 - destino.1 + caja.3 / 2))
    }

    /// Cambia el borde o el monitor de la barra y la desliza desde donde se veía hasta su sitio.
    pub(crate) fn cambiar_posicion(&self, cambio: impl FnOnce(&mut config::Ajustes)) {
        let origen = self.centro_visible();
        // La forma de la barra pasa directamente a la del nuevo borde; si se animara, se vería
        // salir desde donde estaba dentro de la ventana (el centro, arriba) y no desde donde
        // se soltó. Lo que se anima es el deslizamiento de la ventana.
        self.saltar();
        self.modificar_ajustes(cambio);
        let animar = self.estado.borrow().config.ajustes.animaciones;
        let (Some(origen), Some((_, destino)), Some(caja)) = (origen, self.lienzo.get(), self.caja.get()) else {
            return;
        };
        let desde = (origen.0 - (caja.0 + caja.2 / 2), origen.1 - (caja.1 + caja.3 / 2));
        if !animar || desde == (0, 0) {
            self.deslizamiento.set(None);
            plataforma::colocar(destino);
            return;
        }
        self.deslizamiento.set(Some(Deslizamiento { desde, inicio: Instant::now() }));
        self.seguir_deslizamiento();
        let ctx = self.clone();
        // Más rápido que el refresco de la pantalla: así se avanza en cada cuadro (la posición
        // sale del tiempo transcurrido, no de cuántas veces se llamó).
        self.animador.start(TimerMode::Repeated, Duration::from_millis(8), move || ctx.seguir_deslizamiento());
    }

    pub(crate) fn seguir_deslizamiento(&self) {
        let (Some(d), Some((_, destino))) = (self.deslizamiento.get(), self.lienzo.get()) else {
            self.animador.stop();
            self.mostrar_tras_mover();
            return;
        };
        let t = d.inicio.elapsed().as_secs_f32() / DURACION_DESLIZAMIENTO.as_secs_f32();
        let resto = 1.0 - suavizar(t.min(1.0));
        let desplazar = |v: i32| (v as f32 * resto).round() as i32;
        plataforma::colocar((destino.0 + desplazar(d.desde.0), destino.1 + desplazar(d.desde.1), destino.2, destino.3));
        if t >= 1.0 {
            self.deslizamiento.set(None);
            self.animador.stop();
            self.mostrar_tras_mover();
        }
    }

    /// Vuelve a mostrar el fondo y los iconos, ocultos mientras se llevaba la barra a otro sitio.
    pub(crate) fn mostrar_tras_mover(&self) {
        // Si ya se está arrastrando otra vez, sigue oculta.
        if self.movimiento.get().is_some() {
            return;
        }
        if let Some(ui) = self.ui.upgrade() {
            ui.set_moviendo(false);
        }
    }

    /// Empieza a mover la barra con el cursor; al soltar se adhiere al borde más cercano.
    pub(crate) fn iniciar_movimiento(&self) {
        if self.movimiento.get().is_some() || !ventana::boton_izquierdo_pulsado() {
            return;
        }
        // Se cierra lo desplegado y se suelta el ratón en la interfaz (sin clics accidentales).
        if self.estado.borrow().modo != Modo::Reposo {
            self.poner_modo(Modo::Reposo);
        }
        plataforma::cancelar_puntero();
        if let Some(ui) = self.ui.upgrade() {
            ui.set_moviendo(true);
        }
        self.movimiento.set(Some(Movimiento {
            cursor: ventana::posicion_cursor(),
            lienzo: plataforma::lienzo(),
        }));
        let ctx = self.clone();
        self.seguidor.start(TimerMode::Repeated, Duration::from_millis(8), move || ctx.seguir_movimiento());
    }

    pub(crate) fn seguir_movimiento(&self) {
        let Some(m) = self.movimiento.get() else {
            self.seguidor.stop();
            return;
        };
        if !ventana::boton_izquierdo_pulsado() {
            self.terminar_movimiento();
            return;
        }
        let (cx, cy) = ventana::posicion_cursor();
        let (x, y, w, h) = m.lienzo;
        plataforma::colocar((x + cx - m.cursor.0, y + cy - m.cursor.1, w, h));
    }

    /// Al soltar: se elige el monitor bajo el cursor y el borde que corresponde a esa zona
    /// (arriba, abajo si se puede, izquierda o derecha), y la barra se desliza hasta él.
    pub(crate) fn terminar_movimiento(&self) {
        self.seguidor.stop();
        let cursor = ventana::posicion_cursor();
        let monitor = ventana::monitor_cercano(cursor);
        let abajo = self.estado.borrow().config.ajustes.sustituir_barra_de_windows;
        let posicion = borde_cercano(monitor, cursor, abajo);
        self.movimiento.set(None);
        self.cambiar_posicion(|a| {
            a.posicion = posicion;
            a.monitor = Some((monitor.0, monitor.1));
        });
        // Solo el personaje hasta que la barra llega a su sitio (si se desliza, al terminar).
        if self.deslizamiento.get().is_none() {
            self.mostrar_tras_mover();
        }
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_recolocar(move || c.recolocar());
    let c = ctx.clone();
    ui.on_mover_barra(move || {
        // Se difiere: no se puede alterar la interfaz desde dentro de su propio evento de ratón.
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.iniciar_movimiento());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_caja_queda_dentro_del_lienzo_y_la_tira_no_se_mueve() {
        let pantalla = (0, 0, 3440, 1440);
        let lienzo = (1010, 600);
        for posicion in 0..4 {
            let (l1, reposo) = geometria(pantalla, posicion, false, (55, 220), lienzo, 220);
            let (l2, abierta) = geometria(pantalla, posicion, false, (808, 475), lienzo, 220);
            let (l3, oculta) = geometria(pantalla, posicion, true, (8, 100), lienzo, 220);
            // El lienzo es el mismo en todos los modos: la ventana no se mueve.
            assert_eq!(l1, l2);
            assert_eq!(l1, l3);
            for caja in [reposo, abierta, oculta] {
                assert!(caja.0 >= l1.0 && caja.1 >= l1.1, "{posicion}: {caja:?} fuera de {l1:?}");
                assert!(caja.0 + caja.2 <= l1.0 + l1.2 && caja.1 + caja.3 <= l1.1 + l1.3);
            }
            if posicion == 1 || posicion == 2 {
                // En los laterales el borde superior se mantiene al desplegar.
                assert_eq!(reposo.1, abierta.1);
            }
            if posicion == 3 {
                // Abajo, el borde inferior.
                assert_eq!(reposo.1 + reposo.3, abierta.1 + abierta.3);
                assert_eq!(abierta.1 + abierta.3, pantalla.3);
            }
        }
    }

    #[test]
    fn el_monitor_secundario_desplaza_la_barra() {
        let segundo = (-1920, 200, 1920, 1080);
        let (lienzo, caja) = geometria(segundo, 1, false, (55, 220), (400, 300), 220);
        assert_eq!(lienzo.0, -1920);
        assert_eq!(caja.0, -1920);
        assert!(caja.1 >= 200 && caja.1 + caja.3 <= 200 + 1080);
    }

    #[test]
    fn se_adhiere_al_borde_de_la_zona() {
        let m = (0, 0, 1920, 1080);
        assert_eq!(borde_cercano(m, (960, 40), false), Posicion::Arriba);
        assert_eq!(borde_cercano(m, (960, 900), false), Posicion::Arriba);
        // Abajo, solo si se puede y en la mitad inferior.
        assert_eq!(borde_cercano(m, (960, 900), true), Posicion::Abajo);
        assert_eq!(borde_cercano(m, (960, 300), true), Posicion::Arriba);
        assert_eq!(borde_cercano(m, (30, 900), true), Posicion::Izquierda);
        assert_eq!(borde_cercano(m, (30, 600), false), Posicion::Izquierda);
        assert_eq!(borde_cercano(m, (1900, 600), false), Posicion::Derecha);
        // Cerca de la esquina superior pero en la zona lateral: va al lateral, no arriba.
        assert_eq!(borde_cercano(m, (400, 60), false), Posicion::Izquierda);
        assert_eq!(borde_cercano(m, (1500, 60), false), Posicion::Derecha);
        // Segundo monitor a la izquierda: la zona se mide respecto a su propio rectángulo.
        let izq = (-1920, 0, 1920, 1080);
        assert_eq!(borde_cercano(izq, (-10, 500), false), Posicion::Derecha);
        assert_eq!(borde_cercano(izq, (-1800, 100), false), Posicion::Izquierda);
    }

    #[test]
    fn el_deslizamiento_empieza_y_termina_en_su_sitio() {
        assert_eq!(suavizar(0.0), 0.0);
        assert_eq!(suavizar(1.0), 1.0);
        assert!(suavizar(0.5) > 0.5, "debe frenar al final, no al principio");
    }

    #[test]
    fn lateral_crece_hacia_abajo() {
        // Pantalla de 1440, barra en reposo de 200 centrada: empieza en 620.
        assert_eq!(y_desplegado(1440, 200, 200), 620);
        // Desplegada (500 de alto) mantiene el borde superior.
        assert_eq!(y_desplegado(1440, 200, 500), 620);
        // Si no cabe hacia abajo, sube lo justo.
        assert_eq!(y_desplegado(1440, 200, 1000), 440);
        // Nunca por encima del borde de la pantalla.
        assert_eq!(y_desplegado(1440, 200, 2000), 0);
    }
}
