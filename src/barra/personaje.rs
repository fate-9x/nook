//! El personaje de la barra (y el segundo, que aparece con varios clics seguidos): mira al cursor,
//! baila con la música, cuenta las notificaciones y rebota al hacer clic en él. Su figura es una
//! de las de la colección (las del programa y las de la carpeta de personajes del usuario, que se
//! vuelve a leer en cuanto cambia algo en ella).

use crate::*;
use config::Hueco;
use personaje::accesorio_archivo::Archivo;
use personaje::coleccion::{self, Coleccion};
use slint::ModelRc;
use std::collections::HashMap;

/// Personaje: su motor, el reloj con el que se anima, lo que sabe del cursor y lo último que se
/// dibujó (para no repetir cuadros que se verían iguales).
pub(crate) struct Personaje {
    pub(crate) motor: personaje::Motor,
    pub(crate) inicio: Instant,
    /// Última posición del cursor (píxeles de pantalla) y cuándo cambió.
    pub(crate) cursor: (i32, i32),
    pub(crate) movido: Instant,
    /// Cuadro dibujado, con el lado en píxeles y el aspecto (colores y accesorios) con que se
    /// dibujó.
    pub(crate) dibujado: Option<Dibujado>,
    /// Lo mismo para la vista previa de la configuración.
    pub(crate) dibujado_muestra: Option<Dibujado>,
    /// Cuándo llegó la notificación que está contando, si cuenta alguna.
    pub(crate) aviso: Option<Instant>,
    /// Golpes del ritmo de la música que suena.
    pub(crate) ritmo: musica::Ritmo,
    /// Segundo personaje, si está, y cuándo fueron los últimos clics seguidos en el primero.
    pub(crate) segundo: Option<Segundo>,
    pub(crate) clics: Vec<Instant>,
    /// Los personajes que hay, lo que había en su carpeta al leerlos (ver `coleccion::firma`;
    /// sin leer aún, None) y cuándo se miró si había cambiado.
    pub(crate) coleccion: Coleccion,
    pub(crate) firma: Option<u64>,
    pub(crate) revisado: Instant,
}

impl Personaje {
    pub(crate) fn new() -> Self {
        Self {
            motor: personaje::Motor::new(),
            inicio: Instant::now(),
            cursor: ventana::posicion_cursor(),
            movido: Instant::now() - CURSOR_QUIETO,
            dibujado: None,
            dibujado_muestra: None,
            aviso: None,
            ritmo: musica::Ritmo::new(),
            segundo: None,
            clics: Vec::new(),
            coleccion: Coleccion::default(),
            firma: None,
            revisado: Instant::now() - REVISAR_PERSONAJES,
        }
    }
}

/// Segundo personaje: aparece al lado del primero con `CLICS_SEGUNDO` clics seguidos en él y se
/// va a los `VIDA_SEGUNDO`. Hace lo mismo que el primero (mira al cursor, baila, cuenta los
/// avisos), con su propio reloj desfasado para que no parpadeen a la vez.
pub(crate) struct Segundo {
    pub(crate) motor: personaje::Motor,
    pub(crate) desde: Instant,
    pub(crate) dibujado: Option<Dibujado>,
}

/// Último cuadro dibujado de un personaje: el cuadro, el lado en píxeles y su aspecto.
pub(crate) type Dibujado = (personaje::Cuadro, u32, personaje::Aspecto);

/// Lado de la vista previa del personaje en la configuración, en píxeles lógicos (como
/// `lado-muestra` en ui/configuracion.slint), y sección de la configuración en la que está.
pub(crate) const LADO_MUESTRA: f32 = 120.0;
pub(crate) const SECCION_PERSONAJE: i32 = 4;

/// Clics seguidos en el personaje (cada uno a menos de `ENTRE_CLICS` del anterior) que hacen
/// aparecer al segundo, cuánto se queda y cuánto va adelantado su reloj.
pub(crate) const CLICS_SEGUNDO: usize = 5;
pub(crate) const ENTRE_CLICS: Duration = Duration::from_millis(600);
pub(crate) const VIDA_SEGUNDO: Duration = Duration::from_secs(120);
pub(crate) const DESFASE_SEGUNDO: f64 = 3.7;

/// Cada cuánto se mira si cambió algo en la carpeta de los personajes (mientras se anima).
pub(crate) const REVISAR_PERSONAJES: Duration = Duration::from_secs(1);

