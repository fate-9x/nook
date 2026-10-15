//! Búsqueda de aplicaciones instaladas por nombre, como el buscador del menú Inicio.

use crate::config::App;

/// Cuántos resultados se muestran a la vez.
pub const MAX_RESULTADOS: usize = 8;

/// Apps que coinciden con `consulta`, de más a menos relevantes: primero las que empiezan por
/// el texto, luego las que tienen una palabra que empieza por él y al final las que lo
/// contienen. Sin texto se devuelven todas por orden alfabético. No distingue mayúsculas ni
/// tildes.
pub fn filtrar<'a>(apps: &'a [App], consulta: &str) -> Vec<&'a App> {
    let consulta = normalizar(consulta.trim());
    let mut encontradas: Vec<(u8, String, &App)> = apps
        .iter()
        .filter_map(|app| {
            let nombre = normalizar(&app.nombre);
            let puntos = if nombre.starts_with(&consulta) {
                0
            } else if nombre.split(|c: char| !c.is_alphanumeric()).any(|p| p.starts_with(&consulta)) {
                1
            } else if nombre.contains(&consulta) {
                2
            } else {
                return None;
            };
            Some((puntos, nombre, app))
        })
        .collect();
    encontradas.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    encontradas.into_iter().map(|(_, _, app)| app).collect()
}

/// Descarta lo que aparece entre las aplicaciones del menú Inicio pero no es una aplicación:
/// documentos, enlaces web y desinstaladores.
pub fn es_aplicacion(nombre: &str, ruta: &str) -> bool {
    const DOCUMENTOS: [&str; 12] =
        [".txt", ".pdf", ".chm", ".htm", ".html", ".url", ".rtf", ".md", ".doc", ".docx", ".ini", ".log"];
    let ruta = ruta.to_lowercase();
    let nombre = normalizar(nombre);
    !(ruta.contains("http://")
        || ruta.contains("https://")
        || DOCUMENTOS.iter().any(|ext| ruta.ends_with(ext))
        || nombre.starts_with("uninstall")
        || nombre.starts_with("desinstalar"))
}

/// Minúsculas y sin tildes, para comparar como lo haría una persona.
fn normalizar(texto: &str) -> String {
    texto
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            otro => otro,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(nombre: &str) -> App {
        App { nombre: nombre.into(), ruta: String::new() }
    }

    fn nombres(apps: &[&App]) -> Vec<String> {
        apps.iter().map(|a| a.nombre.clone()).collect()
    }

    #[test]
    fn primero_lo_que_empieza_por_el_texto() {
        let apps = [app("Visual Studio Code"), app("Code Writer"), app("Unicode Tool"), app("Notas")];
        assert_eq!(nombres(&filtrar(&apps, "code")), ["Code Writer", "Visual Studio Code", "Unicode Tool"]);
    }

    #[test]
    fn sin_texto_todas_en_orden_alfabetico() {
        let apps = [app("Paint"), app("calculadora"), app("Bloc de notas")];
        assert_eq!(nombres(&filtrar(&apps, "  ")), ["Bloc de notas", "calculadora", "Paint"]);
    }

    #[test]
    fn ignora_mayusculas_y_tildes() {
        let apps = [app("Configuración"), app("Música")];
        assert_eq!(nombres(&filtrar(&apps, "CONFIGURACION")), ["Configuración"]);
        assert_eq!(nombres(&filtrar(&apps, "musi")), ["Música"]);
        assert!(filtrar(&apps, "xyz").is_empty());
    }

    #[test]
    fn descarta_lo_que_no_es_una_aplicacion() {
        assert!(es_aplicacion("Calculadora", r"shell:AppsFolder\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"));
        assert!(es_aplicacion("Bloc de notas", r"shell:AppsFolder\{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\notepad.exe"));
        assert!(!es_aplicacion("Léeme", r"shell:AppsFolder\{6D809377}\Programa\readme.txt"));
        assert!(!es_aplicacion("Sitio web", r"shell:AppsFolder\https://example.com"));
        assert!(!es_aplicacion("Uninstall Programa", r"shell:AppsFolder\{6D809377}\Programa\unins000.exe"));
    }
}
