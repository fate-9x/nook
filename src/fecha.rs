//! La fecha de la tira, junto a la hora: en el formato que elige el usuario y en el idioma de la
//! interfaz. Los nombres de los días y los meses van aquí (y no en los .po) porque cada idioma
//! los ordena a su manera.

use crate::config::FormatoFecha;
use chrono::{Datelike, NaiveDate};

/// Nombres de un idioma: los días (de lunes a domingo, enteros y cortos) y los meses (enteros y
/// cortos; en japonés y coreano no hacen falta: van con su número).
struct Nombres {
    dias: [&'static str; 7],
    dias_cortos: [&'static str; 7],
    meses: [&'static str; 12],
    meses_cortos: [&'static str; 12],
}

const ESPANOL: Nombres = Nombres {
    dias: ["lunes", "martes", "miércoles", "jueves", "viernes", "sábado", "domingo"],
    dias_cortos: ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"],
    meses: [
        "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre",
        "diciembre",
    ],
    meses_cortos: ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic"],
};

const INGLES: Nombres = Nombres {
    dias: ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"],
    dias_cortos: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
    meses: [
        "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November",
        "December",
    ],
    meses_cortos: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
};

const JAPONES: Nombres = Nombres {
    dias: ["月曜日", "火曜日", "水曜日", "木曜日", "金曜日", "土曜日", "日曜日"],
    dias_cortos: ["月", "火", "水", "木", "金", "土", "日"],
    meses: [""; 12],
    meses_cortos: [""; 12],
};

const COREANO: Nombres = Nombres {
    dias: ["월요일", "화요일", "수요일", "목요일", "금요일", "토요일", "일요일"],
    dias_cortos: ["월", "화", "수", "목", "금", "토", "일"],
    meses: [""; 12],
    meses_cortos: [""; 12],
};

/// La fecha tal como se ve en la tira: en una línea (arriba y abajo) y en varias, cortas, para
/// los laterales, donde la tira es estrecha. Sin fecha, las dos vacías. `idioma` es el código
/// del idioma de la interfaz ("es", "en", "ja" o "ko").
pub fn formatear(fecha: NaiveDate, formato: FormatoFecha, idioma: &str) -> (String, String) {
    let (d, m, a) = (fecha.day(), fecha.month(), fecha.year());
    let dia = fecha.weekday().num_days_from_monday() as usize;
    let mes = m as usize - 1;
    match idioma {
        "en" => {
            let n = &INGLES;
            let horizontal = match formato {
                FormatoFecha::Ninguna => String::new(),
                FormatoFecha::Numerica => format!("{m}/{d}/{a}"),
                FormatoFecha::ConDia => format!("{}, {m}/{d}/{a}", n.dias[dia]),
                FormatoFecha::Corta => format!("{}, {} {d}", n.dias_cortos[dia], n.meses_cortos[mes]),
                FormatoFecha::Texto => format!("{}, {} {d}, {a}", n.dias[dia], n.meses[mes]),
            };
            let vertical = match formato {
                FormatoFecha::Ninguna => String::new(),
                FormatoFecha::Numerica => format!("{m}/{d}"),
                FormatoFecha::ConDia => format!("{}\n{m}/{d}", n.dias_cortos[dia]),
                FormatoFecha::Corta | FormatoFecha::Texto => format!("{}\n{}\n{d}", n.dias_cortos[dia], n.meses_cortos[mes]),
            };
            (horizontal, vertical)
        }
        "ja" | "ko" => {
            let (n, ano, mes_, dia_) = if idioma == "ja" { (&JAPONES, "年", "月", "日") } else { (&COREANO, "년 ", "월 ", "일") };
            let horizontal = match formato {
                FormatoFecha::Ninguna => String::new(),
                FormatoFecha::Numerica => format!("{a}/{m}/{d}"),
                FormatoFecha::ConDia => format!("{a}/{m}/{d} ({})", n.dias_cortos[dia]),
                FormatoFecha::Corta => format!("{m}{mes_}{d}{dia_} ({})", n.dias_cortos[dia]),
                FormatoFecha::Texto => format!("{a}{ano}{m}{mes_}{d}{dia_} {}", n.dias[dia]),
            };
            let vertical = match formato {
                FormatoFecha::Ninguna => String::new(),
                FormatoFecha::Numerica => format!("{m}/{d}"),
                _ => format!("({})\n{m}/{d}", n.dias_cortos[dia]),
            };
            (horizontal, vertical)
        }
        _ => {
            let n = &ESPANOL;
            let horizontal = match formato {
                FormatoFecha::Ninguna => String::new(),
                FormatoFecha::Numerica => format!("{d}/{m}/{a}"),
                FormatoFecha::ConDia => format!("{}, {d}/{m}/{a}", mayuscula(n.dias[dia])),
                FormatoFecha::Corta => format!("{} {d} {}", mayuscula(n.dias_cortos[dia]), n.meses_cortos[mes]),
                FormatoFecha::Texto => format!("{}, {d} de {} de {a}", mayuscula(n.dias[dia]), n.meses[mes]),
            };
            let vertical = match formato {
                FormatoFecha::Ninguna => String::new(),
                FormatoFecha::Numerica => format!("{d}/{m}"),
                FormatoFecha::ConDia => format!("{}\n{d}/{m}", n.dias_cortos[dia]),
                FormatoFecha::Corta | FormatoFecha::Texto => format!("{}\n{d}\n{}", n.dias_cortos[dia], n.meses_cortos[mes]),
            };
            (horizontal, vertical)
        }
    }
}

/// El texto con la primera letra en mayúscula ("sábado" → "Sábado").
fn mayuscula(texto: &str) -> String {
    let mut letras = texto.chars();
    letras.next().map(|primera| primera.to_uppercase().chain(letras).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sabado() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 17).unwrap()
    }

    #[test]
    fn formatos_en_espanol() {
        let f = |formato| formatear(sabado(), formato, "es").0;
        assert_eq!(f(FormatoFecha::Ninguna), "");
        assert_eq!(f(FormatoFecha::Numerica), "17/10/2026");
        assert_eq!(f(FormatoFecha::ConDia), "Sábado, 17/10/2026");
        assert_eq!(f(FormatoFecha::Corta), "Sáb 17 oct");
        assert_eq!(f(FormatoFecha::Texto), "Sábado, 17 de octubre de 2026");
    }

    #[test]
    fn formatos_en_otros_idiomas() {
        assert_eq!(formatear(sabado(), FormatoFecha::Texto, "en").0, "Saturday, October 17, 2026");
        assert_eq!(formatear(sabado(), FormatoFecha::Numerica, "en").0, "10/17/2026");
        assert_eq!(formatear(sabado(), FormatoFecha::Texto, "ja").0, "2026年10月17日 土曜日");
        assert_eq!(formatear(sabado(), FormatoFecha::Texto, "ko").0, "2026년 10월 17일 토요일");
    }

    #[test]
    fn en_los_laterales_va_en_lineas_cortas() {
        assert_eq!(formatear(sabado(), FormatoFecha::Numerica, "es").1, "17/10");
        assert_eq!(formatear(sabado(), FormatoFecha::ConDia, "es").1, "sáb\n17/10");
        assert_eq!(formatear(sabado(), FormatoFecha::Texto, "es").1, "sáb\n17\noct");
        assert_eq!(formatear(sabado(), FormatoFecha::Ninguna, "es").1, "");
    }
}