/// Carpeta de los personajes del usuario.
pub(crate) fn carpeta_personajes() -> PathBuf {
    config::carpeta().join("personajes")
}

/// Carpeta de los accesorios del usuario (los de un personaje van en la suya).
pub(crate) fn carpeta_accesorios() -> PathBuf {
    config::carpeta().join("accesorios")
}

/// Cada cuánto se calcula un cuadro del personaje.
pub(crate) const CUADRO_PERSONAJE: Duration = Duration::from_millis(16);
/// Con el cursor quieto este tiempo, el personaje deja de mirarlo y vuelve a su deriva.
pub(crate) const CURSOR_QUIETO: Duration = Duration::from_secs(4);
/// Giro máximo de la cabeza hacia el cursor, en grados, y distancia (píxeles lógicos) a la que el
/// giro llega a unos dos tercios del máximo.
pub(crate) const GIRO_MAXIMO: f32 = 35.0;
pub(crate) const CABECEO_MAXIMO: f32 = 26.0;
pub(crate) const DISTANCIA_MIRADA: f32 = 240.0;
/// El aviso de una notificación dura hasta que el cursor llega a la barra (pasado al menos el
/// mínimo, para que se vea el salto) o, como mucho, el máximo.
pub(crate) const AVISO_MINIMO: Duration = Duration::from_millis(1500);
pub(crate) const AVISO_MAXIMO: Duration = Duration::from_secs(8);
/// Diferencia por debajo de la cual un cuadro nuevo no se dibuja, en píxeles.
pub(crate) const TOLERANCIA_CUADRO: f32 = 0.15;

/// Dibuja el cuadro del instante `t` de un personaje si cambió lo bastante desde el último
/// dibujado (o el lado o el aspecto); si no, None y se deja el que hay.
pub(crate) fn dibujar_personaje(
    motor: &personaje::Motor,
    t: f64,
    lado: u32,
    aspecto: &personaje::Aspecto,
    dibujado: &mut Option<Dibujado>,
) -> Option<Image> {
    let cuadro = motor.muestrear(t);
    let tolerancia = TOLERANCIA_CUADRO / (lado as f32 * personaje::RADIO);
    if let Some((anterior, l, a)) = dibujado {
        if *l == lado && a == aspecto && anterior.casi_igual(&cuadro, tolerancia) {
            return None;
        }
    }
    let imagen = personaje::dibujar(&cuadro, lado, aspecto);
    *dibujado = Some((cuadro, lado, aspecto.clone()));
    Some(imagen)
}

/// Cómo se ve el personaje con los ajustes: el cuerpo del color de acento, los rasgos del color
/// que se lee sobre él (como `sobre-acento` en ui/tema.slint), el circulito de aviso (del color
/// elegido, si se eligió) y los accesorios.
pub(crate) fn aspecto_de(ajustes: &Ajustes, coleccion: &Coleccion) -> personaje::Aspecto {
    let acento = color(&ajustes.color_acento);
    let rasgos = if es_claro(acento) { [0x0B, 0x12, 0x30] } else { [0xFF, 0xFF, 0xFF] };
    personaje::Aspecto {
        cuerpo: [acento.red(), acento.green(), acento.blue()],
        rasgos,
        aviso: ajustes.personaje.color_aviso.as_deref().map(|c| {
            let c = color(c);
            [c.red(), c.green(), c.blue()]
        }),
        accesorios: accesorios_puestos(&ajustes.personaje, coleccion),
    }
}

