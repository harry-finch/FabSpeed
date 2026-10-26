//! Langues des messages produits par le moteur.
//!
//! Le moteur renvoie des **codes stables** (avertissements, leviers, étapes de
//! calcul) accompagnés d'un texte dans la langue du catalogue (`Catalog::lang`).
//! Les nombres suivent la convention de la langue : `16 850,5` en français,
//! `16,850.5` en anglais. Le français reste la langue de référence : un texte
//! de données sans traduction s'affiche en français.

use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    #[default]
    Fr,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Fr, Lang::En];

    /// Étiquette de langue (`en`, `en-US`, `fr-CA`…) ; français par défaut.
    pub fn from_tag(tag: &str) -> Lang {
        let primary = tag.trim().split(['-', '_', ';', ',']).next().unwrap_or("");
        if primary.eq_ignore_ascii_case("en") {
            Lang::En
        } else {
            Lang::Fr
        }
    }

    /// Première langue reconnue d'un en-tête `Accept-Language`.
    pub fn from_accept_language(header: &str) -> Lang {
        header
            .split(',')
            .map(|part| part.split(';').next().unwrap_or("").trim())
            .find_map(|tag| {
                let p = tag.split(['-', '_']).next().unwrap_or("");
                if p.eq_ignore_ascii_case("en") {
                    Some(Lang::En)
                } else if p.eq_ignore_ascii_case("fr") {
                    Some(Lang::Fr)
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::En => "en",
        }
    }

    /// Nombre avec `decimals` décimales, séparateurs de la langue.
    pub fn num(self, v: f64, decimals: usize) -> String {
        let (thousands, point) = match self {
            Lang::Fr => ('\u{202f}', ','),
            Lang::En => (',', '.'),
        };
        let s = format!("{:.decimals$}", v.abs());
        let (int, frac) = s
            .split_once('.')
            .map_or((s.as_str(), None), |(i, f)| (i, Some(f)));
        let mut grouped = String::new();
        for (i, c) in int.chars().enumerate() {
            if i > 0 && (int.len() - i) % 3 == 0 {
                grouped.push(thousands);
            }
            grouped.push(c);
        }
        let sign = if v < 0.0 && s.chars().any(|c| c.is_ascii_digit() && c != '0') {
            "-"
        } else {
            ""
        };
        match frac {
            Some(f) => format!("{sign}{grouped}{point}{f}"),
            None => format!("{sign}{grouped}"),
        }
    }

    /// Comme [`Lang::num`], sans zéros décimaux inutiles : 9 plutôt que 9,00.
    pub fn num_trim(self, v: f64, decimals: usize) -> String {
        let s = self.num(v, decimals);
        let point = if self == Lang::Fr { ',' } else { '.' };
        if s.contains(point) {
            s.trim_end_matches('0').trim_end_matches(point).to_string()
        } else {
            s
        }
    }

    /// Date d'un horodatage RFC 3339 : `22/10/2026` ou `2026-10-22`.
    pub fn date(self, rfc3339: &str) -> String {
        let d = rfc3339.get(..10).unwrap_or(rfc3339);
        match (self, d.get(0..4), d.get(5..7), d.get(8..10)) {
            (Lang::Fr, Some(y), Some(m), Some(day)) => format!("{day}/{m}/{y}"),
            _ => d.to_string(),
        }
    }

    /// « 1 passe » / « 3 passes », « 1 pass » / « 3 passes ».
    pub fn passes(self, n: u32) -> String {
        match (self, n > 1) {
            (Lang::Fr, false) => format!("{n} passe"),
            (Lang::Fr, true) => format!("{n} passes"),
            (Lang::En, false) => format!("{n} pass"),
            (Lang::En, true) => format!("{n} passes"),
        }
    }
}

/// Texte selon la langue : `tr!(lang, "français {x}", "English {x}", args…)`.
/// Les deux gabarits reçoivent les mêmes arguments ; les variables locales
/// peuvent aussi être capturées par leur nom.
#[macro_export]
macro_rules! tr {
    ($lang:expr, $fr:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        match $lang {
            $crate::i18n::Lang::Fr => format!($fr $(, $arg)*),
            $crate::i18n::Lang::En => format!($en $(, $arg)*),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_follow_the_language() {
        assert_eq!(Lang::Fr.num(16850.0, 0), "16\u{202f}850");
        assert_eq!(Lang::En.num(16850.0, 0), "16,850");
        assert_eq!(Lang::Fr.num(0.356, 3), "0,356");
        assert_eq!(Lang::En.num(123456.5, 1), "123,456.5");
        assert_eq!(Lang::Fr.num_trim(9.0, 2), "9");
        assert_eq!(Lang::En.num_trim(4.5, 2), "4.5");
    }

    #[test]
    fn tags_and_headers() {
        assert_eq!(Lang::from_tag("en-US"), Lang::En);
        assert_eq!(Lang::from_tag("de"), Lang::Fr);
        assert_eq!(
            Lang::from_accept_language("de-DE,en;q=0.8,fr;q=0.5"),
            Lang::En
        );
        assert_eq!(
            Lang::from_accept_language("fr-FR,fr;q=0.9,en;q=0.8"),
            Lang::Fr
        );
        assert_eq!(Lang::from_accept_language(""), Lang::Fr);
    }

    #[test]
    fn dates_and_plurals() {
        assert_eq!(Lang::Fr.date("2026-10-22T09:30:00Z"), "22/10/2026");
        assert_eq!(Lang::En.date("2026-10-22T09:30:00Z"), "2026-10-22");
        assert_eq!(Lang::Fr.passes(1), "1 passe");
        assert_eq!(Lang::En.passes(4), "4 passes");
        let x = 3;
        assert_eq!(tr!(Lang::En, "{x} fraises", "{x} tools"), "3 tools");
        assert_eq!(tr!(Lang::Fr, "{} fraises", "{} tools", 2), "2 fraises");
    }
}
