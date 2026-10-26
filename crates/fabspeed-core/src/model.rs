//! Modèles de domaine du catalogue : machines, matières, fraises et préréglages.
//!
//! Ces types décrivent des **données** (chargées depuis `data/`), jamais des
//! algorithmes. Le vocabulaire (`Aggressiveness`, `Evidence`, `OperationKind`)
//! reprend volontairement celui du moteur Feeds & Speeds de FabCAM
//! (`ivac-feeds-speeds`) pour faciliter une intégration future.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Intention de l'utilisateur : à quel point pousser la machine.
///
/// Indépendant du niveau de preuve (`Evidence`) : une recommandation prudente
/// peut reposer sur des données faibles, et inversement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Aggressiveness {
    Prudent,
    #[default]
    Normal,
    Rapid,
}

/// Qualité / provenance d'une donnée de coupe. L'ordre de déclaration est
/// l'ordre de confiance croissante (utilisé par `Ord`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    /// Calcul à partir du modèle et de constantes génériques.
    Theoretical,
    /// Donnée documentée (fabricant, wiki du lab, référence) non qualifiée localement.
    ReferenceData,
    /// Essayée dans un contexte documenté (machine, matière, outil, opération).
    Tested,
    /// Répond aux critères de validation explicites du lab pour ce contexte.
    Validated,
}

/// Type d'opération d'usinage. Détermine l'engagement radial par défaut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    /// Détourage (contour). Traité comme une rainure pleine largeur : prudent.
    Profile,
    /// Évidement / poche : engagement radial = pas latéral.
    Pocket,
    /// Rainure pleine largeur explicite.
    FullSlot,
    /// Passe de finition : faible engagement radial, état de surface prioritaire.
    Finish,
    /// Surfaçage du martyr ou d'un brut.
    Surfacing,
    /// Gravure peu profonde avec une fraise droite.
    Engraving,
}

impl OperationKind {
    pub const ALL: [OperationKind; 6] = [
        OperationKind::Profile,
        OperationKind::Pocket,
        OperationKind::FullSlot,
        OperationKind::Finish,
        OperationKind::Surfacing,
        OperationKind::Engraving,
    ];
}

/// Comment la pièce est maintenue sur la table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Workholding {
    /// Vissée dans le martyr ou bridée.
    ScrewsClamps,
    /// Table aspirante / dépression.
    Vacuum,
    /// Ruban adhésif double face.
    Tape,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolFamily {
    FlatEndmill,
    BallNose,
    Compression,
    VBit,
    DiamondCut,
    Surfacing,
}

/// Sens d'hélice : décrit l'évacuation des copeaux, pas le sens de rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Helix {
    Upcut,
    Downcut,
    Compression,
    Straight,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolMaterial {
    #[default]
    Carbide,
    Hss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialCategory {
    Wood,
    Panel,
    Plastic,
    Foam,
    Metal,
    Composite,
}

/// Fiabilité d'une valeur de profil machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueStatus {
    /// Confirmée (plaque constructeur, mesure, documentation officielle).
    Verified,
    /// Estimation raisonnée, utilisable mais à confirmer.
    Estimate,
    /// Inconnue : valeur provisoire **à vérifier sur la machine**.
    ToVerify,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceNote {
    pub status: ValueStatus,
    pub note: String,
}

/// Identité stable compatible FabCAM (`{ id, revision }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub revision: String,
}

