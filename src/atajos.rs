//! Atajos de teclado globales: funcionan aunque la barra esté oculta o sin foco.
//!
//! Se registran en Windows con `RegisterHotKey`. Windows avisa con un mensaje `WM_HOTKEY`
//! al hilo principal, que recibe el bucle de eventos (ver `plataforma.rs`).

use crate::idioma::tr;
use serde::{Deserialize, Serialize};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
    RegisterHotKey, UnregisterHotKey, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};

/// Una combinación de teclas, por ejemplo Alt + Shift + 1.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Atajo {
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub win: bool,
    /// "0"–"9", "A"–"Z", "F1"–"F12" o "Espacio".
    pub tecla: String,
}

impl Atajo {
    pub fn alt_shift(tecla: &str) -> Self {
        Self { ctrl: false, alt: true, shift: true, win: false, tecla: tecla.into() }
    }

    /// Texto para mostrar, por ejemplo "Alt + Shift + 1" (el nombre de la barra espaciadora, en
    /// el idioma de la interfaz).
    pub fn texto(&self) -> String {
        let mut partes = Vec::new();
        if self.ctrl {
            partes.push("Ctrl");
        }
        if self.alt {
            partes.push("Alt");
        }
        if self.shift {
            partes.push("Shift");
        }
        if self.win {
            partes.push("Win");
        }
        partes.push(if self.tecla == "Espacio" { tr("Espacio") } else { &self.tecla });
        partes.join(" + ")
    }

    /// Código de tecla virtual de Windows.
    pub fn codigo(&self) -> Option<u32> {
        codigo_de_tecla(&self.tecla)
    }

    fn modificadores(&self) -> HOT_KEY_MODIFIERS {
        let mut m = MOD_NOREPEAT;
        if self.ctrl {
            m |= MOD_CONTROL;
        }
        if self.alt {
            m |= MOD_ALT;
        }
        if self.shift {
            m |= MOD_SHIFT;
        }
        if self.win {
            m |= MOD_WIN;
        }
        m
    }
}

fn codigo_de_tecla(tecla: &str) -> Option<u32> {
    if tecla == "Espacio" {
        return Some(0x20);
    }
    if let Some(n) = tecla.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        return (1..=12).contains(&n).then_some(0x70 + n - 1);
    }
    let mut letras = tecla.chars();
    match (letras.next(), letras.next()) {
        (Some(c), None) if c.is_ascii_digit() || c.is_ascii_uppercase() => Some(c as u32),
        _ => None,
    }
}

fn nombre_de_codigo(codigo: u32) -> Option<String> {
    match codigo {
        0x20 => Some("Espacio".into()),
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(codigo).map(String::from),
        0x70..=0x7B => Some(format!("F{}", codigo - 0x70 + 1)),
        _ => None,
    }
}

/// Registra el atajo. Devuelve `false` si Windows lo rechaza (normalmente porque otro programa ya lo usa).
pub fn registrar(id: i32, atajo: &Atajo) -> bool {
    let Some(codigo) = atajo.codigo() else { return false };
    unsafe { RegisterHotKey(None, id, atajo.modificadores(), codigo).is_ok() }
}

pub fn quitar(id: i32) {
    unsafe {
        let _ = UnregisterHotKey(None, id);
    }
}

/// Lee qué combinación se está pulsando físicamente en este momento. Se usa el estado real
/// del teclado y no el texto que produce, porque en teclados como el español Ctrl+Alt
/// equivale a AltGr y cambia el carácter (Ctrl+Alt+2 escribe "@").
/// Devuelve `None` si solo hay modificadores pulsados o no hay ninguno.
pub fn capturar() -> Option<Atajo> {
    let pulsada = |vk: u16| unsafe { GetAsyncKeyState(vk as i32) } < 0;
    let codigo = (0x20..=0x7Bu32).find(|&c| nombre_de_codigo(c).is_some() && pulsada(c as u16))?;
    let atajo = Atajo {
        ctrl: pulsada(VK_CONTROL.0),
        alt: pulsada(VK_MENU.0),
        shift: pulsada(VK_SHIFT.0),
        win: pulsada(VK_LWIN.0) || pulsada(VK_RWIN.0),
        tecla: nombre_de_codigo(codigo)?,
    };
    // Sin Ctrl, Alt ni Win el atajo bloquearía la escritura normal en todos los programas.
    (atajo.ctrl || atajo.alt || atajo.win).then_some(atajo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texto_legible() {
        assert_eq!(Atajo::alt_shift("1").texto(), "Alt + Shift + 1");
    }

    #[test]
    fn codigos_de_tecla() {
        assert_eq!(codigo_de_tecla("1"), Some(0x31));
        assert_eq!(codigo_de_tecla("K"), Some(0x4B));
        assert_eq!(codigo_de_tecla("F12"), Some(0x7B));
        assert_eq!(codigo_de_tecla("Espacio"), Some(0x20));
        assert_eq!(codigo_de_tecla("F13"), None);
        assert_eq!(codigo_de_tecla("ñ"), None);
    }

    #[test]
    fn nombre_y_codigo_son_inversos() {
        for codigo in [0x20, 0x30, 0x39, 0x41, 0x5A, 0x70, 0x7B] {
            let nombre = nombre_de_codigo(codigo).unwrap();
            assert_eq!(codigo_de_tecla(&nombre), Some(codigo));
        }
    }
}
