//! Traduction des textes du catalogue (`data/i18n/<langue>.toml`).
//!
//! Les fichiers de données sont écrits en français. Un fichier de traduction
//! remplace, fiche par fiche (par identifiant), les textes destinés à
//! l'utilisateur : noms, descriptions, conseils, symptômes, notes. Tout texte
//! absent de la traduction reste en français ; les valeurs numériques ne sont
//! jamais traduites.

use crate::catalog::Catalog;
use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialText {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tips: Option<Vec<String>>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SymptomText {
    pub label: Option<String>,
    pub cause: Option<String>,
    pub explanation: Option<String>,
    pub advice: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineText {
    pub name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub notes: Option<Vec<String>>,
    /// Notes de fiabilité des limites, par champ.
    #[serde(default)]
    pub sources: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolText {
    pub name: Option<String>,
    pub notes: Option<Vec<String>>,
}

/// Contenu d'un fichier `data/i18n/<langue>.toml`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Translations {
    #[serde(default)]
    pub materials: BTreeMap<String, MaterialText>,
    #[serde(default)]
    pub symptoms: BTreeMap<String, SymptomText>,
    #[serde(default)]
    pub machines: BTreeMap<String, MachineText>,
    #[serde(default)]
    pub tools: BTreeMap<String, ToolText>,
}

fn set<T: Clone>(target: &mut T, value: &Option<T>) {
    if let Some(v) = value {
        *target = v.clone();
    }
}

impl Catalog {
    /// Copie du catalogue dans `lang`, avec les traductions disponibles.
    pub fn localized(&self, lang: Lang, tr: Option<&Translations>) -> Catalog {
        let mut c = self.clone();
        c.lang = lang;
        let Some(tr) = tr else { return c };
        for m in &mut c.materials {
            if let Some(t) = tr.materials.get(&m.id) {
                set(&mut m.name, &t.name);
                set(&mut m.description, &t.description);
                set(&mut m.tips, &t.tips);
                set(&mut m.source, &t.source);
            }
        }
        for s in &mut c.symptoms {
            if let Some(t) = tr.symptoms.get(&s.id) {
                set(&mut s.label, &t.label);
                set(&mut s.cause, &t.cause);
                set(&mut s.explanation, &t.explanation);
                set(&mut s.advice, &t.advice);
            }
        }
        for m in &mut c.machines {
            if let Some(t) = tr.machines.get(&m.id) {
                set(&mut m.name, &t.name);
                set(&mut m.category, &t.category);
                set(&mut m.description, &t.description);
                set(&mut m.notes, &t.notes);
                for (field, note) in &t.sources {
                    if let Some(src) = m.sources.get_mut(field) {
                        src.note = note.clone();
                    }
                }
            }
        }
        for tool in &mut c.tools {
            if let Some(t) = tr.tools.get(&tool.id) {
                set(&mut tool.name, &t.name);
                set(&mut tool.notes, &t.notes);
            }
        }
        c
    }

    /// Identifiants traduits qui n'existent pas dans le catalogue (fautes de frappe).
    pub fn untranslatable_ids(&self, tr: &Translations) -> Vec<String> {
        let mut out = Vec::new();
        out.extend(
            tr.materials
                .keys()
                .filter(|id| self.materials.iter().all(|m| &m.id != *id))
                .cloned(),
        );
        out.extend(
            tr.symptoms
                .keys()
                .filter(|id| self.symptoms.iter().all(|s| &s.id != *id))
                .cloned(),
        );
        out.extend(
            tr.machines
                .keys()
                .filter(|id| self.machines.iter().all(|m| &m.id != *id))
                .cloned(),
        );
        out.extend(
            tr.tools
                .keys()
                .filter(|id| self.tools.iter().all(|t| &t.id != *id))
                .cloned(),
        );
        out
    }
}
