//! Grupos de aplicaciones y ajustes, guardados en %APPDATA%\nook\grupos.json.

use crate::atajos::Atajo;
use crate::boveda::Boveda;
use crate::idioma::tr;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Config {
    /// Lo que aparece en la tira de la barra, en orden: grupos y apps sueltas.
    /// (Los archivos antiguos lo llamaban `grupos`.)
    #[serde(alias = "grupos")]
    pub elementos: Vec<Elemento>,
    #[serde(default)]
    pub ajustes: Ajustes,
    /// Textos guardados en la bóveda, sueltos o en grupos, del más reciente al más antiguo.
    #[serde(default)]
    pub boveda: Boveda,
}

/// Un botón de la tira: un grupo que se despliega o una app que se abre directamente.
/// En el archivo se distinguen porque una app suelta tiene `ruta` y un grupo no.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Elemento {
    App(AppSuelta),
    Grupo(Grupo),
}

/// App suelta en la barra (sin grupo).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppSuelta {
    #[serde(flatten)]
    pub app: App,
    /// Atajo global que la abre.
    #[serde(default)]
    pub atajo: Option<Atajo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Grupo {
    pub nombre: String,
    #[serde(default)]
    pub apps: Vec<App>,
    /// Índice de la app cuyo icono representa al grupo en la barra.
    #[serde(default)]
    pub icono: usize,
    /// Atajo global que abre este grupo.
    #[serde(default)]
    pub atajo: Option<Atajo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct App {
    pub nombre: String,
    /// Ruta a un .exe, acceso directo (.lnk) o cualquier archivo que Windows sepa abrir.
    pub ruta: String,
}

/// Borde de la pantalla en el que se fija la barra.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Posicion {
    #[default]
    Arriba,
    Izquierda,
    Derecha,
    /// Solo con la barra de tareas de Windows sustituida (si no, estaría en su mismo sitio).
    Abajo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Ajustes {
    pub posicion: Posicion,
    /// Esquina superior izquierda del monitor donde está la barra (None = el principal).
    pub monitor: Option<(i32, i32)>,
    pub ocultar_en_reposo: bool,
    pub mostrar_hora: bool,
    /// Cómo se ve la fecha en la tira, junto a la hora (o si no se ve).
    pub fecha: FormatoFecha,
    /// Botón y atajo de la bóveda de textos.
    pub usar_boveda: bool,
    /// Animar el tamaño de la ventana al abrir y cerrar.
    pub animaciones: bool,
    pub color_fondo: String,
    pub color_acento: String,
    /// Fondo semitransparente, con lo que hay detrás desenfocado.
    pub transparencia: bool,
    /// Opacidad del fondo cuando hay transparencia, en porcentaje.
    pub opacidad: u8,
    /// Tamaño de la barra (y de sus menús), en porcentaje del normal.
    pub tamano: u8,
    /// Colores que el usuario añadió con "+" a cada fila de la paleta.
    pub fondos_propios: Vec<String>,
    pub acentos_propios: Vec<String>,
    /// Combinaciones de colores guardadas, de la más antigua a la más reciente.
    pub presets: Vec<Preset>,
    /// Muestra u oculta la barra (None = sin atajo).
    pub atajo_mostrar: Option<Atajo>,
    /// Abre la bóveda de textos y, con ella abierta, copia el texto marcado (None = sin atajo).
    pub atajo_boveda: Option<Atajo>,
    /// Silencia o activa el micrófono (None = sin atajo).
    pub atajo_microfono: Option<Atajo>,
    /// Abre (o cierra) el buscador para abrir una app (None = sin atajo).
    pub atajo_buscar: Option<Atajo>,
    /// Cómo se personalizó el personaje (sus accesorios).
    pub personaje: Personaje,
    /// Idioma de la interfaz.
    pub idioma: Idioma,
    /// Dónde va el reproductor de lo que suena: en la tira de la barra o (si no) en un panel que
    /// se abre con un clic en el personaje.
    pub reproductor_en_barra: bool,
    /// Botón del volumen general de Windows en la tira.
    pub boton_sonido: bool,
    /// Botón de la conversación con Claude Code en la tira, y la carpeta en la que trabaja
    /// ("" = la del usuario).
    pub boton_claude: bool,
    pub carpeta_claude: String,
    /// Botón para silenciar el micrófono en la tira.
    pub boton_microfono: bool,
    /// Botón para apagar, reiniciar o suspender el equipo en la tira.
    pub boton_energia: bool,
    /// Icono de la batería en la tira (solo se ve en los equipos con batería, como los portátiles).
    pub boton_bateria: bool,
    /// Botón de la lupa en la tira, para buscar una app instalada y abrirla.
    pub boton_buscar: bool,
    /// La tecla Windows sola abre ese buscador en vez del menú Inicio.
    pub tecla_windows_abre_buscador: bool,
    /// Ocultar la barra (y el personaje) mientras hay un juego o una app a pantalla completa.
    pub ocultar_en_pantalla_completa: bool,
    /// Mostrar en la tira, antes que las de la barra, las apps que tienen ventanas abiertas.
    pub apps_abiertas: bool,
    /// Sustituir la barra de tareas de Windows: mientras Nook está abierto, no se ve.
    pub sustituir_barra_de_windows: bool,
    /// Nook activó "ocultar automáticamente" en la barra de tareas de Windows para sustituirla y
    /// tiene que desactivarlo al devolverla (también si se cerró de golpe sin hacerlo).
    pub barra_de_windows_ocultandose: bool,
    /// Addons instalados que no se muestran en la tira (por su id).
    pub addons_ocultos: Vec<String>,
}

/// Idioma de la interfaz (ver `idioma.rs`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Idioma {
    Espanol,
    Ingles,
    Japones,
    Coreano,
    /// El de Windows (o el inglés, si no es ninguno de estos). También el de un idioma de una
    /// versión más nueva del programa que esta no tiene (por eso va el último).
    #[default]
    #[serde(other)]
    Sistema,
}

impl Idioma {
    /// Los que se pueden elegir, en el orden en que salen en la configuración.
    pub const TODOS: [Idioma; 5] = [Idioma::Sistema, Idioma::Espanol, Idioma::Ingles, Idioma::Japones, Idioma::Coreano];
}

/// Personalización del personaje: su figura, los accesorios que lleva y los colores que el
/// usuario añadió con "+" a la fila de colores de los accesorios.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Personaje {
    /// Id de la figura: "nook", "limon" o la de un personaje del usuario (ver
    /// `personaje::coleccion`). Si ya no existe, se ve Nook.
    #[serde(deserialize_with = "leer_figura")]
    pub figura: String,
    /// Solo los accesorios que se han tocado alguna vez (los demás, quitados y con su color
    /// inicial).
    pub accesorios: Vec<Accesorio>,
    pub colores_propios: Vec<String>,
    /// Color del circulito que sale cuando llega una notificación (de la fila de colores de los
    /// accesorios). Sin elegir, el de los ojos.
    pub color_aviso: Option<String>,
    /// La expresión elegida de cada figura que tiene alguna, por el id de la figura; las que no
    /// están, con la normal.
    pub expresiones: std::collections::BTreeMap<String, String>,
}