/// Los accesorios que lleva puestos el personaje, listos para dibujarlo: los que trae el programa
/// y los de archivos que se encuentran (con los de otro personaje o que ya no están, no se dibuja
/// nada).
pub(crate) fn accesorios_puestos(p: &config::Personaje, coleccion: &Coleccion) -> Vec<personaje::Puesto> {
    p.accesorios
        .iter()
        .filter(|a| a.puesto)
        .filter_map(|a| {
            let Some(integrado) = TipoAccesorio::de_id(&a.tipo) else {
                let archivo = coleccion.accesorio(&a.tipo, &p.figura)?.clone();
                let colores = (0..archivo.ranuras.len()).map(|i| leer_hex(a.color_de(i)).map(|c| [c.red(), c.green(), c.blue()])).collect();
                return Some(personaje::Puesto::Archivo { archivo, colores });
            };
            let c = color(&a.color);
            let rgb = [c.red(), c.green(), c.blue()];
            let tipo = match integrado {
                TipoAccesorio::Audifonos => personaje::TipoAccesorio::Audifonos,
                TipoAccesorio::Corona => personaje::TipoAccesorio::Corona,
                TipoAccesorio::GorroDeLana => personaje::TipoAccesorio::GorroDeLana,
                TipoAccesorio::Cuernos => personaje::TipoAccesorio::Cuernos,
                TipoAccesorio::GafasDeSol => personaje::TipoAccesorio::GafasDeSol,
                TipoAccesorio::Monoculo => personaje::TipoAccesorio::Monoculo,
                TipoAccesorio::Antifaz => personaje::TipoAccesorio::Antifaz,
                TipoAccesorio::Mascarilla => personaje::TipoAccesorio::Mascarilla,
                TipoAccesorio::Corbatin => personaje::TipoAccesorio::Corbatin,
                TipoAccesorio::CapaDeVampiro => personaje::TipoAccesorio::CapaDeVampiro,
            };
            Some(personaje::Accesorio { tipo, color: rgb }.into())
        })
        .collect()
}

/// Un color de un accesorio que se puede poner, para la configuración: su nombre (vacío si solo
/// tiene uno) y, si es de un archivo, el del dibujo.
pub(crate) struct RanuraDisponible {
    pub(crate) nombre: String,
    pub(crate) original: Option<[u8; 3]>,
}

/// Un accesorio que se puede poner, para la configuración.
pub(crate) struct Disponible {
    pub(crate) id: String,
    pub(crate) nombre: String,
    pub(crate) ranuras: Vec<RanuraDisponible>,
}

/// Los accesorios que se pueden poner a la figura `figura`: los que trae el programa y los de
/// archivos (los de todos y los de ese personaje).
pub(crate) fn disponibles(coleccion: &Coleccion, figura: &str) -> Vec<Disponible> {
    let integrados = TipoAccesorio::TODOS.iter().map(|t| Disponible {
        id: t.id().to_string(),
        nombre: t.nombre().to_string(),
        ranuras: vec![RanuraDisponible { nombre: String::new(), original: None }],
    });
    let de_archivos = coleccion.accesorios_de(figura).map(|a: &Arc<Archivo>| Disponible {
        id: a.id.clone(),
        nombre: addons::traducido(&a.nombre),
        ranuras: a.ranuras.iter().map(|r| RanuraDisponible { nombre: addons::traducido(&r.nombre), original: Some(r.original) }).collect(),
    });
    integrados.chain(de_archivos).collect()
}

/// El hueco que ocupa cada accesorio de archivo que lo dice (los que trae el programa, ver
/// `config::hueco_integrado`).
fn huecos_de_archivos(coleccion: &Coleccion) -> HashMap<String, Hueco> {
    coleccion.accesorios.iter().filter_map(|a| a.hueco.map(|h| (a.id.clone(), h))).collect()
}

impl Ctx {
    /// La figura elegida en los ajustes (si ya no está, Nook).
    pub(crate) fn figura_elegida(&self) -> Arc<personaje::Figura> {
        let id = self.estado.borrow().config.ajustes.personaje.figura.clone();
        self.personaje.borrow().coleccion.buscar(&id)
    }

    /// Vuelve a leer los personajes si cambió algo en su carpeta (o, con `forzar`, siempre),
    /// mirándolo como mucho una vez por `REVISAR_PERSONAJES`.
    pub(crate) fn revisar_personajes(&self, forzar: bool) {
        {
            let mut p = self.personaje.borrow_mut();
            if !forzar && p.revisado.elapsed() < REVISAR_PERSONAJES {
                return;
            }
            p.revisado = Instant::now();
            let (personajes, accesorios) = (carpeta_personajes(), carpeta_accesorios());
            let firma = coleccion::firma(&personajes, &accesorios);
            if !forzar && p.firma == Some(firma) {
                return;
            }
            p.firma = Some(firma);
            p.coleccion = Coleccion::leer(&personajes, &accesorios, Some(&p.coleccion));
            for (carpeta, error) in &p.coleccion.errores {
                eprintln!("Personaje o accesorio {carpeta}: {error}");
            }
        }
        if let Some(ui) = self.ui.upgrade() {
            let personaje = self.estado.borrow().config.ajustes.personaje.clone();
            self.refrescar_personajes(&ui, &personaje.figura);
            self.refrescar_accesorios(&ui, &personaje);
        }
        self.poner_icono(false);
    }

