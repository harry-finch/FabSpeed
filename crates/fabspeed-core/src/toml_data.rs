//! Lecture des fichiers de données TOML (texte → catalogue), sans IO.
//! Le serveur lit les fichiers du dossier `data/` et les passe ici.

use crate::advisor::Symptom;
use crate::catalog::Catalog;
use crate::model::{DatasetMeta, LabPreset, MachineProfile, MaterialProfile, ToolSpec};
use crate::policy::EnginePolicy;
use crate::trials::Trial;
use serde::{Deserialize, Serialize};

/// Nature d'un fichier de données.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataKind {
    Dataset,
    Machines,
    Materials,
    Tools,
    Presets,
    Policy,
    Advisor,
    Trials,
}

#[derive(Debug, thiserror::Error)]
#[error("{file} : {message}")]
pub struct DataError {
    pub file: String,
    pub message: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Machines {
    #[serde(default)]
    machines: Vec<MachineProfile>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Materials {
    #[serde(default)]
    materials: Vec<MaterialProfile>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Tools {
    #[serde(default)]
    tools: Vec<ToolSpec>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Trials {
    #[serde(default)]
    trials: Vec<Trial>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Presets {
    #[serde(default)]
    presets: Vec<LabPreset>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Advisor {
    #[serde(default)]
    symptoms: Vec<Symptom>,
}

fn parse<T: for<'de> Deserialize<'de>>(file: &str, text: &str) -> Result<T, DataError> {
    toml::from_str(text).map_err(|e| DataError {
        file: file.to_string(),
        message: e.to_string(),
    })
}

impl Catalog {
    /// Ajoute le contenu d'un fichier TOML au catalogue.
    pub fn merge_toml(&mut self, kind: DataKind, file: &str, text: &str) -> Result<(), DataError> {
        match kind {
            DataKind::Dataset => self.meta = parse::<DatasetMeta>(file, text)?,
            DataKind::Machines => self
                .machines
                .extend(parse::<Machines>(file, text)?.machines),
            DataKind::Materials => self
                .materials
                .extend(parse::<Materials>(file, text)?.materials),
            DataKind::Tools => self.tools.extend(parse::<Tools>(file, text)?.tools),
            DataKind::Presets => self.presets.extend(parse::<Presets>(file, text)?.presets),
            DataKind::Policy => self.policy = parse::<EnginePolicy>(file, text)?,
            DataKind::Trials => self.trials.extend(parse::<Trials>(file, text)?.trials),
            DataKind::Advisor => self.symptoms.extend(parse::<Advisor>(file, text)?.symptoms),
        }
        Ok(())
    }
}

/// Lit un fichier de traduction `data/i18n/<langue>.toml`.
pub fn parse_translations(
    file: &str,
    text: &str,
) -> Result<crate::localize::Translations, DataError> {
    parse(file, text)
}

/// Politique par défaut au format TOML (point de départ de `data/policy.toml`).
pub fn default_policy_toml() -> String {
    toml::to_string_pretty(&EnginePolicy::default()).expect("politique sérialisable")
}

const GENERATED: &str = "# Fiche enregistrée par FabSpeed. Modifiable à la main ; recharger ensuite (POST /api/v1/reload).\n";

fn document<T: Serialize>(value: &T) -> String {
    format!(
        "{GENERATED}\n{}",
        toml::to_string_pretty(value).expect("fiche sérialisable")
    )
}

/// Fichier TOML d'une fraise enregistrée par l'application.
pub fn tool_document(tool: &ToolSpec) -> String {
    document(&Tools {
        tools: vec![tool.clone()],
    })
}

/// Fichier TOML d'une machine enregistrée par l'application.
pub fn machine_document(machine: &MachineProfile) -> String {
    document(&Machines {
        machines: vec![machine.clone()],
    })
}

/// Fichier TOML d'une matière enregistrée par l'application.
pub fn material_document(material: &MaterialProfile) -> String {
    document(&Materials {
        materials: vec![material.clone()],
    })
}

/// Fichier TOML d'un essai.
pub fn trial_document(trial: &Trial) -> String {
    document(&Trials {
        trials: vec![trial.clone()],
    })
}
