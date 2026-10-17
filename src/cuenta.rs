//! Cuenta de GitHub (opcional) para tener la misma bóveda de textos en varios ordenadores.
//!
//! - Se inicia sesión con el flujo de dispositivo de OAuth: nook pide un código a GitHub, el
//!   usuario lo escribe en github.com/login/device y nook recibe un token que solo da permiso
//!   sobre sus Gists. No hace falta ningún servidor propio.
//! - La bóveda se guarda en un Gist secreto de esa cuenta, en el archivo `nook-boveda.json`.
//! - El token, el usuario y el Gist se guardan en %APPDATA%\nook\cuenta.dat, cifrados con la
//!   protección de datos de Windows: solo se pueden leer con este usuario de Windows.
//!
//! Lo que habla con GitHub va en un hilo propio, que recibe pedidos y deja eventos; tras dejar
//! uno llama a `despertar` para que la interfaz los recoja con `Cuenta::eventos`.
//!
//! Cuando cambia la bóveda gana el último que la guarda: al arrancar (y al abrir la bóveda) se
//! baja la de la cuenta y sustituye a la local, y cada cambio local se sube un momento después.

use crate::boveda::Boveda;
use crate::idioma::{tr, tr_con};
use crate::red;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windows::Foundation::Uri;
use windows::Storage::Streams::UnicodeEncoding;
use windows::Web::Http::{HttpMethod, HttpRequestMessage, HttpStringContent};
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData};
use windows_core::HSTRING;

/// Identificador de la OAuth App de nook en GitHub (es público: el flujo de dispositivo no usa
/// secreto). La app tiene que tener activado "Enable Device Flow".
const CLIENT_ID: &str = "Ov23lis2bCMFvLomh5aZ";
/// Nombre del archivo del Gist donde va la bóveda.
const ARCHIVO: &str = "nook-boveda.json";
const DESCRIPCION: &str = "Bóveda de textos de Nookly";
const API: &str = "https://api.github.com";
/// Tras un cambio en la bóveda se espera esto antes de subirla, por si llegan más seguidos.
const ESPERA_SUBIR: Duration = Duration::from_secs(2);
/// Al abrir la bóveda se mira si cambió en la cuenta, como mucho una vez cada tanto.
const ESPERA_DESCARGAR: Duration = Duration::from_secs(30);

/// Lo que cuenta el hilo de la cuenta a la interfaz.
pub enum Evento {
    /// Para iniciar sesión hay que escribir este código en `url`.
    Codigo { codigo: String, url: String },
    /// Sesión iniciada. `remota` es la bóveda que ya había en la cuenta, si había una.
    Conectada { remota: Option<Boveda> },
    /// La bóveda de la cuenta es distinta de la última que se vio (se cambió en otro ordenador).
    Descargada(Boveda),
    /// No se pudo iniciar sesión o sincronizar.
    Error(String),
    /// Se volvió a sincronizar después de un error.
    AlDia,
}

enum Pedido {
    Conectar,
    Cancelar,
    Desconectar,
    Subir(Boveda),
    Descargar,
}

/// Lado de la interfaz: hace pedidos al hilo de la cuenta y recoge sus eventos.
pub struct Cuenta {
    pedidos: Sender<Pedido>,
    eventos: Mutex<Receiver<Evento>>,
    /// Usuario de GitHub con la sesión iniciada (lo actualiza el hilo).
    usuario: Arc<Mutex<Option<String>>>,
    ultima_descarga: Mutex<Instant>,
}

impl Cuenta {
    /// Carga la sesión guardada (si hay) y arranca el hilo; con sesión, baja enseguida la bóveda
    /// de la cuenta.
    pub fn iniciar(despertar: impl Fn() + Send + 'static) -> Cuenta {
        let (pedidos, recibidos) = mpsc::channel();
        let (avisos, eventos) = mpsc::channel();
        let sesion = cargar_sesion();
        let usuario = Arc::new(Mutex::new(sesion.as_ref().map(|s| s.usuario.clone())));
        let hay_sesion = sesion.is_some();
        let mut hilo = Hilo {
            sesion,
            ultima: None,
            al_dia: false,
            sin_subir: None,
            con_error: false,
            usuario: usuario.clone(),
            avisos,
            despertar: Box::new(despertar),
        };
        let _ = std::thread::Builder::new().name("cuenta".into()).spawn(move || {
            red::iniciar_com();
            hilo.atender(recibidos);
        });
        if hay_sesion {
            let _ = pedidos.send(Pedido::Descargar);
        }
        Cuenta { pedidos, eventos: Mutex::new(eventos), usuario, ultima_descarga: Mutex::new(Instant::now()) }
    }

