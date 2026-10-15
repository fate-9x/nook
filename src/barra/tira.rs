//! La tira de la barra: sus grupos y apps sueltas (abrirlos, añadirlos, moverlos y quitarlos), el
//! grupo abierto y los archivos que se arrastran desde el Explorador. Delante van las apps
//! abiertas (ver `barra::abiertas`): lo que llega de la interfaz es la posición en la tira, que
//! `ranura` traduce a lo que hay en ella.

use crate::*;

/// Tamaño (en píxeles lógicos) con que se dibujan los iconos en la tira, en la cuadrícula de un
/// grupo y en el buscador (los `IconoApp` de ui/barra.slint, ui/grupo.slint y
/// ui/buscador.slint).
pub(crate) const LADO_ICONO_TIRA: f32 = 22.0;
pub(crate) const LADO_ICONO_CELDA: f32 = 40.0;
pub(crate) const LADO_ICONO_BUSQUEDA: f32 = 26.0;

/// Cuánto crecen como mucho los iconos de la tira con el efecto lupa (`lupa-maxima` en
/// ui/barra.slint).
pub(crate) const LUPA_MAXIMA: f32 = 1.45;

/// Lado en píxeles de pantalla de un icono de `lado` píxeles lógicos, con la escala actual.
pub(crate) fn lado_icono(lado: f32) -> u32 {
    (lado * plataforma::escala()).round().max(1.0) as u32
}

pub(crate) fn icono_de(iconos: &mut HashMap<(String, u32), Option<Image>>, ruta: &str, lado: u32) -> Option<Image> {
    iconos.entry((ruta.to_string(), lado)).or_insert_with(|| sistema::icono(ruta, lado)).clone()
}

/// Índice del grupo siguiente (o anterior) a `actual`, saltando las apps sueltas.
/// `grupos[i]` indica si el elemento `i` es un grupo.
pub(crate) fn siguiente_grupo(grupos: &[bool], actual: usize, adelante: bool) -> Option<usize> {
    let n = grupos.len();
    (1..=n)
        .map(|paso| if adelante { (actual + paso) % n } else { (actual + n - paso) % n })
        .find(|&i| grupos[i])
}

/// Dónde queda el elemento que estaba en `i` después de mover el de `desde` a `hasta`.
pub(crate) fn indice_tras_mover(i: usize, desde: usize, hasta: usize) -> usize {
    if i == desde {
        hasta
    } else if desde < i && i <= hasta {
        i - 1
    } else if hasta <= i && i < desde {
        i + 1
    } else {
        i
    }
}