/// Formato de la fecha de la tira (ver `fecha.rs`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FormatoFecha {
    /// Sin fecha.
    Ninguna,
    /// 17/10/2026
    Numerica,
    /// Sábado, 17/10/2026
    ConDia,
    /// Sábado, 17 de octubre de 2026
    Texto,
    /// Sáb 17 oct. También el de un formato de una versión más nueva del programa que esta no
    /// conoce (por eso va el último).
    #[default]
    #[serde(other)]
    Corta,
}

impl FormatoFecha {
    /// Los que hay, en el orden en que salen en la configuración.
    pub const TODOS: [FormatoFecha; 5] =
        [FormatoFecha::Ninguna, FormatoFecha::Numerica, FormatoFecha::ConDia, FormatoFecha::Corta, FormatoFecha::Texto];
}

impl Default for Personaje {
    fn default() -> Self {
        Personaje {
            figura: FIGURA_INICIAL.into(),
            accesorios: Vec::new(),
            colores_propios: Vec::new(),
            color_aviso: None,
            expresiones: Default::default(),
        }
    }
}

/// La figura de siempre.
pub const FIGURA_INICIAL: &str = "nook";

/// Lee el id de la figura. Nook antes se llamaba «menta».
fn leer_figura<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let figura = String::deserialize(d)?;
    Ok(if figura == "menta" { FIGURA_INICIAL.into() } else { figura })
}

/// Un accesorio del personaje: si lo lleva puesto y de qué colores (se recuerdan aunque se quite).
/// `tipo` es su id: el de uno de los que trae el programa ("corona") o el de uno de los archivos
/// del usuario ("usuario/<carpeta>"). Si ya no existe (se borró su carpeta, o es de una versión más
/// nueva), se conserva pero no se muestra ni se dibuja.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Accesorio {
    pub tipo: String,
    pub puesto: bool,
    /// Su primer color (el único de los que trae el programa). Vacío: el del dibujo.
    pub color: String,
    /// Los colores siguientes de uno de archivo, en el orden de su dibujo; los que faltan o están
    /// vacíos, los del dibujo.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colores_extra: Vec<String>,
}

impl Accesorio {
    /// Su color número `ranura` (0 el primero): vacío si es el del dibujo.
    pub fn color_de(&self, ranura: usize) -> &str {
        if ranura == 0 { &self.color } else { self.colores_extra.get(ranura - 1).map_or("", String::as_str) }
    }

    fn poner_color(&mut self, ranura: usize, color: String) {
        if ranura == 0 {
            self.color = color;
        } else {
            if self.colores_extra.len() < ranura {
                self.colores_extra.resize(ranura, String::new());
            }
            self.colores_extra[ranura - 1] = color;
        }
    }
}

/// Los accesorios que trae el programa, en el orden en que salen en la personalización.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoAccesorio {
    Audifonos,
    Corona,
    GorroDeLana,
    Cuernos,
    GafasDeSol,
    Monoculo,
    Antifaz,
    Mascarilla,
    Corbatin,
    CapaDeVampiro,
}

/// Sitio del personaje que ocupa un accesorio: en cada uno solo cabe uno a la vez.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Hueco {
    /// Lo alto de la cabeza (corona, gorro, cuernos).
    Cabeza,
    /// Los ojos (gafas, monóculo, antifaz).
    Ojos,
    /// La boca (mascarilla).
    Boca,
    /// El cuello (corbatín).
    Cuello,
    /// La espalda (capa).
    Espalda,
}

impl TipoAccesorio {
    pub const TODOS: [TipoAccesorio; 10] = [
        TipoAccesorio::Audifonos,
        TipoAccesorio::Corona,
        TipoAccesorio::GorroDeLana,
        TipoAccesorio::Cuernos,
        TipoAccesorio::GafasDeSol,
        TipoAccesorio::Monoculo,
        TipoAccesorio::Antifaz,
        TipoAccesorio::Mascarilla,
        TipoAccesorio::Corbatin,
        TipoAccesorio::CapaDeVampiro,
    ];

    /// El id con el que se guarda.
    pub fn id(self) -> &'static str {
        match self {
            TipoAccesorio::Audifonos => "audifonos",
            TipoAccesorio::Corona => "corona",
            TipoAccesorio::GorroDeLana => "gorro_de_lana",
            TipoAccesorio::Cuernos => "cuernos",
            TipoAccesorio::GafasDeSol => "gafas_de_sol",
            TipoAccesorio::Monoculo => "monoculo",
            TipoAccesorio::Antifaz => "antifaz",
            TipoAccesorio::Mascarilla => "mascarilla",
            TipoAccesorio::Corbatin => "corbatin",
            TipoAccesorio::CapaDeVampiro => "capa_de_vampiro",
        }
    }

    /// El accesorio con ese id, si es de los que trae el programa.
    pub fn de_id(id: &str) -> Option<TipoAccesorio> {
        TipoAccesorio::TODOS.into_iter().find(|t| t.id() == id)
    }

    /// Su nombre, en el idioma de la interfaz.
    pub fn nombre(self) -> &'static str {
        match self {
            TipoAccesorio::Audifonos => tr("Audífonos"),
            TipoAccesorio::Corona => tr("Corona"),
            TipoAccesorio::GorroDeLana => tr("Gorro de lana"),
            TipoAccesorio::Cuernos => tr("Cuernos"),
            TipoAccesorio::GafasDeSol => tr("Gafas de sol"),
            TipoAccesorio::Monoculo => tr("Monóculo"),
            TipoAccesorio::Antifaz => tr("Antifaz"),
            TipoAccesorio::Mascarilla => tr("Mascarilla"),
            TipoAccesorio::Corbatin => tr("Corbatín"),
            TipoAccesorio::CapaDeVampiro => tr("Capa de vampiro"),
        }
    }

    /// El color con el que se pone la primera vez (uno de `COLORES_ACCESORIOS`).
    pub fn color_inicial(self) -> &'static str {
        let [lila, blanco, negro, rosa, amarillo, rojo] = COLORES_ACCESORIOS;
        match self {
            TipoAccesorio::Audifonos => lila,
            TipoAccesorio::Corona | TipoAccesorio::Monoculo => amarillo,
            TipoAccesorio::GorroDeLana => rosa,
            TipoAccesorio::Mascarilla => blanco,
            TipoAccesorio::Cuernos | TipoAccesorio::Corbatin => rojo,
            TipoAccesorio::GafasDeSol | TipoAccesorio::Antifaz | TipoAccesorio::CapaDeVampiro => negro,
        }
    }

    pub fn hueco(self) -> Option<Hueco> {
        match self {
            TipoAccesorio::Corona | TipoAccesorio::GorroDeLana | TipoAccesorio::Cuernos => Some(Hueco::Cabeza),
            TipoAccesorio::GafasDeSol | TipoAccesorio::Monoculo | TipoAccesorio::Antifaz => Some(Hueco::Ojos),
            TipoAccesorio::Mascarilla => Some(Hueco::Boca),
            TipoAccesorio::Corbatin => Some(Hueco::Cuello),
            TipoAccesorio::CapaDeVampiro => Some(Hueco::Espalda),
            TipoAccesorio::Audifonos => None,
        }
    }
}

