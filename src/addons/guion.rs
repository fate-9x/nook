//! Lógica de un addon: su `logica.rhai`, un script de Rhai que se ejecuta sin acceso a archivos
//! ni a la red, y con límites para que un bucle sin fin no congele la barra.
//!
//! Cada `callback` del panel llama a la función del script con su nombre y sus argumentos. Todas
//! las funciones se llaman con `this` puesto al estado del addon: un mapa que dura mientras el
//! addon está cargado (`this.cuenta += 1`). Desde el script se puede usar:
//!
//! - `poner("propiedad", valor)` y `leer("propiedad")`: las propiedades del panel.
//! - `guardar(valor)` y `cargar()`: datos que se conservan al cerrar nook.
//! - `azar()`: un número entre 0 y 1.
//! - `tras(milisegundos, "funcion")`: llama a esa función pasado ese tiempo.
//! - `personaje("rebote")` o `personaje("aviso")`: el personaje reacciona.
//! - `idioma()`: "es", "en", "ja" o "ko".
//! - `cerrar()`: cierra el panel.
//! - `avisar("evento", valor)`: responde a las herramientas MCP que esperan ese evento.
//! - `esperar("evento", segundos)`: lo que devuelve una herramienta MCP para esperar un `avisar`.
//!
//! Si existe, nook llama a `al_abrir()` cada vez que se abre el panel (el panel puede haberse
//! creado de nuevo: ahí hay que ponerle lo que tenga que mostrar). Las herramientas MCP del
//! manifiesto llaman a `herramienta_<nombre>(argumentos)`.

use rhai::{CallFnOptions, Dynamic, Engine, Scope, AST};
use slint::{Model, ModelRc, VecModel};
use slint_interpreter::Value;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Límites del script: operaciones por llamada, llamadas anidadas y tamaño de textos, listas y
/// mapas.
const MAX_OPERACIONES: u64 = 2_000_000;
const MAX_LLAMADAS: usize = 48;
const MAX_TEXTO: usize = 64 * 1024;
const MAX_LISTA: usize = 10_000;
/// Anidación de las expresiones, fuera y dentro de las funciones (Rhai pone otros límites más
/// bajos en las compilaciones de desarrollo: así un addon funciona igual en las dos).
const MAX_ANIDACION: usize = 64;
const MAX_ANIDACION_EN_FUNCIONES: usize = 48;

/// Lo que el script puede pedirle a nook (lo pone `barra::addons`).
pub struct Anfitrion {
    pub poner: Box<dyn Fn(&str, Value) -> Result<(), String>>,
    pub leer: Box<dyn Fn(&str) -> Option<Value>>,
    pub personaje: Box<dyn Fn(&str)>,
    pub cerrar: Box<dyn Fn()>,
    pub idioma: Box<dyn Fn() -> String>,
    pub avisar: Box<dyn Fn(&str, Dynamic)>,
}

/// Lo que devuelve `esperar`: la herramienta MCP no responde hasta que el script llame a
/// `avisar` con ese evento (o pasen esos segundos).
#[derive(Clone, Debug, PartialEq)]
pub struct Espera {
    pub evento: String,
    pub segundos: u64,
}

pub struct Guion {
    motor: Engine,
    ast: AST,
    /// `this` de las funciones: el estado del addon.
    estado: RefCell<Dynamic>,
    /// Hay una función del script en marcha: las llamadas que lleguen mientras tanto (un
    /// `poner` que dispara un callback, por ejemplo) se dejan para justo después.
    ocupado: Cell<bool>,
}

impl Guion {
    /// Lee y prepara `logica.rhai` (si el addon no tiene, uno vacío). `datos` es el archivo de
    /// `guardar` y `cargar`.
    pub fn cargar(carpeta: &Path, datos: PathBuf, anfitrion: Anfitrion) -> Result<Rc<Guion>, String> {
        let ruta = carpeta.join("logica.rhai");
        let codigo = match std::fs::read_to_string(&ruta) {
            Ok(codigo) => codigo,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(format!("logica.rhai: {e}")),
        };
        let anfitrion = Rc::new(anfitrion);
        let mut error = None;
        let guion = Rc::new_cyclic(|yo: &Weak<Guion>| {
            let motor = motor(yo.clone(), anfitrion, datos);
            let ast = motor.compile(&codigo).unwrap_or_else(|e| {
                error = Some(format!("logica.rhai: {e}"));
                AST::empty()
            });
            Guion { motor, ast, estado: RefCell::new(Dynamic::from_map(Default::default())), ocupado: Cell::new(false) }
        });
        match error {
            Some(e) => Err(e),
            None => Ok(guion),
        }
    }

