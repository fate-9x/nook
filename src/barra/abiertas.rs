//! Apps abiertas en la tira: antes que las de la barra van las que tienen ventanas en la barra de
//! tareas de Windows (con un punto debajo, como allí), en el orden en que se abrieron; las de la
//! barra que están abiertas salen ahí y no se repiten detrás. Un clic trae su ventana (o, si ya
//! estaba delante, la minimiza o pasa a la siguiente) y el clic derecho tiene sus opciones.

use crate::*;

/// App abierta: sus claves (ver `abiertas`), sus ventanas (de la de más arriba a la de más abajo)
/// y con qué se abre otra ventana y se saca su icono (la del menú Inicio, si se encuentra).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Abierta {
    pub(crate) claves: Vec<String>,
    pub(crate) ventanas: Vec<isize>,
    pub(crate) app: App,
}

/// Qué hay en una posición de la tira.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Ranura {
    /// El elemento de la barra con ese índice (abierto o no).
    Elemento(usize),
    /// La app abierta con ese índice, que no está en la barra.
    Abierta(usize),
}

impl Ctx {
    /// Mira qué apps tienen ventanas en la barra de tareas (si está activo mostrarlas) y, si
    /// cambiaron, las pone en la tira. Las que siguen abiertas conservan su sitio y las nuevas van
    /// detrás.
    pub(crate) fn sondear_abiertas(&self) {
        let cambia = {
            let mut guarda = self.estado.borrow_mut();
            let e = &mut *guarda;
            let nuevas = if e.config.ajustes.apps_abiertas {
                let ventanas = abiertas::ventanas(&mut e.ventanas_vistas);
                mantener_orden(&e.abiertas, agrupar(e, ventanas))
            } else {
                Vec::new()
            };
            let cambia = nuevas.len() != e.abiertas.len()
                || nuevas.iter().zip(&e.abiertas).any(|(a, b)| a.claves != b.claves || a.app != b.app);
            e.abiertas = nuevas;
            cambia
        };
        if cambia {
            self.refrescar();
        }
    }

    /// Saca en segundo plano las claves de las apps de la barra que aún no se conocen, para
    /// reconocer sus ventanas.
    pub(crate) fn pedir_claves(&self, rutas: Vec<String>) {
        let c = self.clone();
        let sacar = move || rutas.into_iter().map(|ruta| (abiertas::claves_de_app(&ruta), ruta)).collect::<Vec<_>>();
        segundo_plano::hacer(sacar, move |claves| {
            {
                let mut e = c.estado.borrow_mut();
                for (claves, ruta) in claves {
                    e.claves_pedidas.remove(&ruta);
                    e.claves.insert(ruta, claves);
                }
            }
            c.refrescar();
        });
    }

    /// Clic en la app abierta `k`: como en la barra de tareas.
    pub(crate) fn activar_abierta(&self, k: usize) {
        let (ventanas, previa) = {
            let e = self.estado.borrow();
            let Some(a) = e.abiertas.get(k) else { return };
            (a.ventanas.clone(), e.ultima_ajena)
        };
        self.cerrar();
        abiertas::alternar(&ventanas, previa);
    }

    /// Clic derecho en la app abierta `k` (`elemento`: el de la barra que es, si está en ella).
    pub(crate) fn menu_abierta(&self, k: usize, elemento: Option<usize>) {
        const OTRA: u32 = 1;
        const FIJAR: u32 = 2;
        const QUITAR: u32 = 3;
        const CERRAR: u32 = 4;
        let (app, ventanas) = {
            let e = self.estado.borrow();
            let Some(a) = e.abiertas.get(k) else { return };
            // Las de la barra se abren como en ella.
            let app = elemento.and_then(|i| e.config.elementos.get(i)).and_then(Elemento::app_icono).unwrap_or(&a.app);
            (app.clone(), a.ventanas.clone())
        };
        let mut opciones = vec![Opcion::Normal(OTRA, tr("Abrir otra ventana"))];
        opciones.push(match elemento {
            Some(_) => Opcion::Normal(QUITAR, tr("Quitar de la barra")),
            None => Opcion::Normal(FIJAR, tr("Fijar en la barra")),
        });
        opciones.push(Opcion::Separador);
        opciones.push(Opcion::Normal(CERRAR, if ventanas.len() > 1 { tr("Cerrar todas las ventanas") } else { tr("Cerrar ventana") }));
        self.mostrar_menu(&opciones, move |c, id| match id {
            OTRA => c.abrir_ruta(&app.ruta),
            FIJAR => {
                c.agregar_elementos(vec![Elemento::app(app)]);
            }
            QUITAR => {
                if let Some(i) = elemento {
                    c.quitar_elemento(i);
                }
            }
            CERRAR => {
                for ventana in ventanas {
                    abiertas::cerrar(ventana);
                }
                // Se van al cerrarse (algunas preguntan antes si guardar).
                let c = c.clone();
                Timer::single_shot(Duration::from_millis(400), move || c.sondear_abiertas());
            }
            _ => {}
        });
    }
}