/// El hueco del accesorio `id` si es de los que trae el programa (los de archivos dicen el suyo
/// en su `accesorio.json`: ver `personaje::coleccion`).
pub fn hueco_integrado(id: &str) -> Option<Hueco> {
    TipoAccesorio::de_id(id).and_then(TipoAccesorio::hueco)
}

/// El color con el que se pone la primera vez el accesorio `id`: el de los que trae el programa;
/// los de archivos, el de su dibujo (vacío).
fn color_inicial(id: &str) -> &'static str {
    TipoAccesorio::de_id(id).map_or("", TipoAccesorio::color_inicial)
}

/// Colores base de los accesorios (el usuario añade más con "+"): lila, blanco, negro, rosa,
/// amarillo y rojo.
pub const COLORES_ACCESORIOS: [&str; 6] = ["#9AA8F0", "#F4F5FA", "#22252D", "#FF7A9C", "#FFD86B", "#E0455A"];
/// Color con el que empieza el circulito de aviso cuando se deja de usar el de los ojos: el rojo.
pub const COLOR_AVISO_INICIAL: &str = COLORES_ACCESORIOS[5];

/// Combinación de fondo y acento guardada para volver a ella con un clic.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Preset {
    pub fondo: String,
    pub acento: String,
}

/// Cuántos presets se guardan como mucho (al añadir uno más se olvida el más antiguo).
pub const MAX_PRESETS: usize = 8;

/// Colores base de la paleta (el usuario añade más con "+"). El primero de cada fila es el de
/// por defecto: negro y verde.
/// Los acentos son tonos medios: se leen (contraste ≥ 3) sobre todos los fondos oscuros y sobre
/// el claro, y el texto blanco se lee sobre ellos (botones y interruptores).
pub const FONDOS: [&str; 4] = ["#111318", "#22252D", "#0F2A22", "#F4F5FA"];
pub const ACENTOS: [&str; 4] = ["#2E8B57", "#4A6CF7", "#B04FC9", "#C96A00"];
const FONDO_INICIAL: &str = FONDOS[0];
const ACENTO_INICIAL: &str = ACENTOS[0];

/// Opacidad mínima del fondo con transparencia (por debajo, el texto ya no se lee bien).
pub const OPACIDAD_MINIMA: u8 = 30;

/// Tamaños mínimo y máximo de la barra, en porcentaje.
pub const TAMANO_MINIMO: u8 = 80;
pub const TAMANO_MAXIMO: u8 = 150;

/// Cuántos colores propios caben en cada fila (al añadir uno más se olvida el más antiguo).
pub const MAX_COLORES_PROPIOS: usize = 5;

impl Default for Ajustes {
    fn default() -> Self {
        Self {
            posicion: Posicion::Arriba,
            monitor: None,
            ocultar_en_reposo: false,
            mostrar_hora: true,
            fecha: FormatoFecha::Corta,
            usar_boveda: true,
            animaciones: true,
            color_fondo: FONDO_INICIAL.into(),
            color_acento: ACENTO_INICIAL.into(),
            transparencia: true,
            opacidad: 70,
            tamano: 100,
            fondos_propios: Vec::new(),
            acentos_propios: Vec::new(),
            presets: Vec::new(),
            atajo_mostrar: Some(Atajo::alt_shift("Espacio")),
            atajo_boveda: Some(Atajo::alt_shift("V")),
            atajo_microfono: Some(Atajo::alt_shift("M")),
            atajo_buscar: Some(Atajo::alt_shift("B")),
            personaje: Personaje::default(),
            idioma: Idioma::Sistema,
            reproductor_en_barra: false,
            boton_sonido: true,
            boton_claude: true,
            carpeta_claude: String::new(),
            boton_microfono: true,
            boton_energia: true,
            boton_bateria: true,
            boton_buscar: true,
            tecla_windows_abre_buscador: false,
            ocultar_en_pantalla_completa: true,
            apps_abiertas: true,
            sustituir_barra_de_windows: false,
            barra_de_windows_ocultandose: false,
            addons_ocultos: Vec::new(),
        }
    }
}

impl Personaje {
    /// El accesorio `id` tal como está ahora (quitado y con sus colores iniciales si nunca se tocó).
    pub fn accesorio(&self, id: &str) -> Accesorio {
        self.accesorios.iter().find(|a| a.tipo == id).cloned().unwrap_or(Accesorio {
            tipo: id.to_string(),
            puesto: false,
            color: color_inicial(id).into(),
            colores_extra: Vec::new(),
        })
    }

    fn accesorio_mut(&mut self, id: &str) -> &mut Accesorio {
        let i = match self.accesorios.iter().position(|a| a.tipo == id) {
            Some(i) => i,
            None => {
                self.accesorios.push(self.accesorio(id));
                self.accesorios.len() - 1
            }
        };
        &mut self.accesorios[i]
    }

    /// Pone o quita el accesorio `id`. `hueco` dice el sitio que ocupa cada accesorio (ver
    /// `hueco_integrado`).
    pub fn alternar(&mut self, id: &str, hueco: &dyn Fn(&str) -> Option<Hueco>) {
        if self.accesorio(id).puesto {
            self.accesorio_mut(id).puesto = false;
        } else {
            self.poner(id, hueco);
        }
    }

    /// Pone el accesorio `id` y quita el que ocupara su mismo hueco.
    fn poner(&mut self, id: &str, hueco: &dyn Fn(&str) -> Option<Hueco>) {
        if let Some(sitio) = hueco(id) {
            for otro in &mut self.accesorios {
                if otro.tipo != id && hueco(&otro.tipo) == Some(sitio) {
                    otro.puesto = false;
                }
            }
        }
        self.accesorio_mut(id).puesto = true;
    }

