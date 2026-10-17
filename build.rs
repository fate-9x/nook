// Antes de compilar el programa: compila la interfaz (ui/barra.slint) a código Rust, con las
// traducciones de lang/ dentro, prepara el icono del ejecutable (assets/nook.ico) y decide la
// versión.
use std::path::Path;
use std::process::Command;

fn main() {
    // El panel de los addons (`ComponentContainer`, ver src/addons/panel.rs) todavía es una función
    // experimental de Slint.
    // SAFETY: el script de compilación no tiene más hilos que lean el entorno.
    unsafe { std::env::set_var("SLINT_ENABLE_EXPERIMENTAL_FEATURES", "1") };
    // Los textos de `@tr(…)` se traducen sin contexto: así los .po sirven igual para los de Rust
    // (ver src/idioma.rs).
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("lang")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/barra.slint", config).expect("error compilando ui/barra.slint");
    icono(Path::new("assets/nook.ico"));
    version();
    pro();
}

/// Con la carpeta src/pro (solo está en nook Pro) se compila nook Pro: `cfg(pro)`, con sus
/// módulos de ahí en vez de los de src/gratis (ver src/main.rs).
fn pro() {
    println!("cargo::rustc-check-cfg=cfg(pro)");
    if Path::new("src/pro").is_dir() {
        println!("cargo:rerun-if-changed=src/pro");
        println!("cargo:rustc-cfg=pro");
    }
}

/// Deja la versión en `NOOK_VERSION` (sin la "v"). Sale del tag: el workflow de GitHub Actions
/// la pasa en esa misma variable de entorno; en local se toma el último tag de git (`vX.Y.Z`), y
/// si no hay ninguno, la del Cargo.toml.
fn version() {
    println!("cargo:rerun-if-env-changed=NOOK_VERSION");
    for ruta in [".git/HEAD", ".git/refs/tags", ".git/packed-refs"] {
        if Path::new(ruta).exists() {
            println!("cargo:rerun-if-changed={ruta}");
        }
    }
    let del_entorno = std::env::var("NOOK_VERSION").ok().filter(|v| !v.trim().is_empty());
    let de_git = || {
        let salida = Command::new("git").args(["describe", "--tags", "--abbrev=0", "--match", "v[0-9]*"]).output().ok()?;
        let tag = String::from_utf8(salida.stdout).ok()?;
        (salida.status.success() && !tag.trim().is_empty()).then(|| tag.trim().to_string())
    };
    let version = del_entorno.or_else(de_git).unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap());
    println!("cargo:rustc-env=NOOK_VERSION={}", version.trim().trim_start_matches('v'));
}

/// Mete el icono en el .exe. Se escribe a mano el archivo de recursos (.res) que generaría el
/// compilador de recursos de Windows, y el enlazador de MSVC lo incluye en el ejecutable: así no
/// hace falta tener instalado `rc.exe`.
fn icono(ruta: &Path) {
    println!("cargo:rerun-if-changed={}", ruta.display());
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let ico = std::fs::read(ruta).expect("no se pudo leer assets/nook.ico");
    let destino = Path::new(&std::env::var("OUT_DIR").unwrap()).join("nook.res");
    std::fs::write(&destino, recursos(&ico)).expect("no se pudo escribir el archivo de recursos");
    println!("cargo:rustc-link-arg-bins={}", destino.display());
}

/// Archivo .res con las imágenes del .ico (un recurso RT_ICON por tamaño) y el grupo que las
/// reúne (RT_GROUP_ICON); el primer grupo es el icono que Windows muestra para el ejecutable.
fn recursos(ico: &[u8]) -> Vec<u8> {
    const RT_ICON: u16 = 3;
    const RT_GROUP_ICON: u16 = 14;
    let u16_en = |i: usize| u16::from_le_bytes([ico[i], ico[i + 1]]);
    let u32_en = |i: usize| u32::from_le_bytes([ico[i], ico[i + 1], ico[i + 2], ico[i + 3]]);

    // Cabecera vacía con la que empieza todo archivo .res.
    let mut res = Vec::new();
    recurso(&mut res, 0, 0, 0, &[]);

    let cuantas = u16_en(4) as usize;
    let mut grupo = Vec::new();
    for cabecera in [0u16, 1, cuantas as u16] {
        grupo.extend_from_slice(&cabecera.to_le_bytes());
    }
    for n in 0..cuantas {
        let entrada = 6 + 16 * n;
        let (bytes, desde) = (u32_en(entrada + 8) as usize, u32_en(entrada + 12) as usize);
        let id = n as u16 + 1;
        recurso(&mut res, RT_ICON, id, 0x1010, &ico[desde..desde + bytes]);
        // Entrada del grupo: lo mismo que en el .ico, pero con el id del recurso en lugar de la
        // posición en el archivo.
        grupo.extend_from_slice(&ico[entrada..entrada + 12]);
        grupo.extend_from_slice(&id.to_le_bytes());
    }
    recurso(&mut res, RT_GROUP_ICON, 1, 0x1030, &grupo);
    res
}

/// Añade un recurso con tipo e identificador numéricos.
fn recurso(res: &mut Vec<u8>, tipo: u16, id: u16, banderas: u16, datos: &[u8]) {
    let mut cabecera = Vec::with_capacity(32);
    cabecera.extend_from_slice(&(datos.len() as u32).to_le_bytes());
    cabecera.extend_from_slice(&32u32.to_le_bytes());
    for v in [0xFFFF, tipo, 0xFFFF, id] {
        cabecera.extend_from_slice(&v.to_le_bytes());
    }
    cabecera.extend_from_slice(&0u32.to_le_bytes()); // versión de los datos
    cabecera.extend_from_slice(&banderas.to_le_bytes());
    cabecera.extend_from_slice(&0u16.to_le_bytes()); // idioma neutro
    cabecera.extend_from_slice(&0u32.to_le_bytes()); // versión
    cabecera.extend_from_slice(&0u32.to_le_bytes()); // características
    res.extend_from_slice(&cabecera);
    res.extend_from_slice(datos);
    while res.len() % 4 != 0 {
        res.push(0);
    }
}