    /// Pasa a la configuración los accesorios que se pueden poner al personaje elegido, con
    /// cada uno de sus colores y la fila de colores para elegirlos.
    pub(crate) fn refrescar_accesorios(&self, ui: &Barra, personaje: &config::Personaje) {
        let p = self.personaje.borrow();
        let paleta: Vec<Color> = personaje.paleta().iter().map(|c| color(c)).collect();
        let items = disponibles(&p.coleccion, &personaje.figura).into_iter().map(|d| {
            let puesto = personaje.accesorio(&d.id);
            let colores = d.ranuras.iter().enumerate().map(|(i, r)| {
                // Con el color del dibujo como primera opción, para volver a él.
                let con_original = r.original.is_some();
                let original = r.original.map(|[r, g, b]| Color::from_rgb_u8(r, g, b));
                let elegido = leer_hex(puesto.color_de(i)).or(original).unwrap_or_else(|| color(&puesto.color));
                let muestras: Vec<Color> = original.into_iter().chain(paleta.iter().copied()).collect();
                ColorAccesorio {
                    nombre: r.nombre.clone().into(),
                    color: elegido,
                    paleta: modelo(muestras),
                    base: (COLORES_ACCESORIOS.len() + con_original as usize) as i32,
                    desplazamiento: con_original as i32,
                }
            });
            AccesorioItem { id: d.id.into(), nombre: d.nombre.into(), puesto: puesto.puesto, colores: ModelRc::from(modelo(colores.collect())) }
        });
        ui.set_accesorios(modelo(items.collect()));
    }

    /// Pone o quita el accesorio `id`.
    pub(crate) fn alternar_accesorio(&self, id: &str) {
        let huecos = huecos_de_archivos(&self.personaje.borrow().coleccion);
        let hueco = |id: &str| config::hueco_integrado(id).or_else(|| huecos.get(id).copied());
        self.modificar_ajustes(|a| a.personaje.alternar(id, &hueco));
    }

    /// Cambia el color número `ranura` del accesorio `id` (y se lo pone). Si es el del dibujo,
    /// vuelve a él sin añadir nada a la fila de colores.
    pub(crate) fn poner_color_accesorio(&self, id: &str, ranura: usize, nuevo: Color) {
        let huecos = huecos_de_archivos(&self.personaje.borrow().coleccion);
        let hueco = |id: &str| config::hueco_integrado(id).or_else(|| huecos.get(id).copied());
        let original = self.ranura_original(id, ranura).map(|[r, g, b]| Color::from_rgb_u8(r, g, b));
        let texto = if original == Some(nuevo) { String::new() } else { hex(nuevo) };
        self.modificar_ajustes(|a| a.personaje.poner_color(id, ranura, texto, &hueco));
    }

    /// El color del dibujo de la ranura `ranura` del accesorio `id`, si es de un archivo.
    pub(crate) fn ranura_original(&self, id: &str, ranura: usize) -> Option<[u8; 3]> {
        let figura = self.estado.borrow().config.ajustes.personaje.figura.clone();
        let p = self.personaje.borrow();
        p.coleccion.accesorio(id, &figura)?.ranuras.get(ranura).map(|r| r.original)
    }

    /// Abre la carpeta de los accesorios en el Explorador (creándola si no existe).
    pub(crate) fn abrir_carpeta_accesorios(&self) {
        let carpeta = carpeta_accesorios();
        let _ = std::fs::create_dir_all(&carpeta);
        if let Err(e) = sistema::lanzar(&carpeta.to_string_lossy()) {
            eprintln!("{e}");
        }
    }

