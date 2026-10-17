//! Addons en la barra: sus botones en la tira, abrir su panel (que se compila la primera vez),
//! enlazarlo con su script, pasarle los colores y atender a la IA del usuario por MCP (ver
//! src/addons/mcp.rs).

use crate::addons::catalogo::{self, Entrada};
use crate::addons::guion::{self, Anfitrion, Espera, Guion};
use crate::addons::{self, Addon, conectar as ia, panel, tuberia};
use crate::*;
use serde_json::{Value as Json, json};
use slint_interpreter::{ComponentHandle as _, ComponentInstance, Value};
use std::sync::mpsc;

/// Lo más que una herramienta MCP se queda esperando al usuario antes de responder que vuelva a
/// intentarlo (los clientes suelen cortar hacia el minuto).
const ESPERA_MAXIMA: Duration = Duration::from_secs(50);

/// Lo de los addons que se ha ido cargando.
#[derive(Default)]
pub(crate) struct Addons {
    /// Los abiertos alguna vez, por su id: su script (y su estado) siguen cargados hasta que se
    /// cierra nook, para que lo que pasa en ellos (una partida) siga al hacer clic en otra
    /// ventana o al abrir otro addon.
    cargados: RefCell<HashMap<String, Cargado>>,
    /// El que está puesto en el panel de la barra.
    puesto: RefCell<Option<String>>,
    /// Herramientas MCP esperando a que su addon llame a `avisar`.
    esperas: RefCell<Vec<EsperaMcp>>,
    siguiente_espera: Cell<u64>,
}

struct Cargado {
    factoria: slint::ComponentFactory,
    /// El panel creado (solo mientras está puesto en la barra) y lo que hay que hacer en cuanto
    /// esté (el panel se crea al verse, un momento después de ponerlo).
    instancia: Rc<RefCell<Option<ComponentInstance>>>,
    pendientes: Rc<RefCell<Vec<Box<dyn FnOnce()>>>>,
    guion: Rc<Guion>,
}

/// La sección "Addons" de la configuración.
#[derive(Default)]
pub(crate) struct ConfigAddons {
    /// El catálogo (cuando ya se ha descargado), si se está descargando y el último error.
    catalogo: Option<Vec<Entrada>>,
    buscando: bool,
    error: String,
    /// Addons que se están instalando o quitando, y el último error de cada uno.
    ocupados: HashSet<String>,
    errores: HashMap<String, String>,
    /// Si nook está en Claude Desktop (se mira al arrancar y al conectarlo) y lo último que pasó
    /// al conectarlo.
    conectado_claude: bool,
    mensaje_conexion: String,
}

struct EsperaMcp {
    numero: u64,
    addon: String,
    evento: String,
    responder: mpsc::Sender<Json>,
    _limite: Timer,
}

impl Ctx {
    /// Pasa a la interfaz los botones de los addons que se ven en la tira.
    pub(crate) fn refrescar_addons(&self, ui: &Barra, e: &Estado) {
        let visibles = visibles(e);
        let botones: Vec<BotonAddonItem> = visibles
            .iter()
            .map(|a| BotonAddonItem { icono: a.manifiesto.icono.clone().into(), nombre: a.manifiesto.nombre().into() })
            .collect();
        ui.set_botones_addon(modelo(botones));
        let puesto = self.addons.puesto.borrow();
        let abierto = visibles.iter().position(|a| Some(&a.manifiesto.id) == puesto.as_ref());
        ui.set_addon_abierto(abierto.map_or(-1, |i| i as i32));
        drop(puesto);

        let c = &e.config_addons;
        ui.set_addons_config(modelo(filas(e).into_iter().map(|(_, fila)| fila).collect()));
        ui.set_buscando_addons(c.buscando);
        ui.set_error_addons(c.error.clone().into());
        ui.set_conectado_claude(c.conectado_claude);
        ui.set_mensaje_conexion(c.mensaje_conexion.clone().into());
    }

