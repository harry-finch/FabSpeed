//! Essais sur machine : la base de connaissances du lab se construit à partir
//! d'eux. Un essai noté « Bon » devient un réglage *testé* pour son contexte
//! exact (machine, matière, fraise, opération) ; les autres restent dans
//! l'historique et préviennent si l'on s'apprête à refaire la même chose.
//! L'administrateur peut ensuite *valider* un essai réussi : il devient la
//! référence du contexte (preuve `validated`), avant les essais plus récents.

use crate::catalog::Catalog;
use crate::engine::EngineError;
use crate::i18n::Lang;
use crate::model::{Aggressiveness, Evidence, LabPreset, OperationKind, Workholding};
use crate::tr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrialOutcome {
    /// Coupe satisfaisante : le réglage devient la référence du lab.
    Good,
    /// Coupe possible mais à corriger (bruit, surface, chaleur…).
    Adjust,
    /// Échec : casse, pièce arrachée, perte de pas…
    Failed,
}

/// État de la fraise au moment de l'essai : une fraise usée chauffe et
/// arrache plus vite, ce qui change la lecture du résultat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCondition {
    New,
    Used,
}

/// Saisie d'un essai par l'utilisateur (le serveur ajoute identifiant, date
/// et instantané de la fraise).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialInput {
    #[serde(default)]
    pub operator: String,
    pub machine_id: String,
    pub material_id: String,
    pub tool_id: String,
    pub operation: OperationKind,
    pub aggressiveness: Aggressiveness,
    pub workholding: Workholding,
    #[serde(default)]
    pub thickness_mm: Option<f64>,
    #[serde(default)]
    pub depth_mm: Option<f64>,
    pub stepover_ratio: f64,
    #[serde(default)]
    pub stickout_mm: Option<f64>,
    /// État de la fraise (facultatif).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_condition: Option<ToolCondition>,
    pub rpm: f64,
    pub feed_mm_min: f64,
    pub plunge_mm_min: f64,
    pub doc_mm: f64,
    pub outcome: TrialOutcome,
    /// Identifiants des symptômes observés (`data/advisor.toml`).
    #[serde(default)]
    pub symptoms: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

/// Additive downstream eligibility proof; historical engine semantics stay unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationBinding {
    pub version: u32,
    pub catalogue_id: String,
    pub trial_id: String,
    pub machine: crate::model::Identity,
    pub tool: crate::model::Identity,
    pub material: crate::model::Identity,
    pub context_values_revision: String,
}

/// Validation d'un essai réussi par l'administrateur du poste.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Validation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<ValidationBinding>,
    /// Date et heure UTC au format RFC 3339.
    pub validated_at: String,
    /// Personne qui valide (texte libre : il n'y a pas de comptes nominatifs).
    #[serde(default)]
    pub validated_by: String,
    #[serde(default)]
    pub note: String,
}

/// Essai enregistré.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trial {
    pub id: String,
    /// Date et heure UTC au format RFC 3339.
    pub recorded_at: String,
    #[serde(default)]
    pub operator: String,
    pub machine_id: String,
    pub material_id: String,
    pub tool_id: String,
    /// Instantané de la fraise au moment de l'essai (la fiche peut évoluer).
    pub tool_name: String,
    pub tool_diameter_mm: f64,
    #[serde(default)]
    pub tool_flutes: Option<u8>,
    pub operation: OperationKind,
    pub aggressiveness: Aggressiveness,
    pub workholding: Workholding,
    #[serde(default)]
    pub thickness_mm: Option<f64>,
    #[serde(default)]
    pub depth_mm: Option<f64>,
    pub stepover_ratio: f64,
    #[serde(default)]
    pub stickout_mm: Option<f64>,
    /// État de la fraise (facultatif).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_condition: Option<ToolCondition>,
    pub rpm: f64,
    pub feed_mm_min: f64,
    pub plunge_mm_min: f64,
    pub doc_mm: f64,
    pub outcome: TrialOutcome,
    #[serde(default)]
    pub symptoms: Vec<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub engine_version: String,
    #[serde(default)]
    pub dataset_revision: String,
    /// Présente si l'administrateur a validé ce réglage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation: Option<Validation>,
}

