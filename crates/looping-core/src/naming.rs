//! Physical names, as produced by Looping's name converter (0x140103be0,
//! called through 0x140104340).

use serde::Serialize;

/// Naming convention (`CLoopingDoc+0xa78`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Convention {
    Standard = 0,
    Snake = 1,
    ScreamingSnake = 2,
    Camel = 3,
    Pascal = 4,
    /// "Idem nom conceptuel": unchanged.
    Conceptual = 5,
}

impl Convention {
    pub const ALL: [Convention; 6] = [
        Convention::Standard,
        Convention::Snake,
        Convention::ScreamingSnake,
        Convention::Camel,
        Convention::Pascal,
        Convention::Conceptual,
    ];

    pub fn from_index(i: i32) -> Convention {
        match i {
            1 => Convention::Snake,
            2 => Convention::ScreamingSnake,
            3 => Convention::Camel,
            4 => Convention::Pascal,
            5 => Convention::Conceptual,
            _ => Convention::Standard,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Convention::Standard => "Standard",
            Convention::Snake => "Snake Case",
            Convention::ScreamingSnake => "Screaming Snake Case",
            Convention::Camel => "Camel Case",
            Convention::Pascal => "Pascal Case",
            Convention::Conceptual => "Idem nom conceptuel",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Digit,
    Upper,
    Lower,
    Underscore,
    Dot,
    UpperAccent,
    LowerAccent,
    Other,
}

const UPPER_ACCENTS: &str = "ÆÃÂÄÀÅÁÉÊËÈÏÎÌÍÐÕÔÖÒÓÜÛÙÚŸÝÇÑ";
const LOWER_ACCENTS: &str = "ãæâäàåáéêëèïîìíðõôöòóüûùúÿýçñ";

fn class(c: char) -> Class {
    match c {
        '0'..='9' => Class::Digit,
        'A'..='Z' => Class::Upper,
        'a'..='z' => Class::Lower,
        '_' => Class::Underscore,
        '.' => Class::Dot,
        c if UPPER_ACCENTS.contains(c) => Class::UpperAccent,
        c if LOWER_ACCENTS.contains(c) => Class::LowerAccent,
        _ => Class::Other,
    }
}

fn lower(c: char) -> String {
    c.to_lowercase().collect()
}

fn upper(c: char) -> String {
    c.to_uppercase().collect()
}

/// Removes diacritics from the Latin-1 letters Looping knows about.
pub fn strip_accents(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "A".into(),
            'Æ' => "AE".into(),
            'Ç' => "C".into(),
            'È' | 'É' | 'Ê' | 'Ë' => "E".into(),
            'Ì' | 'Í' | 'Î' | 'Ï' => "I".into(),
            'Ð' => "D".into(),
            'Ñ' => "N".into(),
            'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => "O".into(),
            'Ù' | 'Ú' | 'Û' | 'Ü' => "U".into(),
            'Ý' | 'Ÿ' => "Y".into(),
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a".into(),
            'æ' => "ae".into(),
            'ç' => "c".into(),
            'è' | 'é' | 'ê' | 'ë' => "e".into(),
            'ì' | 'í' | 'î' | 'ï' => "i".into(),
            'ð' => "d".into(),
            'ñ' => "n".into(),
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => "o".into(),
            'ù' | 'ú' | 'û' | 'ü' => "u".into(),
            'ý' | 'ÿ' => "y".into(),
            c => c.to_string(),
        })
        .collect()
}

/// Converts a conceptual name to a physical one.
pub fn convert(name: &str, conv: Convention, remove_accents: bool) -> String {
    let mut out = String::new();
    let mut prev = Class::Other;
    let mut first_word = true;
    for c in name.chars() {
        let k = class(c);
        match conv {
            Convention::Standard => {
                if k == Class::Other {
                    out.push('_');
                } else {
                    out.push(c);
                }
            }
            Convention::Snake | Convention::ScreamingSnake => {
                if matches!(prev, Class::Lower | Class::LowerAccent) && matches!(k, Class::Upper | Class::UpperAccent) {
                    out.push('_');
                }
                if matches!(k, Class::Other | Class::Dot) {
                    out.push('_');
                } else if conv == Convention::Snake {
                    out.push_str(&lower(c));
                } else {
                    out.push_str(&upper(c));
                }
            }
            Convention::Camel | Convention::Pascal => {
                if !matches!(k, Class::Other | Class::Underscore | Class::Dot) {
                    let word_start = matches!(prev, Class::Other | Class::Underscore | Class::Dot);
                    if word_start {
                        if conv == Convention::Camel && first_word {
                            out.push_str(&lower(c));
                        } else {
                            out.push_str(&upper(c));
                        }
                    } else {
                        out.push(c);
                    }
                    first_word = false;
                }
            }
            Convention::Conceptual => out.push(c),
        }
        prev = k;
    }
    if conv != Convention::Conceptual {
        while out.contains("__") {
            out = out.replace("__", "_");
        }
    }
    if remove_accents {
        out = strip_accents(&out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conventions() {
        let n = "Date d'acquisition du chien";
        assert_eq!(convert(n, Convention::Standard, false), "Date_d_acquisition_du_chien");
        assert_eq!(convert(n, Convention::Snake, false), "date_d_acquisition_du_chien");
        assert_eq!(convert(n, Convention::ScreamingSnake, false), "DATE_D_ACQUISITION_DU_CHIEN");
        assert_eq!(convert(n, Convention::Camel, false), "dateDAcquisitionDuChien");
        assert_eq!(convert(n, Convention::Pascal, false), "DateDAcquisitionDuChien");
        assert_eq!(convert(n, Convention::Conceptual, false), n);
        assert_eq!(convert("idUser", Convention::Snake, false), "id_user");
        assert_eq!(convert("Propriétaire", Convention::Standard, true), "Proprietaire");
        assert_eq!(convert("a  b", Convention::Standard, false), "a_b");
    }
}

/// The 301 SQL reserved words Looping knows (from the binary).
const RESERVED: &str = include_str!("reserved.txt");

/// Looping appends `_` to a physical name that is an SQL reserved word
/// (`user` -> `user_`), whatever its case.
pub fn avoid_reserved(name: String) -> String {
    let upper = name.to_uppercase();
    if RESERVED.lines().any(|w| w == upper) {
        name + "_"
    } else {
        name
    }
}
