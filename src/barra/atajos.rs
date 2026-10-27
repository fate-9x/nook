//! Atajos de teclado globales: registrarlos en Windows, cambiarlos desde la configuración y lo que
//! hace cada uno.

use crate::*;

/// Identificador del atajo "mostrar u ocultar". Los elementos de la tira usan 1, 2, 3… (índice + 1).
pub(crate) const ID_MOSTRAR: i32 = 100;

/// Identificador del atajo de la bóveda de textos.
pub(crate) const ID_BOVEDA: i32 = 101;

/// Identificador del atajo que silencia o activa el micrófono.
pub(crate) const ID_MICROFONO: i32 = 102;

/// Identificador del atajo que abre el buscador para abrir una app.
pub(crate) const ID_BUSCAR: i32 = 103;

/// Identificador del atajo de la conversación con Claude Code.
pub(crate) const ID_CLAUDE: i32 = 104;

/// Texto con que se muestra un atajo en la configuración.
pub(crate) fn texto_atajo(atajo: Option<&Atajo>) -> String {
    atajo.map_or(tr("Sin atajo").into(), Atajo::texto)
}

impl Ctx {
    /// Registra en Windows los atajos de todos los elementos y el de mostrar/ocultar.
    pub(crate) fn registrar_atajos(&self) {
        let mut e = self.estado.borrow_mut();
        for id in 1..=e.registrados as i32 {
            atajos::quitar(id);
        }
        atajos::quitar(ID_MOSTRAR);
        atajos::quitar(ID_BOVEDA);
        atajos::quitar(ID_MICROFONO);
        atajos::quitar(ID_BUSCAR);
        atajos::quitar(ID_CLAUDE);
        let ok: Vec<bool> = e
            .config
            .elementos
            .iter()
            .enumerate()
            .map(|(i, el)| el.atajo().is_some_and(|a| atajos::registrar(i as i32 + 1, a)))
            .collect();
        e.registrados = ok.len();
        e.atajos_ok = ok;
        // Sin atajo no hay nada que avisar: se da por bueno.
        e.mostrar_ok = e.config.ajustes.atajo_mostrar.as_ref().is_none_or(|a| atajos::registrar(ID_MOSTRAR, a));
        let boveda = if e.config.ajustes.usar_boveda { e.config.ajustes.atajo_boveda.as_ref() } else { None };
        e.boveda_ok = boveda.is_none_or(|a| atajos::registrar(ID_BOVEDA, a));
        // Los del micrófono y el buscador funcionan aunque su botón no esté en la tira.
        e.microfono_ok = e.config.ajustes.atajo_microfono.as_ref().is_none_or(|a| atajos::registrar(ID_MICROFONO, a));
        e.buscar_ok = e.config.ajustes.atajo_buscar.as_ref().is_none_or(|a| atajos::registrar(ID_BUSCAR, a));
        e.claude_ok = e.config.ajustes.atajo_claude.as_ref().is_none_or(|a| atajos::registrar(ID_CLAUDE, a));
    }

    /// Cambia (o quita, con `None`) el atajo de `objetivo`: -1 mostrar u ocultar, -3 la bóveda, -4
    /// el micrófono, -5 el buscador para abrir apps, -6 la conversación con Claude y 0, 1, 2… el elemento de la barra con ese
    /// índice. Deja de grabar.
    pub(crate) fn poner_atajo(&self, objetivo: i32, atajo: Option<Atajo>) {
        {
            let mut e = self.estado.borrow_mut();
            match objetivo {
                -1 => e.config.ajustes.atajo_mostrar = atajo,
                -3 => e.config.ajustes.atajo_boveda = atajo,
                -4 => e.config.ajustes.atajo_microfono = atajo,
                -5 => e.config.ajustes.atajo_buscar = atajo,
                -6 => e.config.ajustes.atajo_claude = atajo,
                i => {
                    if let Some(el) = e.config.elementos.get_mut(i as usize) {
                        *el.atajo_mut() = atajo;
                    }
                }
            }
            if !PRO {
                e.config.ajustes.quitar_funciones_pro();
            }
            e.grabando = None;
        }
        self.guardar();
        self.registrar_atajos();
        self.refrescar();
    }

    /// Acción de un atajo global.
    pub(crate) fn atajo_pulsado(&self, id: i32) {
        let modo = self.estado.borrow().modo;
        if id == ID_MOSTRAR {
            if modo.desplegado() {
                self.cerrar();
            } else if let Some(grupo) = self.grupo_preferido() {
                self.mostrar_grupo(grupo, false);
            } else {
                self.poner_modo(Modo::Reposo);
            }
        } else if id == ID_BOVEDA {
            // Con la bóveda abierta, el mismo atajo copia el texto marcado (o abre el grupo).
            if modo == Modo::Boveda {
                let seleccion = self.ui.upgrade().map_or(0, |ui| ui.get_seleccion_boveda());
                self.elegir_en_boveda(seleccion.max(0) as usize);
            } else {
                self.abrir_boveda();
            }
        } else if id == ID_MICROFONO {
            self.alternar_microfono();
        } else if id == ID_BUSCAR {
            self.alternar_lanzador();
        } else if id == ID_CLAUDE {
            if PRO {
                self.alternar_claude();
            }
        } else if id == tecla_windows::ID_PULSADA {
            // Aviso del ayudante de la tecla Windows.
            if self.estado.borrow().config.ajustes.tecla_windows_abre_buscador {
                tecla_windows::pulsada();
            }
        } else if id >= 1 {
            self.abrir_elemento(id as usize - 1);
        }
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_quitar_atajo(move |i| c.poner_atajo(i, None));
    let c = ctx.clone();
    ui.on_grabar_atajo(move |i| {
        c.estado.borrow_mut().grabando = Some(i);
        c.refrescar();
        c.enfocar();
    });
    let c = ctx.clone();
    plataforma::al_atajo(move |id| c.atajo_pulsado(id));
}
