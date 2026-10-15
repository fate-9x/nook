//! Lo que suena: si hay música y su ritmo (para que el personaje baile) y lo que muestra el
//! reproductor (qué suena y sus controles, ver `reproductor`).
//!
//! - Si suena: lo dicen los controles multimedia de Windows (los que salen en el panel de
//!   volumen), que conocen las sesiones de Spotify, los navegadores y demás reproductores. Cuenta
//!   cualquier sesión reproduciendo que no sea un vídeo.
//! - El ritmo: mientras suena, se lee el nivel de lo que sale por los altavoces (el medidor de
//!   picos del dispositivo de audio predeterminado) y `Ritmo` busca los golpes en él.
//!
//! Todo esto se consulta en un hilo aparte (las llamadas pueden tardar) y se deja en `Musica`,
//! que el hilo de la interfaz lee sin esperar; las órdenes del reproductor le llegan a ese hilo
//! por un canal.

use crate::reproductor::Reproductor;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as Controles,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as EstadoReproduccion,
};
use windows::Media::MediaPlaybackType;
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};

/// Lo que se sabe de la música que suena.
pub struct Musica {
    sonando: AtomicBool,
    /// Nivel de audio (0–1) como bits de un f32.
    nivel: AtomicU32,
    /// Lo que muestra el reproductor, y cuántas veces ha cambiado (para saber si hay novedades).
    reproduccion: Mutex<Arc<Reproduccion>>,
    cambios: AtomicU64,
    ordenes: Sender<Orden>,
}

impl Musica {
    pub fn sonando(&self) -> bool {
        self.sonando.load(Ordering::Relaxed)
    }

    pub fn nivel(&self) -> f32 {
        f32::from_bits(self.nivel.load(Ordering::Relaxed))
    }

    /// Cuántas veces ha cambiado lo que muestra el reproductor.
    pub fn cambios(&self) -> u64 {
        self.cambios.load(Ordering::Relaxed)
    }

    pub fn reproduccion(&self) -> Arc<Reproduccion> {
        self.reproduccion.lock().unwrap().clone()
    }

    /// Pide algo al reproductor (se hace en el hilo de la música).
    pub fn ordenar(&self, orden: Orden) {
        let _ = self.ordenes.send(orden);
    }