    /// Usuario de GitHub conectado.
    pub fn usuario(&self) -> Option<String> {
        self.usuario.lock().ok()?.clone()
    }

    /// Eventos que ha dejado el hilo desde la última vez.
    pub fn eventos(&self) -> Vec<Evento> {
        self.eventos.lock().map(|r| r.try_iter().collect()).unwrap_or_default()
    }

    /// Empieza a iniciar sesión: llegará un `Evento::Codigo` y luego `Conectada` o `Error`.
    pub fn conectar(&self) {
        let _ = self.pedidos.send(Pedido::Conectar);
    }

    /// Deja de esperar a que se escriba el código.
    pub fn cancelar(&self) {
        let _ = self.pedidos.send(Pedido::Cancelar);
    }

    /// Cierra la sesión. La bóveda sigue en este ordenador y en la cuenta.
    pub fn desconectar(&self) {
        if let Ok(mut u) = self.usuario.lock() {
            *u = None;
        }
        let _ = self.pedidos.send(Pedido::Desconectar);
    }

    /// Sube la bóveda a la cuenta (si hay sesión y cambió desde la última vez).
    pub fn subir(&self, boveda: &Boveda) {
        if self.usuario().is_some() {
            let _ = self.pedidos.send(Pedido::Subir(boveda.clone()));
        }
    }

    /// Mira si la bóveda de la cuenta cambió, si no se miró hace poco.
    pub fn descargar(&self) {
        if self.usuario().is_none() {
            return;
        }
        let Ok(mut ultima) = self.ultima_descarga.lock() else { return };
        if ultima.elapsed() >= ESPERA_DESCARGAR {
            *ultima = Instant::now();
            let _ = self.pedidos.send(Pedido::Descargar);
        }
    }
}

/// Lo que se guarda de la sesión en cuenta.dat.
#[derive(Serialize, Deserialize, Clone)]
struct Sesion {
    token: String,
    usuario: String,
    /// Gist con la bóveda (None hasta que se crea o se encuentra).
    gist: Option<String>,
}

struct Hilo {
    sesion: Option<Sesion>,
    /// Última bóveda que se sabe que hay en la cuenta.
    ultima: Option<Boveda>,
    /// Si ya se miró qué hay en la cuenta. Hasta entonces no se sube nada, para no pisar lo que
    /// se guardó desde otro ordenador.
    al_dia: bool,
    /// Bóveda que no se pudo subir (se reintenta en el próximo pedido).
    sin_subir: Option<Boveda>,
    /// Si lo último que se avisó fue un error al sincronizar.
    con_error: bool,
    usuario: Arc<Mutex<Option<String>>>,
    avisos: Sender<Evento>,
    despertar: Box<dyn Fn() + Send>,
}

/// Por qué falló una llamada a GitHub.
enum Fallo {
    /// El token ya no vale (se retiró el permiso en GitHub).
    SinPermiso,
    NoExiste,
    Otro(String),
}

impl Fallo {
    fn texto(&self) -> String {
        match self {
            Fallo::SinPermiso => tr("GitHub ya no da permiso").into(),
            Fallo::NoExiste => tr("no se encontró en GitHub").into(),
            Fallo::Otro(e) => e.clone(),
        }
    }
}

