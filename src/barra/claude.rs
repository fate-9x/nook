//! Conversación con Claude Code (solo en nook Pro): el botón de la tira abre un panel con la
//! conversación y un campo para escribirle. Cada mensaje se le manda a `claude` en la carpeta
//! elegida, siguiendo la misma conversación hasta que se empieza otra (ver `claude.rs`).

use crate::*;

/// La conversación del panel.
#[derive(Default)]
pub(crate) struct Conversacion {
    mensajes: Vec<MensajeClaude>,
    /// Id de la conversación de Claude Code, para seguirla en la siguiente pregunta.
    sesion: Option<String>,
    respuesta: Option<claude::Respuesta>,
    /// Cuenta las preguntas: lo que llegue de una que se paró se descarta.
    turno: u64,
}

/// Tipos de mensaje del panel (ver ui/claude.slint).
const DEL_USUARIO: i32 = 0;
const DE_CLAUDE: i32 = 1;
const NOTA: i32 = 2;

impl Ctx {
    /// Abre o cierra el panel de Claude.
    pub(crate) fn alternar_claude(&self) {
        if self.estado.borrow().modo == Modo::Claude {
            self.cerrar();
        } else {
            self.mostrar_claude();
            self.poner_modo(Modo::Claude);
        }
    }

    /// Pasa a la interfaz la conversación, si Claude está respondiendo y la carpeta.
    fn mostrar_claude(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let e = self.estado.borrow();
        ui.set_mensajes_claude(modelo(e.claude.mensajes.clone()));
        ui.set_claude_ocupado(e.claude.respuesta.is_some());
        ui.set_carpeta_claude(carpeta_claude(&e.config.ajustes).display().to_string().into());
    }

    fn anotar_claude(&self, tipo: i32, texto: impl Into<SharedString>) {
        self.estado.borrow_mut().claude.mensajes.push(MensajeClaude { tipo, texto: texto.into() });
    }

    /// Le manda un mensaje a Claude (si no está respondiendo otro).
    fn enviar_claude(&self, texto: &str) {
        let texto = texto.trim();
        if texto.is_empty() || self.estado.borrow().claude.respuesta.is_some() {
            return;
        }
        self.anotar_claude(DEL_USUARIO, texto);
        let (carpeta, sesion, turno) = {
            let mut e = self.estado.borrow_mut();
            e.claude.turno += 1;
            (carpeta_claude(&e.config.ajustes), e.claude.sesion.clone(), e.claude.turno)
        };
        let al_llegar = move |suceso| barra::addons::en_la_interfaz(move |ctx| ctx.llega_de_claude(turno, suceso));
        match claude::preguntar(texto, &carpeta, sesion.as_deref(), al_llegar) {
            Ok(respuesta) => self.estado.borrow_mut().claude.respuesta = Some(respuesta),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.anotar_claude(NOTA, tr("No se encuentra Claude Code: instálalo y vuelve a intentarlo."))
            }
            Err(e) => self.anotar_claude(NOTA, tr_con("No se pudo abrir Claude Code: {}", &[&e.to_string()])),
        }
        self.mostrar_claude();
    }

    /// Lo que va respondiendo Claude a la pregunta `turno`.
    fn llega_de_claude(&self, turno: u64, suceso: claude::Suceso) {
        if self.estado.borrow().claude.turno != turno {
            return;
        }
        match suceso {
            claude::Suceso::Sesion(id) => self.estado.borrow_mut().claude.sesion = Some(id),
            claude::Suceso::Texto(texto) => self.anotar_claude(DE_CLAUDE, texto),
            claude::Suceso::Herramienta(uso) => self.anotar_claude(NOTA, format!("→ {uso}")),
            claude::Suceso::Fin { denegadas, error } => {
                self.estado.borrow_mut().claude.respuesta = None;
                if !denegadas.is_empty() {
                    self.anotar_claude(NOTA, tr_con("Claude no tuvo permiso para usar: {}", &[&denegadas.join(", ")]));
                }
                // Algunos errores (como no tener la sesión iniciada) llegan antes como su respuesta.
                let ya_dicho = |error: &str| self.estado.borrow().claude.mensajes.last().is_some_and(|m| m.texto == error);
                if let Some(error) = error.filter(|e| !ya_dicho(e)) {
                    self.anotar_claude(NOTA, tr_con("Claude Code falló: {}", &[&error]));
                }
                // Con el panel cerrado, el personaje avisa de que ya respondió.
                if self.estado.borrow().modo != Modo::Claude {
                    self.notificar();
                }
            }
        }
        self.mostrar_claude();
    }

    /// Corta la respuesta en marcha (si hay).
    fn parar_claude(&self) {
        let respuesta = {
            let mut e = self.estado.borrow_mut();
            e.claude.turno += 1;
            e.claude.respuesta.take()
        };
        if let Some(respuesta) = respuesta {
            respuesta.parar();
            self.anotar_claude(NOTA, tr("Respuesta parada."));
        }
        self.mostrar_claude();
    }

    /// Empieza una conversación nueva.
    fn nueva_conversacion_claude(&self) {
        self.parar_claude();
        let mut e = self.estado.borrow_mut();
        e.claude.mensajes.clear();
        e.claude.sesion = None;
        drop(e);
        self.mostrar_claude();
    }

    /// Elige la carpeta en la que trabaja Claude (con una conversación nueva).
    fn elegir_carpeta_claude(&self) {
        if let Some(carpeta) = claude::elegir_carpeta(plataforma::hwnd()) {
            self.modificar_ajustes(|a| a.carpeta_claude = carpeta.display().to_string());
            self.nueva_conversacion_claude();
        }
    }
}

/// Carpeta en la que trabaja Claude: la elegida o, si no hay, la del usuario.
fn carpeta_claude(ajustes: &Ajustes) -> PathBuf {
    if ajustes.carpeta_claude.is_empty() {
        std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default()
    } else {
        PathBuf::from(&ajustes.carpeta_claude)
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_alternar_claude(move || c.alternar_claude());
    let c = ctx.clone();
    ui.on_enviar_claude(move |texto| c.enviar_claude(&texto));
    let c = ctx.clone();
    ui.on_parar_claude(move || c.parar_claude());
    let c = ctx.clone();
    ui.on_nueva_conversacion_claude(move || c.nueva_conversacion_claude());
    let c = ctx.clone();
    ui.on_elegir_carpeta_claude(move || c.elegir_carpeta_claude());
    let c = ctx.clone();
    ui.on_alternar_boton_claude(move || c.modificar_ajustes(|a| a.boton_claude = !a.boton_claude));
}
