//! Panel de un addon: su `panel.slint`, que se compila al abrirlo por primera vez con el
//! intérprete de Slint y se mete en la barra (el `ComponentContainer` de ui/barra.slint).
//!
//! Los addons pueden usar lo de nook con `import { Tema } from "@nook/tema.slint";` (también
//! `@nook/controles.slint` e `@nook/iconos.slint`): así se ven como el resto de la barra. Solo se
//! leen archivos de la carpeta del addon y esos.

use slint_interpreter::{Compiler, ComponentDefinition};
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// Archivos de ui/ que se ofrecen a los addons como la biblioteca `@nook`.
const BIBLIOTECA: [(&str, &str); 3] = [
    ("tema.slint", include_str!("../../ui/tema.slint")),
    ("controles.slint", include_str!("../../ui/controles.slint")),
    ("iconos.slint", include_str!("../../ui/iconos.slint")),
];

/// Nombre con el que el panel exporta el `Tema` de nook, para que nook le pase los colores. Se
/// añade al principio del archivo, en la misma línea, para que los errores salgan en su línea.
pub const TEMA: &str = "TemaDeNook";
const CABECERA: &str = "import { Tema as TemaDeNook } from \"@nook/tema.slint\"; export { TemaDeNook } ";

/// Componente que tiene que exportar `panel.slint`.
const COMPONENTE: &str = "Panel";

/// Carpeta (que no existe) en la que el compilador cree que está la biblioteca `@nook`.
fn carpeta_biblioteca() -> PathBuf {
    std::env::temp_dir().join("nook-biblioteca")
}

/// Compila el `panel.slint` de la carpeta de un addon. Si falla, devuelve los errores.
pub fn compilar(carpeta: &Path) -> Result<ComponentDefinition, String> {
    let ruta = carpeta.join("panel.slint");
    let fuente = std::fs::read_to_string(&ruta).map_err(|e| format!("{}: {e}", ruta.display()))?;

    let mut compilador = Compiler::default();
    let biblioteca = carpeta_biblioteca();
    compilador.set_library_paths(HashMap::from([("nook".to_string(), biblioteca.clone())]));
    let carpeta_addon = carpeta.to_path_buf();
    compilador.set_file_loader(move |ruta| {
        let respuesta = cargar(ruta, &biblioteca, &carpeta_addon);
        Box::pin(async move { respuesta })
    });

    let resultado = bloquear(compilador.build_from_source(format!("{CABECERA}{fuente}"), ruta));
    let errores: Vec<String> = resultado
        .diagnostics()
        .filter(|d| d.level() == slint_interpreter::DiagnosticLevel::Error)
        .map(|d| {
            let (linea, columna) = d.line_column();
            format!("panel.slint:{linea}:{columna}: {}", d.message())
        })
        .collect();
    if !errores.is_empty() {
        return Err(errores.join("\n"));
    }
    resultado.component(COMPONENTE).ok_or_else(|| format!("panel.slint no exporta el componente {COMPONENTE}"))
}

/// Lo que lee el compilador al importar un archivo: los de la biblioteca de nook salen de dentro
/// del ejecutable, los de la carpeta del addon del disco (`None`: como siempre) y nada más.
fn cargar(ruta: &Path, biblioteca: &Path, carpeta: &Path) -> Option<std::io::Result<String>> {
    if let Ok(relativa) = ruta.strip_prefix(biblioteca) {
        let nombre = relativa.to_string_lossy();
        return Some(
            BIBLIOTECA
                .iter()
                .find(|(archivo, _)| *archivo == nombre)
                .map(|(_, fuente)| fuente.to_string())
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("@nook/{nombre} no existe"))),
        );
    }
    if ruta.starts_with(carpeta) {
        return None;
    }
    Some(Err(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        format!("un addon solo puede usar archivos de su carpeta: {}", ruta.display()),
    )))
}

/// Espera a que termine una tarea que no espera a nada (la compilación con `cargar`, que siempre
/// responde enseguida).
fn bloquear<T>(tarea: impl Future<Output = T>) -> T {
    let mut tarea = pin!(tarea);
    let mut contexto = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(valor) = tarea.as_mut().poll(&mut contexto) {
            return valor;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carpeta_con(panel: &str) -> PathBuf {
        let carpeta = std::env::temp_dir().join(format!("nook-prueba-panel-{}", crate::azar::siguiente().to_bits()));
        std::fs::create_dir_all(&carpeta).unwrap();
        std::fs::write(carpeta.join("panel.slint"), panel).unwrap();
        carpeta
    }

    #[test]
    fn compila_un_panel_con_la_biblioteca_de_nook() {
        let carpeta = carpeta_con(
            r#"import { Tema } from "@nook/tema.slint";
            import { BotonTexto } from "@nook/controles.slint";
            export component Panel inherits Window {
                background: transparent;
                in property <int> numero;
                callback tirar();
                BotonTexto { texto: "Tirar " + root.numero; clicked => { root.tirar(); } }
                Text { color: Tema.texto; }
            }"#,
        );
        let definicion = compilar(&carpeta);
        let _ = std::fs::remove_dir_all(&carpeta);
        let definicion = definicion.unwrap();
        assert!(definicion.callbacks().any(|c| c == "tirar"));
    }

    #[test]
    fn los_errores_dicen_la_linea_del_panel() {
        let carpeta = carpeta_con("export component Panel inherits Window {\n    no-existe: 3;\n}");
        let error = compilar(&carpeta);
        let _ = std::fs::remove_dir_all(&carpeta);
        assert!(error.err().unwrap().contains("panel.slint:2:"));
    }

    #[test]
    fn no_puede_usar_archivos_de_fuera_de_su_carpeta() {
        let carpeta = carpeta_con(r#"import { X } from "../otro.slint"; export component Panel inherits Window { }"#);
        let error = compilar(&carpeta);
        let _ = std::fs::remove_dir_all(&carpeta);
        assert!(error.is_err());
    }
}