impl Ctx {
    /// Pasa a la interfaz los elementos de la barra (para la lista de atajos), la tira (las apps
    /// abiertas y los elementos) y las apps del grupo abierto.
    pub(crate) fn refrescar_tira(&self, ui: &Barra, e: &mut Estado) {
        let (lado_tira, lado_celda) = (lado_icono(LADO_ICONO_TIRA), lado_icono(LADO_ICONO_CELDA));
        let lado_grande = lado_icono(LADO_ICONO_TIRA * LUPA_MAXIMA);
        let mut elementos = Vec::new();
        for (i, el) in e.config.elementos.iter().enumerate() {
            let icono = el.app_icono().and_then(|a| icono_de(&mut e.iconos, &a.ruta, lado_tira));
            let icono_grande = el.app_icono().and_then(|a| icono_de(&mut e.iconos, &a.ruta, lado_grande));
            elementos.push(ElementoItem {
                nombre: el.nombre().into(),
                inicial: config::inicial(el.nombre()).into(),
                tiene_icono: icono.is_some(),
                icono: icono.unwrap_or_default(),
                es_app: el.grupo().is_none(),
                atajo: texto_atajo(el.atajo()).into(),
                atajo_ok: el.atajo().is_none() || e.atajos_ok.get(i).copied().unwrap_or(false),
                tiene_atajo: el.atajo().is_some(),
                icono_grande: icono_grande.unwrap_or_default(),
                abierta: false,
                fija: true,
            });
        }

        // Qué elemento de la barra es cada app abierta (las apps sueltas, por sus claves; las que
        // aún no se conocen se piden y la tira se vuelve a hacer cuando llegan).
        let mut faltan = Vec::new();
        let mut de_la_barra: Vec<Option<usize>> = Vec::new();
        for abierta in &e.abiertas {
            let elemento = e.config.elementos.iter().enumerate().position(|(i, el)| {
                let Elemento::App(suelta) = el else { return false };
                if de_la_barra.contains(&Some(i)) {
                    return false;
                }
                match e.claves.get(&suelta.app.ruta) {
                    Some(claves) => abiertas::misma_app(claves, &abierta.claves),
                    None => {
                        if !e.claves_pedidas.contains(&suelta.app.ruta) && !faltan.contains(&suelta.app.ruta) {
                            faltan.push(suelta.app.ruta.clone());
                        }
                        false
                    }
                }
            });
            de_la_barra.push(elemento);
        }
        if !faltan.is_empty() {
            e.claves_pedidas.extend(faltan.iter().cloned());
            self.pedir_claves(faltan);
        }
        e.tira = ranuras(e.abiertas.len(), e.config.elementos.len(), &de_la_barra);
        let mut tira = Vec::new();
        for (posicion, ranura) in e.tira.iter().enumerate() {
            let abierta = posicion < e.abiertas.len();
            let item = match *ranura {
                Ranura::Elemento(i) => ElementoItem { abierta, fija: !abierta, ..elementos[i].clone() },
                Ranura::Abierta(k) => {
                    let app = &e.abiertas[k].app;
                    let icono = icono_de(&mut e.iconos, &app.ruta, lado_tira);
                    let icono_grande = icono_de(&mut e.iconos, &app.ruta, lado_grande);
                    ElementoItem {
                        nombre: app.nombre.as_str().into(),
                        inicial: app.inicial().into(),
                        tiene_icono: icono.is_some(),
                        icono: icono.unwrap_or_default(),
                        es_app: true,
                        atajo: "".into(),
                        atajo_ok: true,
                        tiene_atajo: false,
                        icono_grande: icono_grande.unwrap_or_default(),
                        abierta: true,
                        fija: false,
                    }
                }
            };
            tira.push(item);
        }
        ui.set_elementos(modelo(elementos));
        ui.set_fijas(tira.iter().filter(|t| t.fija).count() as i32);
        ui.set_tira(modelo(tira));

        let mut apps = Vec::new();
        if let Some(g) = e.config.elementos.get(e.activo).and_then(Elemento::grupo) {
            let indice_icono = if g.icono < g.apps.len() { g.icono } else { 0 };
            for (i, app) in g.apps.iter().enumerate() {
                let icono = icono_de(&mut e.iconos, &app.ruta, lado_celda);
                apps.push(AppItem {
                    nombre: app.nombre.as_str().into(),
                    inicial: app.inicial().into(),
                    tiene_icono: icono.is_some(),
                    icono: icono.unwrap_or_default(),
                    es_icono_grupo: i == indice_icono,
                });
            }
        }
        ui.set_apps(modelo(apps));
        let activo = e.tira.iter().position(|&r| r == Ranura::Elemento(e.activo));
        ui.set_grupo_activo(activo.map_or(-1, |p| p as i32));
    }

    /// Qué hay en la posición `posicion` de la tira.
    pub(crate) fn ranura(&self, posicion: usize) -> Option<Ranura> {
        self.estado.borrow().tira.get(posicion).copied()
    }

    /// La app abierta que es el elemento `indice` de la barra, si lo está (las abiertas son las
    /// primeras posiciones de la tira, en su orden).
    pub(crate) fn abierta_del_elemento(&self, indice: usize) -> Option<usize> {
        let e = self.estado.borrow();
        e.tira.iter().position(|&r| r == Ranura::Elemento(indice)).filter(|&p| p < e.abiertas.len())
    }

    /// Clic en la posición `posicion` de la tira.
    pub(crate) fn pulsar_en_tira(&self, posicion: usize) {
        match self.ranura(posicion) {
            Some(Ranura::Elemento(i)) => self.abrir_elemento(i),
            Some(Ranura::Abierta(k)) => self.activar_abierta(k),
            None => {}
        }
    }

    /// Cambia de sitio un elemento de la tira arrastrándolo (de una posición a otra entre los de
    /// la barra que no están abiertos).
    pub(crate) fn mover_en_tira(&self, desde: usize, hasta: usize) {
        if let (Some(Ranura::Elemento(desde)), Some(Ranura::Elemento(hasta))) = (self.ranura(desde), self.ranura(hasta)) {
            self.mover_elemento(desde, hasta);
        }
    }

    /// Pulsación en el elemento `indice` de la barra (o su atajo): una app suelta se abre (si ya
    /// está abierta, se trae su ventana); un grupo se despliega (o se cierra si ya estaba
    /// abierto).
    pub(crate) fn abrir_elemento(&self, indice: usize) {
        if let Some(k) = self.abierta_del_elemento(indice) {
            self.activar_abierta(k);
            return;
        }
        let (ruta, ya_abierto) = {
            let e = self.estado.borrow();
            let ruta = match e.config.elementos.get(indice) {
                None => return,
                Some(Elemento::App(s)) => Some(s.app.ruta.clone()),
                Some(Elemento::Grupo(_)) => None,
            };
            (ruta, e.modo == Modo::Grupo && e.activo == indice)
        };
        if let Some(ruta) = ruta {
            self.abrir_ruta(&ruta);
        } else if ya_abierto {
            self.cerrar();
        } else {
            self.mostrar_grupo(indice, false);
        }
    }