    /// Pasa a la configuración los personajes que hay, cuál es el elegido (`id`), sus expresiones
    /// (la normal y las suyas) con la elegida, y los que no se pudieron leer.
    pub(crate) fn refrescar_personajes(&self, ui: &Barra, id: &str) {
        let p = self.personaje.borrow();
        let figuras = &p.coleccion.figuras;
        let nombres = figuras.iter().map(|f| SharedString::from(addons::traducido(&f.nombre))).collect();
        ui.set_figuras(modelo(nombres));
        ui.set_figura(figuras.iter().position(|f| f.id == id).unwrap_or(0) as i32);
        let suyas = p.coleccion.buscar(id).expresiones.iter().map(|(n, _)| SharedString::from(n.as_str())).collect::<Vec<_>>();
        let elegida = self.estado.borrow().config.ajustes.personaje.expresiones.get(id).cloned();
        let i = elegida.and_then(|e| suyas.iter().position(|n| n.as_str() == e)).map_or(0, |i| i + 1);
        ui.set_expresiones(modelo(std::iter::once(SharedString::from(tr("Normal"))).chain(suyas).collect()));
        ui.set_expresion(i as i32);
        let errores: Vec<String> = p.coleccion.errores.iter().map(|(carpeta, error)| format!("{carpeta}: {error}")).collect();
        ui.set_errores_personajes(errores.join("\n").into());
    }

    /// Crea un personaje nuevo (una copia de Nook para cambiarla), lo elige y abre su carpeta.
    pub(crate) fn nuevo_personaje(&self) {
        let carpeta = carpeta_personajes();
        match coleccion::crear(&carpeta, tr("Mi personaje")) {
            Ok(id) => {
                self.revisar_personajes(true);
                self.elegir_figura(id.clone());
                if let Some(suya) = coleccion::carpeta_de(&carpeta, &id) {
                    let _ = sistema::lanzar(&suya.to_string_lossy());
                }
            }
            Err(e) => eprintln!("No se pudo crear el personaje: {e}"),
        }
    }

    /// Abre la carpeta de los personajes en el Explorador (creándola si no existe).
    pub(crate) fn abrir_carpeta_personajes(&self) {
        let carpeta = carpeta_personajes();
        let _ = std::fs::create_dir_all(&carpeta);
        if let Err(e) = sistema::lanzar(&carpeta.to_string_lossy()) {
            eprintln!("{e}");
        }
    }

    /// Elige la expresión `i` de la figura elegida (0, la normal; si no, la `i`-ésima suya).
    pub(crate) fn elegir_expresion(&self, i: usize) {
        let id = self.estado.borrow().config.ajustes.personaje.figura.clone();
        let nombre = i.checked_sub(1).and_then(|i| self.personaje.borrow().coleccion.buscar(&id).expresiones.get(i).map(|(n, _)| n.clone()));
        self.modificar_ajustes(|a| match nombre {
            Some(nombre) => {
                a.personaje.expresiones.insert(id, nombre);
            }
            None => {
                a.personaje.expresiones.remove(&id);
            }
        });
    }

    /// Elige la figura `id`: el personaje da un saltito.
    pub(crate) fn elegir_figura(&self, id: String) {
        if self.estado.borrow().config.ajustes.personaje.figura == id {
            return;
        }
        self.modificar_ajustes(|a| a.personaje.figura = id);
        let mut p = self.personaje.borrow_mut();
        let ahora = p.inicio.elapsed().as_secs_f64();
        p.motor.rebotar(ahora);
        if let Some(s) = p.segundo.as_mut() {
            s.motor.rebotar(ahora + DESFASE_SEGUNDO);
        }
    }