impl Hilo {
    fn atender(&mut self, pedidos: Receiver<Pedido>) {
        let mut siguiente = None;
        loop {
            let pedido = match siguiente.take() {
                Some(p) => p,
                None => match pedidos.recv() {
                    Ok(p) => p,
                    Err(_) => return,
                },
            };
            match pedido {
                Pedido::Conectar => self.conectar(&pedidos),
                Pedido::Cancelar => {}
                Pedido::Desconectar => self.desconectar(),
                Pedido::Descargar => {
                    self.descargar();
                }
                Pedido::Subir(mut boveda) => {
                    // Se espera un momento por si llegan más cambios: solo se sube el último.
                    loop {
                        match pedidos.recv_timeout(ESPERA_SUBIR) {
                            Ok(Pedido::Subir(otra)) => boveda = otra,
                            Ok(otro) => {
                                siguiente = Some(otro);
                                break;
                            }
                            Err(RecvTimeoutError::Timeout) => break,
                            Err(RecvTimeoutError::Disconnected) => {
                                self.subir(boveda);
                                return;
                            }
                        }
                    }
                    self.subir(boveda);
                }
            }
        }
    }

    fn avisar(&self, evento: Evento) {
        let _ = self.avisos.send(evento);
        (self.despertar)();
    }

    fn conectar(&mut self, pedidos: &Receiver<Pedido>) {
        if CLIENT_ID.is_empty() {
            self.avisar(Evento::Error(tr("falta el identificador de la app de GitHub").into()));
            return;
        }
        let token = match self.pedir_token(pedidos) {
            Ok(Some(token)) => token,
            Ok(None) => return,
            Err(e) => {
                self.avisar(Evento::Error(e));
                return;
            }
        };
        let conectada = (|| -> Result<_, Fallo> {
            let usuario = api(&token, HttpMethod::Get(), "/user", None)?["login"].as_str().unwrap_or_default().to_string();
            let gist = buscar_gist(&token)?;
            let remota = match &gist {
                Some(id) => leer_gist(&token, id)?,
                None => None,
            };
            Ok((Sesion { token: token.clone(), usuario, gist }, remota))
        })();
        match conectada {
            Ok((sesion, remota)) => {
                guardar_sesion(&sesion);
                if let Ok(mut u) = self.usuario.lock() {
                    *u = Some(sesion.usuario.clone());
                }
                self.sesion = Some(sesion);
                self.ultima = remota.clone();
                self.al_dia = true;
                self.sin_subir = None;
                self.con_error = false;
                self.avisar(Evento::Conectada { remota });
            }
            Err(e) => self.avisar(Evento::Error(tr_con("no se pudo leer la cuenta: {}", &[&e.texto()]))),
        }
    }

    /// Flujo de dispositivo: pide un código, lo enseña y espera a que se escriba en GitHub.
    /// Devuelve el token, o None si se canceló.
    fn pedir_token(&self, pedidos: &Receiver<Pedido>) -> Result<Option<String>, String> {
        let error_red = |e: String| tr_con("no se pudo hablar con GitHub: {}", &[&e]);
        let codigo = formulario("https://github.com/login/device/code", &format!("client_id={CLIENT_ID}&scope=gist")).map_err(error_red)?;
        let (Some(dispositivo), Some(usuario), Some(url)) =
            (codigo["device_code"].as_str(), codigo["user_code"].as_str(), codigo["verification_uri"].as_str())
        else {
            return Err(codigo["error_description"].as_str().unwrap_or(tr("GitHub no dio ningún código")).to_string());
        };
        self.avisar(Evento::Codigo { codigo: usuario.into(), url: url.into() });

        let mut intervalo = Duration::from_secs(codigo["interval"].as_u64().unwrap_or(5));
        let limite = Instant::now() + Duration::from_secs(codigo["expires_in"].as_u64().unwrap_or(900));
        let cuerpo = format!("client_id={CLIENT_ID}&device_code={dispositivo}&grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code");
        loop {
            match pedidos.recv_timeout(intervalo) {
                Ok(Pedido::Cancelar | Pedido::Desconectar) | Err(RecvTimeoutError::Disconnected) => return Ok(None),
                // Mientras se inicia sesión no hay bóveda que subir ni bajar.
                Ok(_) => continue,
                Err(RecvTimeoutError::Timeout) => {}
            }
            if Instant::now() > limite {
                return Err(tr("el código caducó").into());
            }
            let respuesta = formulario("https://github.com/login/oauth/access_token", &cuerpo).map_err(error_red)?;
            if let Some(token) = respuesta["access_token"].as_str() {
                return Ok(Some(token.to_string()));
            }
            match respuesta["error"].as_str() {
                Some("authorization_pending") => {}
                Some("slow_down") => intervalo = Duration::from_secs(respuesta["interval"].as_u64().unwrap_or(intervalo.as_secs() + 5)),
                Some("expired_token") => return Err(tr("el código caducó").into()),
                Some("access_denied") => return Err(tr("se rechazó el permiso en GitHub").into()),
                _ => return Err(respuesta["error_description"].as_str().unwrap_or(tr("GitHub no dio permiso")).to_string()),
            }
        }
    }

