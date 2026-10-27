//! Ventana emergente: los menús del clic derecho (también el del icono del área de notificación).

use crate::*;

/// Lo que se hace con la opción elegida en un menú.
pub(crate) type AccionMenu = Box<dyn FnOnce(&Ctx, u32)>;

/// Opción de un menú emergente.
pub(crate) enum Opcion {
    Normal(u32, &'static str),
    Separador,
}

impl Ctx {
    /// Abre un menú junto al cursor; al elegir una opción se llama a `accion` con su id.
    pub(crate) fn mostrar_menu(&self, opciones: &[Opcion], accion: impl FnOnce(&Ctx, u32) + 'static) {
        let Some(em) = self.emergente.upgrade() else { return };
        let opciones: Vec<OpcionMenu> = opciones
            .iter()
            .map(|o| match *o {
                Opcion::Normal(id, texto) => OpcionMenu { id: id as i32, texto: texto.into(), separador: false },
                Opcion::Separador => OpcionMenu { id: 0, texto: "".into(), separador: true },
            })
            .collect();
        // Relleno de 6 px arriba y abajo; filas de 32 px y separadores de 9 px (como en la Emergente).
        let alto = 12.0 + opciones.iter().map(|o| if o.separador { 9.0 } else { 32.0 }).sum::<f32>();
        let mas_largo = opciones.iter().map(|o| o.texto.clone()).max_by_key(|t| t.chars().count()).unwrap_or_default();
        em.set_tipo(0);
        em.set_alto_menu(alto);
        em.set_texto_mas_largo(mas_largo);
        em.set_opciones(modelo(opciones));
        em.set_marcada(-1);
        *self.accion_menu.borrow_mut() = Some(Box::new(accion));
        self.abrir_emergente(&em);
    }

    pub(crate) fn abrir_emergente(&self, em: &Emergente) {
        aplicar_tema(&self.estado.borrow().config.ajustes, em.global::<Tema>());
        plataforma::mostrar_emergente(em.window(), ventana::posicion_cursor(), (em.get_ancho(), em.get_alto()));
        em.invoke_enfocar();
    }

    /// Cierra la emergente. Si la cerró el usuario desde ella (una opción, Esc, un botón), el
    /// foco vuelve a la barra cuando está desplegada, antes de ocultarla, para que Windows no
    /// se lo dé a otro programa.
    pub(crate) fn cerrar_emergente(&self, devolver_foco: bool) {
        self.accion_menu.borrow_mut().take();
        self.ocultos_cerrados();
        let Some(em) = self.emergente.upgrade() else { return };
        if devolver_foco && self.estado.borrow().modo.desplegado() {
            self.enfocar();
        }
        plataforma::ocultar(em.window());
    }

    pub(crate) fn opcion_elegida(&self, id: u32) {
        let accion = self.accion_menu.borrow_mut().take();
        self.cerrar_emergente(true);
        if let Some(accion) = accion {
            accion(self, id);
        }
    }

    /// La emergente perdió el foco: se cierra. Si el usuario hizo clic en otro programa, también
    /// se cierra lo desplegado de la barra.
    pub(crate) fn emergente_desactivada(&self) {
        // Con los iconos ocultos abiertos, el menú de uno de ellos se lleva el foco: siguen
        // abiertos mientras tanto (ver `vigilar_menu_de_icono`).
        if self.estado.borrow().menu_de_icono.is_some() {
            return;
        }
        self.cerrar_emergente(false);
        let desplegado = self.estado.borrow().modo.desplegado();
        if desplegado && ventana::primer_plano_es_de_otro_proceso() {
            self.cerrar();
        }
    }

    /// Clic derecho en el botón "+": añadir una app suelta (lo mismo que el clic normal) o crear un grupo.
    pub(crate) fn menu_mas(&self) {
        const APP: u32 = 1;
        const GRUPO: u32 = 2;
        self.mostrar_menu(&[Opcion::Normal(APP, tr("Añadir aplicación…")), Opcion::Normal(GRUPO, tr("Nuevo grupo"))], |c, id| {
            match id {
                APP => c.abrir_buscador(Destino::Barra),
                GRUPO => c.nuevo_grupo(),
                _ => {}
            }
        });
    }

    /// Clic derecho en la posición `posicion` de la tira: una app abierta o un elemento de la barra.
    pub(crate) fn menu_tira(&self, posicion: usize) {
        match self.ranura(posicion) {
            Some(Ranura::Abierta(k)) => self.menu_abierta(k, None),
            Some(Ranura::Elemento(i)) => match self.abierta_del_elemento(i) {
                Some(k) => self.menu_abierta(k, Some(i)),
                None => self.menu_elemento(i),
            },
            None => {}
        }
    }

    /// Clic derecho en un elemento de la barra que no está abierto.
    pub(crate) fn menu_elemento(&self, indice: usize) {
        const ABRIR: u32 = 1;
        const EDITAR: u32 = 2;
        const QUITAR: u32 = 3;
        let es_grupo = match self.estado.borrow().config.elementos.get(indice) {
            None => return,
            Some(el) => el.grupo().is_some(),
        };
        let opciones = if es_grupo {
            vec![Opcion::Normal(ABRIR, tr("Abrir")), Opcion::Normal(EDITAR, tr("Editar grupo")), Opcion::Separador, Opcion::Normal(QUITAR, tr("Eliminar grupo"))]
        } else {
            vec![Opcion::Normal(ABRIR, tr("Abrir")), Opcion::Separador, Opcion::Normal(QUITAR, tr("Quitar de la barra"))]
        };
        self.mostrar_menu(&opciones, move |c, id| match id {
            ABRIR if es_grupo => c.mostrar_grupo(indice, false),
            ABRIR => c.abrir_elemento(indice),
            EDITAR => c.mostrar_grupo(indice, true),
            QUITAR => c.quitar_elemento(indice),
            _ => {}
        });
    }