    /// Calcula el cuadro actual del personaje y lo dibuja si se ve distinto del anterior. Mira
    /// al cursor mientras se mueve; con el cursor quieto vuelve a su deriva (o, con la barra
    /// oculta, mira hacia la pantalla desde su escondite).
    pub(crate) fn animar_personaje(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        self.revisar_personajes(false);
        let (oculta, posicion) = {
            let e = self.estado.borrow();
            (e.modo == Modo::Oculta, e.config.ajustes.borde())
        };
        let escala = plataforma::escala();
        let (lx, ly, _, _) = plataforma::lienzo();
        let en_pantalla = |x: f32, y: f32| (lx as f32 + x * escala, ly as f32 + y * escala);
        let centro = en_pantalla(ui.get_centro_personaje_x(), ui.get_centro_personaje_y());
        let centro_2 = en_pantalla(ui.get_centro_personaje_2_x(), ui.get_centro_personaje_2_y());

        let mut p = self.personaje.borrow_mut();
        let ahora = p.inicio.elapsed().as_secs_f64();
        let cursor = ventana::posicion_cursor();
        if cursor != p.cursor {
            p.cursor = cursor;
            p.movido = Instant::now();
        }
        // Hacia dónde mira un personaje con centro `centro` (en píxeles de pantalla). Con el
        // cursor en un menú o en el selector de color no lo sigue: ahí se está eligiendo algo.
        let en_emergente = self.emergente.upgrade().is_some_and(|em| plataforma::cursor_dentro(em.window()));
        let siguiendo = !en_emergente && p.movido.elapsed() < CURSOR_QUIETO;
        let mirada = |centro: (f32, f32)| {
            if siguiendo {
                let (dx, dy) = ((cursor.0 as f32 - centro.0) / escala, (cursor.1 as f32 - centro.1) / escala);
                let mirar = personaje::Mirar {
                    guinada: GIRO_MAXIMO * dx / dx.hypot(DISTANCIA_MIRADA),
                    // El cabeceo es positivo hacia arriba y la y de la pantalla crece hacia abajo.
                    cabeceo: -CABECEO_MAXIMO * dy / dy.hypot(DISTANCIA_MIRADA),
                    mezcla: 1.0,
                    deriva: 0.0,
                };
                (mirar, personaje::Motor::SEGUIR)
            } else if oculta {
                let (guinada, cabeceo) = match posicion {
                    Posicion::Arriba => (0.0, -22.0),
                    Posicion::Abajo => (0.0, 22.0),
                    Posicion::Izquierda => (28.0, 0.0),
                    Posicion::Derecha => (-28.0, 0.0),
                };
                (personaje::Mirar { guinada, cabeceo, mezcla: 1.0, deriva: 1.0 }, 1.0)
            } else {
                (personaje::Mirar::LIBRE, 1.0)
            }
        };
        let (meta, duracion) = mirada(centro);
        p.motor.poner_mirada(meta, ahora, duracion);

        // El segundo se va pasado su rato.
        if p.segundo.as_ref().is_some_and(|s| s.desde.elapsed() > VIDA_SEGUNDO) {
            p.segundo = None;
            ui.set_hay_segundo(false);
        }
        let ahora_2 = ahora + DESFASE_SEGUNDO;
        if let Some(s) = p.segundo.as_mut() {
            let (meta, duracion) = mirada(centro_2);
            s.motor.poner_mirada(meta, ahora_2, duracion);
        }

        // Qué hace: contar el aviso (hasta que se ve o pasa un rato), bailar si suena música o
        // estar en reposo.
        if let Some(desde) = p.aviso {
            let visto = desde.elapsed() > AVISO_MINIMO && self.cursor_encima();
            if visto || desde.elapsed() > AVISO_MAXIMO {
                p.aviso = None;
                ui.set_aviso(false);
            }
        }
        let estado = if p.aviso.is_some() {
            personaje::Estado::Aviso
        } else if self.musica.sonando() {
            personaje::Estado::Musica
        } else {
            personaje::Estado::Reposo
        };
        p.motor.poner_estado(estado, ahora);
        let golpe = if estado == personaje::Estado::Musica { p.ritmo.muestra(ahora, self.musica.nivel()) } else { None };
        if let Some(fuerza) = golpe {
            p.motor.golpe(ahora, fuerza);
        }
        if let Some(s) = p.segundo.as_mut() {
            s.motor.poner_estado(estado, ahora_2);
            if let Some(fuerza) = golpe {
                s.motor.golpe(ahora_2, fuerza);
            }
        }

        let (figura, aspecto) = {
            let e = self.estado.borrow();
            let personaje = &e.config.ajustes.personaje;
            let figura = p.coleccion.buscar(&personaje.figura);
            let figura = match personaje.expresiones.get(&personaje.figura) {
                Some(expresion) => figura.con_expresion(expresion),
                None => figura,
            };
            (figura, aspecto_de(&e.config.ajustes, &p.coleccion))
        };
        if let Some(s) = p.segundo.as_mut() {
            s.motor.poner_figura(figura.clone());
        }
        p.motor.poner_figura(figura);
        let lado = (ui.get_lado_personaje() * escala).round().max(1.0) as u32;
        let p = &mut *p;
        if let Some(imagen) = dibujar_personaje(&p.motor, ahora, lado, &aspecto, &mut p.dibujado) {
            ui.set_personaje(imagen);
        }
        if let Some(s) = p.segundo.as_mut() {
            if let Some(imagen) = dibujar_personaje(&s.motor, ahora_2, lado, &aspecto, &mut s.dibujado) {
                ui.set_personaje_2(imagen);
            }
        }
        // Con la sección del personaje abierta, su vista previa hace lo mismo que el personaje.
        if ui.get_modo() == Modo::Config.numero() && ui.get_seccion() == SECCION_PERSONAJE {
            let lado = (LADO_MUESTRA * escala).round().max(1.0) as u32;
            if let Some(imagen) = dibujar_personaje(&p.motor, ahora, lado, &aspecto, &mut p.dibujado_muestra) {
                ui.set_personaje_muestra(imagen);
            }
        } else {
            p.dibujado_muestra = None;
        }
    }