/// Profil physique d'une machine. Les limites inconnues sont marquées dans
/// `sources` plutôt qu'inventées silencieusement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    pub rpm_min: f64,
    pub rpm_max: f64,
    /// Puissance nominale de la broche (W).
    pub spindle_power_w: f64,
    /// Régime nominal d'une broche à couple constant (sous ce régime, la
    /// puissance disponible baisse proportionnellement). `None` = pas de déclassement.
    #[serde(default)]
    pub spindle_rated_rpm: Option<f64>,
    /// Avance de coupe maximale utilisable (mm/min).
    pub max_feed_mm_min: f64,
    #[serde(default)]
    pub max_plunge_mm_min: Option<f64>,
    /// Effort latéral admissible estimé avant vibrations / perte de pas (N).
    pub max_lateral_force_n: f64,
    /// Modulateur de rigidité appliqué à l'avance par dent (1.0 = référence routeur portique).
    pub chipload_factor: f64,
    /// Contrôleur, pour adapter l'affichage (ex. `shopbot` affiche aussi des mm/s).
    #[serde(default)]
    pub controller: Option<String>,
    /// Course X, Y, Z (mm).
    #[serde(default)]
    pub work_area_mm: Option<[f64; 3]>,
    #[serde(default)]
    pub sources: BTreeMap<String, SourceNote>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// Constantes empiriques d'une matière, normalisées au diamètre de fraise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialProfile {
    pub id: String,
    pub name: String,
    pub category: MaterialCategory,
    /// Identifiants équivalents (ex. identifiants FabCAM).
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub description: String,
    /// Vitesse de coupe cible pour fraise carbure (m/min).
    pub vc_m_min: f64,
    /// Avance par dent nominale par mm de diamètre (fz = D × k_fz).
    pub k_fz: f64,
    /// Profondeur de passe nominale en rainurage, par mm de diamètre (ap = D × k_ap).
    pub k_ap: f64,
    /// Pression spécifique de coupe (N/mm²) pour estimer puissance et effort.
    pub kc_n_mm2: f64,
    /// Épaisseur de copeau sous laquelle la fraise frotte au lieu de couper (mm).
    pub fz_min_mm: f64,
    /// Pas latéral par défaut des poches (fraction du diamètre).
    #[serde(default = "default_stepover")]
    pub default_stepover: f64,
    #[serde(default)]
    pub default_thickness_mm: Option<f64>,
    /// Épaisseur maximale à détourer en plein (ex. métaux : échauffement).
    #[serde(default)]
    pub max_profile_thickness_mm: Option<f64>,
    /// Usinage sous la supervision d'un fabmanager obligatoire.
    #[serde(default)]
    pub supervision_required: bool,
    #[serde(default)]
    pub tips: Vec<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default = "default_evidence")]
    pub evidence: Evidence,
}

fn default_stepover() -> f64 {
    0.4
}

fn default_evidence() -> Evidence {
    Evidence::Theoretical
}

/// Fraise physique (géométrie + données fournisseur). Les réglages de coupe
/// ne vivent pas ici : ils sont calculés ou viennent d'un préréglage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub reference: String,
    #[serde(default)]
    pub supplier: String,
    #[serde(default)]
    pub url: Option<String>,
    pub family: ToolFamily,
    #[serde(default)]
    pub helix: Helix,
    #[serde(default)]
    pub tool_material: ToolMaterial,
    pub diameter_mm: f64,
    /// `None` = nombre de dents inconnu ou non assimilable (taille diamant).
    #[serde(default)]
    pub flutes: Option<u8>,
    /// Longueur utile (coupante).
    pub flute_length_mm: f64,
    #[serde(default)]
    pub shank_mm: Option<f64>,
    #[serde(default)]
    pub overall_length_mm: Option<f64>,
    /// Angle de pointe des fraises à graver (°).
    #[serde(default)]
    pub tip_angle_deg: Option<f64>,
    /// Fraise à compression : longueur de la partie upcut (mm), si connue.
    #[serde(default)]
    pub upcut_length_mm: Option<f64>,
    #[serde(default)]
    pub max_rpm: Option<f64>,
    /// Régime minimal indiqué par le fournisseur.
    #[serde(default)]
    pub supplier_rpm_min: Option<f64>,
    #[serde(default)]
    pub supplier_rpm_recommended: Option<f64>,
    /// Identifiants de matières annoncées par le fournisseur.
    #[serde(default)]
    pub recommended_materials: Vec<String>,
    /// Identifiants de matières déconseillées par le fournisseur.
    #[serde(default)]
    pub excluded_materials: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub fabcam_identity: Option<Identity>,
}

/// Réglage connu (essai enregistré, référence importée) pour un contexte précis.
/// Un préréglage n'est jamais généralisé hors de son périmètre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabPreset {
    pub id: String,
    pub label: String,
    pub source: String,
    pub evidence: Evidence,
    /// Machines concernées (identifiants exacts).
    pub machines: Vec<String>,
    pub material: String,
    pub tool: String,
    pub operations: Vec<OperationKind>,
    pub aggressiveness: Aggressiveness,
    pub rpm: f64,
    pub feed_mm_min: f64,
    pub doc_mm: f64,
    /// Plongée testée ; `None` = calculée par les règles de plongée.
    #[serde(default)]
    pub plunge_mm_min: Option<f64>,
    /// Pas latéral maximal pour lequel le préréglage reste applicable en poche.
    #[serde(default)]
    pub stepover_max: Option<f64>,
    #[serde(default)]
    pub notes: Vec<String>,
    /// Date du réglage (RFC 3339) : à preuve égale, le plus récent l'emporte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// Métadonnées du jeu de données.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DatasetMeta {
    pub revision: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub license: String,
}
