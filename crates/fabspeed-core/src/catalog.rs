//! Catalogue en mémoire : machines, matières, fraises, préréglages, politique
//! et symptômes du diagnostic. Construit par l'appelant (le serveur lit
//! `data/`), puis validé ici sans aucune IO.

use crate::advisor::Symptom;
use crate::i18n::Lang;
use crate::model::{DatasetMeta, LabPreset, MachineProfile, MaterialProfile, ToolSpec};
use crate::policy::EnginePolicy;
use crate::tr;
use crate::trials::Trial;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    pub meta: DatasetMeta,
    pub machines: Vec<MachineProfile>,
    pub materials: Vec<MaterialProfile>,
    pub tools: Vec<ToolSpec>,
    pub presets: Vec<LabPreset>,
    pub policy: EnginePolicy,
    pub symptoms: Vec<Symptom>,
    /// Essais enregistrés au lab (`data/lab/trials/`).
    #[serde(default)]
    pub trials: Vec<Trial>,
    /// Origine des fiches : identifiants écrits par l'application (`data/lab/`)
    /// et fiches de base qu'ils remplacent.
    #[serde(default)]
    pub layers: Layers,
    /// Langue des textes du catalogue et des messages du moteur.
    #[serde(default)]
    pub lang: Lang,
}

/// Fiches modifiables depuis l'application.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Layers {
    /// Machines enregistrées par l'application.
    pub lab_machines: BTreeSet<String>,
    /// Fraises enregistrées par l'application.
    pub lab_tools: BTreeSet<String>,
    /// Matières enregistrées par l'application.
    #[serde(default)]
    pub lab_materials: BTreeSet<String>,
    /// Fiches de base remplacées par une version du lab (même identifiant).
    pub overridden: BTreeSet<String>,
}

/// Problème détecté dans les données. `fatal` empêche le chargement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogIssue {
    pub fatal: bool,
    pub message: String,
}

impl Catalog {
    pub fn machine(&self, id: &str) -> Option<&MachineProfile> {
        self.machines.iter().find(|m| m.id == id)
    }

    /// Recherche par identifiant ou alias (ex. identifiant FabCAM).
    pub fn material(&self, id: &str) -> Option<&MaterialProfile> {
        self.materials.iter().find(|m| m.id == id).or_else(|| {
            self.materials
                .iter()
                .find(|m| m.aliases.iter().any(|a| a == id))
        })
    }

    /// Recherche par identifiant ou identité FabCAM.
    pub fn tool(&self, id: &str) -> Option<&ToolSpec> {
        self.tools.iter().find(|t| t.id == id).or_else(|| {
            self.tools
                .iter()
                .find(|t| t.fabcam_identity.as_ref().is_some_and(|i| i.id == id))
        })
    }

    pub fn symptom(&self, id: &str) -> Option<&Symptom> {
        self.symptoms.iter().find(|s| s.id == id)
    }

