//! Résultat d'une recommandation : valeurs à reporter en FAO, métriques
//! physiques pour les jauges, avertissements, leviers et provenance.

use crate::model::{Aggressiveness, Evidence, OperationKind, Workholding};
use crate::request::RequestPatch;
use crate::trials::Trial;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationStatus {
    Ok,
    /// Le moteur ne sait pas calculer cette combinaison (ex. fraise à graver).
    Unsupported,
}

/// D'où vient chaque valeur affichée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Reprise telle quelle d'un réglage testé (essai réussi enregistré au lab).
    LabPreset,
    /// Adaptée à partir d'un réglage testé (ex. profondeur réduite en mode prudent).
    DerivedFromPreset,
    /// Calculée par le modèle.
    Calculated,
    /// Imposée par l'utilisateur.
    UserOverride,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueOrigins {
    pub rpm: Origin,
    pub feed: Origin,
    pub plunge: Origin,
    pub doc: Origin,
}

/// Valeurs directement utilisables dans la FAO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CuttingValues {
    pub rpm: f64,
    pub feed_mm_min: f64,
    pub plunge_mm_min: f64,
    /// Profondeur de passe à programmer (égalisée sur la profondeur totale).
    pub doc_mm: f64,
    /// Profondeur de passe maximale admise avant égalisation.
    pub doc_max_mm: f64,
    /// Nombre de passes pour la profondeur totale, si elle est connue.
    pub passes: Option<u32>,
    pub depth_mm: Option<f64>,
    /// Engagement radial (mm) et sa fraction du diamètre.
    pub stepover_mm: f64,
    pub stepover_ratio: f64,
    /// Entrée en rampe conseillée plutôt qu'une plongée verticale.
    pub ramp_entry_advised: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChipState {
    /// Copeau trop fin : la fraise frotte, chauffe, brûle.
    Rubbing,
    Ok,
    /// Copeau épais : efforts élevés, risque de casse ou d'arrachement.
    Thick,
}

/// Zone de copeau sain pour la combinaison matière / diamètre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChipZone {
    pub min_mm: f64,
    pub nominal_mm: f64,
    pub max_mm: f64,
    pub state: ChipState,
}

/// Charges en % des limites physiques **estimées** (100 % = limite).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loads {
    pub spindle_pct: f64,
    pub machine_pct: f64,
    pub tool_pct: f64,
    pub holding_pct: f64,
}

impl Loads {
    pub fn max(&self) -> f64 {
        self.spindle_pct
            .max(self.machine_pct)
            .max(self.tool_pct)
            .max(self.holding_pct)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metrics {
    /// Avance par dent programmée (fz).
    pub chipload_mm: f64,
    /// Épaisseur de copeau réelle après amincissement radial (hex).
    pub chip_thickness_mm: f64,
    pub chip_zone: ChipZone,
    pub rct_factor: f64,
    /// Vitesse de coupe au diamètre qui coupe réellement.
    pub cutting_speed_m_min: f64,
    /// Diamètre qui coupe réellement (= diamètre de la fraise, sauf
    /// hémisphérique peu engagée et fraise en V).
    #[serde(default)]
    pub cutting_diameter_mm: f64,
    pub mrr_cm3_min: f64,
    pub cutting_power_w: f64,
    pub available_power_w: f64,
    pub cutting_force_n: f64,
    pub loads: Loads,
    pub stickout_mm: f64,
    pub slenderness: f64,
    /// Durée d'usinage estimée (hors déplacements rapides), si la longueur est connue.
    pub estimated_time_s: Option<f64>,
    /// Temps de coupe pour un mètre de trajectoire, toutes passes comprises
    /// (hors plongées et déplacements rapides).
    #[serde(default)]
    pub time_per_metre_s: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitingFactor {
    SpindleMaxRpm,
    SpindleMinRpm,
    ToolMaxRpm,
    MachineFeed,
    MachinePlunge,
    SpindlePower,
    MachineRigidity,
    ToolStrength,
    Workholding,
    FluteLength,
    UserDocCap,
    Slenderness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Blocking,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Warning {
    pub code: String,
    pub severity: Severity,
    pub message: String,
}

/// Piste pour aller plus vite, classée de la plus sûre à la plus agressive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lever {
    pub id: String,
    pub title: String,
    pub detail: String,
    /// Gain de temps estimé (0.25 = 25 % de temps en moins).
    pub time_gain: Option<f64>,
    pub caution: Option<String>,
    pub patch: RequestPatch,
}

/// Étape du calcul, pour l'onglet « Comment c'est calculé ».
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceStep {
    /// Identifiant stable de l'étape (`spindle_speed`, `feed`, `plunge`…).
    pub key: String,
    pub label: String,
    pub formula: String,
    pub result: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Model,
    LabPreset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub source: SourceKind,
    pub preset_id: Option<String>,
    pub preset_label: Option<String>,
    pub preset_source: Option<String>,
    pub evidence: Evidence,
    pub engine_version: String,
    pub dataset_revision: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedContext {
    pub machine_id: String,
    pub machine_name: String,
    pub material_id: String,
    pub material_name: String,
    pub tool_id: String,
    pub tool_name: String,
    pub tool_diameter_mm: f64,
    pub tool_flutes: Option<u8>,
    pub operation: OperationKind,
    pub aggressiveness: Aggressiveness,
    pub workholding: Workholding,
}

/// Valeurs du modèle seul, affichées pour comparaison quand un préréglage est utilisé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TheoreticalSummary {
    pub rpm: f64,
    pub feed_mm_min: f64,
    pub plunge_mm_min: f64,
    pub doc_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub status: RecommendationStatus,
    pub context: ResolvedContext,
    pub values: Option<CuttingValues>,
    pub origins: Option<ValueOrigins>,
    pub metrics: Option<Metrics>,
    pub limits: Vec<LimitingFactor>,
    pub warnings: Vec<Warning>,
    pub levers: Vec<Lever>,
    pub trace: Vec<TraceStep>,
    pub provenance: Provenance,
    pub theoretical: Option<TheoreticalSummary>,
    /// Essais déjà enregistrés pour ce contexte, du plus récent au plus ancien.
    #[serde(default)]
    pub history: Vec<Trial>,
}
