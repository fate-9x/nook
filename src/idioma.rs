//! Idioma de la interfaz: español, inglés, japonés o coreano.
//!
//! Los textos se escriben en español en el código y sus traducciones están en
//! `lang/<idioma>/LC_MESSAGES/nook.po` (formato gettext). Los de ui/ van con `@tr(…)`
//! y Slint los traduce solo (las traducciones se meten en el ejecutable al compilar, ver
//! build.rs); los que salen de Rust (menús, mensajes de error…) pasan por `tr`, que lee los
//! mismos archivos.

use crate::config::Idioma;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU8, Ordering};

/// Los idiomas con traducción, en el orden de `TRADUCIDOS` (el español es el del código).
const TRADUCIDOS: [Idioma; 3] = [Idioma::Ingles, Idioma::Japones, Idioma::Coreano];

static CATALOGOS: LazyLock<[HashMap<String, String>; 3]> = LazyLock::new(|| {
    [
        leer_po(include_str!("../lang/en/LC_MESSAGES/nook.po")),
        leer_po(include_str!("../lang/ja/LC_MESSAGES/nook.po")),
        leer_po(include_str!("../lang/ko/LC_MESSAGES/nook.po")),
    ]
});

/// Idioma en uso: 0 español y, después, la posición en `TRADUCIDOS` más uno.
static ACTUAL: AtomicU8 = AtomicU8::new(0);

/// Pone el idioma elegido (con `Sistema`, el de Windows). Tiene que llamarse después de crear la
/// primera ventana de Slint; luego hay que volver a pasar a la interfaz los textos que vienen de
/// Rust.
pub fn poner(elegido: Idioma) {
    let idioma = if elegido == Idioma::Sistema { del_sistema() } else { elegido };
    let posicion = TRADUCIDOS.iter().position(|&i| i == idioma).map_or(0, |p| p + 1);
    ACTUAL.store(posicion as u8, Ordering::Relaxed);
    // El español es el texto original: se elige con "" (sin traducción).
    if let Err(e) = slint::select_bundled_translation(codigo(idioma)) {
        eprintln!("No se pudo poner el idioma {idioma:?}: {e}");
    }
}

fn actual() -> Idioma {
    match ACTUAL.load(Ordering::Relaxed) {
        0 => Idioma::Espanol,
        n => TRADUCIDOS[n as usize - 1],
    }
}

/// Nombre de la carpeta de `lang/` con sus traducciones.
fn codigo(idioma: Idioma) -> &'static str {
    match idioma {
        Idioma::Ingles => "en",
        Idioma::Japones => "ja",
        Idioma::Coreano => "ko",
        Idioma::Espanol | Idioma::Sistema => "",
    }
}

/// Fuente de la interfaz. Segoe UI no tiene letras japonesas ni coreanas; las fuentes de Windows
/// para esos idiomas sí tienen las latinas.
pub fn fuente() -> &'static str {
    match actual() {
        Idioma::Japones => "Yu Gothic UI",
        Idioma::Coreano => "Malgun Gothic",
        _ => "Segoe UI",
    }
}

/// El idioma de Windows, o el inglés si no es ninguno de los que hay.
fn del_sistema() -> Idioma {
    let langid = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
    de_langid(langid)
}

/// Idioma de un identificador de idioma de Windows (LANGID): lo que cuenta es el idioma
/// principal, los 10 bits bajos (el resto es la variante: España, México…).
fn de_langid(langid: u16) -> Idioma {
    match langid & 0x3FF {
        0x0A => Idioma::Espanol,
        0x11 => Idioma::Japones,
        0x12 => Idioma::Coreano,
        _ => Idioma::Ingles,
    }
}