/// Détourage et rainure sollicitent la fraise sur toute sa largeur : un essai
/// de l'un vaut pour l'autre. Les autres opérations restent distinctes.
pub fn operation_group(op: OperationKind) -> Vec<OperationKind> {
    match op {
        OperationKind::Profile | OperationKind::FullSlot => {
            vec![OperationKind::Profile, OperationKind::FullSlot]
        }
        other => vec![other],
    }
}

impl Trial {
    /// Construit un essai à partir d'une saisie, après vérification du contexte.
    pub fn from_input(
        input: TrialInput,
        catalog: &Catalog,
        id: String,
        recorded_at: String,
        engine_version: &str,
    ) -> Result<Trial, EngineError> {
        if catalog.machine(&input.machine_id).is_none() {
            return Err(EngineError::UnknownMachine(input.machine_id));
        }
        let material = catalog
            .material(&input.material_id)
            .ok_or_else(|| EngineError::UnknownMaterial(input.material_id.clone()))?;
        let tool = catalog
            .tool(&input.tool_id)
            .ok_or_else(|| EngineError::UnknownTool(input.tool_id.clone()))?;
        let l = catalog.lang;
        for (name, v) in [
            (tr!(l, "Rotation", "Spindle speed"), input.rpm),
            (tr!(l, "Avance", "Feed"), input.feed_mm_min),
            (tr!(l, "Plongée", "Plunge"), input.plunge_mm_min),
            (tr!(l, "Profondeur de passe", "Depth of cut"), input.doc_mm),
        ] {
            if !(v.is_finite() && v > 0.0) {
                return Err(EngineError::InvalidInput(tr!(
                    l,
                    "{name} doit être un nombre > 0",
                    "{name} must be a number > 0"
                )));
            }
        }
        if !(input.stepover_ratio > 0.0 && input.stepover_ratio <= 1.0) {
            return Err(EngineError::InvalidInput(tr!(
                l,
                "Pas latéral : entre 1 et 100 % du diamètre",
                "Stepover: between 1 and 100% of the diameter"
            )));
        }
        if let Some(s) = input.symptoms.iter().find(|s| catalog.symptom(s).is_none()) {
            return Err(EngineError::UnknownSymptom(s.clone()));
        }
        Ok(Trial {
            id,
            recorded_at,
            operator: input.operator.trim().to_string(),
            machine_id: input.machine_id,
            material_id: material.id.clone(),
            tool_id: tool.id.clone(),
            tool_name: tool.name.clone(),
            tool_diameter_mm: tool.diameter_mm,
            tool_flutes: tool.flutes,
            operation: input.operation,
            aggressiveness: input.aggressiveness,
            workholding: input.workholding,
            thickness_mm: input.thickness_mm,
            depth_mm: input.depth_mm,
            stepover_ratio: input.stepover_ratio,
            stickout_mm: input.stickout_mm,
            tool_condition: input.tool_condition,
            rpm: input.rpm,
            feed_mm_min: input.feed_mm_min,
            plunge_mm_min: input.plunge_mm_min,
            doc_mm: input.doc_mm,
            outcome: input.outcome,
            symptoms: input.symptoms,
            notes: input.notes.trim().to_string(),
            engine_version: engine_version.to_string(),
            dataset_revision: catalog.meta.revision.clone(),
            validation: None,
        })
    }

    /// Valide l'essai (seulement un essai « Bon ») ; remplace une validation antérieure.
    pub fn validate(
        &mut self,
        validated_at: String,
        validated_by: &str,
        note: &str,
        lang: Lang,
    ) -> Result<(), EngineError> {
        if self.outcome != TrialOutcome::Good {
            return Err(EngineError::InvalidInput(tr!(
                lang,
                "Seul un essai noté « Bon » peut être validé",
                "Only a test rated “Good” can be validated"
            )));
        }
        self.validation = Some(Validation {
            binding: None,
            validated_at,
            validated_by: validated_by.trim().to_string(),
            note: note.trim().to_string(),
        });
        Ok(())
    }

