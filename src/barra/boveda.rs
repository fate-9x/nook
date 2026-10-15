//! Bóveda de textos: guardar, copiar, etiquetar y agrupar textos, y la cuenta de GitHub con la que
//! se sincroniza.

use crate::*;

/// Para qué es el campo de arriba de la bóveda.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum EntradaBoveda {
    /// Guardar un texto nuevo en la vista que se ve.
    Texto,
    /// Poner la etiqueta del texto con esa posición en la vista.
    Etiqueta(usize),
    /// Poner nombre al grupo con esa posición en la vista de arriba.
    NombreGrupo(usize),
}

/// Lo que se ve de un texto de la bóveda en su tarjeta: sus primeras líneas con contenido,
/// recortadas (la tarjeta corta lo que no quepa con "…").
pub(crate) fn vista_de_texto(texto: &str) -> String {
    const LINEAS: usize = 4;
    const LARGO: usize = 200;
    texto
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(LINEAS)
        .map(|l| l.chars().take(LARGO).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Tarjetas de la vista `vista` de la bóveda (None = la de arriba).
pub(crate) fn tarjetas_boveda(boveda: &boveda::Boveda, vista: Option<usize>) -> Vec<ItemBoveda> {
    let texto = |t: &boveda::Texto| ItemBoveda {
        es_grupo: false,
        titulo: vista_de_texto(&t.texto).into(),
        detalle: "".into(),
        etiqueta: t.etiqueta.clone().unwrap_or_default().into(),
    };
    match vista {
        Some(g) => boveda.grupo(g).map_or(Vec::new(), |g| g.textos.iter().map(texto).collect()),
        None => boveda
            .elementos
            .iter()
            .map(|e| match e {
                boveda::Elemento::Texto(t) => texto(t),
                boveda::Elemento::Grupo(g) => ItemBoveda {
                    es_grupo: true,
                    titulo: g.nombre.as_str().into(),
                    detalle: if g.textos.len() == 1 { "1 texto".into() } else { format!("{} textos", g.textos.len()).into() },
                    etiqueta: "".into(),
                },
            })
            .collect(),
    }
}

impl Ctx {
    /// Pasa a la interfaz la vista de la bóveda que se ve y el estado de la cuenta.
    pub(crate) fn refrescar_boveda(&self, ui: &Barra, e: &mut Estado) {
        // Bóveda: si el grupo que se veía ya no existe, se vuelve a la vista de arriba.
        if e.grupo_boveda.is_some_and(|g| e.config.boveda.grupo(g).is_none()) {
            e.grupo_boveda = None;
            e.entrada_boveda = EntradaBoveda::Texto;
        }
        let vista = e.grupo_boveda;
        ui.set_boveda(modelo(tarjetas_boveda(&e.config.boveda, vista)));
        ui.set_en_grupo_boveda(vista.is_some());
        ui.set_nombre_grupo_boveda(vista.and_then(|g| e.config.boveda.grupo(g)).map_or(String::new(), |g| g.nombre.clone()).into());
        let boveda = &e.config.boveda;
        let (entrada, pista) = match e.entrada_boveda {
            EntradaBoveda::Texto if vista.is_some() => (0, tr("Escribe o pega un texto para guardarlo en este grupo").into()),
            EntradaBoveda::Texto => (0, tr("Escribe o pega un texto y pulsa Intro para guardarlo").into()),
            EntradaBoveda::Etiqueta(i) => match boveda.texto(vista, i).and_then(|t| t.etiqueta.as_deref()) {
                Some(actual) => (1, tr_con("Nueva etiqueta (ahora «{}»)", &[actual])),
                None => (1, tr("Etiqueta para el texto").into()),
            },
            EntradaBoveda::NombreGrupo(g) => match boveda.grupo(g) {
                Some(grupo) if grupo.nombre != boveda::nombre_grupo_nuevo() => (2, tr_con("Nuevo nombre (ahora «{}»)", &[&grupo.nombre])),
                _ => (2, tr("Nombre del grupo").to_string()),
            },
        };
        ui.set_entrada_boveda(entrada);
        ui.set_pista_entrada_boveda(pista.into());
        let ultimo = e.config.boveda.largo(vista).saturating_sub(1) as i32;
        if ui.get_seleccion_boveda() > ultimo {
            ui.set_seleccion_boveda(ultimo);
        }

        let usuario = self.cuenta.usuario();
        ui.set_cuenta_estado(match (&usuario, &e.codigo_cuenta) {
            (Some(_), _) => 3,
            (None, Some(_)) => 2,
            (None, None) if e.conectando_cuenta => 1,
            _ => 0,
        });
        ui.set_cuenta_usuario(usuario.unwrap_or_default().into());
        ui.set_cuenta_codigo(e.codigo_cuenta.clone().unwrap_or_default().into());
        ui.set_cuenta_error(e.error_cuenta.as_str().into());
    }

    /// Activa o desactiva la bóveda: su botón en la tira y su atajo.
    pub(crate) fn alternar_usar_boveda(&self) {
        self.modificar_ajustes(|a| a.usar_boveda = !a.usar_boveda);
        self.registrar_atajos();
        self.refrescar();
    }

    /// Abre la bóveda (en la vista de arriba), recordando qué ventana tenía el foco para
    /// devolvérselo al copiar.
    pub(crate) fn abrir_boveda(&self) {
        if !self.estado.borrow().config.ajustes.usar_boveda {
            return;
        }
        if ventana::primer_plano_es_de_otro_proceso() {
            self.ventana_previa.set(Some(ventana::en_primer_plano()));
        }
        {
            let mut e = self.estado.borrow_mut();
            e.grupo_boveda = None;
            e.entrada_boveda = EntradaBoveda::Texto;
        }
        self.limpiar_campo_boveda(0);
        self.poner_modo(Modo::Boveda);
        // Por si se cambió en otro ordenador.
        self.cuenta.descargar();
    }

    pub(crate) fn alternar_boveda(&self) {
        if self.estado.borrow().modo == Modo::Boveda {
            self.cerrar();
        } else {
            self.abrir_boveda();
        }
    }

    /// Botón de la cuenta en la configuración: conecta, cancela o desconecta según el caso.
    pub(crate) fn boton_cuenta(&self) {
        {
            let mut e = self.estado.borrow_mut();
            e.error_cuenta.clear();
            if self.cuenta.usuario().is_some() {
                self.cuenta.desconectar();
            } else if e.conectando_cuenta {
                self.cuenta.cancelar();
                e.conectando_cuenta = false;
                e.codigo_cuenta = None;
            } else {
                self.cuenta.conectar();
                e.conectando_cuenta = true;
            }
        }
        self.refrescar();
    }

    /// Recoge lo que avisó el hilo de la cuenta.
    pub(crate) fn novedades_cuenta(&self) {
        for evento in self.cuenta.eventos() {
            match evento {
                cuenta::Evento::Codigo { codigo, url } => {
                    // Puede llegar justo después de cancelar.
                    if !self.estado.borrow().conectando_cuenta {
                        continue;
                    }
                    // Se copia el código y se abre la página de GitHub donde hay que pegarlo.
                    portapapeles::poner(&codigo);
                    if let Err(e) = sistema::lanzar(&url) {
                        eprintln!("No se pudo abrir GitHub: {e}");
                    }
                    self.estado.borrow_mut().codigo_cuenta = Some(codigo);
                }
                cuenta::Evento::Conectada { remota } => {
                    {
                        let mut e = self.estado.borrow_mut();
                        e.conectando_cuenta = false;
                        e.codigo_cuenta = None;
                        e.error_cuenta.clear();
                        if let Some(remota) = remota {
                            e.config.boveda.fusionar(remota);
                        }
                    }
                    // Guarda y sube la bóveda juntada.
                    self.guardar();
                }
                cuenta::Evento::Descargada(remota) => {
                    let cambia = self.estado.borrow().config.boveda != remota;
                    if cambia {
                        self.estado.borrow_mut().config.boveda = remota;
                        self.guardar();
                    }
                }
                cuenta::Evento::Error(error) => {
                    let mut e = self.estado.borrow_mut();
                    e.conectando_cuenta = false;
                    e.codigo_cuenta = None;
                    e.error_cuenta = error;
                }
                cuenta::Evento::AlDia => self.estado.borrow_mut().error_cuenta.clear(),
            }
        }
        self.refrescar();
    }

    /// Vacía el campo de la bóveda y marca la tarjeta `seleccion`.
    pub(crate) fn limpiar_campo_boveda(&self, seleccion: i32) {
        if let Some(ui) = self.ui.upgrade() {
            ui.set_texto_boveda("".into());
            ui.set_seleccion_boveda(seleccion);
        }
    }

    /// Clic (o Intro) en una tarjeta: un texto se copia; en un grupo se entra.
    pub(crate) fn elegir_en_boveda(&self, i: usize) {
        let (vista, es_grupo) = {
            let e = self.estado.borrow();
            let vista = e.grupo_boveda;
            (vista, vista.is_none() && e.config.boveda.grupo(i).is_some())
        };
        if es_grupo {
            self.ver_grupo_boveda(Some(i));
        } else {
            self.copiar_de_boveda(vista, i);
        }
    }

    /// Pasa a ver el grupo `g` de la bóveda (None = la vista de arriba).
    pub(crate) fn ver_grupo_boveda(&self, g: Option<usize>) {
        let anterior = {
            let mut e = self.estado.borrow_mut();
            let anterior = e.grupo_boveda;
            e.grupo_boveda = g;
            e.entrada_boveda = EntradaBoveda::Texto;
            anterior
        };
        // Al volver, queda marcado el grupo del que se sale.
        let seleccion = if g.is_none() { anterior.unwrap_or(0) as i32 } else { 0 };
        self.limpiar_campo_boveda(seleccion);
        self.refrescar();
        if let Some(ui) = self.ui.upgrade() {
            ui.invoke_mostrar_seleccion_boveda();
        }
    }

    /// Copia el texto `i` de la vista, cierra la barra y devuelve el foco a la ventana en la
    /// que se estaba, para pegarlo.
    pub(crate) fn copiar_de_boveda(&self, vista: Option<usize>, i: usize) {
        let Some(texto) = self.estado.borrow().config.boveda.texto(vista, i).map(|t| t.texto.clone()) else { return };
        if !portapapeles::poner(&texto) {
            eprintln!("No se pudo copiar el texto: otro programa está usando el portapapeles");
            return;
        }
        self.cerrar();
        if let Some(ventana) = self.ventana_previa.take() {
            ventana::activar(ventana);
        }
    }

    /// Intro en el campo de arriba: guarda el texto nuevo, la etiqueta o el nombre del grupo.
    pub(crate) fn guardar_en_boveda(&self, texto: &str) {
        let (cambiado, seleccion) = {
            let mut e = self.estado.borrow_mut();
            let vista = e.grupo_boveda;
            let entrada = std::mem::replace(&mut e.entrada_boveda, EntradaBoveda::Texto);
            match entrada {
                EntradaBoveda::Texto => (e.config.boveda.guardar(vista, texto), 0),
                // Vacía, se deja como estaba (se quita con "Quitar etiqueta" del menú).
                EntradaBoveda::Etiqueta(i) if texto.trim().is_empty() => (false, i),
                EntradaBoveda::Etiqueta(i) => (e.config.boveda.poner_etiqueta(vista, i, texto), i),
                // Sin nombre se queda el que tenía.
                EntradaBoveda::NombreGrupo(g) => (e.config.boveda.renombrar_grupo(g, texto), g),
            }
        };
        if cambiado {
            self.guardar();
        }
        self.limpiar_campo_boveda(seleccion as i32);
        self.refrescar();
    }

    /// Esc mientras se escribe una etiqueta o un nombre: se deja como estaba.
    pub(crate) fn cancelar_entrada_boveda(&self) {
        self.estado.borrow_mut().entrada_boveda = EntradaBoveda::Texto;
        if let Some(ui) = self.ui.upgrade() {
            ui.set_texto_boveda("".into());
        }
        self.refrescar();
    }

    /// Pone el campo de arriba a escribir la etiqueta de un texto o el nombre de un grupo. El
    /// campo empieza vacío, listo para escribir el nuevo; el actual se ve en la pista.
    pub(crate) fn editar_en_boveda(&self, entrada: EntradaBoveda) {
        self.estado.borrow_mut().entrada_boveda = entrada;
        if let Some(ui) = self.ui.upgrade() {
            ui.set_texto_boveda("".into());
        }
        self.refrescar();
        self.enfocar();
    }

    /// Cambia algo de la bóveda y, si cambió, lo guarda y lo muestra.
    pub(crate) fn modificar_boveda(&self, cambio: impl FnOnce(&mut boveda::Boveda, Option<usize>) -> bool) {
        let cambiado = {
            let mut e = self.estado.borrow_mut();
            let vista = e.grupo_boveda;
            e.entrada_boveda = EntradaBoveda::Texto;
            cambio(&mut e.config.boveda, vista)
        };
        if cambiado {
            self.guardar();
        }
        self.refrescar();
    }

    pub(crate) fn quitar_de_boveda(&self, i: usize) {
        self.modificar_boveda(|b, vista| b.quitar(vista, i));
    }

    /// Texto `desde` soltado sobre `sobre` (en la vista de arriba): entra en el grupo o, si era
    /// otro texto, se crea un grupo con los dos y se pide su nombre.
    pub(crate) fn juntar_en_boveda(&self, desde: usize, sobre: usize) {
        let resultado = {
            let mut e = self.estado.borrow_mut();
            if e.grupo_boveda.is_some() {
                return;
            }
            e.config.boveda.juntar(desde, sobre)
        };
        let Some((grupo, nuevo)) = resultado else { return };
        self.guardar();
        self.limpiar_campo_boveda(grupo as i32);
        if nuevo {
            self.estado.borrow_mut().entrada_boveda = EntradaBoveda::NombreGrupo(grupo);
            self.enfocar();
        }
        self.refrescar();
    }

    /// Clic derecho en una tarjeta de la bóveda.
    pub(crate) fn menu_boveda(&self, i: usize) {
        const COPIAR: u32 = 1;
        const ETIQUETA: u32 = 2;
        const QUITAR_ETIQUETA: u32 = 3;
        const SACAR: u32 = 4;
        const ELIMINAR: u32 = 5;
        const ABRIR: u32 = 6;
        const RENOMBRAR: u32 = 7;
        const DESHACER: u32 = 8;
        let (vista, es_grupo, tiene_etiqueta) = {
            let e = self.estado.borrow();
            let vista = e.grupo_boveda;
            let boveda = &e.config.boveda;
            let es_grupo = vista.is_none() && boveda.grupo(i).is_some();
            (vista, es_grupo, boveda.texto(vista, i).is_some_and(|t| t.etiqueta.is_some()))
        };
        let mut opciones = Vec::new();
        if es_grupo {
            opciones.extend([
                Opcion::Normal(ABRIR, tr("Abrir")),
                Opcion::Normal(RENOMBRAR, tr("Cambiar nombre…")),
                Opcion::Normal(DESHACER, tr("Deshacer grupo")),
                Opcion::Separador,
                Opcion::Normal(ELIMINAR, tr("Eliminar grupo y sus textos")),
            ]);
        } else {
            opciones.push(Opcion::Normal(COPIAR, tr("Copiar")));
            opciones.push(Opcion::Normal(ETIQUETA, if tiene_etiqueta { tr("Cambiar etiqueta…") } else { tr("Poner etiqueta…") }));
            if tiene_etiqueta {
                opciones.push(Opcion::Normal(QUITAR_ETIQUETA, tr("Quitar etiqueta")));
            }
            if vista.is_some() {
                opciones.push(Opcion::Normal(SACAR, tr("Sacar del grupo")));
            }
            opciones.extend([Opcion::Separador, Opcion::Normal(ELIMINAR, tr("Eliminar"))]);
        }
        self.mostrar_menu(&opciones, move |c, id| match id {
            COPIAR => c.copiar_de_boveda(vista, i),
            ABRIR => c.ver_grupo_boveda(Some(i)),
            ETIQUETA => c.editar_en_boveda(EntradaBoveda::Etiqueta(i)),
            RENOMBRAR => c.editar_en_boveda(EntradaBoveda::NombreGrupo(i)),
            QUITAR_ETIQUETA => c.modificar_boveda(|b, vista| b.poner_etiqueta(vista, i, "")),
            SACAR => c.modificar_boveda(|b, vista| vista.is_some_and(|g| b.sacar_del_grupo(g, i))),
            DESHACER => c.modificar_boveda(|b, _| b.deshacer_grupo(i)),
            ELIMINAR => c.quitar_de_boveda(i),
            _ => {}
        });
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_alternar_boveda(move || c.alternar_boveda());
    let c = ctx.clone();
    ui.on_guardar_en_boveda(move |texto| c.guardar_en_boveda(&texto));
    let c = ctx.clone();
    ui.on_elegir_en_boveda(move |i| c.elegir_en_boveda(i.max(0) as usize));
    let c = ctx.clone();
    ui.on_juntar_en_boveda(move |desde, sobre| c.juntar_en_boveda(desde.max(0) as usize, sobre.max(0) as usize));
    let c = ctx.clone();
    ui.on_volver_en_boveda(move || c.ver_grupo_boveda(None));
    let c = ctx.clone();
    ui.on_cancelar_entrada_boveda(move || c.cancelar_entrada_boveda());
    let c = ctx.clone();
    ui.on_quitar_de_boveda(move |i| c.quitar_de_boveda(i.max(0) as usize));
    let c = ctx.clone();
    ui.on_menu_boveda(move |i| {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.menu_boveda(i.max(0) as usize));
    });
    let c = ctx.clone();
    ui.on_boton_cuenta(move || c.boton_cuenta());
    let c = ctx.clone();
    ui.on_novedades_cuenta(move || c.novedades_cuenta());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_vista_de_un_texto_son_sus_primeras_lineas_con_contenido() {
        assert_eq!(vista_de_texto("hola"), "hola");
        assert_eq!(vista_de_texto("\n  primera  \n\nsegunda"), "primera\nsegunda");
        assert_eq!(vista_de_texto("1\n2\n3\n4\n5"), "1\n2\n3\n4");
        assert_eq!(vista_de_texto(&"x".repeat(300)).chars().count(), 200);
        assert_eq!(vista_de_texto(""), "");
    }
}
