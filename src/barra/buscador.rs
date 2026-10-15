//! Buscador de aplicaciones instaladas: las busca y añade la elegida a la barra o a un grupo
//! (botón "+") o la abre (lupa).

use crate::*;

/// Dónde se pone la app elegida en el buscador.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Destino {
    /// Suelta en la tira de la barra.
    Barra,
    /// Dentro del grupo con ese índice (se vuelve a él al terminar).
    Grupo(usize),
    /// En ningún sitio: se abre.
    Abrir,
}

impl Ctx {
    /// Abre el buscador para añadir una app instalada a la barra o a un grupo, o para abrirla.
    pub(crate) fn abrir_buscador(&self, destino: Destino) {
        self.estado.borrow_mut().destino = destino;
        if let Some(ui) = self.ui.upgrade() {
            let lugar = match destino {
                Destino::Barra => tr("Busca una aplicación para la barra").to_string(),
                Destino::Grupo(i) => {
                    let e = self.estado.borrow();
                    let nombre = e.config.elementos.get(i).map_or("", Elemento::nombre);
                    tr_con("Busca una aplicación para «{}»", &[nombre])
                }
                Destino::Abrir => tr("Busca una aplicación para abrirla").to_string(),
            };
            ui.set_buscando_para_abrir(destino == Destino::Abrir);
            ui.set_texto_busqueda(SharedString::new());
            ui.set_lugar_busqueda(lugar.into());
        }
        // Se abre con las apps que se leyeron la última vez, y se vuelven a leer por si se instaló
        // o desinstaló algo (leerlas tarda: se hace en segundo plano).
        self.buscar("");
        self.poner_modo(Modo::Buscar);
        self.reenfocar_buscador();
        self.leer_instaladas();
    }

    /// Lee las apps instaladas en segundo plano. Si cambiaron y el buscador está abierto, se
    /// vuelve a buscar con lo que hay escrito.
    pub(crate) fn leer_instaladas(&self) {
        let c = self.clone();
        segundo_plano::hacer(sistema::apps_instaladas, move |leidas| {
            let (instaladas, claves): (Vec<App>, Vec<Vec<String>>) = leidas.into_iter().unzip();
            let buscando = {
                let mut e = c.estado.borrow_mut();
                // Con sus claves se reconocen las apps abiertas (y se ven con su nombre y su icono).
                for (app, claves) in instaladas.iter().zip(claves) {
                    e.claves.insert(app.ruta.clone(), claves);
                }
                if e.instaladas == instaladas {
                    return;
                }
                e.instaladas = instaladas;
                e.modo == Modo::Buscar
            };
            c.sondear_abiertas();
            if let (true, Some(ui)) = (buscando, c.ui.upgrade()) {
                c.buscar(&ui.get_texto_busqueda());
            }
        });
    }

    /// Muestra las apps que coinciden con el texto escrito.
    pub(crate) fn buscar(&self, texto: &str) {
        let Some(ui) = self.ui.upgrade() else { return };
        let mut guarda = self.estado.borrow_mut();
        let e = &mut *guarda;
        e.resultados = buscador::filtrar(&e.instaladas, texto)
            .into_iter()
            .take(buscador::MAX_RESULTADOS)
            .cloned()
            .collect();
        // Los iconos que aún no se tienen se sacan en segundo plano; mientras, se ve la inicial.
        let lado = lado_icono(LADO_ICONO_BUSQUEDA);
        let mut items = Vec::new();
        let mut faltan = Vec::new();
        for app in &e.resultados {
            let clave = (app.ruta.clone(), lado);
            let icono = match e.iconos.get(&clave) {
                Some(icono) => icono.clone(),
                None => {
                    if e.iconos_pedidos.insert(clave) {
                        faltan.push(app.ruta.clone());
                    }
                    None
                }
            };
            items.push(AppItem {
                nombre: app.nombre.as_str().into(),
                inicial: app.inicial().into(),
                tiene_icono: icono.is_some(),
                icono: icono.unwrap_or_default(),
                es_icono_grupo: false,
            });
        }
        ui.set_resultados(modelo(items));
        ui.set_seleccion(0);
        drop(guarda);
        if !faltan.is_empty() {
            self.pedir_iconos_de_busqueda(faltan, lado);
        }
    }