    /// Si el script tiene la función `nombre` con `parametros` argumentos.
    pub fn tiene(&self, nombre: &str, parametros: usize) -> bool {
        self.ast.iter_functions().any(|f| f.name == nombre && f.params.len() == parametros)
    }

    /// Llama a una función del script (si no existe, no hace nada y devuelve `()`). Si ya había
    /// otra en marcha, la llama justo después y devuelve `()`.
    pub fn llamar(self: &Rc<Self>, nombre: &str, argumentos: Vec<Dynamic>) -> Result<Dynamic, String> {
        if !self.tiene(nombre, argumentos.len()) {
            return Ok(Dynamic::UNIT);
        }
        if self.ocupado.get() {
            let yo = Rc::downgrade(self);
            let nombre = nombre.to_string();
            slint::Timer::single_shot(Duration::ZERO, move || {
                if let Some(yo) = yo.upgrade() {
                    if let Err(e) = yo.llamar(&nombre, argumentos) {
                        eprintln!("Addon: {e}");
                    }
                }
            });
            return Ok(Dynamic::UNIT);
        }
        self.ocupado.set(true);
        let resultado = {
            let mut estado = self.estado.borrow_mut();
            let opciones = CallFnOptions::new().bind_this_ptr(&mut estado).eval_ast(false);
            self.motor.call_fn_with_options::<Dynamic>(opciones, &mut Scope::new(), &self.ast, nombre, argumentos)
        };
        self.ocupado.set(false);
        resultado.map_err(|e| format!("{nombre}: {e}"))
    }
}

/// Motor de Rhai con los límites y las funciones de nook.
fn motor(guion: Weak<Guion>, anfitrion: Rc<Anfitrion>, datos: PathBuf) -> Engine {
    // Con lo básico del lenguaje (operaciones, textos, listas, mapas…): Rhai no trae nada que
    // salga del script (archivos, red, procesos).
    let mut motor = Engine::new();
    motor
        .set_max_operations(MAX_OPERACIONES)
        .set_max_call_levels(MAX_LLAMADAS)
        .set_max_string_size(MAX_TEXTO)
        .set_max_array_size(MAX_LISTA)
        .set_max_map_size(MAX_LISTA)
        .set_max_expr_depths(MAX_ANIDACION, MAX_ANIDACION_EN_FUNCIONES);
    motor.on_print(|texto| eprintln!("Addon: {texto}"));
    motor.on_debug(|texto, _, _| eprintln!("Addon: {texto}"));

    let a = anfitrion.clone();
    motor.register_fn("poner", move |propiedad: &str, valor: Dynamic| -> Result<(), Box<rhai::EvalAltResult>> {
        (a.poner)(propiedad, a_valor(&valor)).map_err(|e| e.into())
    });
    let a = anfitrion.clone();
    motor.register_fn("leer", move |propiedad: &str| (a.leer)(propiedad).map(|v| de_valor(&v)).unwrap_or(Dynamic::UNIT));
    let a = anfitrion.clone();
    motor.register_fn("personaje", move |que: &str| (a.personaje)(que));
    let a = anfitrion.clone();
    motor.register_fn("cerrar", move || (a.cerrar)());
    let a = anfitrion.clone();
    motor.register_fn("idioma", move || (a.idioma)());
    let a = anfitrion;
    motor.register_fn("avisar", move |evento: &str, valor: Dynamic| (a.avisar)(evento, valor));
    motor.register_type_with_name::<Espera>("Espera");
    motor.register_fn("esperar", |evento: &str, segundos: rhai::INT| Espera {
        evento: evento.to_string(),
        segundos: segundos.max(0) as u64,
    });

    motor.register_fn("azar", || crate::azar::siguiente() as f64);

    let ruta = datos.clone();
    motor.register_fn("guardar", move |valor: Dynamic| -> Result<(), Box<rhai::EvalAltResult>> {
        let texto = serde_json::to_string(&valor).map_err(|e| e.to_string())?;
        crate::config::escribir_seguro(&ruta, texto.as_bytes()).map_err(|e| e.to_string().into())
    });
    let ruta = datos;
    motor.register_fn("cargar", move || {
        std::fs::read_to_string(&ruta)
            .ok()
            .and_then(|texto| serde_json::from_str::<Dynamic>(&texto).ok())
            .unwrap_or(Dynamic::UNIT)
    });

    motor.register_fn("tras", move |milisegundos: rhai::INT, funcion: &str| {
        let guion = guion.clone();
        let funcion = funcion.to_string();
        slint::Timer::single_shot(Duration::from_millis(milisegundos.max(0) as u64), move || {
            if let Some(guion) = guion.upgrade() {
                if let Err(e) = guion.llamar(&funcion, Vec::new()) {
                    eprintln!("Addon: {e}");
                }
            }
        });
    });
    motor
}

