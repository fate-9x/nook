//! Descargas por HTTPS (con el cliente HTTP de Windows), para las actualizaciones y los addons.
//!
//! Las llamadas bloquean: hay que hacerlas en un hilo aparte.

use crate::actualizacion::VERSION;
use windows::Foundation::Uri;
use windows::Storage::Streams::{DataReader, InputStreamOptions};
use windows::Web::Http::Filters::{HttpBaseProtocolFilter, HttpCacheReadBehavior};
use windows::Web::Http::{HttpClient, HttpCompletionOption};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows_core::HSTRING;

/// Tamaño de cada trozo de una descarga: cada uno avanza el progreso.
const TROZO: u32 = 64 * 1024;

/// Prepara COM en el hilo que llama (lo necesita el cliente HTTP de Windows).
pub fn iniciar_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

pub fn cliente() -> windows_core::Result<HttpClient> {
    // Sin caché: cada lectura tiene que traer lo último que hay (el Gist de la bóveda, el
    // catálogo de addons…).
    let filtro = HttpBaseProtocolFilter::new()?;
    filtro.CacheControl()?.SetReadBehavior(HttpCacheReadBehavior::NoCache)?;
    let cliente = HttpClient::Create(&filtro)?;
    // La API de GitHub rechaza las peticiones sin User-Agent.
    cliente.DefaultRequestHeaders()?.UserAgent()?.TryParseAdd(&HSTRING::from(format!("nook/{VERSION}")))?;
    Ok(cliente)
}

/// Descarga un texto (una respuesta JSON, por ejemplo).
pub fn texto(url: &str) -> windows_core::Result<String> {
    iniciar_com();
    Ok(cliente()?.GetStringAsync(&Uri::CreateUri(&HSTRING::from(url))?)?.join()?.to_string_lossy())
}

/// Descarga un archivo. Si se sabe cuántos bytes tiene (`bytes`, 0 si no), `progreso` recibe
/// cuánto se lleva descargado (0–1).
pub fn descargar(url: &str, bytes: u64, progreso: impl Fn(f32)) -> windows_core::Result<Vec<u8>> {
    iniciar_com();
    let uri = Uri::CreateUri(&HSTRING::from(url))?;
    // Sin esperar a tenerlo todo, para ir avanzando el progreso.
    let respuesta = cliente()?.GetWithOptionAsync(&uri, HttpCompletionOption::ResponseHeadersRead)?.join()?;
    // Sin esto, una página de error (un 404) se tomaría por el archivo.
    respuesta.EnsureSuccessStatusCode()?;
    let flujo = respuesta.Content()?.ReadAsInputStreamAsync()?.join()?;
    let lector = DataReader::CreateDataReader(&flujo)?;
    lector.SetInputStreamOptions(InputStreamOptions::Partial)?;
    let mut datos = Vec::with_capacity(bytes as usize);
    loop {
        let leidos = lector.LoadAsync(TROZO)?.join()?;
        if leidos == 0 {
            break;
        }
        let inicio = datos.len();
        datos.resize(inicio + leidos as usize, 0);
        lector.ReadBytes(&mut datos[inicio..])?;
        if bytes > 0 {
            progreso((datos.len() as f64 / bytes as f64).min(1.0) as f32);
        }
    }
    Ok(datos)
}