/// `texto` (en español) en el idioma en uso; si no tiene traducción, tal cual.
pub fn tr(texto: &'static str) -> &'static str {
    let Some(i) = TRADUCIDOS.iter().position(|&i| i == actual()) else { return texto };
    CATALOGOS[i].get(texto).map_or(texto, String::as_str)
}

/// Como `tr`, con cada `{}` del texto traducido cambiado por el valor que le toca.
pub fn tr_con(texto: &'static str, valores: &[&str]) -> String {
    let mut partes = tr(texto).split("{}");
    let mut resultado = partes.next().unwrap_or_default().to_string();
    for (parte, valor) in partes.zip(valores.iter().chain(std::iter::repeat(&""))) {
        resultado.push_str(valor);
        resultado.push_str(parte);
    }
    resultado
}

/// Las traducciones de un archivo .po: de cada texto original (`msgid`), el traducido
/// (`msgstr`). Se saltan la cabecera y los que no tienen traducción.
fn leer_po(po: &str) -> HashMap<String, String> {
    let mut catalogo = HashMap::new();
    let (mut original, mut traducido) = (None::<String>, None::<String>);
    let mut guardar = |original: &mut Option<String>, traducido: &mut Option<String>| {
        if let (Some(o), Some(t)) = (original.take(), traducido.take())
            && !o.is_empty()
            && !t.is_empty()
        {
            catalogo.insert(o, t);
        }
    };
    for linea in po.lines().map(str::trim) {
        if let Some(resto) = linea.strip_prefix("msgid ") {
            guardar(&mut original, &mut traducido);
            original = Some(cadena(resto));
        } else if let Some(resto) = linea.strip_prefix("msgstr ") {
            traducido = Some(cadena(resto));
        } else if linea.starts_with('"') {
            // Continuación en otra línea del último msgid o msgstr.
            let ultimo = if traducido.is_some() { &mut traducido } else { &mut original };
            if let Some(s) = ultimo {
                s.push_str(&cadena(linea));
            }
        }
    }
    guardar(&mut original, &mut traducido);
    catalogo
}

/// El contenido de una cadena entre comillas de un .po, con los escapes (`\n`, `\"`, `\\`)
/// resueltos.
fn cadena(entre_comillas: &str) -> String {
    let dentro = entre_comillas.trim().trim_start_matches('"').strip_suffix('"').unwrap_or_default();
    let mut resultado = String::with_capacity(dentro.len());
    let mut letras = dentro.chars();
    while let Some(c) = letras.next() {
        if c != '\\' {
            resultado.push(c);
            continue;
        }
        match letras.next() {
            Some('n') => resultado.push('\n'),
            Some('t') => resultado.push('\t'),
            Some(otra) => resultado.push(otra),
            None => {}
        }
    }
    resultado
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Los textos que se traducen en el código: los literales de `@tr("…")` en ui/ y de `tr("…")`
    /// y `tr_con("…"` en src/.
    fn textos_del_codigo() -> BTreeSet<String> {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut archivos = Vec::new();
        let mut carpetas = vec![raiz.join("src"), raiz.join("ui")];
        while let Some(carpeta) = carpetas.pop() {
            for entrada in std::fs::read_dir(carpeta).unwrap().flatten() {
                let ruta = entrada.path();
                let extension = ruta.extension().and_then(|e| e.to_str());
                if ruta.is_dir() {
                    carpetas.push(ruta);
                } else if matches!(extension, Some("rs" | "slint")) && !ruta.ends_with("idioma.rs") {
                    archivos.push(ruta);
                }
            }
        }
        let mut textos = BTreeSet::new();
        for archivo in archivos {
            let fuente = std::fs::read_to_string(&archivo).unwrap();
            for llamada in ["tr(\"", "tr_con(\""] {
                for (i, _) in fuente.match_indices(llamada) {
                    // Que sea la función y no el final de otro nombre (`@tr` sí vale).
                    let antes = fuente[..i].chars().next_back();
                    if antes.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                        continue;
                    }
                    let resto = &fuente[i + llamada.len()..];
                    let mut fin = 0;
                    let mut escapada = false;
                    for (j, c) in resto.char_indices() {
                        if escapada {
                            escapada = false;
                        } else if c == '\\' {
                            escapada = true;
                        } else if c == '"' {
                            fin = j;
                            break;
                        }
                    }
                    textos.insert(cadena(&format!("\"{}\"", &resto[..fin])));
                }
            }
        }
        textos
    }

    #[test]
    fn cada_texto_tiene_su_traduccion_en_cada_idioma_y_ninguna_sobra() {
        let textos = textos_del_codigo();
        assert!(textos.len() > 100, "solo se encontraron {} textos", textos.len());
        for (idioma, catalogo) in TRADUCIDOS.iter().zip(CATALOGOS.iter()) {
            for texto in &textos {
                let traducido = catalogo.get(texto).unwrap_or_else(|| panic!("{idioma:?}: falta la traducción de {texto:?}"));
                assert_eq!(texto.matches("{}").count(), traducido.matches("{}").count(), "{idioma:?}: {texto:?}");
            }
            for original in catalogo.keys() {
                assert!(textos.contains(original), "{idioma:?}: {original:?} ya no se usa");
            }
        }
    }

    #[test]
    fn el_idioma_de_windows_se_reconoce_por_su_idioma_principal() {
        assert_eq!(de_langid(0x0C0A), Idioma::Espanol); // España
        assert_eq!(de_langid(0x080A), Idioma::Espanol); // México
        assert_eq!(de_langid(0x0409), Idioma::Ingles);
        assert_eq!(de_langid(0x0411), Idioma::Japones);
        assert_eq!(de_langid(0x0412), Idioma::Coreano);
        // Los demás, en inglés.
        assert_eq!(de_langid(0x040C), Idioma::Ingles); // francés
        assert_eq!(de_langid(0x0804), Idioma::Ingles); // chino
    }

    #[test]
    fn se_leen_los_po_con_escapes_y_lineas_partidas() {
        let po = "msgid \"\"\nmsgstr \"\"\n\"Language: en\\n\"\n\n# comentario\nmsgid \"Hola\\n\\\"tú\\\"\"\nmsgstr \"Hi\\n\"\n\"\\\"you\\\"\"\n\nmsgid \"Sin traducir\"\nmsgstr \"\"\n";
        let catalogo = leer_po(po);
        assert_eq!(catalogo.len(), 1);
        assert_eq!(catalogo["Hola\n\"tú\""], "Hi\n\"you\"");
    }

    #[test]
    fn tr_con_rellena_los_huecos_en_orden() {
        // En español (el idioma de los tests) el texto queda tal cual.
        assert_eq!(tr_con("Descargando {}… {} %", &["v1.2", "40"]), "Descargando v1.2… 40 %");
        assert_eq!(tr("Un texto sin traducción"), "Un texto sin traducción");
    }
}
