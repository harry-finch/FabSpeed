//! Requête de recommandation (ce que l'interface ou FabCAM envoie).

use crate::model::{Aggressiveness, OperationKind, ToolSpec, Workholding};
use serde::{Deserialize, Serialize};

/// Valeurs imposées par l'utilisateur. Le moteur ne les modifie jamais : il
/// les évalue (charges, copeau) et signale les risques.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Overrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rpm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed_mm_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plunge_mm_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_mm: Option<f64>,
}

impl Overrides {
    pub fn is_empty(&self) -> bool {
        self.rpm.is_none()
            && self.feed_mm_min.is_none()
            && self.plunge_mm_min.is_none()
            && self.doc_mm.is_none()
    }

    /// Fusionne `other` par-dessus `self` (les champs présents gagnent).
    pub fn merged(&self, other: &Overrides) -> Overrides {
        Overrides {
            rpm: other.rpm.or(self.rpm),
            feed_mm_min: other.feed_mm_min.or(self.feed_mm_min),
            plunge_mm_min: other.plunge_mm_min.or(self.plunge_mm_min),
            doc_mm: other.doc_mm.or(self.doc_mm),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecommendationRequest {
    pub machine_id: String,
    /// Identifiant ou alias de matière.
    pub material_id: String,
    /// Fraise du catalogue (identifiant ou identité FabCAM)…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_id: Option<String>,
    /// …ou fraise décrite directement (fraise personnelle, outil FabCAM).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolSpec>,
    pub operation: OperationKind,
    /// Pas latéral (fraction de D) pour les poches et le surfaçage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stepover_ratio: Option<f64>,
    /// Profondeur totale à usiner (mm) ; pour un détourage, l'épaisseur de la plaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_mm: Option<f64>,
    #[serde(default)]
    pub aggressiveness: Aggressiveness,
    #[serde(default)]
    pub workholding: Workholding,
    /// Longueur de fraise sortie de la pince (mm). Défaut : longueur utile + marge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stickout_mm: Option<f64>,
    /// Plafond de profondeur de passe imposé par l'utilisateur ou la FAO.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_doc_mm: Option<f64>,
    /// Longueur de parcours par passe (mm), pour estimer la durée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut_length_mm: Option<f64>,
    /// Utiliser les préréglages du lab lorsqu'ils correspondent exactement.
    #[serde(default = "default_true")]
    pub use_presets: bool,
    #[serde(default, skip_serializing_if = "Overrides::is_empty")]
    pub overrides: Overrides,
    /// Proposer aussi les leviers d'accélération pour une matière sous
    /// supervision (métaux). Décision du fabmanager : le serveur ne l'honore
    /// qu'avec une session administrateur.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub supervised_speedups: bool,
}

/// Modification de requête proposée par un levier ou un diagnostic. L'interface
/// l'applique telle quelle ; rien n'est appliqué sans action de l'utilisateur.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RequestPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggressiveness: Option<Aggressiveness>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workholding: Option<Workholding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stickout_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overrides: Option<Overrides>,
    /// Effacer les valeurs imposées avant d'appliquer ce patch.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clear_overrides: bool,
}

impl RecommendationRequest {
    pub fn apply_patch(&self, patch: &RequestPatch) -> RecommendationRequest {
        let mut next = self.clone();
        if patch.clear_overrides {
            next.overrides = Overrides::default();
        }
        if let Some(a) = patch.aggressiveness {
            next.aggressiveness = a;
        }
        if let Some(w) = patch.workholding {
            next.workholding = w;
        }
        if let Some(s) = patch.stickout_mm {
            next.stickout_mm = Some(s);
        }
        if let Some(o) = &patch.overrides {
            next.overrides = next.overrides.merged(o);
        }
        next
    }
}