    fn desconectar(&mut self) {
        self.sesion = None;
        self.ultima = None;
        self.al_dia = false;
        self.sin_subir = None;
        if let Ok(mut u) = self.usuario.lock() {
            *u = None;
        }
        let _ = std::fs::remove_file(ruta_sesion());
    }

    /// Si GitHub ya no da permiso se cierra la sesión; si no, se avisa del error.
    fn fallo(&mut self, que: &str, fallo: Fallo) {
        if let Fallo::SinPermiso = fallo {
            self.desconectar();
            self.avisar(Evento::Error(tr("GitHub retiró el permiso: vuelve a conectar la cuenta").into()));
        } else {
            self.con_error = true;
            self.avisar(Evento::Error(format!("{que}: {}", fallo.texto())));
        }
    }

    /// Una sincronización salió bien: si antes hubo un error, se avisa de que ya pasó.
    fn bien(&mut self) {
        if self.con_error {
            self.con_error = false;
            self.avisar(Evento::AlDia);
        }
    }

    /// Baja la bóveda de la cuenta y avisa si es distinta de la última vista. Devuelve si se pudo.
    fn descargar(&mut self) -> bool {
        let Some(sesion) = self.sesion.clone() else { return false };
        // Lo que no se pudo subir antes va primero, si ya se sabe qué hay en la cuenta.
        if self.al_dia
            && let Some(boveda) = self.sin_subir.take()
        {
            self.subir(boveda);
        }
        let leida = (|| {
            // Sin Gist apuntado, se busca: puede haberlo creado otro ordenador.
            let gist = match &sesion.gist {
                Some(id) => Some(id.clone()),
                None => buscar_gist(&sesion.token)?,
            };
            let remota = match &gist {
                Some(id) => match leer_gist(&sesion.token, id) {
                    Err(Fallo::NoExiste) => None,
                    otro => otro?,
                },
                None => None,
            };
            Ok((if remota.is_some() { gist } else { None }, remota))
        })();
        let (gist, remota) = match leida {
            Ok(leida) => leida,
            Err(fallo) => {
                self.fallo(tr("no se pudo bajar la bóveda"), fallo);
                return false;
            }
        };
        if gist != sesion.gist
            && let Some(s) = &mut self.sesion
        {
            s.gist = gist;
            guardar_sesion(s);
        }
        self.al_dia = true;
        self.bien();
        if let Some(remota) = remota {
            if self.ultima.as_ref() != Some(&remota) {
                self.avisar(Evento::Descargada(remota.clone()));
            }
            self.ultima = Some(remota);
        }
        true
    }

    fn subir(&mut self, boveda: Boveda) {
        if self.sesion.is_none() {
            return;
        }
        if !self.al_dia {
            // Primero se mira qué hay en la cuenta. Si es otra bóveda, la interfaz la pondrá en
            // lugar de la local y no hay que subir esta.
            if !self.descargar() {
                self.sin_subir = Some(boveda);
                return;
            }
            if self.ultima.as_ref().is_some_and(|u| *u != boveda) {
                return;
            }
        }
        if self.ultima.as_ref() == Some(&boveda) {
            return;
        }
        let Some(sesion) = self.sesion.clone() else { return };
        let contenido = serde_json::to_string_pretty(&boveda).unwrap_or_else(|_| "[]".into());
        let escrito = match &sesion.gist {
            Some(id) => match escribir_gist(&sesion.token, id, &contenido) {
                // Se borró el Gist: se crea otro.
                Err(Fallo::NoExiste) => crear_gist(&sesion.token, &contenido).map(Some),
                otro => otro.map(|_| None),
            },
            None => crear_gist(&sesion.token, &contenido).map(Some),
        };
        match escrito {
            Ok(nuevo) => {
                if let (Some(id), Some(s)) = (nuevo, &mut self.sesion) {
                    s.gist = Some(id);
                    guardar_sesion(s);
                }
                self.ultima = Some(boveda);
                self.bien();
            }
            Err(fallo) => {
                self.sin_subir = Some(boveda);
                self.fallo(tr("no se pudo subir la bóveda"), fallo);
            }
        }
    }
}

