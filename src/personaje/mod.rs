//! Personaje de Nook: una figura que cambia de forma, con un rulo en la cabeza, una sonrisa con
//! colmillo y dos ojos que miran al cursor, y los accesorios que se le pongan.
//!
//! El dibujo usa la técnica de bloub (https://github.com/jeremy-prt/bloub, de Jérémy Perret,
//! licencia MIT; ver AVISOS_DE_TERCEROS.md): la silueta como perfil radial que se interpola para
//! cambiar de forma, los ojos pintados sobre una esfera, la vida en reposo (deriva de la mirada y
//! parpadeos) como función pura del tiempo y las transiciones que frenan sin pasarse. El
//! personaje (forma, ojos, colores) es propio.

mod accesorios;
mod cara;
mod dibujo;
mod estados;
mod forma;
mod motor;

pub use accesorios::{Accesorio, Tipo as TipoAccesorio};
pub use dibujo::{Aspecto, RADIO, dibujar};
pub use estados::Estado;
pub use motor::{Cuadro, Figura, Mirar, Motor};
