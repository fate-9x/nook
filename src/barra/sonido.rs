//! Volumen general de Windows: el botón del altavoz de la tira, su panel (silencio y volumen), la
//! rueda del ratón sobre ellos y sus teclas; y el botón que silencia el micrófono.

use crate::*;

/// Cuánto sube o baja el volumen con cada paso de la rueda y con cada flecha del teclado.
const PASO_RUEDA: f32 = 0.02;
const PASO_TECLA: f32 = 0.05;
/// Giro de la rueda (en píxeles lógicos de Slint) que corresponde a un paso (ver plataforma.rs).
const GIRO_POR_PASO: f32 = 50.0;

impl Ctx {
    /// Abre o cierra el panel del volumen.
    pub(crate) fn alternar_sonido(&self) {
        if self.estado.borrow().modo == Modo::Sonido {
            self.cerrar();
        } else {
            self.mostrar_sonido();
            self.poner_modo(Modo::Sonido);
        }
    }

    /// Pasa a la interfaz cómo está la salida de audio (la cambian también las teclas de volumen
    /// y otros programas: se vuelve a mirar de vez en cuando, ver `sondear`).
    pub(crate) fn mostrar_sonido(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let salida = sonido::leer();
        ui.set_hay_sonido(salida.is_some());
        let salida = salida.unwrap_or(sonido::Salida { volumen: 0.0, silencio: false, nombre: String::new() });
        if (ui.get_volumen_sistema() - salida.volumen).abs() > 0.001 {
            ui.set_volumen_sistema(salida.volumen);
        }
        ui.set_silencio(salida.silencio);
        ui.set_nombre_salida(salida.nombre.into());
    }

    /// Pasa a la interfaz si hay micrófono y si está silenciado (también lo cambian otros
    /// programas: se vuelve a mirar de vez en cuando, ver `sondear`).
    pub(crate) fn mostrar_microfono(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let silenciado = sonido::microfono_silenciado();
        ui.set_hay_microfono(silenciado.is_some());
        ui.set_microfono_silenciado(silenciado.unwrap_or(false));
    }

    pub(crate) fn alternar_microfono(&self) {
        if let Some(silenciado) = sonido::microfono_silenciado() {
            sonido::silenciar_microfono(!silenciado);
        }
        self.mostrar_microfono();
    }

    pub(crate) fn elegir_volumen_sistema(&self, volumen: f32) {
        sonido::poner_volumen(volumen);
        self.mostrar_sonido();
    }

    /// Sube el volumen `paso` (si es negativo, lo baja).
    fn cambiar_volumen(&self, paso: f32) {
        let Some(salida) = sonido::leer() else { return };
        self.elegir_volumen_sistema((salida.volumen + paso).clamp(0.0, 1.0));
    }

    pub(crate) fn alternar_silencio(&self) {
        if let Some(salida) = sonido::leer() {
            sonido::poner_silencio(!salida.silencio);
        }
        self.mostrar_sonido();
    }

    /// Giro de la rueda sobre el botón o el panel: hacia delante sube.
    pub(crate) fn rueda_sonido(&self, giro: f32) {
        self.cambiar_volumen(PASO_RUEDA * giro / GIRO_POR_PASO);
    }

    /// Teclas con el panel abierto: las flechas de arriba y abajo suben y bajan el volumen, y el
    /// espacio o la M lo silencian.
    pub(crate) fn tecla_sonido(&self, texto: &str) -> bool {
        let es = |tecla: Key| SharedString::from(tecla).as_str() == texto;
        if es(Key::UpArrow) || es(Key::RightArrow) {
            self.cambiar_volumen(PASO_TECLA);
        } else if es(Key::DownArrow) || es(Key::LeftArrow) {
            self.cambiar_volumen(-PASO_TECLA);
        } else if texto == " " || texto.eq_ignore_ascii_case("m") {
            self.alternar_silencio();
        } else {
            return false;
        }
        true
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_alternar_sonido(move || c.alternar_sonido());
    let c = ctx.clone();
    ui.on_elegir_volumen_sistema(move |v| c.elegir_volumen_sistema(v));
    let c = ctx.clone();
    ui.on_alternar_silencio(move || c.alternar_silencio());
    let c = ctx.clone();
    ui.on_rueda_sonido(move |giro| c.rueda_sonido(giro));
    let c = ctx.clone();
    ui.on_alternar_boton_sonido(move || c.modificar_ajustes(|a| a.boton_sonido = !a.boton_sonido));
    let c = ctx.clone();
    ui.on_alternar_microfono(move || c.alternar_microfono());
    let c = ctx.clone();
    ui.on_alternar_boton_microfono(move || c.modificar_ajustes(|a| a.boton_microfono = !a.boton_microfono));
}