// ---- GitHub ----

/// Busca entre los Gists de la cuenta el que tiene la bóveda.
fn buscar_gist(token: &str) -> Result<Option<String>, Fallo> {
    for pagina in 1..=10 {
        let lista = api(token, HttpMethod::Get(), &format!("/gists?per_page=100&page={pagina}"), None)?;
        let Some(lista) = lista.as_array().filter(|l| !l.is_empty()) else { break };
        if let Some(gist) = lista.iter().find(|g| g["files"].get(ARCHIVO).is_some()) {
            return Ok(gist["id"].as_str().map(String::from));
        }
    }
    Ok(None)
}

/// Bóveda guardada en el Gist (None si el Gist no tiene el archivo).
fn leer_gist(token: &str, id: &str) -> Result<Option<Boveda>, Fallo> {
    let gist = api(token, HttpMethod::Get(), &format!("/gists/{id}"), None)?;
    let archivo = &gist["files"][ARCHIVO];
    if archivo.is_null() {
        return Ok(None);
    }
    // Los archivos grandes vienen cortados: entonces se baja entero de su dirección.
    let contenido = if archivo["truncated"].as_bool() == Some(true) {
        let url = archivo["raw_url"].as_str().ok_or(Fallo::Otro(tr("el Gist no tiene dirección").into()))?;
        let r = peticion(HttpMethod::Get(), url, None, None, None).map_err(Fallo::Otro)?;
        r.1
    } else {
        archivo["content"].as_str().unwrap_or_default().to_string()
    };
    serde_json::from_str(&contenido).map(Some).map_err(|e| Fallo::Otro(format!("la bóveda de la cuenta no es válida ({e})")))
}

fn cuerpo_gist(contenido: &str) -> String {
    serde_json::json!({ "description": DESCRIPCION, "files": { ARCHIVO: { "content": contenido } } }).to_string()
}

fn escribir_gist(token: &str, id: &str, contenido: &str) -> Result<(), Fallo> {
    api(token, HttpMethod::Patch(), &format!("/gists/{id}"), Some(cuerpo_gist(contenido))).map(|_| ())
}

/// Crea un Gist secreto con la bóveda y devuelve su id.
fn crear_gist(token: &str, contenido: &str) -> Result<String, Fallo> {
    let mut cuerpo: serde_json::Value = serde_json::from_str(&cuerpo_gist(contenido)).unwrap_or_default();
    cuerpo["public"] = false.into();
    let gist = api(token, HttpMethod::Post(), "/gists", Some(cuerpo.to_string()))?;
    gist["id"].as_str().map(String::from).ok_or(Fallo::Otro(tr("GitHub no devolvió el Gist creado").into()))
}

/// Llamada a la API de GitHub con el token; devuelve su respuesta JSON.
fn api(token: &str, metodo: windows_core::Result<HttpMethod>, ruta: &str, cuerpo: Option<String>) -> Result<serde_json::Value, Fallo> {
    let cuerpo = cuerpo.map(|c| (c, "application/json"));
    let (estado, texto) = peticion(metodo, &format!("{API}{ruta}"), cuerpo, Some(token), Some("application/vnd.github+json")).map_err(Fallo::Otro)?;
    match estado {
        200..=299 => serde_json::from_str(&texto).map_err(|e| Fallo::Otro(e.to_string())),
        401 => Err(Fallo::SinPermiso),
        404 => Err(Fallo::NoExiste),
        _ => Err(Fallo::Otro(tr_con("GitHub respondió {}", &[&estado.to_string()]))),
    }
}

/// POST de un formulario a los puntos del inicio de sesión de GitHub; devuelve su respuesta JSON.
fn formulario(url: &str, cuerpo: &str) -> Result<serde_json::Value, String> {
    let (_, texto) = peticion(HttpMethod::Post(), url, Some((cuerpo.to_string(), "application/x-www-form-urlencoded")), None, Some("application/json"))?;
    serde_json::from_str(&texto).map_err(|e| e.to_string())
}