    /// Cambia el color número `ranura` (0 el primero) del accesorio `id` (y se lo pone). Si el
    /// color no estaba en la fila, se añade a ella; vacío es el del dibujo (en uno de archivo).
    pub fn poner_color(&mut self, id: &str, ranura: usize, color: String, hueco: &dyn Fn(&str) -> Option<Hueco>) {
        if !color.is_empty() {
            self.agregar_a_la_fila(&color);
        }
        self.accesorio_mut(id).poner_color(ranura, color);
        self.poner(id, hueco);
    }

    /// Añade el color a la fila de colores propios si no estaba (olvidando el más antiguo si ya
    /// hay `MAX_COLORES_PROPIOS`).
    fn agregar_a_la_fila(&mut self, color: &str) {
        if !self.paleta().iter().any(|c| c.eq_ignore_ascii_case(color)) {
            self.colores_propios.push(color.to_string());
            if self.colores_propios.len() > MAX_COLORES_PROPIOS {
                self.colores_propios.remove(0);
            }
        }
    }

    /// Cambia el color del circulito de aviso. Si el color no estaba en la fila, se añade a ella.
    pub fn poner_color_aviso(&mut self, color: String) {
        self.agregar_a_la_fila(&color);
        self.color_aviso = Some(color);
    }

    /// Quita un color propio de la fila. Los accesorios que lo tenían vuelven a su color inicial
    /// (los de archivos, al de su dibujo) y el circulito de aviso, al de los ojos.
    pub fn quitar_color(&mut self, color: &str) {
        self.colores_propios.retain(|c| !c.eq_ignore_ascii_case(color));
        if self.color_aviso.as_deref().is_some_and(|c| c.eq_ignore_ascii_case(color)) {
            self.color_aviso = None;
        }
        for accesorio in &mut self.accesorios {
            if accesorio.color.eq_ignore_ascii_case(color) {
                accesorio.color = color_inicial(&accesorio.tipo).into();
            }
            for extra in &mut accesorio.colores_extra {
                if extra.eq_ignore_ascii_case(color) {
                    extra.clear();
                }
            }
        }
    }

    /// Fila de colores de los accesorios: los base y luego los propios.
    pub fn paleta(&self) -> Vec<String> {
        COLORES_ACCESORIOS.iter().map(|c| c.to_string()).chain(self.colores_propios.iter().cloned()).collect()
    }
}

impl Ajustes {
    /// El borde en el que va la barra: abajo solo con la barra de Windows sustituida; si se deja
    /// de sustituir, arriba (y vuelve abajo si se sustituye otra vez).
    pub fn borde(&self) -> Posicion {
        if self.posicion == Posicion::Abajo && !self.sustituir_barra_de_windows { Posicion::Arriba } else { self.posicion }
    }

    /// Apaga lo que solo tiene nook Pro: en la versión gratuita no se puede encender, aunque lo
    /// hiciera una versión anterior.
    pub fn quitar_funciones_pro(&mut self) {
        self.sustituir_barra_de_windows = false;
        self.apps_abiertas = false;
        self.ocultar_en_pantalla_completa = false;
        self.boton_buscar = false;
        self.tecla_windows_abre_buscador = false;
        self.atajo_buscar = None;
        self.reproductor_en_barra = false;
        self.boton_sonido = false;
        self.boton_claude = false;
        self.boton_microfono = false;
        self.boton_energia = false;
        self.boton_bateria = false;
        self.atajo_microfono = None;
    }

    /// Tamaño de la interfaz (1 = normal).
    pub fn tamano_interfaz(&self) -> f32 {
        self.tamano.clamp(TAMANO_MINIMO, TAMANO_MAXIMO) as f32 / 100.0
    }

    /// Opacidad con la que se dibuja el fondo, de 0 a 1 (1 sin transparencia).
    pub fn opacidad_fondo(&self) -> f32 {
        if self.transparencia { self.opacidad.clamp(OPACIDAD_MINIMA, 100) as f32 / 100.0 } else { 1.0 }
    }

    /// Combinación de colores actual.
    pub fn preset_actual(&self) -> Preset {
        Preset { fondo: self.color_fondo.clone(), acento: self.color_acento.clone() }
    }

    /// Si la combinación actual ya está guardada.
    pub fn preset_guardado(&self) -> bool {
        let actual = self.preset_actual();
        self.presets.iter().any(|p| {
            p.fondo.eq_ignore_ascii_case(&actual.fondo) && p.acento.eq_ignore_ascii_case(&actual.acento)
        })
    }

    /// Guarda la combinación actual (si no estaba ya), olvidando la más antigua si no caben más.
    pub fn guardar_preset(&mut self) {
        if self.preset_guardado() {
            return;
        }
        self.presets.push(self.preset_actual());
        if self.presets.len() > MAX_PRESETS {
            self.presets.remove(0);
        }
    }

    /// Cambia un color (el fondo o el acento) del preset `indice`. Si así queda igual que otro
    /// preset, sobra y se quita.
    pub fn editar_preset(&mut self, indice: usize, acento: bool, color: String) {
        let Some(preset) = self.presets.get_mut(indice) else { return };
        if acento {
            preset.acento = color;
        } else {
            preset.fondo = color;
        }
        self.quitar_preset_repetido(indice);
    }

    /// Pone la combinación de colores actual en el preset `indice`.
    pub fn reemplazar_preset(&mut self, indice: usize) {
        let actual = self.preset_actual();
        let Some(preset) = self.presets.get_mut(indice) else { return };
        *preset = actual;
        self.quitar_preset_repetido(indice);
    }

    /// Quita el preset `indice` si hay otro con los mismos colores.
    fn quitar_preset_repetido(&mut self, indice: usize) {
        let Some(editado) = self.presets.get(indice).cloned() else { return };
        let repetido = self.presets.iter().enumerate().any(|(i, p)| {
            i != indice && p.fondo.eq_ignore_ascii_case(&editado.fondo) && p.acento.eq_ignore_ascii_case(&editado.acento)
        });
        if repetido {
            self.presets.remove(indice);
        }
    }

    /// Colores de la fila de fondos (o de acentos): los base y luego los propios.
    pub fn paleta(&self, acento: bool) -> Vec<String> {
        let (base, propios) = if acento { (&ACENTOS, &self.acentos_propios) } else { (&FONDOS, &self.fondos_propios) };
        base.iter().map(|c| c.to_string()).chain(propios.iter().cloned()).collect()
    }