    fn publicar(&self, nueva: Reproduccion) {
        let mut actual = self.reproduccion.lock().unwrap();
        if **actual != nueva {
            *actual = Arc::new(nueva);
            self.cambios.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Lo que muestra el reproductor: la sesión multimedia actual (la que Windows pone en su panel de
/// volumen), suene o esté en pausa.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Reproduccion {
    /// Hay alguna sesión.
    pub hay: bool,
    pub sonando: bool,
    pub titulo: String,
    /// El artista o, si no lo dice, el nombre de la app.
    pub artista: String,
    pub portada: Option<Portada>,
    pub puede_anterior: bool,
    pub puede_siguiente: bool,
    /// Volumen de la app en el mezclador de Windows (0–1), si se encontró su audio.
    pub volumen: Option<f32>,
}

/// Portada de lo que suena: un cuadrado de `lado` píxeles RGBA con las esquinas redondeadas.
#[derive(Clone, Debug)]
pub struct Portada {
    pub pixeles: Arc<Vec<u8>>,
    pub lado: u32,
}

impl PartialEq for Portada {
    fn eq(&self, otra: &Self) -> bool {
        Arc::ptr_eq(&self.pixeles, &otra.pixeles)
    }
}

/// Lo que se le puede pedir al reproductor.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Orden {
    Anterior,
    PausaReproducir,
    Siguiente,
    /// Poner el volumen de la app (0–1).
    Volumen(f32),
}

/// Cada cuánto se pregunta si suena algo, y cada cuánto se lee el nivel mientras suena.
const CONSULTA: Duration = Duration::from_secs(1);
const LECTURA_NIVEL: Duration = Duration::from_millis(15);
/// Tras pasar de canción o pausar, se vuelve a preguntar enseguida para que se vea el cambio.
const TRAS_ORDEN: Duration = Duration::from_millis(250);

/// Empieza a vigilar la música en un hilo aparte.
pub fn vigilar() -> Arc<Musica> {
    let (ordenes, recibidas) = mpsc::channel();
    let musica = Arc::new(Musica {
        sonando: AtomicBool::new(false),
        nivel: AtomicU32::new(0),
        reproduccion: Mutex::new(Arc::default()),
        cambios: AtomicU64::new(0),
        ordenes,
    });
    let m = musica.clone();
    let _ = std::thread::Builder::new().name("musica".into()).spawn(move || bucle(&m, recibidas));
    musica
}

fn bucle(musica: &Musica, ordenes: Receiver<Orden>) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let mut controles = None;
    let mut medidor: Option<IAudioMeterInformation> = None;
    let mut reproductor = Reproductor::default();
    let mut proxima = Instant::now();
    loop {
        if Instant::now() >= proxima {
            proxima = Instant::now() + CONSULTA;
            let sonando = suena_musica(&mut controles);
            musica.sonando.store(sonando, Ordering::Relaxed);
            if !sonando {
                medidor = None;
                musica.nivel.store(0f32.to_bits(), Ordering::Relaxed);
            }
            reproductor.consultar(controles.as_ref());
            musica.publicar(reproductor.estado.clone());
        }
        // Mientras suena se lee el nivel a menudo; si no, se espera a la próxima consulta. En los
        // dos casos, atendiendo a las órdenes que lleguen.
        let espera = if musica.sonando() { LECTURA_NIVEL } else { proxima.saturating_duration_since(Instant::now()) };
        match ordenes.recv_timeout(espera) {
            Ok(orden) => {
                if reproductor.ejecutar(orden) {
                    proxima = proxima.min(Instant::now() + TRAS_ORDEN);
                }
                musica.publicar(reproductor.estado.clone());
                continue;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => std::thread::sleep(espera),
        }
        if !musica.sonando() {
            continue;
        }
        if medidor.is_none() {
            medidor = medidor_de_audio();
        }
        // Si falla (por ejemplo, al cambiar de altavoces a auriculares), se vuelve a buscar.
        match medidor.as_ref().map(|m| unsafe { m.GetPeakValue() }) {
            Some(Ok(nivel)) => musica.nivel.store(nivel.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed),
            _ => medidor = None,
        }
    }
}

/// Si alguna sesión multimedia está reproduciendo algo que no es un vídeo.
fn suena_musica(controles: &mut Option<Controles>) -> bool {
    if controles.is_none() {
        *controles = Controles::RequestAsync().and_then(|operacion| operacion.join()).ok();
    }
    let Some(gestor) = controles.as_ref() else { return false };
    let Ok(sesiones) = gestor.GetSessions() else {
        *controles = None;
        return false;
    };
    (0..sesiones.Size().unwrap_or(0)).any(|i| {
        let Ok(info) = sesiones.GetAt(i).and_then(|s| s.GetPlaybackInfo()) else { return false };
        let reproduciendo = info.PlaybackStatus().is_ok_and(|e| e == EstadoReproduccion::Playing);
        let es_video = info.PlaybackType().and_then(|t| t.Value()).is_ok_and(|t| t == MediaPlaybackType::Video);
        reproduciendo && !es_video
    })
}

/// Medidor de picos del dispositivo de salida de audio predeterminado.
fn medidor_de_audio() -> Option<IAudioMeterInformation> {
    unsafe {
        let enumerador: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let dispositivo = enumerador.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        dispositivo.Activate(CLSCTX_ALL, None).ok()
    }
}

/// Busca los golpes del ritmo en el nivel de audio: un golpe es una subida brusca por encima del
/// nivel medio reciente, y entre dos golpes pasa al menos un cuarto de segundo.
pub struct Ritmo {
    media: f32,
    anterior: f32,
    ultimo: f64,
}

impl Ritmo {
    const SEPARACION: f64 = 0.25;

    pub fn new() -> Self {
        Ritmo { media: 0.0, anterior: 0.0, ultimo: f64::NEG_INFINITY }
    }

    /// Nivel (0–1) en el instante `ahora` (segundos); si hay golpe, devuelve su fuerza (0,4–1).
    pub fn muestra(&mut self, ahora: f64, nivel: f32) -> Option<f32> {
        let media = self.media;
        let subida = nivel - self.anterior;
        self.media += (nivel - self.media) * 0.08;
        self.anterior = nivel;
        if nivel > 0.05 && nivel > media * 1.35 && subida > 0.02 && ahora - self.ultimo >= Self::SEPARACION {
            self.ultimo = ahora;
            return Some(((nivel - media) / media.max(0.05)).clamp(0.4, 1.0));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nivel de una canción sintética: un golpe fuerte cada medio segundo sobre un fondo suave.
    fn nivel(t: f64) -> f32 {
        let fase = (t % 0.5) as f32;
        0.15 + if fase < 0.06 { 0.6 * (1.0 - fase / 0.06) } else { 0.0 }
    }

    #[test]
    fn encuentra_los_golpes_de_un_ritmo_regular() {
        let mut ritmo = Ritmo::new();
        let golpes: Vec<f64> = (0..360)
            .filter_map(|i| {
                let t = i as f64 / 60.0;
                ritmo.muestra(t, nivel(t)).map(|_| t)
            })
            .collect();
        // Seis segundos con dos golpes por segundo (el primero puede perderse mientras la media
        // se ajusta).
        assert!((11..=12).contains(&golpes.len()), "{} golpes", golpes.len());
        for par in golpes.windows(2) {
            assert!((par[1] - par[0] - 0.5).abs() < 0.04);
        }
    }

    #[test]
    fn el_silencio_y_un_nivel_constante_no_tienen_golpes() {
        let mut ritmo = Ritmo::new();
        assert!((0..300).all(|i| ritmo.muestra(i as f64 / 60.0, 0.0).is_none()));
        let mut ritmo = Ritmo::new();
        let golpes = (0..300).filter(|&i| ritmo.muestra(i as f64 / 60.0, 0.4).is_some()).count();
        // Solo la subida inicial desde el silencio.
        assert!(golpes <= 1);
    }
}