/// Petición HTTP; devuelve el código de estado y el cuerpo de la respuesta.
fn peticion(
    metodo: windows_core::Result<HttpMethod>,
    url: &str,
    cuerpo: Option<(String, &str)>,
    token: Option<&str>,
    acepta: Option<&str>,
) -> Result<(u16, String), String> {
    (|| -> windows_core::Result<(u16, String)> {
        let peticion = HttpRequestMessage::Create(&metodo?, &Uri::CreateUri(&HSTRING::from(url))?)?;
        let cabeceras = peticion.Headers()?;
        if let Some(acepta) = acepta {
            cabeceras.TryAppendWithoutValidation(&HSTRING::from("Accept"), &HSTRING::from(acepta))?;
        }
        if let Some(token) = token {
            cabeceras.TryAppendWithoutValidation(&HSTRING::from("Authorization"), &HSTRING::from(format!("Bearer {token}")))?;
        }
        if let Some((cuerpo, tipo)) = cuerpo {
            let contenido = HttpStringContent::CreateFromStringWithEncodingAndMediaType(&HSTRING::from(cuerpo), UnicodeEncoding::Utf8, &HSTRING::from(tipo))?;
            peticion.SetContent(&contenido)?;
        }
        let respuesta = red::cliente()?.SendRequestAsync(&peticion)?.join()?;
        let estado = respuesta.StatusCode()?.0 as u16;
        let texto = respuesta.Content()?.ReadAsStringAsync()?.join()?.to_string_lossy();
        Ok((estado, texto))
    })()
    .map_err(|e| e.message().to_string())
}

// ---- Sesión guardada ----

fn ruta_sesion() -> PathBuf {
    crate::config::ruta_archivo().with_file_name("cuenta.dat")
}

fn cargar_sesion() -> Option<Sesion> {
    let datos = descifrar(&std::fs::read(ruta_sesion()).ok()?)?;
    serde_json::from_slice(&datos).ok()
}

fn guardar_sesion(sesion: &Sesion) {
    let Some(datos) = serde_json::to_vec(sesion).ok().and_then(|d| cifrar(&d)) else { return };
    let ruta = ruta_sesion();
    if let Some(carpeta) = ruta.parent() {
        let _ = std::fs::create_dir_all(carpeta);
    }
    if let Err(e) = std::fs::write(&ruta, datos) {
        eprintln!("No se pudo guardar la sesión de GitHub: {e}");
    }
}

/// Cifra con la protección de datos de Windows (DPAPI), ligada al usuario de Windows.
fn cifrar(datos: &[u8]) -> Option<Vec<u8>> {
    let entrada = CRYPT_INTEGER_BLOB { cbData: datos.len() as u32, pbData: datos.as_ptr() as *mut u8 };
    let mut salida = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(&entrada, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut salida).ok()?;
        Some(tomar_blob(salida))
    }
}

fn descifrar(datos: &[u8]) -> Option<Vec<u8>> {
    let entrada = CRYPT_INTEGER_BLOB { cbData: datos.len() as u32, pbData: datos.as_ptr() as *mut u8 };
    let mut salida = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(&entrada, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut salida).ok()?;
        Some(tomar_blob(salida))
    }
}

/// Copia los datos que dejó Windows y libera su memoria.
unsafe fn tomar_blob(blob: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    unsafe {
        let datos = std::slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(blob.pbData as _)));
        datos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifra_y_descifra_con_windows() {
        let datos = br#"{"token":"abc","usuario":"yo","gist":null}"#;
        let cifrado = cifrar(datos).unwrap();
        assert_ne!(cifrado, datos);
        assert_eq!(descifrar(&cifrado).unwrap(), datos);
        assert!(descifrar(b"basura").is_none());
    }

    #[test]
    fn el_gist_lleva_la_boveda_en_su_archivo() {
        let cuerpo: serde_json::Value = serde_json::from_str(&cuerpo_gist(r#"["hola"]"#)).unwrap();
        assert_eq!(cuerpo["files"][ARCHIVO]["content"], r#"["hola"]"#);
    }
}