    /// Vuelve a leer los addons instalados (al arrancar y al instalar o quitar uno).
    pub(crate) fn leer_addons(&self) {
        self.estado.borrow_mut().addons = addons::instalados();
    }

    /// Botón de un addon en la tira: abre su panel o, si ya está abierto, lo cierra.
    pub(crate) fn alternar_addon(&self, posicion: usize) {
        let addon = visibles(&self.estado.borrow()).get(posicion).map(|a| (*a).clone());
        let Some(addon) = addon else { return };
        let abierto = self.estado.borrow().modo == Modo::Addon
            && self.addons.puesto.borrow().as_deref() == Some(addon.manifiesto.id.as_str());
        if abierto {
            self.cerrar();
        } else {
            self.abrir_addon(&addon);
        }
    }

    /// Abre el panel de un addon (lo carga si es la primera vez) y llama a su `al_abrir`. Si no
    /// se puede cargar, el panel lo dice.
    pub(crate) fn abrir_addon(&self, addon: &Addon) -> bool {
        let Some(ui) = self.ui.upgrade() else { return false };
        let id = addon.manifiesto.id.clone();
        ui.set_ancho_addon(addon.manifiesto.panel.ancho);
        ui.set_alto_addon(addon.manifiesto.panel.alto);
        let cargado = !self.addons.cargados.borrow().contains_key(&id);
        if cargado {
            match self.cargar_addon(addon) {
                Ok(c) => {
                    self.addons.cargados.borrow_mut().insert(id.clone(), c);
                }
                Err(e) => {
                    eprintln!("Addon {id}: {e}");
                    ui.set_error_addon(tr_con("No se pudo abrir este addon:\n{}", &[&e]).into());
                    *self.addons.puesto.borrow_mut() = Some(id);
                    ui.set_panel_addon(Default::default());
                    self.poner_modo(Modo::Addon);
                    return false;
                }
            }
        }
        ui.set_error_addon("".into());
        // Poner su panel en la barra (el que estaba se deshace: su script sigue cargado).
        let anterior = self.addons.puesto.replace(Some(id.clone()));
        if anterior.as_ref() != Some(&id) {
            if let Some(anterior) = anterior.and_then(|a| self.addons.cargados.borrow().get(&a).map(|c| c.instancia.clone())) {
                *anterior.borrow_mut() = None;
            }
            if let Some(c) = self.addons.cargados.borrow().get(&id) {
                ui.set_panel_addon(c.factoria.clone());
            }
        }
        self.poner_modo(Modo::Addon);
        self.cuando_este(&id, |guion| {
            if let Err(e) = guion.llamar("al_abrir", Vec::new()) {
                eprintln!("Addon: {e}");
            }
        });
        true
    }

    /// Hace algo con el script de un addon cargado en cuanto su panel esté creado.
    fn cuando_este(&self, id: &str, f: impl FnOnce(&Rc<Guion>) + 'static) {
        let cargados = self.addons.cargados.borrow();
        let Some(c) = cargados.get(id) else { return };
        let guion = c.guion.clone();
        if c.instancia.borrow().is_some() {
            drop(cargados);
            f(&guion);
        } else {
            c.pendientes.borrow_mut().push(Box::new(move || f(&guion)));
        }
    }

