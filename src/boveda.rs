//! Bóveda de textos: textos guardados para copiarlos con un clic, sueltos o en grupos, cada uno
//! con una etiqueta opcional.
//!
//! Lo que se ve en cada momento es una vista: la de arriba (`None`), con los grupos y los textos
//! sueltos mezclados, o la de dentro de un grupo (`Some(g)`), con sus textos. Las operaciones
//! reciben la vista y la posición dentro de ella.

use crate::idioma::tr;
use serde::{Deserialize, Serialize};

/// Cuántos textos caben en la bóveda (al guardar uno más se olvida el más antiguo).
pub const MAX_TEXTOS: usize = 100;

/// Nombre de un grupo recién creado juntando dos textos (luego se le pone otro), en el idioma
/// de la interfaz.
pub fn nombre_grupo_nuevo() -> &'static str {
    tr("Grupo nuevo")
}

/// Un texto guardado.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(from = "TextoGuardado")]
pub struct Texto {
    pub texto: String,
    /// Etiqueta corta para reconocerlo (opcional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etiqueta: Option<String>,
}

/// Cómo puede venir un texto en el archivo: las versiones anteriores lo guardaban solo, sin
/// etiqueta.
#[derive(Deserialize)]
#[serde(untagged)]
enum TextoGuardado {
    Solo(String),
    Completo {
        texto: String,
        #[serde(default)]
        etiqueta: Option<String>,
    },
}

impl From<TextoGuardado> for Texto {
    fn from(guardado: TextoGuardado) -> Self {
        match guardado {
            TextoGuardado::Solo(texto) => Texto::new(texto),
            TextoGuardado::Completo { texto, etiqueta } => Texto { texto, etiqueta },
        }
    }
}

impl Texto {
    pub fn new(texto: impl Into<String>) -> Self {
        Texto { texto: texto.into(), etiqueta: None }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Grupo {
    pub nombre: String,
    pub textos: Vec<Texto>,
}

/// Lo que hay en la vista de arriba: un grupo o un texto suelto.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Elemento {
    Grupo(Grupo),
    Texto(Texto),
}

impl Elemento {
    fn texto(&self) -> Option<&Texto> {
        match self {
            Elemento::Texto(t) => Some(t),
            Elemento::Grupo(_) => None,
        }
    }
}

/// Todos los textos de la bóveda, de arriba abajo (lo más reciente primero).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(transparent)]
pub struct Boveda {
    pub elementos: Vec<Elemento>,
}

impl Boveda {
    pub fn grupo(&self, g: usize) -> Option<&Grupo> {
        match self.elementos.get(g) {
            Some(Elemento::Grupo(grupo)) => Some(grupo),
            _ => None,
        }
    }

    fn grupo_mut(&mut self, g: usize) -> Option<&mut Grupo> {
        match self.elementos.get_mut(g) {
            Some(Elemento::Grupo(grupo)) => Some(grupo),
            _ => None,
        }
    }

    /// Cuántos elementos tiene la vista.
    pub fn largo(&self, vista: Option<usize>) -> usize {
        match vista {
            None => self.elementos.len(),
            Some(g) => self.grupo(g).map_or(0, |grupo| grupo.textos.len()),
        }
    }

    /// El texto de la posición `i` de la vista (None si ahí hay un grupo o nada).
    pub fn texto(&self, vista: Option<usize>, i: usize) -> Option<&Texto> {
        match vista {
            None => self.elementos.get(i)?.texto(),
            Some(g) => self.grupo(g)?.textos.get(i),
        }
    }

    fn texto_mut(&mut self, vista: Option<usize>, i: usize) -> Option<&mut Texto> {
        match vista {
            None => match self.elementos.get_mut(i)? {
                Elemento::Texto(t) => Some(t),
                Elemento::Grupo(_) => None,
            },
            Some(g) => self.grupo_mut(g)?.textos.get_mut(i),
        }
    }

