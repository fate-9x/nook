//! Apps abiertas (solo en nook Pro): en la versión gratuita no se ve ninguna.

use std::collections::HashMap;
use windows::Win32::UI::Shell::IShellItem2;

/// Ventana que sale en la barra de tareas.
#[derive(Clone, Debug, PartialEq)]
pub struct Ventana {
    pub hwnd: isize,
    pub claves: Vec<String>,
    pub exe: String,
    pub id: Option<String>,
}

pub type Vistas = HashMap<isize, (Vec<String>, String, Option<String>)>;

pub fn ventanas(_vistas: &mut Vistas) -> Vec<Ventana> {
    Vec::new()
}

pub fn claves_de_app(_ruta: &str) -> Vec<String> {
    Vec::new()
}

pub fn claves_de_item(_item: &IShellItem2, _extra: Option<&str>) -> Vec<String> {
    Vec::new()
}

pub fn misma_app(_a: &[String], _b: &[String]) -> bool {
    false
}

pub fn titulo(_hwnd: isize) -> String {
    String::new()
}

pub fn alternar(_ventanas: &[isize], _previa: Option<isize>) {}

pub fn cerrar(_hwnd: isize) {}