    /// Compila el panel y prepara el script.
    fn cargar_addon(&self, addon: &Addon) -> Result<Cargado, String> {
        let definicion = panel::compilar(&addon.carpeta)?;
        let instancia: Rc<RefCell<Option<ComponentInstance>>> = Rc::default();
        let pendientes: Rc<RefCell<Vec<Box<dyn FnOnce()>>>> = Rc::default();
        let anfitrion = self.anfitrion(&addon.manifiesto.id, &instancia);
        let guion = Guion::cargar(&addon.carpeta, addons::ruta_datos(&addon.manifiesto.id), anfitrion)?;

        let callbacks: Vec<String> = definicion.callbacks().collect();
        let (guardada, en_cola, g) = (instancia.clone(), pendientes.clone(), Rc::downgrade(&guion));
        let estado = self.estado.clone();
        let factoria = slint::ComponentFactory::new(move |contexto| {
            let creada = definicion
                .create_embedded(contexto)
                .map_err(|e| eprintln!("No se pudo crear el panel del addon: {e}"))
                .ok()?;
            aplicar_tema(&estado.borrow().config.ajustes, &creada);
            for nombre in &callbacks {
                let (g, funcion) = (g.clone(), nombre.replace('-', "_"));
                let _ = creada.set_callback(nombre, move |argumentos| {
                    let Some(guion) = g.upgrade() else { return Value::Void };
                    let argumentos = argumentos.iter().map(guion::de_valor).collect();
                    match guion.llamar(&funcion, argumentos) {
                        Ok(resultado) => guion::a_valor(&resultado),
                        Err(e) => {
                            eprintln!("Addon: {e}");
                            Value::Void
                        }
                    }
                });
            }
            *guardada.borrow_mut() = Some(creada.clone_strong());
            // Lo que esperaba al panel, justo después (ahora se está creando).
            let en_cola = en_cola.clone();
            Timer::single_shot(Duration::ZERO, move || {
                let pendientes = std::mem::take(&mut *en_cola.borrow_mut());
                for f in pendientes {
                    f();
                }
            });
            Some(creada)
        });
        Ok(Cargado { factoria, instancia, pendientes, guion })
    }

    /// Lo que el script de un addon le puede pedir a nook.
    fn anfitrion(&self, id: &str, instancia: &Rc<RefCell<Option<ComponentInstance>>>) -> Anfitrion {
        let (i, j) = (instancia.clone(), instancia.clone());
        let (c, d, f) = (self.clone(), self.clone(), self.clone());
        let id = id.to_string();
        Anfitrion {
            poner: Box::new(move |propiedad, valor| {
                let instancia = i.borrow();
                let Some(instancia) = instancia.as_ref() else { return Ok(()) };
                instancia.set_property(propiedad, valor).map_err(|e| format!("poner(\"{propiedad}\"): {e}"))
            }),
            leer: Box::new(move |propiedad| j.borrow().as_ref()?.get_property(propiedad).ok()),
            personaje: Box::new(move |que| match que {
                "rebote" => {
                    let mut p = c.personaje.borrow_mut();
                    let ahora = p.inicio.elapsed().as_secs_f64();
                    p.motor.rebotar(ahora);
                }
                "aviso" => c.notificar(),
                _ => eprintln!("Addon: personaje(\"{que}\") no existe"),
            }),
            cerrar: Box::new(move || {
                if d.estado.borrow().modo == Modo::Addon {
                    d.cerrar();
                }
            }),
            idioma: Box::new(|| idioma::codigo_actual().to_string()),
            avisar: Box::new(move |evento, valor| f.avisar_addon(&id, evento, &valor)),
        }
    }

    /// Pasa los colores elegidos al panel del addon puesto (cuando cambian).
    pub(crate) fn refrescar_tema_addon(&self, ajustes: &Ajustes) {
        let Some(puesto) = self.addons.puesto.borrow().clone() else { return };
        if let Some(c) = self.addons.cargados.borrow().get(&puesto) {
            if let Some(instancia) = c.instancia.borrow().as_ref() {
                aplicar_tema(ajustes, instancia);
            }
        }
    }

    // ---- Sección "Addons" de la configuración ----

    /// Descarga el catálogo (en otro hilo) y lo pasa a la sección.
    pub(crate) fn buscar_addons(&self) {
        {
            let mut e = self.estado.borrow_mut();
            if e.config_addons.buscando {
                return;
            }
            e.config_addons.buscando = true;
            e.config_addons.error.clear();
        }
        self.refrescar();
        let _ = std::thread::Builder::new().name("catalogo".into()).spawn(|| {
            let resultado = catalogo::descargar();
            en_la_interfaz(move |ctx| {
                {
                    let mut e = ctx.estado.borrow_mut();
                    e.config_addons.buscando = false;
                    match resultado {
                        Ok(catalogo) => e.config_addons.catalogo = Some(catalogo.addons),
                        Err(error) => e.config_addons.error = error,
                    }
                }
                ctx.refrescar();
            });
        });
    }