    /// Saca en segundo plano los iconos de esas apps y los pone en los resultados que se estén
    /// viendo cuando estén listos.
    pub(crate) fn pedir_iconos_de_busqueda(&self, rutas: Vec<String>, lado: u32) {
        let c = self.clone();
        let sacar = move || rutas.into_iter().map(|ruta| (sistema::pixeles_icono(&ruta, lado), ruta)).collect::<Vec<_>>();
        segundo_plano::hacer(sacar, move |iconos| {
            let Some(ui) = c.ui.upgrade() else { return };
            let mut guarda = c.estado.borrow_mut();
            let e = &mut *guarda;
            for (pixeles, ruta) in iconos {
                let clave = (ruta, lado);
                e.iconos_pedidos.remove(&clave);
                e.iconos.insert(clave, pixeles.map(|p| sistema::imagen(&p, lado)));
            }
            let resultados = ui.get_resultados();
            for (i, app) in e.resultados.iter().enumerate() {
                let Some(Some(icono)) = e.iconos.get(&(app.ruta.clone(), lado)) else { continue };
                if let Some(item) = resultados.row_data(i).filter(|item| !item.tiene_icono) {
                    resultados.set_row_data(i, AppItem { icono: icono.clone(), tiene_icono: true, ..item });
                }
            }
        });
    }

    /// Al momento de abrirse, se vuelve a poner el foco en el campo, para poder escribir siempre
    /// nada más abrirlo: Windows puede tardar en dejar la barra delante, y con la tecla Windows
    /// (con la barra delante le llega al pulsarse, y tapada por el gancho se suelta después, ya con
    /// el buscador abierto) el campo podría creerla aún pulsada y tomar lo escrito por un atajo:
    /// se da por soltada.
    fn reenfocar_buscador(&self) {
        let c = self.clone();
        Timer::single_shot(Duration::from_millis(80), move || {
            let Some(ui) = c.ui.upgrade() else { return };
            if c.estado.borrow().modo != Modo::Buscar {
                return;
            }
            ui.window().dispatch_event(slint::platform::WindowEvent::KeyReleased { text: Key::Meta.into() });
            c.enfocar();
        });
    }

    /// Lupa: abre el buscador para abrir una app, o lo cierra si ya está abierto para eso (en nook
    /// Pro).
    pub(crate) fn alternar_lanzador(&self) {
        if !PRO {
            return;
        }
        let abierto = {
            let e = self.estado.borrow();
            e.modo == Modo::Buscar && e.destino == Destino::Abrir
        };
        if abierto {
            self.cerrar();
        } else {
            self.abrir_buscador(Destino::Abrir);
        }
    }

    /// Añade el resultado `i` donde se abrió el buscador (o lo abre).
    pub(crate) fn elegir_resultado(&self, i: usize) {
        let (app, destino) = {
            let e = self.estado.borrow();
            (e.resultados.get(i).cloned(), e.destino)
        };
        let Some(app) = app else { return };
        match destino {
            Destino::Barra => {
                self.agregar_elementos(vec![Elemento::app(app)]);
                self.cerrar();
            }
            Destino::Grupo(g) => {
                if let Some(grupo) = self.estado.borrow_mut().config.elementos.get_mut(g).and_then(Elemento::grupo_mut) {
                    grupo.apps.push(app);
                }
                self.guardar();
                self.mostrar_grupo(g, true);
            }
            Destino::Abrir => self.abrir_ruta(&app.ruta),
        }
    }

    /// Esc en el buscador: vuelve al grupo que se estaba editando, o cierra.
    pub(crate) fn salir_del_buscador(&self) {
        let destino = self.estado.borrow().destino;
        match destino {
            Destino::Grupo(g) => self.mostrar_grupo(g, true),
            Destino::Barra | Destino::Abrir => self.cerrar(),
        }
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_agregar_a_barra(move || c.abrir_buscador(Destino::Barra));
    let c = ctx.clone();
    ui.on_alternar_lanzador(move || c.alternar_lanzador());
    let c = ctx.clone();
    ui.on_alternar_boton_buscar(move || c.modificar_ajustes(|a| a.boton_buscar = !a.boton_buscar));
    let c = ctx.clone();
    ui.on_buscar(move |texto| c.buscar(&texto));
    let c = ctx.clone();
    ui.on_elegir_resultado(move |i| {
        // Se difiere: cambia el modo y la interfaz desde dentro de su propio evento.
        let c = c.clone();
        Timer::single_shot(Duration::ZERO, move || c.elegir_resultado(i as usize));
    });
    let c = ctx.clone();
    ui.on_salir_del_buscador(move || c.salir_del_buscador());
}