/// De un valor del script a uno de Slint: números, textos, booleanos, listas (modelos) y mapas
/// (structs). Lo demás queda vacío.
pub fn a_valor(valor: &Dynamic) -> Value {
    if let Some(n) = valor.as_int().ok() {
        Value::Number(n as f64)
    } else if let Some(n) = valor.as_float().ok() {
        Value::Number(n)
    } else if let Some(b) = valor.as_bool().ok() {
        Value::Bool(b)
    } else if valor.is_string() {
        Value::String(valor.clone().into_string().unwrap_or_default().into())
    } else if let Some(lista) = valor.read_lock::<rhai::Array>() {
        Value::Model(ModelRc::new(VecModel::from(lista.iter().map(a_valor).collect::<Vec<_>>())))
    } else if let Some(mapa) = valor.read_lock::<rhai::Map>() {
        Value::Struct(mapa.iter().map(|(clave, v)| (clave.to_string(), a_valor(v))).collect())
    } else {
        Value::Void
    }
}

/// De un valor de Slint a uno del script (los números enteros, como enteros).
pub fn de_valor(valor: &Value) -> Dynamic {
    match valor {
        Value::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => Dynamic::from_int(*n as rhai::INT),
        Value::Number(n) => Dynamic::from_float(*n),
        Value::Bool(b) => Dynamic::from_bool(*b),
        Value::String(s) => Dynamic::from(s.to_string()),
        Value::Model(modelo) => Dynamic::from_array(modelo.iter().map(|v| de_valor(&v)).collect()),
        Value::Struct(s) => Dynamic::from_map(s.iter().map(|(clave, v)| (clave.into(), de_valor(v))).collect()),
        _ => Dynamic::UNIT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sin_anfitrion() -> Anfitrion {
        Anfitrion {
            poner: Box::new(|_, _| Ok(())),
            leer: Box::new(|_| None),
            personaje: Box::new(|_| {}),
            cerrar: Box::new(|| {}),
            idioma: Box::new(|| "es".into()),
            avisar: Box::new(|_, _| {}),
        }
    }

    /// Un script de prueba en su propia carpeta temporal.
    fn guion(codigo: &str) -> Rc<Guion> {
        let carpeta = std::env::temp_dir().join(format!("nook-prueba-guion-{}", crate::azar::siguiente().to_bits()));
        std::fs::create_dir_all(&carpeta).unwrap();
        std::fs::write(carpeta.join("logica.rhai"), codigo).unwrap();
        let guion = Guion::cargar(&carpeta, carpeta.join("datos.json"), sin_anfitrion()).unwrap();
        let _ = std::fs::remove_dir_all(&carpeta);
        guion
    }

    #[test]
    fn convierte_valores_en_los_dos_sentidos() {
        let mut mapa = rhai::Map::new();
        mapa.insert("n".into(), Dynamic::from_int(3));
        mapa.insert("lista".into(), Dynamic::from_array(vec![Dynamic::from_bool(true), Dynamic::from("a".to_string())]));
        let original = Dynamic::from_map(mapa);
        let vuelta = de_valor(&a_valor(&original));
        let vuelta = vuelta.read_lock::<rhai::Map>().unwrap();
        assert_eq!(vuelta["n"].as_int(), Ok(3));
        let lista = vuelta["lista"].read_lock::<rhai::Array>().unwrap();
        assert_eq!(lista[0].as_bool(), Ok(true));
        assert_eq!(lista[1].clone().into_string().unwrap(), "a");
        assert_eq!(a_valor(&Dynamic::from_float(1.5)), Value::Number(1.5));
        assert_eq!(de_valor(&Value::Number(2.0)).as_int(), Ok(2));
    }

    #[test]
    fn el_estado_dura_entre_llamadas() {
        let g = guion("fn sumar() { if this.n == () { this.n = 0; } this.n += 1; this.n }");
        let _ = g.llamar("sumar", Vec::new()).unwrap();
        assert_eq!(g.llamar("sumar", Vec::new()).unwrap().as_int(), Ok(2));
        // Lo que no existe no hace nada.
        assert!(g.llamar("no_existe", Vec::new()).unwrap().is_unit());
    }

    #[test]
    fn un_bucle_sin_fin_se_corta() {
        let g = guion("fn bucle() { loop { } }");
        assert!(g.llamar("bucle", Vec::new()).is_err());
    }

    #[test]
    fn esperar_devuelve_la_espera() {
        let g = guion(r#"fn herramienta_turno(a) { esperar("disparo", 30) }"#);
        let espera = g.llamar("herramienta_turno", vec![Dynamic::UNIT]).unwrap().try_cast::<Espera>().unwrap();
        assert_eq!(espera, Espera { evento: "disparo".into(), segundos: 30 });
    }
}