    /// Grupo que abre el atajo "mostrar": el último abierto, o el primero que haya.
    pub(crate) fn grupo_preferido(&self) -> Option<usize> {
        let e = self.estado.borrow();
        let es_grupo = |i: usize| e.config.elementos.get(i).is_some_and(|el| el.grupo().is_some());
        if es_grupo(e.activo) { Some(e.activo) } else { (0..e.config.elementos.len()).find(|&i| es_grupo(i)) }
    }

    pub(crate) fn mostrar_grupo(&self, indice: usize, editar: bool) {
        let nombre = {
            let mut e = self.estado.borrow_mut();
            e.activo = indice;
            e.config.elementos.get(indice).map(|el| el.nombre().to_string()).unwrap_or_default()
        };
        if let Some(ui) = self.ui.upgrade() {
            ui.set_nombre_grupo(nombre.into());
        }
        self.poner_modo(Modo::Grupo);
        if let Some(ui) = self.ui.upgrade() {
            ui.set_editando(editar);
        }
    }

    /// Abre la app `i` del grupo abierto.
    pub(crate) fn lanzar(&self, i: usize) {
        let ruta = {
            let e = self.estado.borrow();
            let grupo = e.config.elementos.get(e.activo).and_then(Elemento::grupo);
            grupo.and_then(|g| g.apps.get(i)).map(|a| a.ruta.clone())
        };
        if let Some(ruta) = ruta {
            self.abrir_ruta(&ruta);
        }
    }

    pub(crate) fn abrir_ruta(&self, ruta: &str) {
        self.cerrar();
        if let Err(e) = sistema::lanzar(ruta) {
            eprintln!("{e}");
        }
    }

    /// Ejecuta `cambio` sobre el grupo abierto y guarda.
    pub(crate) fn modificar_grupo(&self, cambio: impl FnOnce(&mut Grupo)) {
        {
            let mut e = self.estado.borrow_mut();
            let activo = e.activo;
            let Some(grupo) = e.config.elementos.get_mut(activo).and_then(Elemento::grupo_mut) else { return };
            cambio(grupo);
        }
        self.guardar();
        self.refrescar();
    }

    /// Añade elementos al final de la tira y devuelve el índice del primero.
    pub(crate) fn agregar_elementos(&self, nuevos: Vec<Elemento>) -> usize {
        let indice = {
            let mut e = self.estado.borrow_mut();
            let indice = e.config.elementos.len();
            e.config.elementos.extend(nuevos);
            e.config.completar_atajos(indice);
            indice
        };
        self.guardar();
        self.registrar_atajos();
        self.refrescar();
        indice
    }

    /// Cambia de sitio un elemento de la tira (arrastrándolo a lo largo de ella).
    pub(crate) fn mover_elemento(&self, desde: usize, hasta: usize) {
        {
            let mut e = self.estado.borrow_mut();
            let n = e.config.elementos.len();
            if desde >= n || hasta >= n || desde == hasta {
                return;
            }
            let elemento = e.config.elementos.remove(desde);
            e.config.elementos.insert(hasta, elemento);
            e.activo = indice_tras_mover(e.activo, desde, hasta);
            e.grabando = None;
        }
        self.guardar();
        // Los atajos se registran por posición en la tira.
        self.registrar_atajos();
        self.refrescar();
    }

    /// Crea un grupo vacío y lo abre para ponerle nombre y apps.
    pub(crate) fn nuevo_grupo(&self) {
        let indice = self.agregar_elementos(vec![Elemento::Grupo(Grupo::nuevo(tr("Nuevo grupo")))]);
        self.mostrar_grupo(indice, true);
    }

    /// Quita el elemento `indice` de la tira (un grupo, con todas sus apps).
    pub(crate) fn quitar_elemento(&self, indice: usize) {
        let era_el_abierto = {
            let mut e = self.estado.borrow_mut();
            if indice >= e.config.elementos.len() {
                return;
            }
            e.config.elementos.remove(indice);
            let era_el_abierto = e.activo == indice;
            // Los elementos que había detrás se desplazan una posición.
            if e.activo > indice {
                e.activo -= 1;
            } else if era_el_abierto {
                e.activo = 0;
            }
            era_el_abierto
        };
        self.guardar();
        self.registrar_atajos();
        if era_el_abierto && self.estado.borrow().modo == Modo::Grupo {
            self.cerrar();
        } else {
            self.refrescar();
            self.recolocar();
        }
    }