    /// Elige un color para el fondo o el acento y, si no estaba en su fila, lo añade a ella.
    pub fn poner_color(&mut self, acento: bool, color: String) {
        let (base, propios, actual) = if acento {
            (&ACENTOS, &mut self.acentos_propios, &mut self.color_acento)
        } else {
            (&FONDOS, &mut self.fondos_propios, &mut self.color_fondo)
        };
        let ya_esta = base.iter().map(|c| c.to_string()).chain(propios.iter().cloned()).any(|c| c.eq_ignore_ascii_case(&color));
        if !ya_esta {
            propios.push(color.clone());
            if propios.len() > MAX_COLORES_PROPIOS {
                propios.remove(0);
            }
        }
        *actual = color;
    }

    /// Quita un color propio de su fila. Si era el elegido, se vuelve al de por defecto.
    pub fn quitar_color(&mut self, acento: bool, color: &str) {
        let (propios, actual, inicial) = if acento {
            (&mut self.acentos_propios, &mut self.color_acento, ACENTO_INICIAL)
        } else {
            (&mut self.fondos_propios, &mut self.color_fondo, FONDO_INICIAL)
        };
        propios.retain(|c| !c.eq_ignore_ascii_case(color));
        if actual.eq_ignore_ascii_case(color) {
            *actual = inicial.into();
        }
    }

    /// Aplica un preset; sus colores se añaden a la paleta si no estaban.
    pub fn aplicar_preset(&mut self, preset: &Preset) {
        self.poner_color(false, preset.fondo.clone());
        self.poner_color(true, preset.acento.clone());
    }

    /// Los colores elegidos deben verse en su fila (los de una versión anterior con otra paleta
    /// pasan a ser colores propios).
    fn asegurar_colores_en_paleta(&mut self) {
        self.poner_color(false, self.color_fondo.clone());
        self.poner_color(true, self.color_acento.clone());
    }

    /// Las versiones anteriores usaban azul y rojo por defecto y los guardaban en el archivo:
    /// quien los tenga pasa a los nuevos colores por defecto.
    fn actualizar_colores_antiguos(&mut self) {
        if self.color_fondo.eq_ignore_ascii_case("#00125C") && self.color_acento.eq_ignore_ascii_case("#DE3B3B") {
            self.color_fondo = FONDO_INICIAL.into();
            self.color_acento = ACENTO_INICIAL.into();
        }
    }
}

impl App {
    /// Crea una aplicación usando el nombre del archivo (sin extensión) como nombre visible.
    pub fn desde_ruta(ruta: &Path) -> Self {
        let nombre = ruta
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| ruta.display().to_string());
        Self { nombre, ruta: ruta.display().to_string() }
    }

    /// Primera letra del nombre en mayúscula (se muestra cuando no hay icono).
    pub fn inicial(&self) -> String {
        inicial(&self.nombre)
    }
}

impl Grupo {
    pub fn nuevo(nombre: &str) -> Self {
        Self { nombre: nombre.into(), apps: Vec::new(), icono: 0, atajo: None }
    }

    /// App que representa al grupo (la elegida, o la primera si ya no existe).
    pub fn app_icono(&self) -> Option<&App> {
        self.apps.get(self.icono).or_else(|| self.apps.first())
    }
}

impl Elemento {
    pub fn app(app: App) -> Self {
        Elemento::App(AppSuelta { app, atajo: None })
    }

    pub fn nombre(&self) -> &str {
        match self {
            Elemento::App(s) => &s.app.nombre,
            Elemento::Grupo(g) => &g.nombre,
        }
    }

    pub fn atajo(&self) -> Option<&Atajo> {
        match self {
            Elemento::App(s) => s.atajo.as_ref(),
            Elemento::Grupo(g) => g.atajo.as_ref(),
        }
    }

    pub fn atajo_mut(&mut self) -> &mut Option<Atajo> {
        match self {
            Elemento::App(s) => &mut s.atajo,
            Elemento::Grupo(g) => &mut g.atajo,
        }
    }

    /// App cuyo icono se muestra en la tira.
    pub fn app_icono(&self) -> Option<&App> {
        match self {
            Elemento::App(s) => Some(&s.app),
            Elemento::Grupo(g) => g.app_icono(),
        }
    }

    pub fn grupo(&self) -> Option<&Grupo> {
        match self {
            Elemento::Grupo(g) => Some(g),
            Elemento::App(_) => None,
        }
    }

    pub fn grupo_mut(&mut self) -> Option<&mut Grupo> {
        match self {
            Elemento::Grupo(g) => Some(g),
            Elemento::App(_) => None,
        }
    }
}

pub fn inicial(nombre: &str) -> String {
    nombre.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_else(|| "?".into())
}

impl Config {
    /// Da a cada elemento sin atajo, desde el de índice `desde` (los recién añadidos), el primer
    /// `Alt+Shift+número` libre (del 1 al 9). Los anteriores se dejan como están: si no tienen
    /// atajo es porque el usuario lo quitó.
    pub fn completar_atajos(&mut self, desde: usize) {
        for i in desde..self.elementos.len() {
            if self.elementos[i].atajo().is_some() {
                continue;
            }
            let libre = (1..=9).map(|d| Atajo::alt_shift(&d.to_string())).find(|a| {
                let usado = |otro: &Option<Atajo>| otro.as_ref() == Some(a);
                !usado(&self.ajustes.atajo_mostrar)
                    && !usado(&self.ajustes.atajo_boveda)
                    && !usado(&self.ajustes.atajo_microfono)
                    && !usado(&self.ajustes.atajo_buscar)
                    && self.elementos.iter().all(|e| e.atajo() != Some(a))
            });
            *self.elementos[i].atajo_mut() = libre;
        }
    }
}

pub fn ruta_archivo() -> PathBuf {
    carpeta().join("grupos.json")
}

/// Carga los grupos. Si el archivo no existe se crea uno de ejemplo; si está dañado
/// se guarda una copia (.dañado) para no perder nada y se empieza con el ejemplo.
pub fn cargar() -> Config {
    let ruta = ruta_archivo();
    let mut config = match fs::read_to_string(&ruta) {
        Ok(texto) => match serde_json::from_str(&texto) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("grupos.json no es válido ({e}); se guarda una copia y se usa el ejemplo");
                let _ = fs::copy(&ruta, ruta.with_extension("json.dañado"));
                ejemplo()
            }
        },
        Err(_) => ejemplo(),
    };
    config.ajustes.actualizar_colores_antiguos();
    config.ajustes.asegurar_colores_en_paleta();
    if !crate::PRO {
        config.ajustes.quitar_funciones_pro();
    }
    config
}

/// Carpeta de nook en %APPDATA% (la de grupos.json).
pub fn carpeta() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| ".".into());
    base.join("nook")
}

pub fn guardar(config: &Config) -> io::Result<()> {
    escribir_seguro(&ruta_archivo(), serde_json::to_string_pretty(config)?.as_bytes())
}