    pub(crate) fn menu_icono(&self) {
        const MOSTRAR: u32 = 1;
        const APP: u32 = 2;
        const GRUPO: u32 = 3;
        const CONFIGURACION: u32 = 4;
        const SALIR: u32 = 5;
        let opciones = [
            Opcion::Normal(MOSTRAR, tr("Mostrar u ocultar")),
            Opcion::Separador,
            Opcion::Normal(APP, tr("Añadir aplicación…")),
            Opcion::Normal(GRUPO, tr("Nuevo grupo")),
            Opcion::Normal(CONFIGURACION, tr("Configuración")),
            Opcion::Separador,
            Opcion::Normal(SALIR, tr("Cerrar aplicación")),
        ];
        self.mostrar_menu(&opciones, |c, id| match id {
            MOSTRAR => c.atajo_pulsado(ID_MOSTRAR),
            APP => c.abrir_buscador(Destino::Barra),
            GRUPO => c.nuevo_grupo(),
            CONFIGURACION => c.poner_modo(Modo::Config),
            SALIR => {
                crate::sistema::cerrar_otras_copias();
                let _ = slint::quit_event_loop();
            }
            _ => {}
        });
    }

    pub(crate) fn aviso(&self, aviso: Aviso) {
        match aviso {
            Aviso::ClicIcono => self.atajo_pulsado(ID_MOSTRAR),
            Aviso::MenuIcono => self.menu_icono(),
            Aviso::IconoPerdido => self.poner_icono(true),
            Aviso::OtraInstancia => {
                let desplegado = self.estado.borrow().modo.desplegado();
                if !desplegado {
                    self.atajo_pulsado(ID_MOSTRAR);
                }
            }
            // Llega mientras Windows mueve la ventana: se recoloca después, con la nueva escala.
            // Los iconos se vuelven a pedir al tamaño de la nueva escala.
            Aviso::EscalaCambiada => {
                let ctx = self.clone();
                Timer::single_shot(Duration::ZERO, move || {
                    ctx.refrescar();
                    ctx.recolocar();
                    ctx.poner_icono(false);
                });
            }
            Aviso::Notificacion => self.notificar(),
            // Windows tarda un poco en dejar las pantallas como estaban: la barra se reajusta
            // ahora y otra vez un rato después.
            Aviso::Reanudado | Aviso::PantallasCambiadas => {
                for espera in [Duration::ZERO, REAJUSTE_TARDIO] {
                    let ctx = self.clone();
                    Timer::single_shot(espera, move || ctx.reajustar());
                }
            }
        }
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    // Los menús se difieren: abren su propio bucle de mensajes y no deben hacerlo desde dentro
    // del evento de ratón de la interfaz.
    let c = ctx.clone();
    ui.on_menu_mas(move || {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.menu_mas());
    });
    let c = ctx.clone();
    ui.on_menu_personaje(move || {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.menu_icono());
    });
    let c = ctx.clone();
    ui.on_menu_elemento(move |i| {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.menu_tira(i.max(0) as usize));
    });
}

/// Conecta los eventos de la ventana emergente. Lo que hace se difiere: oculta o cambia la propia
/// ventana desde la que llega el evento.
pub(crate) fn conectar_emergente(emergente: &Emergente, ctx: &Ctx) {
    let diferir = |c: &Ctx, f: Box<dyn FnOnce(&Ctx)>| {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || f(&c));
    };
    let c = ctx.clone();
    emergente.on_elegida(move |id| diferir(&c, Box::new(move |c| c.opcion_elegida(id as u32))));
    let c = ctx.clone();
    emergente.on_cancelar(move || diferir(&c, Box::new(|c| c.cerrar_emergente(true))));
    let c = ctx.clone();
    emergente.on_color_elegido(move |color| diferir(&c, Box::new(move |c| c.color_elegido(color))));
    let c = ctx.clone();
    emergente.on_tono_cambiado(move |tono| {
        if let Some(em) = c.emergente.upgrade() {
            em.set_cuadro(dibujo::cuadro_de_color(tono, LADO_CUADRO, ALTO_CUADRO, 10.0));
        }
    });
    let c = ctx.clone();
    emergente.on_color_cambiado(move |color| c.color_cambiado(color));
    let c = ctx.clone();
    emergente.on_hex_editado(move |texto| c.hex_editado(&texto));
    // Con transparencia, antes de cada dibujo se dice dónde está la forma visible (el desenfoque
    // va debajo): la emergente es toda la ventana.
    let em = emergente.as_weak();
    plataforma::al_presentar(emergente.window(), move || {
        let Some(em) = em.upgrade() else { return };
        let tamano = em.window().size().to_logical(em.window().scale_factor());
        let forma = [pieza_redondeada(tamano.width, tamano.height, RADIO_EMERGENTE), Pieza::default(), Pieza::default(), Pieza::default()];
        plataforma::recortar(em.window(), Some(recorte(em.window(), forma)));
    });
    let c = ctx.clone();
    plataforma::al_activarse_ventana(emergente.window(), move |activa| {
        if !activa {
            diferir(&c, Box::new(|c| c.emergente_desactivada()));
        }
    });
}
