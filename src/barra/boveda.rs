//! Bóveda de textos: guardar, copiar, etiquetar y agrupar textos, y la cuenta de GitHub con la que
//! se sincroniza. En nook Pro, también imágenes y archivos, que se pasan al móvil vinculado (ver
//! `enlace`).

use crate::*;

/// Sección de la configuración donde están la cuenta y el móvil.
const SECCION_CUENTA: i32 = 1;
/// Lado del icono de un archivo en su tarjeta (ver ui/boveda.slint).
const LADO_ICONO_ARCHIVO: f32 = 28.0;
/// Lo que se ve marcado lo que se copia antes de cerrarse la barra (ver `MarcaCopiado` en
/// ui/boveda.slint, que aparece en 140 ms y cuya marca crece en 300).
const ESPERA_COPIADO: Duration = Duration::from_millis(450);

/// Para qué es el campo de arriba de la bóveda.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum EntradaBoveda {
    /// Guardar un texto nuevo en la vista que se ve.
    Texto,
    /// Poner la etiqueta del texto con esa posición en la vista.
    Etiqueta(usize),
    /// Poner nombre al grupo con esa posición en la vista de arriba.
    NombreGrupo(usize),
}

/// Lo que se ve de un texto de la bóveda en su tarjeta: sus primeras líneas con contenido,
/// recortadas (la tarjeta corta lo que no quepa con "…").
pub(crate) fn vista_de_texto(texto: &str) -> String {
    const LINEAS: usize = 4;
    const LARGO: usize = 200;
    texto
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(LINEAS)
        .map(|l| l.chars().take(LARGO).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Lo que mide un archivo, para su tarjeta: "512 B", "48 KB", "2,4 MB" (con `coma`, como se escribe
/// en español) o "35 MB".
pub(crate) fn tamano_visible(bytes: u64, coma: bool) -> String {
    const MB: u64 = 1 << 20;
    match bytes {
        b if b < 1024 => format!("{b} B"),
        b if b < MB => format!("{} KB", (b + 512) / 1024),
        b if b < 10 * MB => {
            let texto = format!("{:.1} MB", b as f64 / MB as f64);
            if coma { texto.replace('.', ",") } else { texto }
        }
        b => format!("{} MB", (b + MB / 2) / MB),
    }
}

/// `texto` con `largo` caracteres como mucho: si es más largo, con "…" en medio, que deja ver el
/// principio y el final (la extensión, en un nombre de archivo).
fn recortar(texto: &str, largo: usize) -> String {
    let n = texto.chars().count();
    if n <= largo {
        return texto.to_string();
    }
    let final_ = (largo - 1) / 3;
    let inicio: String = texto.chars().take(largo - 1 - final_).collect();
    let fin: String = texto.chars().skip(n - final_).collect();
    format!("{inicio}…{fin}")
}

/// Por qué no se pudo algo de los archivos o del móvil, para enseñarlo.
fn motivo(motivo: &enlace::Motivo) -> String {
    match motivo {
        enlace::Motivo::Carpeta => tr("las carpetas no se pueden guardar").into(),
        enlace::Motivo::Grande => tr("pasa de 100 MB").into(),
        enlace::Motivo::Llena => tr("la bóveda está llena (1 GB)").into(),
        enlace::Motivo::NoCoincide => tr("en el móvil se dijo que el código no coincide").into(),
        enlace::Motivo::SinTerminar => tr("no se terminó de comprobar el código").into(),
        enlace::Motivo::OtroAparato => tr("otro aparato intentó vincularse a la vez").into(),
        enlace::Motivo::Otro(e) => e.clone(),
    }
}

/// Tarjetas de la vista `vista` de la bóveda (None = la de arriba).
pub(crate) fn tarjetas_boveda(boveda: &boveda::Boveda, vista: Option<usize>) -> Vec<ItemBoveda> {
    let texto = |t: &boveda::Texto| ItemBoveda {
        es_grupo: false,
        titulo: vista_de_texto(&t.texto).into(),
        detalle: "".into(),
        etiqueta: t.etiqueta.clone().unwrap_or_default().into(),
        ..Default::default()
    };
    match vista {
        Some(g) => boveda.grupo(g).map_or(Vec::new(), |g| g.textos.iter().map(texto).collect()),
        None => boveda
            .elementos
            .iter()
            .map(|e| match e {
                boveda::Elemento::Texto(t) => texto(t),
                boveda::Elemento::Grupo(g) => ItemBoveda {
                    es_grupo: true,
                    titulo: g.nombre.as_str().into(),
                    detalle: if g.textos.len() == 1 { "1 texto".into() } else { format!("{} textos", g.textos.len()).into() },
                    ..Default::default()
                },
            })
            .collect(),
    }
}

impl Ctx {
    /// Pasa a la interfaz la vista de la bóveda que se ve y el estado de la cuenta.
    pub(crate) fn refrescar_boveda(&self, ui: &Barra, e: &mut Estado) {
        // Bóveda: si el grupo que se veía ya no existe, se vuelve a la vista de arriba.
        if e.grupo_boveda.is_some_and(|g| e.config.boveda.grupo(g).is_none()) {
            e.grupo_boveda = None;
            e.entrada_boveda = EntradaBoveda::Texto;
        }
        let vista = e.grupo_boveda;
        let mut tarjetas = tarjetas_boveda(&e.config.boveda, vista);
        // Los archivos van detrás de los textos, en la vista de arriba. Solo se miran con la bóveda
        // abierta: leer sus miniaturas la primera vez cuesta.
        if vista.is_none() && e.modo == Modo::Boveda {
            let coma = idioma::codigo_actual() == "es";
            for archivo in self.enlace.archivos() {
                let foto = if archivo.foto { self.enlace.foto(&archivo.id) } else { None };
                let es_foto = foto.is_some();
                let icono = || icono_de_tipo(&mut e.iconos, &archivo.nombre, lado_icono(LADO_ICONO_ARCHIVO));
                tarjetas.push(ItemBoveda {
                    titulo: archivo.nombre.as_str().into(),
                    detalle: tamano_visible(archivo.tamano, coma).into(),
                    es_archivo: true,
                    es_foto,
                    miniatura: foto.or_else(icono).unwrap_or_default(),
                    ..Default::default()
                });
            }
        }
        let largo = tarjetas.len();
        ui.set_boveda(modelo(tarjetas));
        ui.set_en_grupo_boveda(vista.is_some());
        ui.set_nombre_grupo_boveda(vista.and_then(|g| e.config.boveda.grupo(g)).map_or(String::new(), |g| g.nombre.clone()).into());
        let boveda = &e.config.boveda;
        let (entrada, pista) = match e.entrada_boveda {
            EntradaBoveda::Texto if vista.is_some() => (0, tr("Escribe o pega un texto para guardarlo en este grupo").into()),
            EntradaBoveda::Texto if let Some(aviso) = &e.aviso_boveda => (0, aviso.clone()),
            EntradaBoveda::Texto if PRO => (0, tr("Escribe o pega un texto, una imagen o un archivo").into()),
            EntradaBoveda::Texto => (0, tr("Escribe o pega un texto y pulsa Intro para guardarlo").into()),
            EntradaBoveda::Etiqueta(i) => match boveda.texto(vista, i).and_then(|t| t.etiqueta.as_deref()) {
                Some(actual) => (1, tr_con("Nueva etiqueta (ahora «{}»)", &[actual])),
                None => (1, tr("Etiqueta para el texto").into()),
            },
            EntradaBoveda::NombreGrupo(g) => match boveda.grupo(g) {
                Some(grupo) if grupo.nombre != boveda::nombre_grupo_nuevo() => (2, tr_con("Nuevo nombre (ahora «{}»)", &[&grupo.nombre])),
                _ => (2, tr("Nombre del grupo").to_string()),
            },
        };
        ui.set_entrada_boveda(entrada);
        ui.set_pista_entrada_boveda(pista.into());
        let ultimo = largo.saturating_sub(1) as i32;
        if ui.get_seleccion_boveda() > ultimo {
            ui.set_seleccion_boveda(ultimo);
        }

        let usuario = self.cuenta.usuario();
        ui.set_cuenta_estado(match (&usuario, &e.codigo_cuenta) {
            (Some(_), _) => 3,
            (None, Some(_)) => 2,
            (None, None) if e.conectando_cuenta => 1,
            _ => 0,
        });
        ui.set_cuenta_usuario(usuario.unwrap_or_default().into());
        ui.set_cuenta_codigo(e.codigo_cuenta.clone().unwrap_or_default().into());
        ui.set_cuenta_error(e.error_cuenta.as_str().into());

        let (estado, nombre, codigo) = match self.enlace.movil() {
            enlace::Movil::SinVincular => (0, String::new(), String::new()),
            enlace::Movil::Esperando => (1, String::new(), String::new()),
            enlace::Movil::Codigo { nombre, codigo, confirmado } => (if confirmado { 3 } else { 2 }, nombre, codigo),
            enlace::Movil::Vinculado(nombre) => (4, nombre, String::new()),
        };
        ui.set_movil_estado(estado);
        ui.set_movil_nombre(nombre.into());
        ui.set_movil_codigo(codigo.into());
        ui.set_movil_error(e.error_movil.as_str().into());
    }

    /// Activa o desactiva la bóveda: su botón en la tira y su atajo.
    pub(crate) fn alternar_usar_boveda(&self) {
        self.modificar_ajustes(|a| a.usar_boveda = !a.usar_boveda);
        self.registrar_atajos();
        self.refrescar();
    }

    /// Abre la bóveda (en la vista de arriba), recordando qué ventana tenía el foco para
    /// devolvérselo al copiar.
    pub(crate) fn abrir_boveda(&self) {
        if !self.estado.borrow().config.ajustes.usar_boveda {
            return;
        }
        if ventana::primer_plano_es_de_otro_proceso() {
            self.ventana_previa.set(Some(ventana::en_primer_plano()));
        }
        {
            let mut e = self.estado.borrow_mut();
            e.grupo_boveda = None;
            e.entrada_boveda = EntradaBoveda::Texto;
            e.aviso_boveda = None;
        }
        self.limpiar_campo_boveda(0);
        if let Some(ui) = self.ui.upgrade() {
            ui.set_copiado_boveda(-1);
        }
        self.poner_modo(Modo::Boveda);
        // Por si se cambió en otro ordenador.
        self.cuenta.descargar();
    }

    pub(crate) fn alternar_boveda(&self) {
        if self.estado.borrow().modo == Modo::Boveda {
            self.cerrar();
        } else {
            self.abrir_boveda();
        }
    }

    /// Botón de la cuenta en la configuración: conecta, cancela o desconecta según el caso.
    pub(crate) fn boton_cuenta(&self) {
        {
            let mut e = self.estado.borrow_mut();
            e.error_cuenta.clear();
            if self.cuenta.usuario().is_some() {
                self.cuenta.desconectar();
            } else if e.conectando_cuenta {
                self.cuenta.cancelar();
                e.conectando_cuenta = false;
                e.codigo_cuenta = None;
            } else {
                self.cuenta.conectar();
                e.conectando_cuenta = true;
            }
        }
        self.refrescar();
    }

    /// Botón del móvil en la configuración: vincula, cancela o desvincula según el caso.
    pub(crate) fn boton_movil(&self) {
        self.estado.borrow_mut().error_movil.clear();
        match self.enlace.movil() {
            enlace::Movil::SinVincular => self.enlace.vincular(),
            enlace::Movil::Vinculado(_) => self.enlace.desvincular(),
            _ => self.enlace.cancelar(),
        }
        self.refrescar();
    }

    /// Recoge lo que avisó el enlace (cambió algo de los archivos o del móvil).
    pub(crate) fn novedades_enlace(&self) {
        for evento in self.enlace.eventos() {
            match evento {
                // Para comprobar el código hay que verlo: se abre la configuración donde está.
                enlace::Evento::Codigo => {
                    if let Some(ui) = self.ui.upgrade() {
                        ui.set_seccion(SECCION_CUENTA);
                    }
                    if self.estado.borrow().modo != Modo::Config {
                        self.poner_modo(Modo::Config);
                    }
                }
                enlace::Evento::NoGuardado { nombre, motivo: m } => {
                    let nombre = recortar(&nombre, 32);
                    self.estado.borrow_mut().aviso_boveda = Some(tr_con("No se pudo guardar «{}»: {}", &[&nombre, &motivo(&m)]));
                }
                enlace::Evento::ErrorMovil(m) => self.estado.borrow_mut().error_movil = motivo(&m),
            }
        }
        self.refrescar();
    }

    /// Abre en el Explorador la carpeta donde se guardan las imágenes y los archivos.
    pub(crate) fn abrir_carpeta_boveda(&self) {
        if let Err(e) = sistema::lanzar(&self.enlace.carpeta().to_string_lossy()) {
            eprintln!("{e}");
        }
    }

    /// Guarda en la bóveda una copia de estos archivos (soltados en la barra con la bóveda abierta).
    pub(crate) fn guardar_archivos_en_boveda(&self, rutas: Vec<PathBuf>) {
        self.estado.borrow_mut().aviso_boveda = None;
        self.enlace.agregar(rutas);
        // Se ven en la vista de arriba.
        if self.estado.borrow().grupo_boveda.is_some() {
            self.ver_grupo_boveda(None);
        }
    }

    /// Ctrl+V en la bóveda: guarda los archivos o la imagen que se copiaron, si no es un texto.
    pub(crate) fn pegar_en_boveda(&self) -> bool {
        let nombre = format!("{} {}.png", tr("Imagen"), Local::now().format("%Y-%m-%d %H.%M.%S"));
        if !self.enlace.pegar(nombre) {
            return false;
        }
        self.estado.borrow_mut().aviso_boveda = None;
        if self.estado.borrow().grupo_boveda.is_some() {
            self.ver_grupo_boveda(None);
        }
        true
    }

    /// El archivo de la tarjeta `i` de la vista, si es un archivo (van detrás de los textos de la
    /// vista de arriba).
    fn archivo_de_boveda(&self, i: usize) -> Option<enlace::Archivo> {
        let e = self.estado.borrow();
        if e.grupo_boveda.is_some() {
            return None;
        }
        let j = i.checked_sub(e.config.boveda.largo(None))?;
        self.enlace.archivos().into_iter().nth(j)
    }

    /// Copia el archivo de la tarjeta `i` para pegarlo y, como con los textos, lo marca, cierra la
    /// barra y devuelve el foco a la ventana en la que se estaba.
    fn copiar_archivo(&self, i: usize, id: &str) {
        if !self.enlace.copiar(id) {
            eprintln!("No se pudo copiar el archivo: otro programa está usando el portapapeles");
            return;
        }
        self.tras_copiar(i);
    }

    /// Marca como copiada la tarjeta `i` y, al rato (lo que tarda en verse la marca), cierra la
    /// barra y devuelve el foco a la ventana en la que se estaba, para pegar lo copiado.
    fn tras_copiar(&self, i: usize) {
        let Some(ui) = self.ui.upgrade() else { return };
        ui.set_copiado_boveda(i as i32);
        let c = self.clone();
        Timer::single_shot(ESPERA_COPIADO, move || {
            let Some(ui) = c.ui.upgrade() else { return };
            // Si mientras tanto se cerró o se copió otra cosa, ya no toca.
            if c.estado.borrow().modo != Modo::Boveda || ui.get_copiado_boveda() != i as i32 {
                return;
            }
            c.cerrar();
            ui.set_copiado_boveda(-1);
            if let Some(ventana) = c.ventana_previa.take() {
                ventana::activar(ventana);
            }
        });
    }

    fn quitar_archivo(&self, id: &str) {
        self.enlace.quitar(id);
        self.refrescar();
    }

    /// Clic derecho en la tarjeta de un archivo.
    fn menu_archivo(&self, i: usize, archivo: enlace::Archivo) {
        const COPIAR: u32 = 1;
        const ABRIR: u32 = 2;
        const ELIMINAR: u32 = 3;
        let opciones = [
            Opcion::Normal(COPIAR, tr("Copiar")),
            Opcion::Normal(ABRIR, tr("Abrir")),
            Opcion::Separador,
            Opcion::Normal(ELIMINAR, tr("Eliminar")),
        ];
        self.mostrar_menu(&opciones, move |c, id| match id {
            COPIAR => c.copiar_archivo(i, &archivo.id),
            ABRIR => {
                if let Err(e) = sistema::lanzar(&archivo.ruta.to_string_lossy()) {
                    eprintln!("{e}");
                }
            }
            ELIMINAR => c.quitar_archivo(&archivo.id),
            _ => {}
        });
    }

    /// Recoge lo que avisó el hilo de la cuenta.
    pub(crate) fn novedades_cuenta(&self) {
        for evento in self.cuenta.eventos() {
            match evento {
                cuenta::Evento::Codigo { codigo, url } => {
                    // Puede llegar justo después de cancelar.
                    if !self.estado.borrow().conectando_cuenta {
                        continue;
                    }
                    // Se copia el código y se abre la página de GitHub donde hay que pegarlo.
                    portapapeles::poner(&codigo);
                    if let Err(e) = sistema::lanzar(&url) {
                        eprintln!("No se pudo abrir GitHub: {e}");
                    }
                    self.estado.borrow_mut().codigo_cuenta = Some(codigo);
                }
                cuenta::Evento::Conectada { remota } => {
                    {
                        let mut e = self.estado.borrow_mut();
                        e.conectando_cuenta = false;
                        e.codigo_cuenta = None;
                        e.error_cuenta.clear();
                        if let Some(remota) = remota {
                            e.config.boveda.fusionar(remota);
                        }
                    }
                    // Guarda y sube la bóveda juntada.
                    self.guardar();
                }
                cuenta::Evento::Descargada(remota) => {
                    let cambia = self.estado.borrow().config.boveda != remota;
                    if cambia {
                        self.estado.borrow_mut().config.boveda = remota;
                        self.guardar();
                    }
                }
                cuenta::Evento::Error(error) => {
                    let mut e = self.estado.borrow_mut();
                    e.conectando_cuenta = false;
                    e.codigo_cuenta = None;
                    e.error_cuenta = error;
                }
                cuenta::Evento::AlDia => self.estado.borrow_mut().error_cuenta.clear(),
            }
        }
        self.refrescar();
    }

    /// Vacía el campo de la bóveda y marca la tarjeta `seleccion`.
    pub(crate) fn limpiar_campo_boveda(&self, seleccion: i32) {
        if let Some(ui) = self.ui.upgrade() {
            ui.set_texto_boveda("".into());
            ui.set_seleccion_boveda(seleccion);
        }
    }

    /// Clic (o Intro) en una tarjeta: un texto o un archivo se copia; en un grupo se entra.
    pub(crate) fn elegir_en_boveda(&self, i: usize) {
        if let Some(archivo) = self.archivo_de_boveda(i) {
            return self.copiar_archivo(i, &archivo.id);
        }
        let (vista, es_grupo) = {
            let e = self.estado.borrow();
            let vista = e.grupo_boveda;
            (vista, vista.is_none() && e.config.boveda.grupo(i).is_some())
        };
        if es_grupo {
            self.ver_grupo_boveda(Some(i));
        } else {
            self.copiar_de_boveda(vista, i);
        }
    }

    /// Pasa a ver el grupo `g` de la bóveda (None = la vista de arriba).
    pub(crate) fn ver_grupo_boveda(&self, g: Option<usize>) {
        let anterior = {
            let mut e = self.estado.borrow_mut();
            let anterior = e.grupo_boveda;
            e.grupo_boveda = g;
            e.entrada_boveda = EntradaBoveda::Texto;
            anterior
        };
        // Al volver, queda marcado el grupo del que se sale.
        let seleccion = if g.is_none() { anterior.unwrap_or(0) as i32 } else { 0 };
        self.limpiar_campo_boveda(seleccion);
        self.refrescar();
        if let Some(ui) = self.ui.upgrade() {
            ui.invoke_mostrar_seleccion_boveda();
        }
    }

    /// Copia el texto `i` de la vista y, tras marcarlo, cierra la barra y devuelve el foco a la
    /// ventana en la que se estaba, para pegarlo.
    pub(crate) fn copiar_de_boveda(&self, vista: Option<usize>, i: usize) {
        let Some(texto) = self.estado.borrow().config.boveda.texto(vista, i).map(|t| t.texto.clone()) else { return };
        if !portapapeles::poner(&texto) {
            eprintln!("No se pudo copiar el texto: otro programa está usando el portapapeles");
            return;
        }
        self.tras_copiar(i);
    }

    /// Intro en el campo de arriba: guarda el texto nuevo, la etiqueta o el nombre del grupo.
    pub(crate) fn guardar_en_boveda(&self, texto: &str) {
        let (cambiado, seleccion) = {
            let mut e = self.estado.borrow_mut();
            let vista = e.grupo_boveda;
            let entrada = std::mem::replace(&mut e.entrada_boveda, EntradaBoveda::Texto);
            match entrada {
                EntradaBoveda::Texto => (e.config.boveda.guardar(vista, texto), 0),
                // Vacía, se deja como estaba (se quita con "Quitar etiqueta" del menú).
                EntradaBoveda::Etiqueta(i) if texto.trim().is_empty() => (false, i),
                EntradaBoveda::Etiqueta(i) => (e.config.boveda.poner_etiqueta(vista, i, texto), i),
                // Sin nombre se queda el que tenía.
                EntradaBoveda::NombreGrupo(g) => (e.config.boveda.renombrar_grupo(g, texto), g),
            }
        };
        if cambiado {
            self.guardar();
        }
        self.limpiar_campo_boveda(seleccion as i32);
        self.refrescar();
    }

    /// Esc mientras se escribe una etiqueta o un nombre: se deja como estaba.
    pub(crate) fn cancelar_entrada_boveda(&self) {
        self.estado.borrow_mut().entrada_boveda = EntradaBoveda::Texto;
        if let Some(ui) = self.ui.upgrade() {
            ui.set_texto_boveda("".into());
        }
        self.refrescar();
    }

    /// Pone el campo de arriba a escribir la etiqueta de un texto o el nombre de un grupo. El
    /// campo empieza vacío, listo para escribir el nuevo; el actual se ve en la pista.
    pub(crate) fn editar_en_boveda(&self, entrada: EntradaBoveda) {
        self.estado.borrow_mut().entrada_boveda = entrada;
        if let Some(ui) = self.ui.upgrade() {
            ui.set_texto_boveda("".into());
        }
        self.refrescar();
        self.enfocar();
    }

    /// Cambia algo de la bóveda y, si cambió, lo guarda y lo muestra.
    pub(crate) fn modificar_boveda(&self, cambio: impl FnOnce(&mut boveda::Boveda, Option<usize>) -> bool) {
        let cambiado = {
            let mut e = self.estado.borrow_mut();
            let vista = e.grupo_boveda;
            e.entrada_boveda = EntradaBoveda::Texto;
            cambio(&mut e.config.boveda, vista)
        };
        if cambiado {
            self.guardar();
        }
        self.refrescar();
    }

    pub(crate) fn quitar_de_boveda(&self, i: usize) {
        if let Some(archivo) = self.archivo_de_boveda(i) {
            return self.quitar_archivo(&archivo.id);
        }
        self.modificar_boveda(|b, vista| b.quitar(vista, i));
    }

    /// Texto `desde` soltado sobre `sobre` (en la vista de arriba): entra en el grupo o, si era
    /// otro texto, se crea un grupo con los dos y se pide su nombre.
    pub(crate) fn juntar_en_boveda(&self, desde: usize, sobre: usize) {
        let resultado = {
            let mut e = self.estado.borrow_mut();
            if e.grupo_boveda.is_some() {
                return;
            }
            e.config.boveda.juntar(desde, sobre)
        };
        let Some((grupo, nuevo)) = resultado else { return };
        self.guardar();
        self.limpiar_campo_boveda(grupo as i32);
        if nuevo {
            self.estado.borrow_mut().entrada_boveda = EntradaBoveda::NombreGrupo(grupo);
            self.enfocar();
        }
        self.refrescar();
    }

    /// Clic derecho en una tarjeta de la bóveda.
    pub(crate) fn menu_boveda(&self, i: usize) {
        const COPIAR: u32 = 1;
        const ETIQUETA: u32 = 2;
        const QUITAR_ETIQUETA: u32 = 3;
        const SACAR: u32 = 4;
        const ELIMINAR: u32 = 5;
        const ABRIR: u32 = 6;
        const RENOMBRAR: u32 = 7;
        const DESHACER: u32 = 8;
        if let Some(archivo) = self.archivo_de_boveda(i) {
            return self.menu_archivo(i, archivo);
        }
        let (vista, es_grupo, tiene_etiqueta) = {
            let e = self.estado.borrow();
            let vista = e.grupo_boveda;
            let boveda = &e.config.boveda;
            let es_grupo = vista.is_none() && boveda.grupo(i).is_some();
            (vista, es_grupo, boveda.texto(vista, i).is_some_and(|t| t.etiqueta.is_some()))
        };
        let mut opciones = Vec::new();
        if es_grupo {
            opciones.extend([
                Opcion::Normal(ABRIR, tr("Abrir")),
                Opcion::Normal(RENOMBRAR, tr("Cambiar nombre…")),
                Opcion::Normal(DESHACER, tr("Deshacer grupo")),
                Opcion::Separador,
                Opcion::Normal(ELIMINAR, tr("Eliminar grupo y sus textos")),
            ]);
        } else {
            opciones.push(Opcion::Normal(COPIAR, tr("Copiar")));
            opciones.push(Opcion::Normal(ETIQUETA, if tiene_etiqueta { tr("Cambiar etiqueta…") } else { tr("Poner etiqueta…") }));
            if tiene_etiqueta {
                opciones.push(Opcion::Normal(QUITAR_ETIQUETA, tr("Quitar etiqueta")));
            }
            if vista.is_some() {
                opciones.push(Opcion::Normal(SACAR, tr("Sacar del grupo")));
            }
            opciones.extend([Opcion::Separador, Opcion::Normal(ELIMINAR, tr("Eliminar"))]);
        }
        self.mostrar_menu(&opciones, move |c, id| match id {
            COPIAR => c.copiar_de_boveda(vista, i),
            ABRIR => c.ver_grupo_boveda(Some(i)),
            ETIQUETA => c.editar_en_boveda(EntradaBoveda::Etiqueta(i)),
            RENOMBRAR => c.editar_en_boveda(EntradaBoveda::NombreGrupo(i)),
            QUITAR_ETIQUETA => c.modificar_boveda(|b, vista| b.poner_etiqueta(vista, i, "")),
            SACAR => c.modificar_boveda(|b, vista| vista.is_some_and(|g| b.sacar_del_grupo(g, i))),
            DESHACER => c.modificar_boveda(|b, _| b.deshacer_grupo(i)),
            ELIMINAR => c.quitar_de_boveda(i),
            _ => {}
        });
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_alternar_boveda(move || c.alternar_boveda());
    let c = ctx.clone();
    ui.on_guardar_en_boveda(move |texto| c.guardar_en_boveda(&texto));
    let c = ctx.clone();
    ui.on_elegir_en_boveda(move |i| c.elegir_en_boveda(i.max(0) as usize));
    let c = ctx.clone();
    ui.on_juntar_en_boveda(move |desde, sobre| c.juntar_en_boveda(desde.max(0) as usize, sobre.max(0) as usize));
    let c = ctx.clone();
    ui.on_volver_en_boveda(move || c.ver_grupo_boveda(None));
    let c = ctx.clone();
    ui.on_cancelar_entrada_boveda(move || c.cancelar_entrada_boveda());
    let c = ctx.clone();
    ui.on_quitar_de_boveda(move |i| c.quitar_de_boveda(i.max(0) as usize));
    let c = ctx.clone();
    ui.on_menu_boveda(move |i| {
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.menu_boveda(i.max(0) as usize));
    });
    let c = ctx.clone();
    ui.on_pegar_en_boveda(move || c.pegar_en_boveda());
    let c = ctx.clone();
    ui.on_abrir_carpeta_boveda(move || c.abrir_carpeta_boveda());
    let c = ctx.clone();
    ui.on_boton_cuenta(move || c.boton_cuenta());
    let c = ctx.clone();
    ui.on_novedades_cuenta(move || c.novedades_cuenta());
    let c = ctx.clone();
    ui.on_boton_movil(move || c.boton_movil());
    let c = ctx.clone();
    ui.on_responder_movil(move |si| {
        c.enlace.responder(si);
        c.refrescar();
    });
    let c = ctx.clone();
    ui.on_novedades_enlace(move || c.novedades_enlace());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_vista_de_un_texto_son_sus_primeras_lineas_con_contenido() {
        assert_eq!(vista_de_texto("hola"), "hola");
        assert_eq!(vista_de_texto("\n  primera  \n\nsegunda"), "primera\nsegunda");
        assert_eq!(vista_de_texto("1\n2\n3\n4\n5"), "1\n2\n3\n4");
        assert_eq!(vista_de_texto(&"x".repeat(300)).chars().count(), 200);
        assert_eq!(vista_de_texto(""), "");
    }

    #[test]
    fn los_nombres_largos_se_recortan_por_en_medio() {
        assert_eq!(recortar("foto.jpg", 32), "foto.jpg");
        let recortado = recortar("Captura de pantalla 2026-11-04 215633.png", 20);
        assert_eq!(recortado, "Captura de pa…33.png");
        assert_eq!(recortado.chars().count(), 20);
    }

    #[test]
    fn lo_que_mide_un_archivo_se_lee_de_un_vistazo() {
        assert_eq!(tamano_visible(0, true), "0 B");
        assert_eq!(tamano_visible(1023, true), "1023 B");
        assert_eq!(tamano_visible(48 * 1024 + 100, true), "48 KB");
        assert_eq!(tamano_visible(2_516_582, true), "2,4 MB");
        assert_eq!(tamano_visible(2_516_582, false), "2.4 MB");
        assert_eq!(tamano_visible(35 << 20, true), "35 MB");
    }
}