    fn id_de_fila(&self, fila: usize) -> Option<String> {
        let e = self.estado.borrow();
        filas(&e).into_iter().nth(fila).map(|(id, _)| id)
    }

    /// Instala (o actualiza) el addon de una fila, en otro hilo.
    fn instalar_addon(&self, fila: usize) {
        let Some(id) = self.id_de_fila(fila) else { return };
        let entrada = {
            let mut e = self.estado.borrow_mut();
            let entrada = e.config_addons.catalogo.as_ref().and_then(|c| c.iter().find(|a| a.id == id).cloned());
            if entrada.is_none() || !e.config_addons.ocupados.insert(id.clone()) {
                return;
            }
            e.config_addons.errores.remove(&id);
            entrada
        };
        let Some(entrada) = entrada else { return };
        self.refrescar();
        let _ = std::thread::Builder::new().name("instalar-addon".into()).spawn(move || {
            let resultado = catalogo::instalar(&entrada);
            en_la_interfaz(move |ctx| {
                // La versión que estuviera cargada se deja: la nueva se carga al abrirlo.
                ctx.descargar_addon(&entrada.id);
                {
                    let mut e = ctx.estado.borrow_mut();
                    e.config_addons.ocupados.remove(&entrada.id);
                    if let Err(error) = resultado {
                        e.config_addons.errores.insert(entrada.id.clone(), tr_con("No se pudo instalar: {}", &[&error]));
                    }
                }
                ctx.leer_addons();
                ctx.refrescar();
                ctx.recolocar();
            });
        });
    }

    /// Quita el addon de una fila (y lo que tenía guardado).
    fn quitar_addon(&self, fila: usize) {
        let Some(id) = self.id_de_fila(fila) else { return };
        self.descargar_addon(&id);
        if let Err(error) = catalogo::quitar(&id) {
            self.estado.borrow_mut().config_addons.errores.insert(id.clone(), tr_con("No se pudo quitar: {}", &[&error]));
        }
        self.leer_addons();
        self.modificar_ajustes(|a| a.addons_ocultos.retain(|oculto| *oculto != id));
    }

    /// Muestra u oculta en la tira el addon de una fila.
    fn alternar_addon_visible(&self, fila: usize) {
        let Some(id) = self.id_de_fila(fila) else { return };
        if self.addons.puesto.borrow().as_deref() == Some(id.as_str()) && self.estado.borrow().modo == Modo::Addon {
            self.cerrar();
        }
        self.modificar_ajustes(|a| {
            if a.addons_ocultos.contains(&id) {
                a.addons_ocultos.retain(|oculto| *oculto != id);
            } else {
                a.addons_ocultos.push(id);
            }
        });
    }

    /// Deja de tener cargado un addon (para instalar otra versión o quitarlo): sus herramientas
    /// MCP que esperaban responden que ya no está.
    fn descargar_addon(&self, id: &str) {
        self.addons.cargados.borrow_mut().remove(id);
        self.addons.esperas.borrow_mut().retain(|e| {
            if e.addon == id {
                let _ = e.responder.send(tuberia::error("el addon se ha cerrado"));
                false
            } else {
                true
            }
        });
        if self.addons.puesto.borrow().as_deref() == Some(id) {
            *self.addons.puesto.borrow_mut() = None;
            if let Some(ui) = self.ui.upgrade() {
                ui.set_panel_addon(Default::default());
            }
            if self.estado.borrow().modo == Modo::Addon {
                self.cerrar();
            }
        }
    }

    fn conectar_claude_desktop(&self) {
        let mensaje = match ia::con_claude_desktop() {
            Ok(_) => tr("Listo: reinicia Claude Desktop para que vea Nookly.").to_string(),
            Err(e) => e,
        };
        {
            let mut e = self.estado.borrow_mut();
            e.config_addons.conectado_claude = ia::conectado_con_claude_desktop();
            e.config_addons.mensaje_conexion = mensaje;
        }
        self.refrescar();
    }

