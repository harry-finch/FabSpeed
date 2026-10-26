//! Assistant de diagnostic : « Que constatez-vous ? » → corrections proposées.
//!
//! Les symptômes et leurs facteurs vivent dans `data/advisor.toml`. Le
//! diagnostic part des valeurs actuelles (recommandées ou imposées), applique
//! les facteurs, puis réévalue le résultat avec le moteur : la correction
//! proposée est vérifiée comme n'importe quel réglage.

use crate::catalog::Catalog;
use crate::engine::{recommend, EngineError};
use crate::model::MaterialCategory;
use crate::recommendation::{CuttingValues, Recommendation};
use crate::request::{Overrides, RecommendationRequest, RequestPatch};
use serde::{Deserialize, Serialize};

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Symptom {
    pub id: String,
    /// Ce que l'utilisateur voit / entend, en langage courant.
    pub label: String,
    /// Nom d'icône Lucide pour l'interface.
    #[serde(default)]
    pub icon: String,
    /// Cause physique probable.
    pub cause: String,
    pub explanation: String,
    #[serde(default = "one")]
    pub rpm_factor: f64,
    #[serde(default = "one")]
    pub feed_factor: f64,
    #[serde(default = "one")]
    pub doc_factor: f64,
    #[serde(default)]
    pub advice: Vec<String>,
    /// Catégories de matière concernées (vide = toutes).
    #[serde(default)]
    pub categories: Vec<MaterialCategory>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnosis {
    pub symptom: Symptom,
    pub before: CuttingValues,
    /// Recommandation réévaluée avec les corrections appliquées.
    pub after: Recommendation,
    /// Patch à appliquer à la requête pour adopter la correction.
    pub patch: RequestPatch,
}

pub fn diagnose(
    catalog: &Catalog,
    req: &RecommendationRequest,
    symptom_id: &str,
) -> Result<Diagnosis, EngineError> {
    let symptom = catalog
        .symptom(symptom_id)
        .ok_or_else(|| EngineError::UnknownSymptom(symptom_id.to_string()))?
        .clone();
    let current = recommend(catalog, req)?;
    let before = current.values.ok_or_else(|| {
        EngineError::InvalidInput(crate::tr!(
            catalog.lang,
            "Pas de valeurs à corriger pour cette fraise",
            "No values to correct for this tool"
        ))
    })?;
    let machine = catalog
        .machine(&req.machine_id)
        .ok_or_else(|| EngineError::UnknownMachine(req.machine_id.clone()))?;
    let rnd = &catalog.policy.rounding;
    let floor = |v: f64, step: f64| ((v / step) + 1e-9).floor() * step;

    let rpm = (before.rpm * symptom.rpm_factor / rnd.rpm_step).round() * rnd.rpm_step;
    let rpm = rpm.clamp(machine.rpm_min, machine.rpm_max);
    let feed = floor(before.feed_mm_min * symptom.feed_factor, rnd.feed_step)
        .clamp(rnd.feed_step, machine.max_feed_mm_min);
    let doc =
        floor(before.doc_max_mm * symptom.doc_factor, rnd.doc_step).max(catalog.policy.doc_min_mm);
    // Les trois valeurs sont figées pour que la correction reste explicite.
    let patch = RequestPatch {
        overrides: Some(Overrides {
            rpm: Some(rpm),
            feed_mm_min: Some(feed),
            doc_mm: Some(doc),
            plunge_mm_min: None,
        }),
        ..Default::default()
    };
    let after = recommend(catalog, &req.apply_patch(&patch))?;
    Ok(Diagnosis {
        symptom,
        before,
        after,
        patch,
    })
}
