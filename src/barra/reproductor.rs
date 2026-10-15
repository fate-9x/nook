//! Reproductor de lo que suena: abrirlo y cerrarlo, mostrar lo que suena y sus teclas.

use crate::*;

/// Cuánto sube o baja el volumen con cada flecha en el reproductor, y cada cuánto se mira si
/// cambió lo que suena.
pub(crate) const PASO_VOLUMEN: f32 = 0.05;
pub(crate) const CONSULTA_REPRODUCTOR: Duration = Duration::from_millis(100);

impl Ctx {
    /// Abre o cierra el reproductor; se abre solo si hay algo que mostrar.
    pub(crate) fn alternar_reproductor(&self) {
        if self.estado.borrow().modo == Modo::Musica {
            self.cerrar();
        } else if self.musica.reproduccion().hay {
            self.poner_modo(Modo::Musica);
        }
    }

    /// Pasa a la interfaz lo que muestra el reproductor si cambió desde la última vez (`visto`:
    /// los cambios que se vieron y la portada que se puso).
    pub(crate) fn mostrar_reproduccion(&self, visto: &mut (Option<u64>, Option<Arc<Vec<u8>>>)) {
        let cambios = self.musica.cambios();
        if visto.0 == Some(cambios) {
            return;
        }
        visto.0 = Some(cambios);
        let Some(ui) = self.ui.upgrade() else { return };
        let r = self.musica.reproduccion();
        ui.set_hay_musica(r.hay);
        ui.set_reproduciendo(r.sonando);
        ui.set_cancion(r.titulo.as_str().into());
        ui.set_artista(r.artista.as_str().into());
        ui.set_puede_anterior(r.puede_anterior);
        ui.set_puede_siguiente(r.puede_siguiente);
        ui.set_hay_volumen(r.volumen.is_some());
        ui.set_volumen(r.volumen.unwrap_or(0.0));
        let pixeles = r.portada.as_ref().map(|p| p.pixeles.clone());
        let misma = match (&pixeles, &visto.1) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !misma {
            let imagen = r.portada.as_ref().map(|p| {
                Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&p.pixeles, p.lado, p.lado))
            });
            ui.set_tiene_portada(imagen.is_some());
            ui.set_portada(imagen.unwrap_or_default());
            visto.1 = pixeles;
        }
    }

    /// Teclas con el reproductor abierto: espacio pausa o reproduce, las flechas a los lados
    /// cambian de canción y las de arriba y abajo suben y bajan el volumen.
    pub(crate) fn tecla_reproductor(&self, texto: &str) -> bool {
        let es = |tecla: Key| SharedString::from(tecla).as_str() == texto;
        let orden = if texto == " " {
            Orden::PausaReproducir
        } else if es(Key::LeftArrow) {
            Orden::Anterior
        } else if es(Key::RightArrow) {
            Orden::Siguiente
        } else if es(Key::UpArrow) || es(Key::DownArrow) {
            let Some(volumen) = self.musica.reproduccion().volumen else { return true };
            let paso = if es(Key::UpArrow) { PASO_VOLUMEN } else { -PASO_VOLUMEN };
            Orden::Volumen((volumen + paso).clamp(0.0, 1.0))
        } else {
            return false;
        };
        self.musica.ordenar(orden);
        true
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_orden_musica(move |i| {
        c.musica.ordenar(match i {
            0 => Orden::Anterior,
            2 => Orden::Siguiente,
            _ => Orden::PausaReproducir,
        })
    });
    let c = ctx.clone();
    ui.on_elegir_volumen(move |v| c.musica.ordenar(Orden::Volumen(v)));
    let c = ctx.clone();
    ui.on_alternar_reproductor(move || c.alternar_reproductor());
    let c = ctx.clone();
    ui.on_alternar_reproductor_en_barra(move || c.modificar_ajustes(|a| a.reproductor_en_barra = !a.reproductor_en_barra));
}