    pub(crate) fn eliminar_grupo_activo(&self) {
        let activo = self.estado.borrow().activo;
        self.quitar_elemento(activo);
    }

    pub(crate) fn arrastre(&self, evento: Arrastre) {
        let Some(ui) = self.ui.upgrade() else { return };
        match evento {
            Arrastre::Entra => {
                ui.set_arrastrando(true);
                self.revelar();
            }
            Arrastre::Mueve(x, y) => self.seguir_cursor(x, y),
            Arrastre::Sale => ui.set_arrastrando(false),
            Arrastre::Suelta(rutas, x, y) => {
                self.seguir_cursor(x, y);
                self.soltar(rutas);
                ui.set_arrastrando(false);
            }
        }
    }

    /// Pasa a la interfaz la posición del cursor (en píxeles de pantalla) relativa a la caja.
    pub(crate) fn seguir_cursor(&self, cx: i32, cy: i32) {
        let (Some(ui), Some((x, y, _, _))) = (self.ui.upgrade(), self.caja.get()) else { return };
        let escala = plataforma::escala();
        ui.set_arrastre_x((cx - x) as f32 / escala);
        ui.set_arrastre_y((cy - y) as f32 / escala);
    }

    /// Añade los archivos soltados al grupo que está bajo el cursor, o sueltos a la barra si
    /// se sueltan en "+".
    pub(crate) fn soltar(&self, rutas: Vec<PathBuf>) {
        let Some(ui) = self.ui.upgrade() else { return };
        let apps: Vec<App> = rutas.iter().map(|r| App::desde_ruta(r)).collect();
        if apps.is_empty() {
            return;
        }
        // Posición en la tira: la que sigue a la última es el "+".
        let destino = ui.get_destino_arrastre();
        let n = self.estado.borrow().tira.len() as i32;
        if destino == n {
            self.agregar_elementos(apps.into_iter().map(Elemento::app).collect());
            return;
        }
        let Some(Ranura::Elemento(indice)) = usize::try_from(destino).ok().and_then(|p| self.ranura(p)) else { return };
        let agregadas = match self.estado.borrow_mut().config.elementos.get_mut(indice) {
            Some(Elemento::Grupo(g)) => {
                g.apps.extend(apps);
                true
            }
            _ => false,
        };
        if agregadas {
            self.guardar();
            self.mostrar_grupo(indice, false);
        }
    }
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_abrir_elemento(move |posicion| c.pulsar_en_tira(posicion.max(0) as usize));
    let c = ctx.clone();
    ui.on_mover_elemento(move |desde, hasta| c.mover_en_tira(desde.max(0) as usize, hasta.max(0) as usize));
    let c = ctx.clone();
    ui.on_lanzar(move |i| c.lanzar(i as usize));
    let c = ctx.clone();
    ui.on_quitar_app(move |i| {
        c.modificar_grupo(|g| {
            if (i as usize) < g.apps.len() {
                g.apps.remove(i as usize);
            }
        })
    });
    let c = ctx.clone();
    ui.on_usar_como_icono(move |i| c.modificar_grupo(|g| g.icono = i as usize));
    let c = ctx.clone();
    ui.on_renombrar(move |nombre| c.modificar_grupo(|g| g.nombre = nombre.to_string()));
    let c = ctx.clone();
    ui.on_eliminar_grupo(move || c.eliminar_grupo_activo());
    let c = ctx.clone();
    ui.on_agregar_apps(move || {
        let activo = c.estado.borrow().activo;
        c.abrir_buscador(Destino::Grupo(activo));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_al_mover_un_elemento() {
        // Mover el 1 al 3 en [a, b, c, d] da [a, c, d, b].
        assert_eq!([0, 1, 2, 3].map(|i| indice_tras_mover(i, 1, 3)), [0, 3, 1, 2]);
        // Mover el 3 al 0 en [a, b, c, d] da [d, a, b, c].
        assert_eq!([0, 1, 2, 3].map(|i| indice_tras_mover(i, 3, 0)), [1, 2, 3, 0]);
    }

    #[test]
    fn tab_salta_las_apps_sueltas() {
        // Grupo, app, grupo, app.
        let grupos = [true, false, true, false];
        assert_eq!(siguiente_grupo(&grupos, 0, true), Some(2));
        assert_eq!(siguiente_grupo(&grupos, 2, true), Some(0));
        assert_eq!(siguiente_grupo(&grupos, 0, false), Some(2));
        assert_eq!(siguiente_grupo(&grupos, 2, false), Some(0));
        // Un solo grupo: se queda en él.
        assert_eq!(siguiente_grupo(&[false, true], 1, true), Some(1));
        assert_eq!(siguiente_grupo(&[false, false], 0, true), None);
        assert_eq!(siguiente_grupo(&[], 0, true), None);
    }
}