    /// Guarda un texto el primero de la vista (si ya estaba en ella, sube arriba y conserva su
    /// etiqueta). Los textos vacíos no se guardan. Devuelve si se guardó.
    pub fn guardar(&mut self, vista: Option<usize>, texto: &str) -> bool {
        if texto.trim().is_empty() {
            return false;
        }
        match vista {
            None => {
                let anterior = self.elementos.iter().position(|e| e.texto().is_some_and(|t| t.texto == texto));
                let nuevo = match anterior {
                    Some(i) => self.elementos.remove(i),
                    None => Elemento::Texto(Texto::new(texto)),
                };
                self.elementos.insert(0, nuevo);
            }
            Some(g) => {
                let Some(grupo) = self.grupo_mut(g) else { return false };
                let anterior = grupo.textos.iter().position(|t| t.texto == texto);
                let nuevo = match anterior {
                    Some(i) => grupo.textos.remove(i),
                    None => Texto::new(texto),
                };
                grupo.textos.insert(0, nuevo);
            }
        }
        self.limitar();
        true
    }

    /// Olvida los textos más antiguos que sobren: primero los sueltos del final y luego los del
    /// final del último grupo.
    fn limitar(&mut self) {
        let total = |b: &Boveda| {
            b.elementos.iter().map(|e| match e {
                Elemento::Grupo(g) => g.textos.len(),
                Elemento::Texto(_) => 1,
            }).sum::<usize>()
        };
        while total(self) > MAX_TEXTOS {
            if let Some(i) = self.elementos.iter().rposition(|e| e.texto().is_some()) {
                self.elementos.remove(i);
            } else if let Some(Elemento::Grupo(g)) = self.elementos.last_mut() {
                g.textos.pop();
                if g.textos.is_empty() {
                    self.elementos.pop();
                }
            }
        }
    }

    /// Quita lo que hay en la posición `i` de la vista (un texto, o arriba un grupo entero con
    /// sus textos). Un grupo que se queda vacío desaparece. Devuelve si se quitó algo.
    pub fn quitar(&mut self, vista: Option<usize>, i: usize) -> bool {
        match vista {
            None if i < self.elementos.len() => {
                self.elementos.remove(i);
                true
            }
            None => false,
            Some(g) => {
                let Some(grupo) = self.grupo_mut(g) else { return false };
                if i >= grupo.textos.len() {
                    return false;
                }
                grupo.textos.remove(i);
                if grupo.textos.is_empty() {
                    self.elementos.remove(g);
                }
                true
            }
        }
    }

    /// Pone (o quita, con una etiqueta vacía) la etiqueta del texto `i` de la vista.
    pub fn poner_etiqueta(&mut self, vista: Option<usize>, i: usize, etiqueta: &str) -> bool {
        let Some(texto) = self.texto_mut(vista, i) else { return false };
        let etiqueta = etiqueta.trim();
        texto.etiqueta = (!etiqueta.is_empty()).then(|| etiqueta.to_string());
        true
    }

    /// Cambia el nombre del grupo `g` (un nombre vacío no se acepta).
    pub fn renombrar_grupo(&mut self, g: usize, nombre: &str) -> bool {
        let nombre = nombre.trim();
        match self.grupo_mut(g) {
            Some(grupo) if !nombre.is_empty() => {
                grupo.nombre = nombre.to_string();
                true
            }
            _ => false,
        }
    }

    /// Junta el texto suelto `desde` con lo que hay en `sobre` (arriba): si es un grupo, el texto
    /// entra en él (el primero); si es otro texto, se crea un grupo con los dos en el sitio de
    /// `sobre`. Devuelve dónde queda el grupo y si es nuevo.
    pub fn juntar(&mut self, desde: usize, sobre: usize) -> Option<(usize, bool)> {
        if desde == sobre || sobre >= self.elementos.len() {
            return None;
        }
        let Elemento::Texto(texto) = self.elementos.get(desde)?.clone() else { return None };
        // Al quitar `desde`, lo que estaba detrás sube una posición.
        let destino = if desde < sobre { sobre - 1 } else { sobre };
        self.elementos.remove(desde);
        let nuevo = match &mut self.elementos[destino] {
            Elemento::Grupo(grupo) => {
                grupo.textos.insert(0, texto);
                false
            }
            Elemento::Texto(otro) => {
                let otro = otro.clone();
                self.elementos[destino] = Elemento::Grupo(Grupo { nombre: nombre_grupo_nuevo().into(), textos: vec![otro, texto] });
                true
            }
        };
        Some((destino, nuevo))
    }