    fn copiar_comando_claude_code(&self) {
        let mensaje = if portapapeles::poner(&ia::comando_claude_code()) {
            tr("Comando copiado: pégalo en una terminal.")
        } else {
            tr("No se pudo copiar el comando.")
        };
        self.estado.borrow_mut().config_addons.mensaje_conexion = mensaje.to_string();
        self.refrescar();
    }

    // ---- MCP ----

    /// Petición de `nook.exe --mcp` (ver addons/mcp.rs): la lista de herramientas o usar una. La
    /// respuesta va por `responder`.
    fn peticion_mcp(&self, peticion: Json, responder: mpsc::Sender<Json>) {
        match peticion["orden"].as_str() {
            Some("herramientas") => {
                let _ = responder.send(json!({ "herramientas": self.herramientas_mcp() }));
            }
            Some("llamar") => {
                let nombre = peticion["nombre"].as_str().unwrap_or_default();
                if let Err(e) = self.llamar_herramienta(nombre, &peticion["argumentos"], responder.clone()) {
                    let _ = responder.send(tuberia::error(&e));
                }
            }
            _ => {
                let _ = responder.send(tuberia::error("petición desconocida"));
            }
        }
    }

    /// Las herramientas de los addons que se ven en la tira, como las pide MCP.
    fn herramientas_mcp(&self) -> Vec<Json> {
        let e = self.estado.borrow();
        visibles(&e)
            .iter()
            .flat_map(|a| {
                let nombre = a.manifiesto.nombre();
                a.manifiesto.herramientas.iter().map(move |h| {
                    json!({
                        "name": format!("{}__{}", a.manifiesto.id, h.nombre),
                        "description": format!("[{nombre}] {}", h.descripcion),
                        "inputSchema": h.parametros,
                    })
                })
            })
            .collect()
    }

    /// Usa una herramienta MCP: abre el panel de su addon (así el usuario ve lo que hace su IA)
    /// y llama a `herramienta_<nombre>` de su script.
    fn llamar_herramienta(&self, nombre: &str, argumentos: &Json, responder: mpsc::Sender<Json>) -> Result<(), String> {
        let (id, herramienta) = nombre.split_once("__").ok_or_else(|| format!("herramienta desconocida: {nombre}"))?;
        let addon = visibles(&self.estado.borrow())
            .into_iter()
            .find(|a| a.manifiesto.id == id && a.manifiesto.herramientas.iter().any(|h| h.nombre == herramienta))
            .cloned()
            .ok_or_else(|| format!("herramienta desconocida: {nombre}"))?;
        let argumentos = rhai::serde::to_dynamic(argumentos).map_err(|e| e.to_string())?;
        if !self.abrir_addon(&addon) {
            return Err(self.ui.upgrade().map(|ui| ui.get_error_addon().to_string()).unwrap_or_default());
        }
        let (c, id, funcion) = (self.clone(), id.to_string(), format!("herramienta_{herramienta}"));
        self.cuando_este(&id.clone(), move |guion| {
            if !guion.tiene(&funcion, 1) {
                let _ = responder.send(tuberia::error(&format!("el addon no tiene {funcion}(argumentos)")));
                return;
            }
            match guion.llamar(&funcion, vec![argumentos]) {
                Ok(resultado) => match resultado.clone().try_cast::<Espera>() {
                    Some(espera) => c.esperar_addon(&id, espera, responder),
                    None => {
                        let _ = responder.send(json!({ "texto": como_texto(&resultado) }));
                    }
                },
                Err(e) => {
                    let _ = responder.send(tuberia::error(&e));
                }
            }
        });
        Ok(())
    }