    /// Vérifie la cohérence des données. Les erreurs `fatal` signalent des
    /// valeurs qui rendraient un calcul dangereux ou indéterministe.
    /// Messages dans la langue du catalogue.
    pub fn validate(&self) -> Vec<CatalogIssue> {
        let l = self.lang;
        let mut issues = Vec::new();
        let mut fatal = |m: String| {
            issues.push(CatalogIssue {
                fatal: true,
                message: m,
            })
        };

        let kinds: [(String, Vec<&str>); 6] = [
            (
                "machine".into(),
                self.machines.iter().map(|m| m.id.as_str()).collect(),
            ),
            (
                tr!(l, "matière", "material"),
                self.materials.iter().map(|m| m.id.as_str()).collect(),
            ),
            (
                tr!(l, "fraise", "tool"),
                self.tools.iter().map(|t| t.id.as_str()).collect(),
            ),
            (
                tr!(l, "préréglage", "preset"),
                self.presets.iter().map(|p| p.id.as_str()).collect(),
            ),
            (
                tr!(l, "symptôme", "symptom"),
                self.symptoms.iter().map(|s| s.id.as_str()).collect(),
            ),
            (
                tr!(l, "essai", "test"),
                self.trials.iter().map(|t| t.id.as_str()).collect(),
            ),
        ];
        for (kind, ids) in &kinds {
            duplicates(l, kind, ids.iter().copied())
                .into_iter()
                .for_each(&mut fatal);
        }

        for m in &self.machines {
            let id = &m.id;
            if !(m.rpm_min > 0.0 && m.rpm_max >= m.rpm_min) {
                fatal(tr!(
                    l,
                    "Machine {id} : plage de rotation invalide",
                    "Machine {id}: invalid speed range"
                ));
            }
            for (name, v) in [
                ("spindle_power_w", m.spindle_power_w),
                ("max_feed_mm_min", m.max_feed_mm_min),
                ("max_lateral_force_n", m.max_lateral_force_n),
                ("chipload_factor", m.chipload_factor),
            ] {
                if !(v.is_finite() && v > 0.0) {
                    fatal(tr!(
                        l,
                        "Machine {id} : {name} doit être > 0",
                        "Machine {id}: {name} must be > 0"
                    ));
                }
            }
        }
        for m in &self.materials {
            let id = &m.id;
            for (name, v) in [
                ("vc_m_min", m.vc_m_min),
                ("k_fz", m.k_fz),
                ("k_ap", m.k_ap),
                ("kc_n_mm2", m.kc_n_mm2),
                ("fz_min_mm", m.fz_min_mm),
            ] {
                if !(v.is_finite() && v > 0.0) {
                    fatal(tr!(
                        l,
                        "Matière {id} : {name} doit être > 0",
                        "Material {id}: {name} must be > 0"
                    ));
                }
            }
            if !(0.0 < m.default_stepover && m.default_stepover <= 1.0) {
                fatal(tr!(
                    l,
                    "Matière {id} : default_stepover hors de ]0, 1]",
                    "Material {id}: default_stepover outside ]0, 1]"
                ));
            }
        }
        for t in &self.tools {
            let id = &t.id;
            if !(t.diameter_mm.is_finite() && t.diameter_mm > 0.0) {
                fatal(tr!(
                    l,
                    "Fraise {id} : diamètre invalide",
                    "Tool {id}: invalid diameter"
                ));
            }
            if !(t.flute_length_mm.is_finite() && t.flute_length_mm > 0.0) {
                fatal(tr!(
                    l,
                    "Fraise {id} : longueur utile invalide",
                    "Tool {id}: invalid flute length"
                ));
            }
            if t.flutes == Some(0) {
                fatal(tr!(
                    l,
                    "Fraise {id} : utiliser flutes = absent plutôt que 0",
                    "Tool {id}: leave flutes empty rather than 0"
                ));
            }
        }
        let material_known = |id: &str| self.material(id).is_some();
        for p in &self.presets {
            let id = &p.id;
            if self.tool(&p.tool).is_none() {
                let tool = &p.tool;
                fatal(tr!(
                    l,
                    "Préréglage {id} : fraise inconnue {tool}",
                    "Preset {id}: unknown tool {tool}"
                ));
            }
            if !material_known(&p.material) {
                let mat = &p.material;
                fatal(tr!(
                    l,
                    "Préréglage {id} : matière inconnue {mat}",
                    "Preset {id}: unknown material {mat}"
                ));
            }
            for m in &p.machines {
                if self.machine(m).is_none() {
                    fatal(tr!(
                        l,
                        "Préréglage {id} : machine inconnue {m}",
                        "Preset {id}: unknown machine {m}"
                    ));
                }
            }
            if !(p.rpm > 0.0 && p.feed_mm_min > 0.0 && p.doc_mm > 0.0) {
                fatal(tr!(
                    l,
                    "Préréglage {id} : valeurs de coupe invalides",
                    "Preset {id}: invalid cutting values"
                ));
            }
            if p.operations.is_empty() {
                fatal(tr!(
                    l,
                    "Préréglage {id} : aucune opération",
                    "Preset {id}: no operation"
                ));
            }
        }
        if self.policy.plunge_rules.is_empty() {
            fatal(tr!(l, "Aucune règle de plongée", "No plunge rule"));
        }
        for t in &self.trials {
            let id = &t.id;
            let missing = [
                (self.machine(&t.machine_id).is_none(), &t.machine_id),
                (self.material(&t.material_id).is_none(), &t.material_id),
                (self.tool(&t.tool_id).is_none(), &t.tool_id),
            ];
            for (absent, what) in missing {
                if absent {
                    issues.push(CatalogIssue {
                        fatal: false,
                        message: tr!(
                            l,
                            "Essai {id} : {what} absent du catalogue, essai ignoré",
                            "Test {id}: {what} missing from the catalog, test ignored"
                        ),
                    });
                }
            }
        }
        for t in &self.tools {
            let tid = &t.id;
            for id in t.recommended_materials.iter().chain(&t.excluded_materials) {
                if !material_known(id) {
                    issues.push(CatalogIssue {
                        fatal: false,
                        message: tr!(
                            l,
                            "Fraise {tid} : matière référencée inconnue {id}",
                            "Tool {tid}: unknown referenced material {id}"
                        ),
                    });
                }
            }
        }
        issues
    }
}

fn duplicates<'a>(l: Lang, kind: &str, ids: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    ids.filter(|id| !seen.insert(*id))
        .map(|id| {
            tr!(
                l,
                "Identifiant de {kind} en double : {id}",
                "Duplicate {kind} identifier: {id}"
            )
        })
        .collect()
}