    pub fn label(&self, lang: Lang) -> String {
        let date = lang.date(&self.recorded_at);
        if self.operator.is_empty() {
            tr!(lang, "Essai du {date}", "Test of {date}")
        } else {
            let op = &self.operator;
            tr!(lang, "Essai du {date} · {op}", "Test of {date} · {op}")
        }
    }

    /// Réglage testé (ou validé) correspondant, seulement pour un essai réussi.
    pub fn as_preset(&self, lang: Lang) -> Option<LabPreset> {
        if self.outcome != TrialOutcome::Good {
            return None;
        }
        let swept = matches!(
            self.operation,
            OperationKind::Pocket | OperationKind::Surfacing
        );
        let (id, rpm, feed, doc) = (
            &self.id,
            lang.num(self.rpm, 0),
            lang.num(self.feed_mm_min, 0),
            lang.num(self.doc_mm, 2),
        );
        let mut notes = Vec::new();
        if !self.notes.is_empty() {
            notes.push(self.notes.clone());
        }
        let label = match &self.validation {
            Some(v) => {
                let (label, date) = (self.label(lang), lang.date(&v.validated_at));
                if !v.note.is_empty() {
                    notes.push(v.note.clone());
                }
                tr!(
                    lang,
                    "{label} · validé le {date}",
                    "{label} · validated on {date}"
                )
            }
            None => self.label(lang),
        };
        Some(LabPreset {
            id: format!("trial:{}", self.id),
            label,
            source: tr!(
                lang,
                "Essai {id} : {rpm} tr/min, {feed} mm/min, passe {doc} mm",
                "Test {id}: {rpm} rpm, {feed} mm/min, depth {doc} mm"
            ),
            evidence: if self.validation.is_some() {
                Evidence::Validated
            } else {
                Evidence::Tested
            },
            machines: vec![self.machine_id.clone()],
            material: self.material_id.clone(),
            tool: self.tool_id.clone(),
            operations: operation_group(self.operation),
            aggressiveness: self.aggressiveness,
            rpm: self.rpm,
            feed_mm_min: self.feed_mm_min,
            doc_mm: self.doc_mm,
            plunge_mm_min: Some(self.plunge_mm_min),
            // En poche, l'essai vaut pour un pas latéral égal ou plus faible.
            stepover_max: swept.then_some(self.stepover_ratio),
            notes,
            // Entre réglages validés, le dernier validé fait référence.
            date: Some(
                self.validation
                    .as_ref()
                    .map_or(&self.recorded_at, |v| &v.validated_at)
                    .clone(),
            ),
        })
    }
}

impl Catalog {
    /// Réglages testés issus des essais réussis dont le contexte existe encore.
    pub fn tested_presets(&self) -> Vec<LabPreset> {
        self.trials
            .iter()
            .filter(|t| {
                self.machine(&t.machine_id).is_some()
                    && self.material(&t.material_id).is_some()
                    && self.tool(&t.tool_id).is_some()
            })
            .filter_map(|t| t.as_preset(self.lang))
            .collect()
    }

    /// Essais du même contexte (machine, matière, fraise, opération), du plus récent au plus ancien.
    pub fn trial_history(
        &self,
        machine_id: &str,
        material_id: &str,
        tool_id: &str,
        operation: OperationKind,
    ) -> Vec<Trial> {
        let ops = operation_group(operation);
        let mut out: Vec<Trial> = self
            .trials
            .iter()
            .filter(|t| {
                t.machine_id == machine_id
                    && self
                        .material(&t.material_id)
                        .is_some_and(|m| m.id == material_id)
                    && t.tool_id == tool_id
                    && ops.contains(&t.operation)
            })
            .cloned()
            .collect();
        out.sort_by(|a, b| {
            b.recorded_at
                .cmp(&a.recorded_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        out
    }
}