    /// Empieza o deja de animar el personaje (sin verse, no se anima: no gasta).
    pub(crate) fn animar(&self, animar: bool) {
        if !animar {
            self.animador_personaje.stop();
        } else if !self.animador_personaje.running() {
            let c = self.clone();
            self.animador_personaje.start(TimerMode::Repeated, CUADRO_PERSONAJE, move || c.animar_personaje());
        }
    }

    /// Clic en un personaje (0 el primero, 1 el segundo): da un rebote y, si el reproductor no va
    /// en la tira, lo abre o lo cierra. Con `CLICS_SEGUNDO` clics seguidos en el primero aparece
    /// el segundo a su lado, de un salto (los clics seguidos no vuelven a abrir y cerrar el
    /// reproductor).
    pub(crate) fn clic_personaje(&self, cual: i32) {
        let seguido = {
            let mut p = self.personaje.borrow_mut();
            let ahora = p.inicio.elapsed().as_secs_f64();
            if cual == 1 {
                if let Some(s) = p.segundo.as_mut() {
                    s.motor.rebotar(ahora + DESFASE_SEGUNDO);
                }
                false
            } else {
                p.motor.rebotar(ahora);
                let seguido = p.clics.last().is_some_and(|ultimo| ultimo.elapsed() <= ENTRE_CLICS);
                if !seguido {
                    p.clics.clear();
                }
                p.clics.push(Instant::now());
                if p.clics.len() >= CLICS_SEGUNDO && p.segundo.is_none() {
                    p.clics.clear();
                    let mut motor = personaje::Motor::new();
                    motor.rebotar(ahora + DESFASE_SEGUNDO);
                    p.segundo = Some(Segundo { motor, desde: Instant::now(), dibujado: None });
                    if let Some(ui) = self.ui.upgrade() {
                        ui.set_hay_segundo(true);
                    }
                }
                seguido
            }
        };
        if !seguido && !self.estado.borrow().config.ajustes.reproductor_en_barra {
            self.alternar_reproductor();
        }
    }

    /// Llegó una notificación: el personaje lo cuenta (o vuelve a empezar si ya contaba otra) y,
    /// con la barra oculta, se asoma más.
    pub(crate) fn notificar(&self) {
        {
            let mut p = self.personaje.borrow_mut();
            let ahora = p.inicio.elapsed().as_secs_f64();
            if p.motor.estado() == personaje::Estado::Aviso {
                p.motor.reiniciar_estado(ahora);
            } else {
                p.motor.poner_estado(personaje::Estado::Aviso, ahora);
            }
            if let Some(s) = p.segundo.as_mut() {
                let ahora_2 = ahora + DESFASE_SEGUNDO;
                if s.motor.estado() == personaje::Estado::Aviso {
                    s.motor.reiniciar_estado(ahora_2);
                } else {
                    s.motor.poner_estado(personaje::Estado::Aviso, ahora_2);
                }
            }
            p.aviso = Some(Instant::now());
        }
        if let Some(ui) = self.ui.upgrade() {
            ui.set_aviso(true);
        }
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_clic_personaje(move |cual| c.clic_personaje(cual));
    let c = ctx.clone();
    ui.on_elegir_figura(move |i| {
        let id = c.personaje.borrow().coleccion.figuras.get(i as usize).map(|f| f.id.clone());
        if let Some(id) = id {
            c.elegir_figura(id);
        }
    });
    let c = ctx.clone();
    ui.on_elegir_expresion(move |i| c.elegir_expresion(i.max(0) as usize));
    let c = ctx.clone();
    ui.on_nuevo_personaje(move || {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.nuevo_personaje());
    });
    let c = ctx.clone();
    ui.on_abrir_carpeta_personajes(move || c.abrir_carpeta_personajes());
    let c = ctx.clone();
    ui.on_abrir_carpeta_accesorios(move || c.abrir_carpeta_accesorios());
}
