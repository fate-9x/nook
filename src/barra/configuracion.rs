//! Configuración: los colores (con su selector, la paleta y los presets), la apariencia, el
//! personaje, el comportamiento y el icono del área de notificación.

use crate::*;

/// Para qué es el color que se elige en el selector.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum DestinoColor {
    /// Un color nuevo para la fila de fondos o de acentos.
    Paleta { acento: bool },
    /// El color de un accesorio del personaje.
    Accesorio(TipoAccesorio),
    /// El color del circulito que sale en el personaje cuando llega una notificación.
    Aviso,
    /// El fondo o el acento de un preset guardado.
    Preset { indice: usize, acento: bool },
}

/// Tamaño del cuadro de saturación y brillo del selector de color (como en
/// ui/emergente.slint).
pub(crate) const LADO_CUADRO: f32 = 228.0;
pub(crate) const ALTO_CUADRO: f32 = 150.0;

/// Avisos de terceros (créditos y licencias), incluidos en el programa.
pub(crate) const AVISOS: &str = include_str!("../../AVISOS_DE_TERCEROS.md");

/// "#RRGGBB" (o "RRGGBB") → color, si el texto es un código válido.
pub(crate) fn leer_hex(texto: &str) -> Option<Color> {
    let valor = texto.trim().trim_start_matches('#');
    if valor.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(valor, 16).ok()?;
    Some(Color::from_rgb_u8((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// "#RRGGBB" → color (negro si el texto no es válido).
pub(crate) fn color(hex: &str) -> Color {
    let valor = hex.trim_start_matches('#');
    match u32::from_str_radix(valor, 16) {
        Ok(v) if valor.len() == 6 => Color::from_rgb_u8((v >> 16) as u8, (v >> 8) as u8, v as u8),
        _ => Color::from_rgb_u8(0x11, 0x13, 0x18),
    }
}

pub(crate) fn hex(c: Color) -> String {
    format!("#{:02X}{:02X}{:02X}", c.red(), c.green(), c.blue())
}

/// Si el fondo es claro, el texto debe ser oscuro.
pub(crate) fn es_claro(c: Color) -> bool {
    0.2126 * c.red() as f32 + 0.7152 * c.green() as f32 + 0.0722 * c.blue() as f32 > 150.0
}

/// Pasa los colores elegidos a la interfaz (la barra y la emergente tienen cada una su `Tema`).
pub(crate) fn aplicar_tema(ajustes: &Ajustes, tema: Tema<'_>) {
    let (fondo, acento) = (color(&ajustes.color_fondo), color(&ajustes.color_acento));
    tema.set_fondo(fondo);
    tema.set_acento(acento);
    tema.set_claro(es_claro(fondo));
    tema.set_acento_claro(es_claro(acento));
    tema.set_opacidad(ajustes.opacidad_fondo());
    tema.set_fuente(idioma::fuente().into());
    tema.set_pro(PRO);
}

impl Ctx {
    /// Pasa a la interfaz los ajustes: colores, apariencia, comportamiento, atajos y personaje.
    pub(crate) fn refrescar_ajustes(&self, ui: &Barra, e: &Estado) {
        let ajustes = &e.config.ajustes;
        aplicar_tema(ajustes, ui.global::<Tema>());
        self.refrescar_tema_addon(ajustes);

        ui.set_posicion(match ajustes.borde() {
            Posicion::Arriba => 0,
            Posicion::Izquierda => 1,
            Posicion::Derecha => 2,
            Posicion::Abajo => 3,
        });
        ui.set_mostrar_hora(ajustes.mostrar_hora);
        // La fecha y, para elegir su formato, cómo se ve hoy en cada uno.
        let formato = FormatoFecha::TODOS.iter().position(|&f| f == ajustes.fecha).unwrap_or(0);
        ui.set_formato_fecha(formato as i32);
        let hoy = Local::now();
        let ejemplos = FormatoFecha::TODOS.map(|f| match f {
            FormatoFecha::Ninguna => SharedString::from(tr("Sin fecha")),
            f => fecha::formatear(hoy.date_naive(), f, idioma::codigo_actual()).0.into(),
        });
        ui.set_ejemplos_fecha(modelo(ejemplos.to_vec()));
        actualizar_reloj(ui, hoy, ajustes.fecha);
        ui.set_boton_sonido(ajustes.boton_sonido);
        ui.set_boton_microfono(ajustes.boton_microfono);
        ui.set_boton_energia(ajustes.boton_energia);
        ui.set_boton_bateria(ajustes.boton_bateria);
        ui.set_boton_buscar(ajustes.boton_buscar);
        ui.set_tecla_windows_abre_buscador(ajustes.tecla_windows_abre_buscador);
        ui.set_usar_boveda(ajustes.usar_boveda);
        ui.set_reproductor_en_barra(ajustes.reproductor_en_barra);
        ui.set_animaciones(ajustes.animaciones);
        ui.set_ocultar_en_reposo(ajustes.ocultar_en_reposo);
        ui.set_ocultar_en_pantalla_completa(ajustes.ocultar_en_pantalla_completa);
        ui.set_apps_abiertas(ajustes.apps_abiertas);
        ui.set_sustituir_barra_de_windows(ajustes.sustituir_barra_de_windows);
        ui.set_iniciar_con_windows(e.inicia_con_windows);
        ui.set_atajo_mostrar(texto_atajo(ajustes.atajo_mostrar.as_ref()).into());
        ui.set_atajo_mostrar_ok(e.mostrar_ok);
        ui.set_atajo_mostrar_definido(ajustes.atajo_mostrar.is_some());
        ui.set_atajo_boveda(texto_atajo(ajustes.atajo_boveda.as_ref()).into());
        ui.set_atajo_boveda_ok(e.boveda_ok);
        ui.set_atajo_boveda_definido(ajustes.atajo_boveda.is_some());
        ui.set_atajo_microfono(texto_atajo(ajustes.atajo_microfono.as_ref()).into());
        ui.set_atajo_microfono_ok(e.microfono_ok);
        ui.set_atajo_microfono_definido(ajustes.atajo_microfono.is_some());
        ui.set_atajo_buscar(texto_atajo(ajustes.atajo_buscar.as_ref()).into());
        ui.set_atajo_buscar_ok(e.buscar_ok);
        ui.set_atajo_buscar_definido(ajustes.atajo_buscar.is_some());
        ui.set_grabando_atajo(e.grabando.unwrap_or(-2));
        ui.set_fondos(modelo(ajustes.paleta(false).iter().map(|c| color(c)).collect()));
        ui.set_acentos(modelo(ajustes.paleta(true).iter().map(|c| color(c)).collect()));
        ui.set_colores_base(FONDOS.len() as i32);
        ui.set_transparencia(ajustes.transparencia);
        ui.set_opacidad_elegida(ajustes.opacidad.clamp(config::OPACIDAD_MINIMA, 100) as f32 / 100.0);
        plataforma::poner_desenfoque(ajustes.transparencia);
        ui.set_tamano_provisional(ajustes.tamano_interfaz());
        plataforma::poner_tamano(ajustes.tamano_interfaz());
        let presets = ajustes.presets.iter().map(|p| PresetItem { fondo: color(&p.fondo), acento: color(&p.acento) });
        ui.set_presets(modelo(presets.collect()));
        ui.set_preset_guardado(ajustes.preset_guardado());
        let accesorios = TipoAccesorio::TODOS.iter().map(|&tipo| {
            let a = ajustes.personaje.accesorio(tipo);
            AccesorioItem { nombre: tipo.nombre().into(), puesto: a.puesto, color: color(&a.color) }
        });
        ui.set_accesorios(modelo(accesorios.collect()));
        ui.set_colores_accesorios(modelo(ajustes.personaje.paleta().iter().map(|c| color(c)).collect()));
        ui.set_colores_accesorios_base(COLORES_ACCESORIOS.len() as i32);
        ui.set_aviso_automatico(ajustes.personaje.color_aviso.is_none());
        ui.set_color_aviso(color(ajustes.personaje.color_aviso.as_deref().unwrap_or(config::COLOR_AVISO_INICIAL)));
        self.refrescar_personajes(ui, &ajustes.personaje.figura);
        let idioma = config::Idioma::TODOS.iter().position(|&i| i == ajustes.idioma).unwrap_or(0);
        ui.set_idioma(idioma as i32);
    }

    /// Abre el selector de color: para la fila de fondos o de acentos (su "+") parte del color
    /// elegido ahora; para un preset, del color que tiene.
    pub(crate) fn abrir_selector_color(&self, destino: DestinoColor) {
        let Some(em) = self.emergente.upgrade() else { return };
        let (actual, titulo) = {
            let a = &self.estado.borrow().config.ajustes;
            match destino {
                DestinoColor::Paleta { acento: true } => (a.color_acento.clone(), tr("NUEVO COLOR DE ACENTO").to_string()),
                DestinoColor::Paleta { acento: false } => (a.color_fondo.clone(), tr("NUEVO COLOR DE FONDO").to_string()),
                DestinoColor::Preset { indice, acento } => {
                    let Some(p) = a.presets.get(indice) else { return };
                    if acento { (p.acento.clone(), tr("ACENTO DEL PRESET").into()) } else { (p.fondo.clone(), tr("FONDO DEL PRESET").into()) }
                }
                DestinoColor::Accesorio(tipo) => {
                    (a.personaje.accesorio(tipo).color, tr_con("COLOR DE {}", &[&tipo.nombre().to_uppercase()]))
                }
                DestinoColor::Aviso => {
                    let actual = a.personaje.color_aviso.clone().unwrap_or_else(|| config::COLOR_AVISO_INICIAL.into());
                    (actual, tr("COLOR DEL AVISO").into())
                }
            }
        };
        let actual = color(&actual);
        let hsv = actual.to_hsva();
        self.destino_color.set(destino);
        em.set_tipo(1);
        em.set_titulo_color(titulo.into());
        em.set_tono(hsv.hue);
        em.set_saturacion(hsv.saturation);
        em.set_valor(hsv.value);
        em.set_texto_hex(hex(actual).into());
        em.set_franja(dibujo::franja_de_tonos(LADO_CUADRO, 14.0));
        em.set_cuadro(dibujo::cuadro_de_color(hsv.hue, LADO_CUADRO, ALTO_CUADRO, 10.0));
        self.abrir_emergente(&em);
    }

    /// El usuario escribió un código de color: si es válido, el selector se mueve a él.
    pub(crate) fn hex_editado(&self, texto: &str) {
        let (Some(em), Some(c)) = (self.emergente.upgrade(), leer_hex(texto)) else { return };
        let hsv = c.to_hsva();
        em.set_tono(hsv.hue);
        em.set_saturacion(hsv.saturation);
        em.set_valor(hsv.value);
        em.set_cuadro(dibujo::cuadro_de_color(hsv.hue, LADO_CUADRO, ALTO_CUADRO, 10.0));
    }

    /// El color del selector cambió: se muestra su código, salvo que el texto escrito ya sea
    /// ese color (al pasar por el tono, el brillo y la saturación puede variar en una unidad).
    pub(crate) fn color_cambiado(&self, c: Color) {
        let Some(em) = self.emergente.upgrade() else { return };
        let escrito = leer_hex(&em.get_texto_hex());
        let parecido = escrito.is_some_and(|e| {
            (e.red() as i32 - c.red() as i32).abs() <= 2
                && (e.green() as i32 - c.green() as i32).abs() <= 2
                && (e.blue() as i32 - c.blue() as i32).abs() <= 2
        });
        if !parecido {
            em.set_texto_hex(hex(c).into());
        }
    }

    /// El color elegido en el selector: se añade a su fila y se aplica, o se pone en el preset.
    pub(crate) fn color_elegido(&self, c: Color) {
        let destino = self.destino_color.get();
        self.cerrar_emergente(true);
        match destino {
            DestinoColor::Paleta { acento } => {
                self.modificar_ajustes(|a| a.poner_color(acento, hex(c)));
            }
            DestinoColor::Preset { indice, acento } => self.modificar_ajustes(|a| a.editar_preset(indice, acento, hex(c))),
            DestinoColor::Accesorio(tipo) => self.modificar_ajustes(|a| a.personaje.poner_color(tipo, hex(c))),
            DestinoColor::Aviso => self.modificar_ajustes(|a| a.personaje.poner_color_aviso(hex(c))),
        }
    }

    /// Clic derecho en un color propio de la fila de los accesorios.
    pub(crate) fn menu_color_accesorio(&self, indice: usize) {
        const QUITAR: u32 = 1;
        self.mostrar_menu(&[Opcion::Normal(QUITAR, tr("Quitar color"))], move |c, id| {
            if id != QUITAR {
                return;
            }
            c.modificar_ajustes(|a| {
                if let Some(color) = a.personaje.paleta().get(indice).cloned() {
                    a.personaje.quitar_color(&color);
                }
            });
        });
    }

    /// Clic derecho en un color propio de la paleta.
    pub(crate) fn menu_color(&self, acento: bool, indice: usize) {
        const QUITAR: u32 = 1;
        self.mostrar_menu(&[Opcion::Normal(QUITAR, tr("Quitar color"))], move |c, id| {
            if id != QUITAR {
                return;
            }
            c.modificar_ajustes(|a| {
                if let Some(color) = a.paleta(acento).get(indice).cloned() {
                    a.quitar_color(acento, &color);
                }
            });
        });
    }

    pub(crate) fn aplicar_preset(&self, i: usize) {
        let Some(preset) = self.estado.borrow().config.ajustes.presets.get(i).cloned() else { return };
        self.modificar_ajustes(|a| a.aplicar_preset(&preset));
    }

    /// Clic derecho en un preset: aplicarlo, editar sus colores o eliminarlo.
    pub(crate) fn menu_preset(&self, i: usize) {
        const APLICAR: u32 = 1;
        const ELIMINAR: u32 = 2;
        const FONDO: u32 = 3;
        const ACENTO: u32 = 4;
        const ACTUALES: u32 = 5;
        let opciones = [
            Opcion::Normal(APLICAR, tr("Aplicar")),
            Opcion::Separador,
            Opcion::Normal(FONDO, tr("Cambiar el fondo…")),
            Opcion::Normal(ACENTO, tr("Cambiar el acento…")),
            Opcion::Normal(ACTUALES, tr("Guardar aquí los colores actuales")),
            Opcion::Separador,
            Opcion::Normal(ELIMINAR, tr("Eliminar preset")),
        ];
        self.mostrar_menu(&opciones, move |c, id| match id {
            APLICAR => c.aplicar_preset(i),
            // El selector se abre después: el menú se está cerrando en este momento.
            FONDO | ACENTO => {
                let c = c.clone();
                Timer::single_shot(Duration::ZERO, move || {
                    c.abrir_selector_color(DestinoColor::Preset { indice: i, acento: id == ACENTO })
                });
            }
            ACTUALES => c.modificar_ajustes(|a| a.reemplazar_preset(i)),
            ELIMINAR => c.modificar_ajustes(|a| {
                if i < a.presets.len() {
                    a.presets.remove(i);
                }
            }),
            _ => {}
        });
    }

    /// Pone (o actualiza) el icono del área de notificación: el personaje como se ve en la barra,
    /// con su figura, el color de acento y los accesorios. Solo se vuelve a dibujar si cambió algo
    /// de eso o el tamaño del icono, o con `forzar` (el Explorador se reinició y lo perdió).
    pub(crate) fn poner_icono(&self, forzar: bool) {
        let Some(hwnd) = plataforma::hwnd() else { return };
        let figura = self.figura_elegida();
        let aspecto = aspecto_de(&self.estado.borrow().config.ajustes);
        let clave = (figura, aspecto, bandeja::lado(hwnd));
        if !forzar && self.icono_puesto.borrow().as_ref() == Some(&clave) {
            return;
        }
        let (figura, aspecto, lado) = &clave;
        bandeja::poner(hwnd, &icono::pixeles(figura, aspecto, *lado), *lado);
        *self.icono_puesto.borrow_mut() = Some(clave);
    }

    pub(crate) fn modificar_ajustes(&self, cambio: impl FnOnce(&mut config::Ajustes)) {
        {
            let ajustes = &mut self.estado.borrow_mut().config.ajustes;
            cambio(ajustes);
            if !PRO {
                ajustes.quitar_funciones_pro();
            }
        }
        self.guardar();
        self.refrescar();
        self.recolocar();
        self.vigilar();
        // Si cambió el personaje o el color de acento, también el icono.
        self.poner_icono(false);
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    ui.on_ver_licencias(|| {
        // Se abren como texto con el programa que tenga Windows para ello (el Bloc de notas).
        let ruta = std::env::temp_dir().join("nook-avisos-de-terceros.txt");
        let abierto = std::fs::write(&ruta, AVISOS).map_err(|e| e.to_string()).and_then(|_| sistema::lanzar(&ruta.to_string_lossy()));
        if let Err(e) = abierto {
            eprintln!("No se pudieron abrir las licencias: {e}");
        }
    });
    let c = ctx.clone();
    ui.on_alternar_config(move || {
        if c.estado.borrow().modo == Modo::Config {
            c.cerrar();
        } else {
            c.poner_modo(Modo::Config);
        }
    });
    // Configuración.
    let c = ctx.clone();
    ui.on_elegir_fondo(move |color| c.modificar_ajustes(|a| a.poner_color(false, hex(color))));
    let c = ctx.clone();
    ui.on_elegir_acento(move |color| c.modificar_ajustes(|a| a.poner_color(true, hex(color))));
    let c = ctx.clone();
    ui.on_elegir_posicion(move |p| {
        c.cambiar_posicion(|a| {
            a.posicion = match p {
                1 => Posicion::Izquierda,
                2 => Posicion::Derecha,
                3 => Posicion::Abajo,
                _ => Posicion::Arriba,
            }
        })
    });
    let c = ctx.clone();
    ui.on_alternar_ocultar(move || c.modificar_ajustes(|a| a.ocultar_en_reposo = !a.ocultar_en_reposo));
    let c = ctx.clone();
    ui.on_alternar_sustituir_barra_de_windows(move || {
        // Con la barra abajo, al dejar de sustituir la de Windows se va arriba (y al revés):
        // deslizándose, como al cambiarla de sitio.
        c.cambiar_posicion(|a| a.sustituir_barra_de_windows = !a.sustituir_barra_de_windows);
        c.aplicar_barra_de_windows();
        c.aplicar_area_de_notificacion();
    });
    let c = ctx.clone();
    ui.on_alternar_tecla_windows(move || {
        c.modificar_ajustes(|a| a.tecla_windows_abre_buscador = !a.tecla_windows_abre_buscador);
        let activa = c.estado.borrow().config.ajustes.tecla_windows_abre_buscador;
        tecla_windows::activar(activa);
        // Al activarla, Windows pide permiso de administrador para el ayudante (ver tecla_windows).
        if activa {
            tecla_windows::preparar_ayudante();
        }
    });
    let c = ctx.clone();
    ui.on_alternar_pantalla_completa(move || {
        c.modificar_ajustes(|a| a.ocultar_en_pantalla_completa = !a.ocultar_en_pantalla_completa);
        c.sondear_pantalla_completa();
    });
    let c = ctx.clone();
    ui.on_alternar_animaciones(move || c.modificar_ajustes(|a| a.animaciones = !a.animaciones));
    let c = ctx.clone();
    ui.on_alternar_transparencia(move || c.modificar_ajustes(|a| a.transparencia = !a.transparencia));
    let c = ctx.clone();
    ui.on_elegir_tamano(move |v| {
        let tamano = (v * 100.0).round().clamp(config::TAMANO_MINIMO as f32, config::TAMANO_MAXIMO as f32) as u8;
        c.cambiar_tamano(tamano);
    });
    let c = ctx.clone();
    ui.on_elegir_opacidad(move |v| {
        let opacidad = (v * 100.0).round().clamp(config::OPACIDAD_MINIMA as f32, 100.0) as u8;
        if c.estado.borrow().config.ajustes.opacidad != opacidad {
            c.modificar_ajustes(|a| a.opacidad = opacidad);
        }
    });
    // La emergente se abre en diferido: cambia el foco y no debe hacerse desde dentro del evento
    // de ratón de la interfaz.
    let c = ctx.clone();
    ui.on_agregar_color(move |fila| {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.abrir_selector_color(DestinoColor::Paleta { acento: fila == 1 }));
    });
    let c = ctx.clone();
    ui.on_menu_color(move |fila, i| {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.menu_color(fila == 1, i as usize));
    });
    let c = ctx.clone();
    ui.on_guardar_preset(move || c.modificar_ajustes(|a| a.guardar_preset()));
    // Personalización del personaje: los accesorios, cada uno su posición en
    // `TipoAccesorio::TODOS` (la figura, en barra/personaje.rs).
    let c = ctx.clone();
    ui.on_elegir_idioma(move |i| {
        let Some(&idioma) = config::Idioma::TODOS.get(i as usize) else { return };
        // Los textos de la interfaz cambian solos; los que vienen de Rust se vuelven a pasar al
        // refrescar.
        idioma::poner(idioma);
        c.modificar_ajustes(|a| a.idioma = idioma);
    });
    let accesorio = |i: i32| TipoAccesorio::TODOS.get(i as usize).copied();
    let c = ctx.clone();
    ui.on_alternar_accesorio(move |i| {
        if let Some(tipo) = accesorio(i) {
            c.modificar_ajustes(|a| a.personaje.alternar(tipo));
        }
    });
    let c = ctx.clone();
    ui.on_elegir_color_accesorio(move |i, color| {
        if let Some(tipo) = accesorio(i) {
            c.modificar_ajustes(|a| a.personaje.poner_color(tipo, hex(color)));
        }
    });
    let c = ctx.clone();
    ui.on_agregar_color_accesorio(move |i| {
        let Some(tipo) = accesorio(i) else { return };
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.abrir_selector_color(DestinoColor::Accesorio(tipo)));
    });
    // El circulito de aviso: sin elegir color usa el de los ojos; al quitar "automático" empieza
    // en rojo, el de las notificaciones de siempre.
    let c = ctx.clone();
    ui.on_alternar_aviso_automatico(move || {
        c.modificar_ajustes(|a| {
            a.personaje.color_aviso = match a.personaje.color_aviso {
                Some(_) => None,
                None => Some(config::COLOR_AVISO_INICIAL.into()),
            };
        });
    });
    let c = ctx.clone();
    ui.on_elegir_color_aviso(move |color| c.modificar_ajustes(|a| a.personaje.poner_color_aviso(hex(color))));
    let c = ctx.clone();
    ui.on_agregar_color_aviso(move || {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.abrir_selector_color(DestinoColor::Aviso));
    });
    let c = ctx.clone();
    ui.on_probar_aviso(move || c.notificar());
    let c = ctx.clone();
    ui.on_menu_color_accesorio(move |i| {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.menu_color_accesorio(i as usize));
    });
    let c = ctx.clone();
    ui.on_aplicar_preset(move |i| c.aplicar_preset(i as usize));
    let c = ctx.clone();
    ui.on_menu_preset(move |i| {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.menu_preset(i as usize));
    });
    let c = ctx.clone();
    ui.on_alternar_hora(move || c.modificar_ajustes(|a| a.mostrar_hora = !a.mostrar_hora));
    let c = ctx.clone();
    ui.on_elegir_formato_fecha(move |i| {
        if let Some(&formato) = FormatoFecha::TODOS.get(i as usize) {
            c.modificar_ajustes(|a| a.fecha = formato);
        }
    });
    let c = ctx.clone();
    ui.on_alternar_usar_boveda(move || c.alternar_usar_boveda());
    let c = ctx.clone();
    ui.on_alternar_iniciar(move || {
        let actual = c.estado.borrow().inicia_con_windows;
        if let Err(e) = sistema::iniciar_con_windows(!actual) {
            eprintln!("No se pudo cambiar el inicio con Windows: {e}");
        }
        c.estado.borrow_mut().inicia_con_windows = sistema::inicia_con_windows();
        c.refrescar();
    });
    ui.on_salir(|| {
        let _ = slint::quit_event_loop();
    });
    ui.on_actualizar({
        let ui = ui.as_weak();
        move || actualizar(ui.clone())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_ida_y_vuelta() {
        for c in FONDOS.iter().chain(config::ACENTOS.iter()) {
            assert_eq!(hex(color(c)), *c);
        }
    }

    #[test]
    fn leer_codigos_de_color() {
        assert_eq!(leer_hex(" #2e8b57 ").map(hex).as_deref(), Some("#2E8B57"));
        assert_eq!(leer_hex("4A6CF7").map(hex).as_deref(), Some("#4A6CF7"));
        assert!(leer_hex("#12345").is_none());
        assert!(leer_hex("#GGGGGG").is_none());
    }

    #[test]
    fn color_invalido_usa_el_negro() {
        assert_eq!(hex(color("rojo")), "#111318");
    }

    /// Luminancia relativa (WCAG) y contraste entre dos colores.
    fn contraste(a: &str, b: &str) -> f32 {
        let luminancia = |hex_: &str| {
            let c = color(hex_);
            let canal = |v: u8| {
                let v = v as f32 / 255.0;
                if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
            };
            0.2126 * canal(c.red()) + 0.7152 * canal(c.green()) + 0.0722 * canal(c.blue())
        };
        let (la, lb) = (luminancia(a), luminancia(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[test]
    fn los_acentos_se_leen_sobre_todos_los_fondos() {
        for acento in config::ACENTOS {
            assert!(contraste(acento, "#FFFFFF") >= 3.0, "texto blanco sobre {acento}");
            for fondo in FONDOS {
                assert!(contraste(acento, fondo) >= 3.0, "{acento} sobre {fondo}: {}", contraste(acento, fondo));
            }
        }
    }

    #[test]
    fn fondos_claros_y_oscuros() {
        assert!(!es_claro(color("#00125C")));
        assert!(!es_claro(color("#111318")));
        assert!(es_claro(color("#F4F5FA")));
    }
}
