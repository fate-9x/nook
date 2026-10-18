//! Personaje de Nook: una figura (Nook, el Limón o una hecha por el usuario, ver `figura.rs`) con
//! ojos que miran al cursor, que se mueve, salta y baila, y los accesorios que se le pongan.
//!
//! El dibujo usa la técnica de bloub (https://github.com/jeremy-prt/bloub, de Jérémy Perret,
//! licencia MIT; ver AVISOS_DE_TERCEROS.md): los ojos pintados sobre una esfera, la vida en reposo
//! (deriva de la mirada y parpadeos) como función pura del tiempo y las transiciones que frenan
//! sin pasarse. Las figuras (forma, ojos, colores) son propias.

mod accesorios;
mod cara;
pub mod coleccion;
mod dibujo;
mod estados;
pub mod figura;
mod forma;
mod motor;
mod svg;

pub use accesorios::{Accesorio, Tipo as TipoAccesorio};
pub use dibujo::{Aspecto, RADIO, dibujar};
pub use estados::Estado;
pub use figura::Figura;
pub use motor::{Cuadro, Mirar, Motor};