    /// Saca el texto `i` del grupo `g` y lo deja suelto justo detrás del grupo (o en su sitio,
    /// si el grupo se queda vacío y desaparece).
    pub fn sacar_del_grupo(&mut self, g: usize, i: usize) -> bool {
        let Some(grupo) = self.grupo_mut(g) else { return false };
        if i >= grupo.textos.len() {
            return false;
        }
        let texto = grupo.textos.remove(i);
        if grupo.textos.is_empty() {
            self.elementos[g] = Elemento::Texto(texto);
        } else {
            self.elementos.insert(g + 1, Elemento::Texto(texto));
        }
        true
    }

    /// Deshace el grupo `g`: sus textos quedan sueltos en su sitio, en el mismo orden.
    pub fn deshacer_grupo(&mut self, g: usize) -> bool {
        let Some(grupo) = self.grupo(g).cloned() else { return false };
        self.elementos.splice(g..=g, grupo.textos.into_iter().map(Elemento::Texto));
        true
    }

    /// Junta con esta bóveda `otra` (la de la cuenta, al conectarla): queda `otra` y detrás lo
    /// que solo estaba en esta. Los textos de un grupo van al grupo de `otra` con el mismo
    /// nombre, si lo hay.
    pub fn fusionar(&mut self, otra: Boveda) {
        let propia = std::mem::replace(self, otra);
        for elemento in propia.elementos {
            match elemento {
                Elemento::Texto(t) => {
                    if !self.contiene(&t.texto) {
                        self.elementos.push(Elemento::Texto(t));
                    }
                }
                Elemento::Grupo(g) => {
                    let textos: Vec<Texto> = g.textos.into_iter().filter(|t| !self.contiene(&t.texto)).collect();
                    if textos.is_empty() {
                        continue;
                    }
                    let mismo = self.elementos.iter_mut().find_map(|e| match e {
                        Elemento::Grupo(o) if o.nombre == g.nombre => Some(o),
                        _ => None,
                    });
                    match mismo {
                        Some(grupo) => grupo.textos.extend(textos),
                        None => self.elementos.push(Elemento::Grupo(Grupo { nombre: g.nombre, textos })),
                    }
                }
            }
        }
        self.limitar();
    }