    /// Deja una herramienta MCP esperando a que su addon llame a `avisar` con ese evento.
    fn esperar_addon(&self, id: &str, espera: Espera, responder: mpsc::Sender<Json>) {
        let numero = self.addons.siguiente_espera.get();
        self.addons.siguiente_espera.set(numero + 1);
        let limite = Timer::default();
        let c = self.clone();
        let segundos = Duration::from_secs(espera.segundos).min(ESPERA_MAXIMA);
        limite.start(TimerMode::SingleShot, segundos, move || {
            let mut esperas = c.addons.esperas.borrow_mut();
            if let Some(i) = esperas.iter().position(|e| e.numero == numero) {
                let espera = esperas.remove(i);
                let _ = espera.responder.send(json!({
                    "texto": "Todavía no ha pasado nada: vuelve a usar esta herramienta para seguir esperando.",
                }));
            }
        });
        self.addons.esperas.borrow_mut().push(EsperaMcp {
            numero,
            addon: id.to_string(),
            evento: espera.evento,
            responder,
            _limite: limite,
        });
    }

    /// `avisar` del script de un addon: responde a sus herramientas que esperaban ese evento.
    fn avisar_addon(&self, id: &str, evento: &str, valor: &rhai::Dynamic) {
        let texto = como_texto(valor);
        self.addons.esperas.borrow_mut().retain(|e| {
            if e.addon == id && e.evento == evento {
                let _ = e.responder.send(json!({ "texto": texto }));
                false
            } else {
                true
            }
        });
    }
}

/// Los addons que se ven en la tira (los instalados que funcionan con esta versión y que el
/// usuario no ha ocultado).
fn visibles(e: &Estado) -> Vec<&Addon> {
    let ocultos = &e.config.ajustes.addons_ocultos;
    e.addons.iter().filter(|a| a.compatible() && !ocultos.contains(&a.manifiesto.id)).collect()
}

/// Las filas de la sección "Addons", con su id: primero los instalados y luego los demás del
/// catálogo, cada grupo por su nombre.
fn filas(e: &Estado) -> Vec<(String, AddonFila)> {
    let c = &e.config_addons;
    let catalogo = c.catalogo.as_deref().unwrap_or_default();
    let ocultos = &e.config.ajustes.addons_ocultos;
    let en_catalogo = |id: &str| catalogo.iter().find(|entrada| entrada.id == id);
    let comun = |id: &str| (c.ocupados.contains(id), c.errores.get(id).cloned().unwrap_or_default());

    let mut instalados: Vec<(String, AddonFila)> = e
        .addons
        .iter()
        .map(|a| {
            let m = &a.manifiesto;
            let nueva = en_catalogo(&m.id)
                .filter(|entrada| entrada.compatible() && actualizacion::es_mas_nueva(&entrada.version, &m.version))
                .map(|entrada| entrada.version.clone())
                .unwrap_or_default();
            let (ocupado, error) = comun(&m.id);
            let fila = AddonFila {
                nombre: m.nombre().into(),
                descripcion: m.descripcion().into(),
                icono: m.icono.clone().into(),
                version: m.version.clone().into(),
                nueva: nueva.into(),
                instalado: true,
                visible: !ocultos.contains(&m.id),
                compatible: a.compatible(),
                ocupado,
                error: error.into(),
            };
            (m.id.clone(), fila)
        })
        .collect();
    let mut otros: Vec<(String, AddonFila)> = catalogo
        .iter()
        .filter(|entrada| !e.addons.iter().any(|a| a.manifiesto.id == entrada.id))
        .map(|entrada| {
            let (ocupado, error) = comun(&entrada.id);
            let fila = AddonFila {
                nombre: addons::traducido(&entrada.nombre).into(),
                descripcion: addons::traducido(&entrada.descripcion).into(),
                icono: entrada.icono.clone().into(),
                version: entrada.version.clone().into(),
                nueva: "".into(),
                instalado: false,
                visible: false,
                compatible: entrada.compatible(),
                ocupado,
                error: error.into(),
            };
            (entrada.id.clone(), fila)
        })
        .collect();
    otros.sort_by_key(|(_, fila)| fila.nombre.to_lowercase());
    instalados.append(&mut otros);
    instalados
}

