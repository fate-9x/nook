//! El botón de apagado de la tira (su menú: suspender, reiniciar o apagar el equipo) y el icono de
//! la batería, que solo se ve en los equipos que tienen (los portátiles).

use crate::*;

/// Texto del cartel del icono de la batería: el porcentaje y si se está cargando, si está
/// enchufada o, con la batería, lo que le queda.
pub(crate) fn texto_bateria(b: &energia::Bateria) -> String {
    let porcentaje = format!("{} %", (b.nivel * 100.0).round() as u32);
    let detalle = if b.cargando {
        tr("cargando").to_string()
    } else if b.enchufada {
        tr("enchufada").to_string()
    } else if let Some(minutos) = b.minutos {
        if minutos >= 60 { format!("{} h {} min", minutos / 60, minutos % 60) } else { format!("{minutos} min") }
    } else {
        return porcentaje;
    };
    format!("{porcentaje} · {detalle}")
}

impl Ctx {
    /// Clic en el botón de apagado: el menú para suspender, reiniciar o apagar el equipo.
    pub(crate) fn menu_energia(&self) {
        const SUSPENDER: u32 = 1;
        const REINICIAR: u32 = 2;
        const APAGAR: u32 = 3;
        let opciones = [
            Opcion::Normal(SUSPENDER, tr("Suspender")),
            Opcion::Normal(REINICIAR, tr("Reiniciar")),
            Opcion::Normal(APAGAR, tr("Apagar")),
        ];
        self.mostrar_menu(&opciones, |c, id| {
            c.cerrar();
            match id {
                SUSPENDER => energia::suspender(),
                REINICIAR => energia::reiniciar(),
                APAGAR => energia::apagar(),
                _ => {}
            }
        });
    }

    /// Pasa a la interfaz cómo está la batería (cambia sola: se vuelve a mirar de vez en cuando,
    /// ver `sondear`). Sin batería, el icono no se ve.
    pub(crate) fn mostrar_bateria(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let bateria = energia::bateria();
        ui.set_hay_bateria(bateria.is_some());
        if let Some(b) = bateria {
            ui.set_nivel_bateria(b.nivel);
            ui.set_cargando_bateria(b.cargando || b.enchufada);
            ui.set_texto_bateria(texto_bateria(&b).into());
        }
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_menu_energia(move || {
        // Se difiere: abre la emergente y no debe hacerse desde dentro del evento de ratón.
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.menu_energia());
    });
    // La configuración de la batería de Windows.
    ui.on_abrir_bateria(|| {
        if let Err(e) = sistema::lanzar("ms-settings:batterysaver") {
            eprintln!("{e}");
        }
    });
    let c = ctx.clone();
    ui.on_alternar_boton_energia(move || c.modificar_ajustes(|a| a.boton_energia = !a.boton_energia));
    let c = ctx.clone();
    ui.on_alternar_boton_bateria(move || {
        c.modificar_ajustes(|a| a.boton_bateria = !a.boton_bateria);
        c.mostrar_bateria();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bateria(nivel: f32, cargando: bool, enchufada: bool, minutos: Option<u32>) -> energia::Bateria {
        energia::Bateria { nivel, cargando, enchufada, minutos }
    }

    #[test]
    fn texto_de_la_bateria() {
        assert_eq!(texto_bateria(&bateria(0.854, true, true, None)), "85 % · cargando");
        assert_eq!(texto_bateria(&bateria(1.0, false, true, None)), "100 % · enchufada");
        assert_eq!(texto_bateria(&bateria(0.4, false, false, Some(200))), "40 % · 3 h 20 min");
        assert_eq!(texto_bateria(&bateria(0.1, false, false, Some(25))), "10 % · 25 min");
        assert_eq!(texto_bateria(&bateria(0.5, false, false, None)), "50 %");
    }
}