    /// Si el texto está en la bóveda, suelto o en un grupo.
    fn contiene(&self, texto: &str) -> bool {
        self.elementos.iter().any(|e| match e {
            Elemento::Texto(t) => t.texto == texto,
            Elemento::Grupo(g) => g.textos.iter().any(|t| t.texto == texto),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn textos(b: &Boveda, vista: Option<usize>) -> Vec<String> {
        (0..b.largo(vista)).map(|i| b.texto(vista, i).map_or("[grupo]".into(), |t| t.texto.clone())).collect()
    }

    fn con(lista: &[&str]) -> Boveda {
        let mut b = Boveda::default();
        for t in lista.iter().rev() {
            b.guardar(None, t);
        }
        b
    }

    #[test]
    fn guarda_arriba_sin_repetir_ni_textos_vacios() {
        let mut b = Boveda::default();
        assert!(b.guardar(None, "hola"));
        assert!(b.guardar(None, "adiós"));
        b.poner_etiqueta(None, 1, "saludo");
        assert!(b.guardar(None, "hola"));
        assert_eq!(textos(&b, None), ["hola", "adiós"]);
        // Al subir, conserva su etiqueta.
        assert_eq!(b.texto(None, 0).unwrap().etiqueta.as_deref(), Some("saludo"));
        assert!(!b.guardar(None, "   \n"));
        assert_eq!(b.largo(None), 2);
    }

    #[test]
    fn no_guarda_mas_de_la_cuenta() {
        let mut b = Boveda::default();
        for i in 0..MAX_TEXTOS + 5 {
            b.guardar(None, &format!("texto {i}"));
        }
        assert_eq!(b.largo(None), MAX_TEXTOS);
        assert_eq!(b.texto(None, 0).unwrap().texto, format!("texto {}", MAX_TEXTOS + 4));
    }

    #[test]
    fn un_texto_sobre_otro_crea_un_grupo_y_sobre_un_grupo_entra() {
        let mut b = con(&["a", "b", "c", "d"]);
        assert_eq!(b.juntar(3, 1), Some((1, true)));
        assert_eq!(textos(&b, None), ["a", "[grupo]", "c"]);
        assert_eq!(b.grupo(1).unwrap().nombre, nombre_grupo_nuevo());
        assert_eq!(textos(&b, Some(1)), ["b", "d"]);
        // Desde delante del grupo: el grupo se corre una posición.
        assert_eq!(b.juntar(0, 1), Some((0, false)));
        assert_eq!(textos(&b, None), ["[grupo]", "c"]);
        assert_eq!(textos(&b, Some(0)), ["a", "b", "d"]);
        // Un grupo no se puede arrastrar dentro de nada, ni algo sobre sí mismo.
        assert_eq!(b.juntar(0, 1), None);
        assert_eq!(b.juntar(1, 1), None);
    }

    #[test]
    fn guardar_y_etiquetar_dentro_de_un_grupo() {
        let mut b = con(&["a", "b"]);
        b.juntar(1, 0);
        assert!(b.guardar(Some(0), "c"));
        assert!(b.poner_etiqueta(Some(0), 0, "  nueva "));
        assert_eq!(textos(&b, Some(0)), ["c", "a", "b"]);
        assert_eq!(b.texto(Some(0), 0).unwrap().etiqueta.as_deref(), Some("nueva"));
        assert!(b.poner_etiqueta(Some(0), 0, ""));
        assert_eq!(b.texto(Some(0), 0).unwrap().etiqueta, None);
        assert!(b.renombrar_grupo(0, "Correos"));
        assert!(!b.renombrar_grupo(0, "  "));
        assert_eq!(b.grupo(0).unwrap().nombre, "Correos");
    }

    #[test]
    fn sacar_quitar_y_deshacer_grupos() {
        let mut b = con(&["a", "b", "c", "x"]);
        b.juntar(1, 0);
        b.juntar(1, 0); // grupo [c, a, b], luego x
        assert!(b.sacar_del_grupo(0, 1));
        assert_eq!(textos(&b, None), ["[grupo]", "a", "x"]);
        assert!(b.deshacer_grupo(0));
        assert_eq!(textos(&b, None), ["c", "b", "a", "x"]);

        // Un grupo que se queda vacío desaparece.
        let mut b = con(&["a", "b"]);
        b.juntar(1, 0);
        assert!(b.quitar(Some(0), 0));
        assert!(b.sacar_del_grupo(0, 0));
        assert_eq!(textos(&b, None), ["b"]);
        assert!(b.grupo(0).is_none());
    }

    #[test]
    fn fusionar_pone_la_otra_y_detras_lo_que_solo_estaba_en_esta() {
        // Esta: [a, Correos[b, c], d]. Otra: [Correos[c], x].
        let mut esta = con(&["a", "b", "c", "d"]);
        esta.juntar(2, 1);
        esta.renombrar_grupo(1, "Correos");
        let mut otra = con(&["c", "y", "x"]);
        otra.juntar(1, 0);
        otra.renombrar_grupo(0, "Correos");
        otra.quitar(Some(0), 1);
        esta.fusionar(otra);
        assert_eq!(textos(&esta, None), ["[grupo]", "x", "a", "d"]);
        assert_eq!(textos(&esta, Some(0)), ["c", "b"]);
    }

    #[test]
    fn lee_la_boveda_antigua_y_guarda_la_nueva() {
        let antigua: Boveda = serde_json::from_str(r#"["hola", "adiós"]"#).unwrap();
        assert_eq!(textos(&antigua, None), ["hola", "adiós"]);

        let mut b = con(&["a", "b", "c"]);
        b.juntar(2, 1);
        b.poner_etiqueta(None, 0, "uno");
        let json = serde_json::to_string(&b).unwrap();
        let leida: Boveda = serde_json::from_str(&json).unwrap();
        assert_eq!(b, leida);
    }
}
