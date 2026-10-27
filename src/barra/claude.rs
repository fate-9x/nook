//! Conversación con Claude Code (solo en nook Pro): el botón de la tira abre un panel con la
//! conversación y un campo para escribirle. Cada mensaje se le manda a `claude` en la carpeta
//! elegida, siguiendo la misma conversación hasta que se empieza otra (ver `claude.rs`).

use crate::*;

/// La conversación del panel.
#[derive(Default)]
pub(crate) struct Conversacion {
    mensajes: Vec<MensajeClaude>,
    /// Id de la conversación de Claude Code, para seguirla en la siguiente pregunta.
    sesion: Option<String>,
    respuesta: Option<claude::Respuesta>,
    /// Cuenta las preguntas: lo que llegue de una que se paró se descarta.
    turno: u64,
    /// Lo último que se sabe del uso de los límites de 5 horas y semanal (ver `claude::Suceso::Uso`).
    uso: (Option<(f32, i64)>, Option<(f32, i64)>),
}

/// Modelos y esfuerzos que se pueden elegir: lo que se le pasa a `claude` y cómo se llama ("" =
/// los de su configuración).
fn modelos() -> Vec<(&'static str, &'static str)> {
    vec![("", tr("Predeterminado")), ("fable", "Fable"), ("opus", "Opus"), ("sonnet", "Sonnet"), ("haiku", "Haiku")]
}

fn esfuerzos() -> Vec<(&'static str, &'static str)> {
    vec![
        ("", tr("Predeterminado")),
        ("low", tr("Bajo")),
        ("medium", tr("Medio")),
        ("high", tr("Alto")),
        ("xhigh", tr("Muy alto")),
        ("max", tr("Máximo")),
    ]
}