/// Junta las ventanas por app (cada una con su `App`).
fn agrupar(e: &Estado, ventanas: Vec<abiertas::Ventana>) -> Vec<Abierta> {
    let mut grupos: Vec<Abierta> = Vec::new();
    for ventana in ventanas {
        if let Some(grupo) = grupos.iter_mut().find(|g| abiertas::misma_app(&g.claves, &ventana.claves)) {
            grupo.ventanas.push(ventana.hwnd);
            for clave in ventana.claves {
                if !grupo.claves.contains(&clave) {
                    grupo.claves.push(clave);
                }
            }
        } else {
            let app = app_de(e, &ventana);
            grupos.push(Abierta { claves: ventana.claves, ventanas: vec![ventana.hwnd], app });
        }
    }
    grupos
}

/// La app de una ventana: la del menú Inicio, si se encuentra; si no, su ejecutable o (las de la
/// Tienda) su identificador.
fn app_de(e: &Estado, ventana: &abiertas::Ventana) -> App {
    let instalada = e.instaladas.iter().find(|a| e.claves.get(&a.ruta).is_some_and(|c| abiertas::misma_app(c, &ventana.claves)));
    if let Some(app) = instalada {
        return app.clone();
    }
    if !ventana.exe.is_empty() {
        return App::desde_ruta(std::path::Path::new(&ventana.exe));
    }
    let id = ventana.id.clone().unwrap_or_default();
    App { nombre: abiertas::titulo(ventana.hwnd), ruta: format!(r"shell:AppsFolder\{id}") }
}

/// Las apps abiertas `nuevas` con las que ya estaban en el mismo orden que antes y las recién
/// abiertas detrás.
fn mantener_orden(anteriores: &[Abierta], mut nuevas: Vec<Abierta>) -> Vec<Abierta> {
    let mut ordenadas = Vec::with_capacity(nuevas.len());
    for anterior in anteriores {
        if let Some(i) = nuevas.iter().position(|n| abiertas::misma_app(&n.claves, &anterior.claves)) {
            ordenadas.push(nuevas.remove(i));
        }
    }
    ordenadas.extend(nuevas);
    ordenadas
}

/// Qué va en cada posición de la tira: primero las apps abiertas (las que están en la barra, como
/// su elemento) y luego los elementos de la barra que no están abiertos. `de_la_barra[k]` es el
/// elemento que es la abierta `k`, si lo es.
pub(crate) fn ranuras(abiertas: usize, elementos: usize, de_la_barra: &[Option<usize>]) -> Vec<Ranura> {
    let abiertos: Vec<usize> = de_la_barra.iter().flatten().copied().collect();
    (0..abiertas)
        .map(|k| de_la_barra.get(k).copied().flatten().map_or(Ranura::Abierta(k), Ranura::Elemento))
        .chain((0..elementos).filter(|i| !abiertos.contains(i)).map(Ranura::Elemento))
        .collect()
}

/// Conecta los eventos de la interfaz de esta parte.
pub(crate) fn conectar(ui: &Barra, ctx: &Ctx) {
    let c = ctx.clone();
    ui.on_alternar_apps_abiertas(move || {
        c.modificar_ajustes(|a| a.apps_abiertas = !a.apps_abiertas);
        c.sondear_abiertas();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(pro)]
    fn abierta(clave: &str) -> Abierta {
        Abierta { claves: vec![clave.into()], ventanas: vec![1], app: App { nombre: clave.into(), ruta: clave.into() } }
    }

    // Reconocer la misma app es de nook Pro (en la gratuita no hay apps abiertas).
    #[cfg(pro)]
    #[test]
    fn las_abiertas_conservan_su_sitio_y_las_nuevas_van_detras() {
        let antes = [abierta("a"), abierta("b"), abierta("c")];
        // Ahora (de la ventana de más arriba a la de más abajo): d nueva, c, a; b se cerró.
        let ahora = mantener_orden(&antes, vec![abierta("d"), abierta("c"), abierta("a")]);
        let claves: Vec<&str> = ahora.iter().map(|a| a.claves[0].as_str()).collect();
        assert_eq!(claves, ["a", "c", "d"]);
    }

    #[test]
    fn las_abiertas_van_primero_y_las_de_la_barra_no_se_repiten() {
        // Tres abiertas (la segunda es el elemento 2 de la barra) y cuatro elementos.
        assert_eq!(
            ranuras(3, 4, &[None, Some(2), None]),
            [
                Ranura::Abierta(0),
                Ranura::Elemento(2),
                Ranura::Abierta(2),
                Ranura::Elemento(0),
                Ranura::Elemento(1),
                Ranura::Elemento(3)
            ]
        );
        // Sin abiertas, la barra tal cual.
        assert_eq!(ranuras(0, 2, &[]), [Ranura::Elemento(0), Ranura::Elemento(1)]);
    }
}