/// Escribe un archivo (creando su carpeta si hace falta) primero en uno temporal y luego lo
/// reemplaza: si se corta a medias, el anterior no se pierde.
pub fn escribir_seguro(ruta: &Path, contenido: &[u8]) -> io::Result<()> {
    if let Some(carpeta) = ruta.parent() {
        fs::create_dir_all(carpeta)?;
    }
    let mut temporal = ruta.as_os_str().to_owned();
    temporal.push(".tmp");
    fs::write(&temporal, contenido)?;
    fs::rename(&temporal, ruta)
}

/// Grupo de ejemplo para la primera ejecución (con los nombres en el idioma de la interfaz).
fn ejemplo() -> Config {
    let windows = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let app = |nombre: &str, ruta: String| App { nombre: nombre.into(), ruta };
    let mut config = Config {
        elementos: vec![Elemento::Grupo(Grupo {
            nombre: tr("Sistema").into(),
            apps: vec![
                app(tr("Explorador"), format!(r"{windows}\explorer.exe")),
                app(tr("Bloc de notas"), format!(r"{windows}\System32\notepad.exe")),
                app(tr("Terminal"), format!(r"{windows}\System32\cmd.exe")),
                app(tr("Tareas"), format!(r"{windows}\System32\Taskmgr.exe")),
            ],
            icono: 0,
            atajo: None,
        })],
        ajustes: Ajustes::default(),
        boveda: Boveda::default(),
    };
    config.completar_atajos(0);
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_version_gratuita_apaga_las_funciones_pro() {
        let mut ajustes = Ajustes { sustituir_barra_de_windows: true, posicion: Posicion::Abajo, ..Ajustes::default() };
        ajustes.quitar_funciones_pro();
        assert!(!ajustes.sustituir_barra_de_windows && !ajustes.apps_abiertas && !ajustes.ocultar_en_pantalla_completa);
        assert!(!ajustes.boton_buscar && !ajustes.tecla_windows_abre_buscador && ajustes.atajo_buscar.is_none());
        assert!(!ajustes.reproductor_en_barra);
        assert!(!ajustes.boton_sonido && !ajustes.boton_microfono && ajustes.atajo_microfono.is_none());
        assert!(!ajustes.boton_energia && !ajustes.boton_bateria && !ajustes.boton_claude);
        // Sin la barra de Windows sustituida, la barra no puede ir abajo.
        assert_eq!(ajustes.borde(), Posicion::Arriba);
        // Lo demás no cambia.
        assert!(ajustes.usar_boveda && ajustes.atajo_mostrar.is_some() && ajustes.atajo_boveda.is_some());
    }

    #[test]
    fn nombre_desde_acceso_directo() {
        let app = App::desde_ruta(Path::new(r"C:\Menu\Visual Studio Code.lnk"));
        assert_eq!(app.nombre, "Visual Studio Code");
        assert_eq!(app.ruta, r"C:\Menu\Visual Studio Code.lnk");
    }

    #[test]
    fn inicial_en_mayuscula() {
        let app = App { nombre: "ñandú".into(), ruta: String::new() };
        assert_eq!(app.inicial(), "Ñ");
    }

    #[test]
    fn guardar_y_leer_da_lo_mismo() {
        let mut original = ejemplo();
        original.completar_atajos(0);
        let texto = serde_json::to_string(&original).unwrap();
        let leido: Config = serde_json::from_str(&texto).unwrap();
        assert_eq!(original, leido);
    }

    #[test]
    fn archivo_antiguo_sin_ajustes_se_lee() {
        let config: Config = serde_json::from_str(r#"{"grupos":[{"nombre":"Juegos"}]}"#).unwrap();
        assert_eq!(config.elementos[0], Elemento::Grupo(Grupo::nuevo("Juegos")));
        assert_eq!(config.ajustes, Ajustes::default());
    }

    #[test]
    fn atajos_por_defecto_no_se_repiten() {
        let mut config = Config::default();
        config.elementos = vec![
            Elemento::Grupo(Grupo::nuevo("A")),
            Elemento::app(App { nombre: "B".into(), ruta: "b.exe".into() }),
            Elemento::Grupo(Grupo::nuevo("C")),
        ];
        *config.elementos[1].atajo_mut() = Some(Atajo::alt_shift("1"));
        config.completar_atajos(0);
        let teclas: Vec<_> = config.elementos.iter().map(|e| e.atajo().unwrap().tecla.clone()).collect();
        assert_eq!(teclas, ["2", "1", "3"]);
    }

    #[test]
    fn un_atajo_quitado_no_vuelve_al_anadir_elementos() {
        let mut config = Config::default();
        config.elementos = vec![Elemento::Grupo(Grupo::nuevo("A"))];
        config.elementos.push(Elemento::Grupo(Grupo::nuevo("B")));
        config.completar_atajos(1);
        assert_eq!(config.elementos[0].atajo(), None);
        assert_eq!(config.elementos[1].atajo(), Some(&Atajo::alt_shift("1")));
    }

    #[test]
    fn sin_atajo_se_guarda_y_se_lee() {
        let mut original = ejemplo();
        original.ajustes.atajo_mostrar = None;
        original.ajustes.atajo_boveda = None;
        *original.elementos[0].atajo_mut() = None;
        let leido: Config = serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
        assert_eq!(original, leido);
    }

    #[test]
    fn grupos_y_apps_sueltas_se_guardan_y_se_distinguen() {
        let mut original = ejemplo();
        original.elementos.push(Elemento::app(App { nombre: "Notas".into(), ruta: "notas.exe".into() }));
        original.completar_atajos(0);
        let texto = serde_json::to_string(&original).unwrap();
        let leido: Config = serde_json::from_str(&texto).unwrap();
        assert_eq!(original, leido);
        assert!(leido.elementos[0].grupo().is_some());
        assert!(leido.elementos[1].grupo().is_none());
        assert_eq!(leido.elementos[1].app_icono().unwrap().ruta, "notas.exe");
    }

    #[test]
    fn los_colores_antiguos_por_defecto_pasan_a_negro_y_verde() {
        let mut ajustes = Ajustes { color_fondo: "#00125C".into(), color_acento: "#DE3B3B".into(), ..Ajustes::default() };
        ajustes.actualizar_colores_antiguos();
        assert_eq!((ajustes.color_fondo.as_str(), ajustes.color_acento.as_str()), ("#111318", "#2E8B57"));
        // Si solo uno coincide, lo eligió el usuario: no se toca.
        let mut elegido = Ajustes { color_fondo: "#00125C".into(), color_acento: "#2E8B57".into(), ..Ajustes::default() };
        elegido.actualizar_colores_antiguos();
        assert_eq!(elegido.color_fondo, "#00125C");
    }

    #[test]
    fn los_colores_propios_se_anaden_a_su_fila() {
        let mut ajustes = Ajustes::default();
        assert_eq!(ajustes.paleta(false).len(), 4);
        ajustes.poner_color(false, "#123456".into());
        assert_eq!(ajustes.color_fondo, "#123456");
        assert_eq!(ajustes.paleta(false).last().unwrap(), "#123456");
        // Elegir uno base o repetir uno propio no añade nada.
        ajustes.poner_color(false, FONDOS[1].into());
        ajustes.poner_color(false, "#123456".into());
        assert_eq!(ajustes.fondos_propios.len(), 1);
        // La fila de acentos es independiente.
        assert_eq!(ajustes.paleta(true).len(), 4);
        // Con demasiados se olvida el más antiguo.
        for i in 0..MAX_COLORES_PROPIOS + 2 {
            ajustes.poner_color(true, format!("#AA00{i:02X}"));
        }
        assert_eq!(ajustes.acentos_propios.len(), MAX_COLORES_PROPIOS);
        assert_eq!(ajustes.acentos_propios[0], "#AA0002");
    }

    #[test]
    fn la_opacidad_solo_cuenta_con_transparencia_y_tiene_minimo() {
        let mut ajustes = Ajustes { opacidad: 64, ..Ajustes::default() };
        assert_eq!(ajustes.opacidad_fondo(), 0.64);
        ajustes.opacidad = 5;
        assert_eq!(ajustes.opacidad_fondo(), OPACIDAD_MINIMA as f32 / 100.0);
        ajustes.transparencia = false;
        assert_eq!(ajustes.opacidad_fondo(), 1.0);
    }

    #[test]
    fn el_tamano_se_queda_entre_los_limites() {
        let mut ajustes = Ajustes::default();
        assert_eq!(ajustes.tamano_interfaz(), 1.0);
        ajustes.tamano = 250;
        assert_eq!(ajustes.tamano_interfaz(), TAMANO_MAXIMO as f32 / 100.0);
        ajustes.tamano = 10;
        assert_eq!(ajustes.tamano_interfaz(), TAMANO_MINIMO as f32 / 100.0);
    }

    #[test]
    fn quitar_el_color_elegido_vuelve_al_de_por_defecto() {
        let mut ajustes = Ajustes::default();
        ajustes.poner_color(true, "#ABCDEF".into());
        ajustes.quitar_color(true, "#abcdef");
        assert!(ajustes.acentos_propios.is_empty());
        assert_eq!(ajustes.color_acento, ACENTO_INICIAL);
    }

    #[test]
    fn un_color_de_otra_paleta_pasa_a_ser_propio() {
        let mut ajustes = Ajustes { color_fondo: "#2A1220".into(), ..Ajustes::default() };
        ajustes.asegurar_colores_en_paleta();
        assert_eq!(ajustes.fondos_propios, ["#2A1220"]);
        assert!(ajustes.acentos_propios.is_empty());
    }

    #[test]
    fn editar_un_preset_cambia_solo_ese_color_y_no_deja_repetidos() {
        let mut ajustes = Ajustes::default();
        ajustes.presets = vec![
            Preset { fondo: "#111111".into(), acento: "#AA0000".into() },
            Preset { fondo: "#222222".into(), acento: "#00AA00".into() },
        ];
        ajustes.editar_preset(1, true, "#0000AA".into());
        assert_eq!(ajustes.presets[1].acento, "#0000AA");
        assert_eq!(ajustes.presets[1].fondo, "#222222");
        // Los colores elegidos no cambian al editar un preset.
        assert_eq!(ajustes.color_acento, Ajustes::default().color_acento);
        // Si queda igual que otro, se quita.
        ajustes.editar_preset(1, false, "#111111".into());
        ajustes.editar_preset(1, true, "#aa0000".into());
        assert_eq!(ajustes.presets.len(), 1);
        // Un índice que no existe no hace nada.
        ajustes.editar_preset(7, true, "#FFFFFF".into());
        assert_eq!(ajustes.presets.len(), 1);
    }

    #[test]
    fn reemplazar_un_preset_guarda_los_colores_actuales() {
        let mut ajustes = Ajustes::default();
        ajustes.presets = vec![Preset { fondo: "#111111".into(), acento: "#AA0000".into() }];
        ajustes.reemplazar_preset(0);
        assert!(ajustes.preset_guardado());
        assert_eq!(ajustes.presets.len(), 1);
    }

    #[test]
    fn una_configuracion_antigua_carga_la_boveda_vacia_y_su_atajo() {
        let config: Config = serde_json::from_str(r#"{ "elementos": [] }"#).unwrap();
        assert!(config.boveda.elementos.is_empty());
        assert_eq!(config.ajustes.atajo_boveda, Some(Atajo::alt_shift("V")));
    }

    #[test]
    fn los_presets_no_se_repiten_y_tienen_limite() {
        let mut ajustes = Ajustes::default();
        ajustes.guardar_preset();
        ajustes.guardar_preset();
        assert_eq!(ajustes.presets.len(), 1);
        assert!(ajustes.preset_guardado());
        for i in 0..MAX_PRESETS + 3 {
            ajustes.color_acento = format!("#0000{i:02X}");
            ajustes.guardar_preset();
        }
        assert_eq!(ajustes.presets.len(), MAX_PRESETS);
        // Se olvidan los más antiguos: el último guardado sigue ahí.
        assert_eq!(ajustes.presets.last().unwrap().acento, format!("#0000{:02X}", MAX_PRESETS + 2));
    }

    #[test]
    fn icono_del_grupo_cae_en_la_primera_app_si_no_existe() {
        let mut grupo = Grupo::nuevo("X");
        grupo.apps.push(App { nombre: "Uno".into(), ruta: String::new() });
        grupo.icono = 5;
        assert_eq!(grupo.app_icono().unwrap().nombre, "Uno");
    }

    #[test]
    fn los_accesorios_se_ponen_quitan_y_recuerdan_su_color() {
        let mut p = Personaje::default();
        let audifonos = TipoAccesorio::Audifonos.id();
        assert!(!p.accesorio(audifonos).puesto);
        assert_eq!(p.accesorio(audifonos).color, COLORES_ACCESORIOS[0]);
        p.alternar(audifonos, &hueco_integrado);
        assert!(p.accesorio(audifonos).puesto);
        // Un color nuevo se añade a la fila y pone el accesorio.
        p.alternar(audifonos, &hueco_integrado);
        p.poner_color(audifonos, 0, "#123456".into(), &hueco_integrado);
        assert!(p.accesorio(audifonos).puesto);
        assert_eq!(p.paleta().last().unwrap(), "#123456");
        // Quitado, recuerda el color; si se quita el color, vuelve al inicial.
        p.alternar(audifonos, &hueco_integrado);
        assert_eq!(p.accesorio(audifonos).color, "#123456");
        p.quitar_color("#123456");
        assert_eq!(p.accesorio(audifonos).color, COLORES_ACCESORIOS[0]);
        assert_eq!(p.accesorios.len(), 1);
    }

    #[test]
    fn el_circulito_de_aviso_recuerda_su_color_hasta_que_se_quita() {
        let mut p = Personaje::default();
        // Sin elegir, es el de los ojos.
        assert_eq!(p.color_aviso, None);
        p.poner_color_aviso("#123456".into());
        assert_eq!(p.color_aviso.as_deref(), Some("#123456"));
        // El color nuevo entra en la fila de los accesorios.
        assert_eq!(p.paleta().last().unwrap(), "#123456");
        // Uno que ya estaba no se repite.
        p.poner_color_aviso(COLORES_ACCESORIOS[3].to_lowercase());
        assert_eq!(p.colores_propios, ["#123456"]);
        // Quitar el color propio que tenía lo devuelve al de los ojos; otro color no lo toca.
        p.poner_color_aviso("#654321".into());
        p.quitar_color("#123456");
        assert_eq!(p.color_aviso.as_deref(), Some("#654321"));
        p.quitar_color("#654321");
        assert_eq!(p.color_aviso, None);
    }

    #[test]
    fn en_cada_hueco_solo_cabe_un_accesorio() {
        use TipoAccesorio::*;
        let mut p = Personaje::default();
        for t in [Corona, GafasDeSol, Audifonos] {
            p.alternar(t.id(), &hueco_integrado);
        }
        // El gorro quita la corona; las gafas y los audífonos se quedan.
        p.alternar(GorroDeLana.id(), &hueco_integrado);
        let puesto = |p: &Personaje, t: TipoAccesorio| p.accesorio(t.id()).puesto;
        assert!(!puesto(&p, Corona) && puesto(&p, GorroDeLana));
        assert!(puesto(&p, GafasDeSol) && puesto(&p, Audifonos));
        // Elegir el color del antifaz lo pone y quita las gafas.
        p.poner_color(Antifaz.id(), 0, "#000000".into(), &hueco_integrado);
        assert!(puesto(&p, Antifaz) && !puesto(&p, GafasDeSol));
        // La mascarilla, el corbatín y la capa no se estorban.
        for t in [Mascarilla, Corbatin, CapaDeVampiro] {
            p.alternar(t.id(), &hueco_integrado);
        }
        assert!(puesto(&p, Mascarilla) && puesto(&p, Corbatin) && puesto(&p, CapaDeVampiro));
    }

    #[test]
    fn un_accesorio_de_archivo_ocupa_su_hueco_y_guarda_cada_color() {
        // Los de archivos dicen su hueco ellos mismos: este es un sombrero de la cabeza.
        let hueco = |id: &str| if id == "usuario/sombrero" { Some(Hueco::Cabeza) } else { hueco_integrado(id) };
        let mut p = Personaje::default();
        p.alternar("corona", &hueco);
        // Sin tocar, su color es el de su dibujo (vacío).
        assert_eq!(p.accesorio("usuario/sombrero").color, "");
        // El tercer color: los anteriores se quedan sin cambiar.
        p.poner_color("usuario/sombrero", 2, "#112233".into(), &hueco);
        let a = p.accesorio("usuario/sombrero");
        assert!(a.puesto && !p.accesorio("corona").puesto);
        assert_eq!((a.color_de(0), a.color_de(1), a.color_de(2), a.color_de(3)), ("", "", "#112233", ""));
        // Volver al color del dibujo no añade nada a la fila.
        let en_la_fila = p.paleta().len();
        p.poner_color("usuario/sombrero", 1, String::new(), &hueco);
        assert_eq!(p.paleta().len(), en_la_fila);
        // Se guarda y se lee igual; quitar el color de la fila lo devuelve al del dibujo.
        let leido: Personaje = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(leido, p);
        p.quitar_color("#112233");
        assert_eq!(p.accesorio("usuario/sombrero").color_de(2), "");
        // Los que no tienen más que un color no guardan nada de más.
        let texto = serde_json::to_string(&Accesorio { tipo: "corona".into(), puesto: true, color: "#FFFFFF".into(), colores_extra: Vec::new() }).unwrap();
        assert!(!texto.contains("colores_extra"), "{texto}");
    }

    #[test]
    fn la_figura_se_guarda_y_la_antigua_menta_es_nook() {
        let mut p = Personaje::default();
        assert_eq!(p.figura, "nook");
        p.figura = "limon".into();
        let texto = serde_json::to_string(&p).unwrap();
        assert!(texto.contains("\"limon\""), "{texto}");
        assert_eq!(serde_json::from_str::<Personaje>(&texto).unwrap().figura, "limon");
        let propia: Personaje = serde_json::from_str(r#"{"figura":"usuario/gato"}"#).unwrap();
        assert_eq!(propia.figura, "usuario/gato");
        let antigua: Personaje = serde_json::from_str(r#"{"figura":"menta"}"#).unwrap();
        assert_eq!(antigua.figura, "nook");
        // Sin figura, la de siempre.
        assert_eq!(serde_json::from_str::<Personaje>("{}").unwrap().figura, "nook");
    }

    #[test]
    fn los_accesorios_se_guardan_con_su_nombre() {
        let mut p = Personaje::default();
        p.alternar(TipoAccesorio::CapaDeVampiro.id(), &hueco_integrado);
        let texto = serde_json::to_string(&p).unwrap();
        assert!(texto.contains("\"capa_de_vampiro\""), "{texto}");
        let leido: Personaje = serde_json::from_str(&texto).unwrap();
        assert_eq!(leido, p);
    }

    #[test]
    fn la_configuracion_sin_personaje_o_con_accesorios_desconocidos_se_lee() {
        let config: Config = serde_json::from_str(r#"{"grupos":[],"ajustes":{"tamano":100}}"#).unwrap();
        assert_eq!(config.ajustes.personaje, Personaje::default());
        let texto = r##"{"grupos":[],"ajustes":{"personaje":{"accesorios":[
            {"tipo":"sombrero","puesto":true,"color":"#FFFFFF"},
            {"tipo":"audifonos","puesto":true,"color":"#000000"}]}}}"##;
        let config: Config = serde_json::from_str(texto).unwrap();
        let p = &config.ajustes.personaje;
        // El que no se conoce se conserva (por si es de una versión más nueva, o de un archivo).
        assert_eq!(p.accesorios[0].tipo, "sombrero");
        assert!(p.accesorio(TipoAccesorio::Audifonos.id()).puesto);
        assert!(p.accesorio("sombrero").puesto);
    }
}