/// Cómo se llama la opción `valor` de una lista.
fn nombre_de(lista: &[(&str, &'static str)], valor: &str) -> &'static str {
    lista.iter().find(|(v, _)| *v == valor).map_or(lista[0].1, |(_, nombre)| nombre)
}

/// Parte usada de un límite (-1 si aún no se sabe) y cuándo se reinicia ("hasta 14:20").
fn uso_y_reinicio(uso: Option<(f32, i64)>) -> (f32, String) {
    let Some((parte, reinicio)) = uso else { return (-1.0, String::new()) };
    let hasta = chrono::DateTime::from_timestamp(reinicio, 0).map(|d| d.with_timezone(&Local)).map_or(String::new(), |d| {
        let formato = if d.date_naive() == Local::now().date_naive() { "%H:%M" } else { "%d/%m %H:%M" };
        format!("{} {}", tr("hasta"), d.format(formato))
    });
    (parte, hasta)
}

/// Tipos de mensaje del panel (ver ui/claude.slint).
const DEL_USUARIO: i32 = 0;
const DE_CLAUDE: i32 = 1;
const NOTA: i32 = 2;

impl Ctx {
    /// Abre o cierra el panel de Claude.
    pub(crate) fn alternar_claude(&self) {
        if self.estado.borrow().modo == Modo::Claude {
            self.cerrar();
        } else {
            self.mostrar_claude();
            self.poner_modo(Modo::Claude);
        }
    }

    /// Pasa a la interfaz la conversación, si Claude está respondiendo y la carpeta.
    fn mostrar_claude(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let e = self.estado.borrow();
        ui.set_mensajes_claude(modelo(e.claude.mensajes.clone()));
        ui.set_claude_ocupado(e.claude.respuesta.is_some());
        ui.set_carpeta_claude(carpeta_claude(&e.config.ajustes).display().to_string().into());
        ui.set_modelo_claude(tr_con("Modelo: {}", &[nombre_de(&modelos(), &e.config.ajustes.modelo_claude)]).into());
        ui.set_esfuerzo_claude(tr_con("Esfuerzo: {}", &[nombre_de(&esfuerzos(), &e.config.ajustes.esfuerzo_claude)]).into());
        let (cinco_horas, hasta) = uso_y_reinicio(e.claude.uso.0);
        ui.set_uso_cinco_horas_claude(cinco_horas);
        ui.set_reinicio_cinco_horas_claude(hasta.into());
        let (semana, hasta) = uso_y_reinicio(e.claude.uso.1);
        ui.set_uso_semana_claude(semana);
        ui.set_reinicio_semana_claude(hasta.into());
    }

    /// Menú para elegir el modelo (`esfuerzo` false) o el esfuerzo con que responde Claude.
    fn elegir_opcion_claude(&self, esfuerzo: bool) {
        let lista = if esfuerzo { esfuerzos() } else { modelos() };
        let opciones: Vec<Opcion> = lista.iter().enumerate().map(|(i, (_, nombre))| Opcion::Normal(i as u32, nombre)).collect();
        self.mostrar_menu(&opciones, move |ctx, i| {
            let Some((valor, _)) = lista.get(i as usize) else { return };
            ctx.modificar_ajustes(|a| *if esfuerzo { &mut a.esfuerzo_claude } else { &mut a.modelo_claude } = valor.to_string());
            ctx.mostrar_claude();
        });
    }

    fn anotar_claude(&self, tipo: i32, texto: impl Into<SharedString>) {
        let texto = texto.into();
        let bloques = if tipo == DE_CLAUDE { modelo(bloques_con_formato(&texto)) } else { ModelRc::default() };
        self.estado.borrow_mut().claude.mensajes.push(MensajeClaude { tipo, texto, bloques });
    }

    /// Le manda un mensaje a Claude (si no está respondiendo otro).
    fn enviar_claude(&self, texto: &str) {
        let texto = texto.trim();
        if texto.is_empty() || self.estado.borrow().claude.respuesta.is_some() {
            return;
        }
        self.anotar_claude(DEL_USUARIO, texto);
        let (carpeta, sesion, turno, modelo, esfuerzo) = {
            let mut e = self.estado.borrow_mut();
            e.claude.turno += 1;
            let a = &e.config.ajustes;
            let opcion = |valor: &String| Some(valor.clone()).filter(|v| !v.is_empty());
            (carpeta_claude(a), e.claude.sesion.clone(), e.claude.turno, opcion(&a.modelo_claude), opcion(&a.esfuerzo_claude))
        };
        let al_llegar = move |suceso| barra::addons::en_la_interfaz(move |ctx| ctx.llega_de_claude(turno, suceso));
        match claude::preguntar(texto, &carpeta, sesion.as_deref(), modelo.as_deref(), esfuerzo.as_deref(), al_llegar) {
            Ok(respuesta) => self.estado.borrow_mut().claude.respuesta = Some(respuesta),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.anotar_claude(NOTA, tr("No se encuentra Claude Code: instálalo y vuelve a intentarlo."))
            }
            Err(e) => self.anotar_claude(NOTA, tr_con("No se pudo abrir Claude Code: {}", &[&e.to_string()])),
        }
        self.mostrar_claude();
    }

    /// Lo que va respondiendo Claude a la pregunta `turno`.
    fn llega_de_claude(&self, turno: u64, suceso: claude::Suceso) {
        if self.estado.borrow().claude.turno != turno {
            return;
        }
        match suceso {
            claude::Suceso::Sesion(id) => self.estado.borrow_mut().claude.sesion = Some(id),
            claude::Suceso::Texto(texto) => self.anotar_claude(DE_CLAUDE, texto),
            claude::Suceso::Herramienta(uso) => self.anotar_claude(NOTA, format!("→ {uso}")),
            claude::Suceso::Uso { cinco_horas, semana } => self.estado.borrow_mut().claude.uso = (cinco_horas, semana),
            claude::Suceso::Fin { denegadas, error } => {
                self.estado.borrow_mut().claude.respuesta = None;
                if !denegadas.is_empty() {
                    self.anotar_claude(NOTA, tr_con("Claude no tuvo permiso para usar: {}", &[&denegadas.join(", ")]));
                }
                if let Some(error) = error {
                    // Algunos errores (como no tener la sesión iniciada) llegan antes como su
                    // respuesta: se cambia por el aviso.
                    let mut e = self.estado.borrow_mut();
                    if e.claude.mensajes.last().is_some_and(|m| m.tipo == DE_CLAUDE && m.texto == error.as_str()) {
                        e.claude.mensajes.pop();
                    }
                    drop(e);
                    if falta_iniciar_sesion(&error) {
                        self.anotar_claude(NOTA, tr("Claude Code no tiene la sesión iniciada: abre una terminal, escribe claude y entra con /login. Luego vuelve a enviar el mensaje."));
                    } else {
                        self.anotar_claude(NOTA, tr_con("Claude Code falló: {}", &[&error]));
                    }
                }
                // Con el panel cerrado, el personaje avisa de que ya respondió.
                if self.estado.borrow().modo != Modo::Claude {
                    self.notificar();
                }
            }
        }
        self.mostrar_claude();
    }

    /// Corta la respuesta en marcha (si hay).
    fn parar_claude(&self) {
        let respuesta = {
            let mut e = self.estado.borrow_mut();
            e.claude.turno += 1;
            e.claude.respuesta.take()
        };
        if let Some(respuesta) = respuesta {
            respuesta.parar();
            self.anotar_claude(NOTA, tr("Respuesta parada."));
        }
        self.mostrar_claude();
    }

    /// Empieza una conversación nueva.
    fn nueva_conversacion_claude(&self) {
        self.parar_claude();
        let mut e = self.estado.borrow_mut();
        e.claude.mensajes.clear();
        e.claude.sesion = None;
        drop(e);
        self.mostrar_claude();
    }

    /// Elige la carpeta en la que trabaja Claude (con una conversación nueva).
    fn elegir_carpeta_claude(&self) {
        if let Some(carpeta) = claude::elegir_carpeta(plataforma::hwnd()) {
            self.modificar_ajustes(|a| a.carpeta_claude = carpeta.display().to_string());
            self.nueva_conversacion_claude();
        }
    }
}

/// Si el error de Claude Code es porque no tiene la sesión iniciada (o caducó).
fn falta_iniciar_sesion(error: &str) -> bool {
    let error = error.to_lowercase();
    ["authenticat", "oauth", "/login", "not logged in", "log in"].iter().any(|p| error.contains(p))
}

/// Tipos de bloque de un mensaje de Claude (ver ui/claude.slint).
const PARRAFOS: i32 = 0;
const TITULO: i32 = 1;
const CITA: i32 = 2;
const CODIGO: i32 = 3;
const LINEA: i32 = 4;
/// Alternativas para elegir (un bloque de código `opciones`, ver `INSTRUCCIONES` en claude.rs).
const OPCIONES: i32 = 5;

/// Un mensaje de Claude en bloques con formato.
fn bloques_con_formato(texto: &str) -> Vec<BloqueClaude> {
    partes_markdown(texto)
        .into_iter()
        .map(|(tipo, texto)| {
            let opciones: Vec<SharedString> = if tipo == OPCIONES {
                texto.lines().map(|o| o.trim().trim_start_matches(['-', '*', '•']).trim()).filter(|o| !o.is_empty()).map(Into::into).collect()
            } else {
                Vec::new()
            };
            let texto = if matches!(tipo, PARRAFOS | CITA) { sin_marcas(&texto) } else { texto };
            BloqueClaude { tipo, texto: texto.into(), opciones: modelo(opciones) }
        })
        .collect()
}

/// Un texto en markdown sin las marcas de formato de dentro de las líneas (negritas, código,
/// enlaces) y con viñetas en las listas, para leerlo como texto que se puede seleccionar.
fn sin_marcas(texto: &str) -> String {
    let linea_sin_marcas = |linea: &str| {
        let resto = linea.trim_start();
        let sangria = &linea[..linea.len() - resto.len()];
        let resto = match resto.strip_prefix("- ").or_else(|| resto.strip_prefix("* ")).or_else(|| resto.strip_prefix("+ ")) {
            Some(elemento) => format!("• {elemento}"),
            None => resto.to_string(),
        };
        format!("{sangria}{}", enlaces_en_texto(&resto).replace("**", "").replace('`', ""))
    };
    texto.lines().map(linea_sin_marcas).collect::<Vec<_>>().join("\n")
}

/// "[texto](dirección)" pasa a "texto (dirección)".
fn enlaces_en_texto(linea: &str) -> String {
    let mut resultado = String::new();
    let mut resto = linea;
    while let Some(i) = resto.find('[') {
        let Some(j) = resto[i..].find("](").map(|j| i + j) else { break };
        let Some(k) = resto[j..].find(')').map(|k| j + k) else { break };
        resultado.push_str(&format!("{}{} ({})", &resto[..i], &resto[i + 1..j], &resto[j + 2..k]));
        resto = &resto[k + 1..];
    }
    resultado + resto
}

/// Las partes de un texto en markdown: los párrafos (con sus listas), títulos, citas, bloques de
/// código, tablas, líneas y alternativas, que se muestran cada uno a su manera.
fn partes_markdown(texto: &str) -> Vec<(i32, String)> {
    let mut partes: Vec<(i32, String)> = Vec::new();
    let mut en_codigo = false;
    let mut tras_vacia = false;
    for linea in texto.lines() {
        let recortada = linea.trim();
        if recortada.starts_with("```") {
            en_codigo = !en_codigo;
            if en_codigo {
                let opciones = recortada.trim_start_matches('`').trim() == "opciones";
                partes.push((if opciones { OPCIONES } else { CODIGO }, String::new()));
            }
            // Lo que sigue al bloque de código va aparte (aunque sea una tabla).
            tras_vacia = !en_codigo;
            continue;
        }
        if en_codigo {
            if let Some((_, codigo)) = partes.last_mut() {
                if !codigo.is_empty() {
                    codigo.push('\n');
                }
                codigo.push_str(linea);
            }
            continue;
        }
        if recortada.is_empty() {
            tras_vacia = true;
            continue;
        }
        let almohadillas = recortada.chars().take_while(|&c| c == '#').count();
        let (tipo, contenido) = if (1..=6).contains(&almohadillas) && recortada[almohadillas..].starts_with(' ') {
            (TITULO, recortada[almohadillas..].trim().replace("**", ""))
        } else if let Some(cita) = recortada.strip_prefix('>') {
            (CITA, cita.trim().to_string())
        } else if recortada.len() >= 3 && recortada.chars().all(|c| c == '-' || c == '*' || c == '_' || c == ' ') {
            (LINEA, String::new())
        } else if recortada.starts_with('|') {
            // La fila que separa la cabecera de una tabla no se muestra.
            if recortada.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')) {
                continue;
            }
            (CODIGO, recortada.to_string())
        } else {
            (PARRAFOS, linea.to_string())
        };
        // Las líneas seguidas de párrafos, citas y tablas van juntas (los párrafos, también
        // separados por líneas vacías).
        match partes.last_mut() {
            Some((anterior, junto)) if *anterior == tipo && tipo != TITULO && tipo != LINEA && (!tras_vacia || tipo == PARRAFOS) => {
                junto.push_str(if tras_vacia { "\n\n" } else { "\n" });
                junto.push_str(&contenido);
            }
            _ => partes.push((tipo, contenido)),
        }
        tras_vacia = false;
    }
    partes
}

/// Carpeta en la que trabaja Claude: la elegida o, si no hay, la del usuario.
fn carpeta_claude(ajustes: &Ajustes) -> PathBuf {
    if ajustes.carpeta_claude.is_empty() {
        std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default()
    } else {
        PathBuf::from(&ajustes.carpeta_claude)
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_alternar_claude(move || c.alternar_claude());
    let c = ctx.clone();
    ui.on_enviar_claude(move |texto| c.enviar_claude(&texto));
    let c = ctx.clone();
    ui.on_parar_claude(move || c.parar_claude());
    let c = ctx.clone();
    ui.on_nueva_conversacion_claude(move || c.nueva_conversacion_claude());
    let c = ctx.clone();
    ui.on_elegir_carpeta_claude(move || c.elegir_carpeta_claude());
    let c = ctx.clone();
    ui.on_elegir_modelo_claude(move || c.elegir_opcion_claude(false));
    let c = ctx.clone();
    ui.on_elegir_esfuerzo_claude(move || c.elegir_opcion_claude(true));
    ui.on_copiar_claude(|texto| {
        portapapeles::poner(&texto);
    });
    let c = ctx.clone();
    ui.on_alternar_boton_claude(move || c.modificar_ajustes(|a| a.boton_claude = !a.boton_claude));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separa_el_markdown_en_bloques() {
        let texto = "# Título **uno**\nUn párrafo\nque sigue.\n\n- lista\n- más\n\n> cita\n> larga\n\n\
            ```rust\nfn main() {}\n\n```\n| a | b |\n|---|:-:|\n| 1 | 2 |\n---\nFin";
        assert_eq!(
            partes_markdown(texto),
            [
                (TITULO, "Título uno".to_string()),
                (PARRAFOS, "Un párrafo\nque sigue.\n\n- lista\n- más".to_string()),
                (CITA, "cita\nlarga".to_string()),
                (CODIGO, "fn main() {}\n".to_string()),
                (CODIGO, "| a | b |\n| 1 | 2 |".to_string()),
                (LINEA, String::new()),
                (PARRAFOS, "Fin".to_string()),
            ]
        );
        // Un bloque de código sin cerrar llega hasta el final.
        assert_eq!(partes_markdown("```\nx"), [(CODIGO, "x".to_string())]);
    }

    #[test]
    fn las_alternativas_salen_como_opciones() {
        let bloques = bloques_con_formato("¿Cuál prefieres?\n```opciones\n- Té\nCafé\n\n```");
        assert_eq!(bloques.len(), 2);
        assert_eq!(bloques[1].tipo, OPCIONES);
        assert_eq!(bloques[1].opciones.iter().collect::<Vec<_>>(), ["Té", "Café"]);
    }

    #[test]
    fn quita_las_marcas_de_formato_de_las_lineas() {
        assert_eq!(
            sin_marcas("Usa **`cargo test`** y mira [la guía](https://x.y/z).\n- uno\n  * dos"),
            "Usa cargo test y mira la guía (https://x.y/z).\n• uno\n  • dos"
        );
        assert_eq!(sin_marcas("[a] sin enlace"), "[a] sin enlace");
    }

    #[test]
    fn reconoce_los_errores_de_sesion() {
        assert!(falta_iniciar_sesion("Failed to authenticate: OAuth session expired and could not be refreshed"));
        assert!(falta_iniciar_sesion("Not logged in · Please run /login"));
        assert!(!falta_iniciar_sesion("exit code: 1"));
    }
}