/// Lo que devuelve el script, como texto para la IA: los textos tal cual, lo demás en JSON.
fn como_texto(valor: &rhai::Dynamic) -> String {
    if valor.is_unit() {
        "Hecho.".into()
    } else if valor.is_string() {
        valor.clone().into_string().unwrap_or_default()
    } else {
        serde_json::to_string(valor).unwrap_or_default()
    }
}

/// Lo mismo que `configuracion::aplicar_tema`, pero en el `Tema` del panel de un addon.
fn aplicar_tema(ajustes: &Ajustes, instancia: &ComponentInstance) {
    let (fondo, acento) = (color(&ajustes.color_fondo), color(&ajustes.color_acento));
    let valores = [
        ("fondo", Value::Brush(fondo.into())),
        ("acento", Value::Brush(acento.into())),
        ("claro", Value::Bool(es_claro(fondo))),
        ("acento-claro", Value::Bool(es_claro(acento))),
        ("opacidad", Value::Number(ajustes.opacidad_fondo() as f64)),
        ("fuente", Value::String(idioma::fuente().into())),
        ("pro", Value::Bool(PRO)),
    ];
    for (propiedad, valor) in valores {
        if let Err(e) = instancia.set_global_property(panel::TEMA, propiedad, valor) {
            eprintln!("No se pudo pasar {propiedad} al addon: {e}");
        }
    }
}

thread_local! {
    /// Para lo que llega de otros hilos (la tubería, las descargas) al hilo de la interfaz.
    static ATENDER: RefCell<Option<Ctx>> = const { RefCell::new(None) };
}

/// Hace algo en el hilo de la interfaz desde otro hilo.
fn en_la_interfaz(f: impl FnOnce(&Ctx) + Send + 'static) {
    let _ = slint::invoke_from_event_loop(move || {
        ATENDER.with(|a| {
            if let Some(ctx) = a.borrow().as_ref() {
                f(ctx);
            }
        });
    });
}

/// Conecta los eventos de la interfaz de esta parte y empieza a atender a `nook.exe --mcp`.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    ATENDER.with(|a| *a.borrow_mut() = Some(ctx.clone()));
    let c = ctx.clone();
    ui.on_alternar_addon(move |i| c.alternar_addon(i.max(0) as usize));
    let c = ctx.clone();
    ui.on_buscar_addons(move || c.buscar_addons());
    let c = ctx.clone();
    ui.on_instalar_addon(move |i| c.instalar_addon(i.max(0) as usize));
    let c = ctx.clone();
    ui.on_quitar_addon(move |i| c.quitar_addon(i.max(0) as usize));
    let c = ctx.clone();
    ui.on_alternar_addon_visible(move |i| c.alternar_addon_visible(i.max(0) as usize));
    let c = ctx.clone();
    ui.on_conectar_claude_desktop(move || c.conectar_claude_desktop());
    let c = ctx.clone();
    ui.on_copiar_comando_claude_code(move || c.copiar_comando_claude_code());
    ctx.estado.borrow_mut().config_addons.conectado_claude = ia::conectado_con_claude_desktop();
    // Con addons instalados, se mira enseguida si hay versiones nuevas.
    if !ctx.estado.borrow().addons.is_empty() {
        ctx.buscar_addons();
    }

    tuberia::servir(|peticion| {
        let (responder, respuesta) = mpsc::channel();
        let en_la_interfaz = responder.clone();
        let enviada = slint::invoke_from_event_loop(move || {
            ATENDER.with(|a| match a.borrow().as_ref() {
                Some(ctx) => ctx.peticion_mcp(peticion, en_la_interfaz),
                None => {
                    let _ = en_la_interfaz.send(tuberia::error("Nookly se está cerrando"));
                }
            });
        });
        if enviada.is_err() {
            let _ = responder.send(tuberia::error("Nookly se está cerrando"));
        }
        respuesta
    });
}
