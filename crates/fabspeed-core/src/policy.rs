//! Politique du moteur : tous les facteurs de correction nommés.
//!
//! Aucun coefficient empirique ne doit apparaître « en dur » dans l'algorithme :
//! il vit ici, documenté, avec une valeur par défaut surchargeable par
//! `data/policy.toml`. Voir `docs/CALCULATION_MODEL.md`.

use crate::model::{
    Aggressiveness, Helix, MaterialCategory, OperationKind, ToolFamily, Workholding,
};
use serde::{Deserialize, Serialize};

/// Facteurs appliqués selon l'intention (Prudent / Normal / Rapide).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeFactors {
    /// Multiplicateur de l'avance par dent cible.
    pub chipload: f64,
    /// Multiplicateur de la profondeur de passe cible.
    pub doc: f64,
    /// Fraction des limites physiques estimées que le moteur s'autorise à utiliser.
    pub load_target: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modes {
    pub prudent: ModeFactors,
    pub normal: ModeFactors,
    pub rapid: ModeFactors,
}

impl Modes {
    pub fn get(&self, a: Aggressiveness) -> &ModeFactors {
        match a {
            Aggressiveness::Prudent => &self.prudent,
            Aggressiveness::Normal => &self.normal,
            Aggressiveness::Rapid => &self.rapid,
        }
    }
}

/// Paramètres propres à un type d'opération.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationPolicy {
    /// Engagement radial fixe (fraction de D). `None` = pas latéral demandé.
    pub engagement: Option<f64>,
    /// Pas latéral par défaut quand il n'est ni fixe ni demandé.
    /// `None` = pas latéral par défaut de la matière.
    #[serde(default)]
    pub default_engagement: Option<f64>,
    /// Multiplicateur d'épaisseur de copeau (finition : copeau plus fin).
    pub chipload: f64,
    /// Multiplicateur de profondeur de passe.
    pub doc: f64,
}

impl OperationPolicy {
    const fn fixed(engagement: f64, chipload: f64, doc: f64) -> Self {
        Self {
            engagement: Some(engagement),
            default_engagement: None,
            chipload,
            doc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operations {
    pub profile: OperationPolicy,
    pub pocket: OperationPolicy,
    pub full_slot: OperationPolicy,
    pub finish: OperationPolicy,
    pub surfacing: OperationPolicy,
    pub engraving: OperationPolicy,
}

impl Operations {
    pub fn get(&self, op: OperationKind) -> &OperationPolicy {
        match op {
            OperationKind::Profile => &self.profile,
            OperationKind::Pocket => &self.pocket,
            OperationKind::FullSlot => &self.full_slot,
            OperationKind::Finish => &self.finish,
            OperationKind::Surfacing => &self.surfacing,
            OperationKind::Engraving => &self.engraving,
        }
    }
}

/// Paramètres propres à un type de fraise (s'ajoutent à ceux de l'opération).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FamilyPolicy {
    /// Multiplicateur d'épaisseur de copeau visée.
    pub chipload: f64,
    /// Multiplicateur de profondeur de passe.
    pub doc: f64,
    /// Diamètre maximal pris en compte pour le copeau et la passe (mm).
    /// `None` = diamètre réel. Évite des copeaux et des passes démesurés
    /// avec les grosses fraises de surfaçage.
    #[serde(default)]
    pub reference_diameter_cap_mm: Option<f64>,
}

impl FamilyPolicy {
    const fn plain() -> Self {
        Self {
            chipload: 1.0,
            doc: 1.0,
            reference_diameter_cap_mm: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Families {
    pub flat_endmill: FamilyPolicy,
    pub ball_nose: FamilyPolicy,
    pub compression: FamilyPolicy,
    pub v_bit: FamilyPolicy,
    pub diamond_cut: FamilyPolicy,
    pub surfacing: FamilyPolicy,
}

impl Families {
    pub fn get(&self, f: ToolFamily) -> &FamilyPolicy {
        match f {
            ToolFamily::FlatEndmill => &self.flat_endmill,
            ToolFamily::BallNose => &self.ball_nose,
            ToolFamily::Compression => &self.compression,
            ToolFamily::VBit => &self.v_bit,
            ToolFamily::DiamondCut => &self.diamond_cut,
            ToolFamily::Surfacing => &self.surfacing,
        }
    }
}

/// Effort latéral que le maintien de pièce supporte avant que la pièce bouge (N).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkholdingLimits {
    pub screws_clamps: f64,
    pub vacuum: f64,
    pub tape: f64,
    pub unknown: f64,
}

impl WorkholdingLimits {
    pub fn get(&self, w: Workholding) -> f64 {
        match w {
            Workholding::ScrewsClamps => self.screws_clamps,
            Workholding::Vacuum => self.vacuum,
            Workholding::Tape => self.tape,
            Workholding::Unknown => self.unknown,
        }
    }
}

/// Bonus de profondeur quand l'engagement radial est faible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngagementDocBonus {
    /// S'applique si ae/D <= max_ratio (premier palier correspondant).
    pub max_ratio: f64,
    pub factor: f64,
}

/// Règle de plongée : `min(feed_ratio × avance, per_mm_diameter × D, cap)`.
/// La première règle dont les filtres correspondent s'applique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlungeRule {
    pub id: String,
    /// Catégories de matière concernées (vide = toutes).
    #[serde(default)]
    pub categories: Vec<MaterialCategory>,
    /// Hélices concernées (vide = toutes).
    #[serde(default)]
    pub helix: Vec<Helix>,
    pub feed_ratio: f64,
    pub per_mm_diameter: f64,
    pub cap_mm_min: f64,
    #[serde(default)]
    pub note: String,
}

/// Seuils de déclenchement des leviers « Aller plus vite ».
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeverPolicy {
    /// Charge maximale (%) sous laquelle on propose une passe plus profonde.
    pub deeper_below_load_pct: f64,
    /// Charge visée (%) lorsque l'on approfondit la passe.
    pub deeper_target_load_pct: f64,
    /// Hausse maximale de profondeur proposée en une fois (1.5 = +50 %).
    pub deeper_max_step: f64,
    /// Profondeur maximale proposée en pleine largeur (multiple de D).
    pub deeper_full_width_max_ratio: f64,
    /// Gain maximal proposé pour la hausse homothétique RPM + avance.
    pub homothetic_max_gain: f64,
    /// Gain minimal pour qu'un levier homothétique vaille la peine.
    pub homothetic_min_gain: f64,
    /// Hausse d'avance seule proposée (copeau plus épais).
    pub chip_step: f64,
    /// Charge maximale (%) pour proposer un copeau plus épais ou un mode plus rapide.
    pub faster_below_load_pct: f64,
}

/// Pas d'arrondi des valeurs **calculées** (les préréglages restent exacts).
/// L'avance, la plongée et la profondeur sont arrondies vers le bas (prudent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rounding {
    pub rpm_step: f64,
    pub feed_step: f64,
    pub plunge_step: f64,
    pub doc_step: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnginePolicy {
    /// Rendement mécanique broche → outil.
    pub spindle_efficiency: f64,
    /// Facteur de vitesse de coupe des fraises acier rapide (HSS) vs carbure.
    pub hss_vc_factor: f64,
    /// Engagement (ae/D) sous lequel on compense l'amincissement radial du copeau.
    pub rct_threshold_ratio: f64,
    /// Plafond du facteur d'amincissement radial (évite des avances extrêmes).
    pub rct_max_factor: f64,
    /// Profondeur de passe maximale en multiple de D.
    pub doc_max_ratio: f64,
    /// Profondeur de passe minimale proposée (mm).
    pub doc_min_mm: f64,
    /// Diamètre de référence pour la réduction de profondeur des petites fraises.
    pub doc_reference_diameter_mm: f64,
    /// Exposant de réduction : facteur = (D / D_ref)^exposant pour D < D_ref.
    pub doc_small_tool_exponent: f64,
    /// Plancher du facteur petite fraise.
    pub doc_small_tool_floor: f64,
    pub engagement_doc_bonus: Vec<EngagementDocBonus>,
    /// Effort admissible par la fraise ≈ coeff × D² (N, D en mm).
    pub tool_force_coeff: f64,
    /// Sortie de pince par défaut = longueur utile + cette marge (mm).
    pub stickout_margin_mm: f64,
    /// Élancement (sortie / D) à partir duquel on pénalise.
    pub slenderness_threshold: f64,
    /// Pénalité minimale (fraise très élancée).
    pub slenderness_penalty_floor: f64,
    /// Copeau « épais » au-delà de nominal × ce facteur.
    pub chip_thick_factor: f64,
    /// Si l'avance est plafonnée par la machine, baisser les tr/min pour
    /// conserver l'épaisseur de copeau (évite de brûler).
    pub reduce_rpm_when_feed_limited: bool,
    /// Fraise hémisphérique en passe peu profonde : plafond de la compensation
    /// (rotation et avance multipliées au plus par ce facteur) quand le
    /// diamètre qui coupe est plus petit que le diamètre de la fraise.
    pub ball_nose_max_compensation: f64,
    /// Fraise en V : la pointe est soutenue par le corps de la fraise ; le
    /// copeau et la passe visés se calent sur au moins cette fraction du
    /// diamètre (sinon, en gravure peu profonde, le copeau tombe au minimum
    /// anti-frottement). La résistance de la fraise est évaluée sur ce même
    /// diamètre de référence.
    pub v_bit_reference_min_ratio: f64,
    pub modes: Modes,
    pub operations: Operations,
    pub families: Families,
    pub workholding: WorkholdingLimits,
    pub plunge_rules: Vec<PlungeRule>,
    pub levers: LeverPolicy,
    pub rounding: Rounding,
}

impl Default for EnginePolicy {
    fn default() -> Self {
        Self {
            spindle_efficiency: 0.8,
            hss_vc_factor: 0.5,
            rct_threshold_ratio: 0.5,
            rct_max_factor: 2.0,
            doc_max_ratio: 2.0,
            doc_min_mm: 0.1,
            doc_reference_diameter_mm: 6.0,
            doc_small_tool_exponent: 0.5,
            doc_small_tool_floor: 0.4,
            engagement_doc_bonus: vec![
                EngagementDocBonus {
                    max_ratio: 0.25,
                    factor: 1.8,
                },
                EngagementDocBonus {
                    max_ratio: 0.5,
                    factor: 1.2,
                },
            ],
            tool_force_coeff: 4.0,
            stickout_margin_mm: 3.0,
            slenderness_threshold: 5.0,
            slenderness_penalty_floor: 0.35,
            chip_thick_factor: 2.5,
            reduce_rpm_when_feed_limited: true,
            ball_nose_max_compensation: 1.5,
            v_bit_reference_min_ratio: 0.5,
            modes: Modes {
                prudent: ModeFactors {
                    chipload: 0.85,
                    doc: 0.6,
                    load_target: 0.6,
                },
                normal: ModeFactors {
                    chipload: 1.0,
                    doc: 1.0,
                    load_target: 0.8,
                },
                rapid: ModeFactors {
                    chipload: 1.15,
                    doc: 1.35,
                    load_target: 1.0,
                },
            },
            operations: Operations {
                profile: OperationPolicy::fixed(1.0, 1.0, 1.0),
                pocket: OperationPolicy {
                    engagement: None,
                    default_engagement: None,
                    chipload: 1.0,
                    doc: 1.0,
                },
                full_slot: OperationPolicy::fixed(1.0, 1.0, 1.0),
                finish: OperationPolicy::fixed(0.1, 0.5, 1.0),
                surfacing: OperationPolicy {
                    engagement: None,
                    default_engagement: Some(0.5),
                    chipload: 0.8,
                    doc: 0.25,
                },
                engraving: OperationPolicy::fixed(1.0, 0.7, 0.5),
            },
            families: Families {
                flat_endmill: FamilyPolicy::plain(),
                // Pointe hémisphérique : peu de matière au centre, passes moins profondes.
                ball_nose: FamilyPolicy {
                    doc: 0.5,
                    ..FamilyPolicy::plain()
                },
                compression: FamilyPolicy::plain(),
                // Pointe en V fragile : copeau visé plus fin, calculé sur le diamètre qui coupe.
                v_bit: FamilyPolicy {
                    chipload: 0.7,
                    ..FamilyPolicy::plain()
                },
                diamond_cut: FamilyPolicy::plain(),
                // Fraises de surfaçage (Ø 20 à 50 mm) : copeau et passe calculés
                // comme pour une fraise de 12 mm, la largeur reste celle de l'outil.
                surfacing: FamilyPolicy {
                    reference_diameter_cap_mm: Some(12.0),
                    ..FamilyPolicy::plain()
                },
            },
            workholding: WorkholdingLimits {
                screws_clamps: 400.0,
                vacuum: 150.0,
                tape: 60.0,
                unknown: 150.0,
            },
            plunge_rules: vec![
                PlungeRule {
                    id: "metal".into(),
                    categories: vec![MaterialCategory::Metal],
                    helix: vec![],
                    feed_ratio: 0.10,
                    per_mm_diameter: 30.0,
                    cap_mm_min: 100.0,
                    note: "Métaux non ferreux (estimation CAM du pack fablab).".into(),
                },
                PlungeRule {
                    id: "plastic".into(),
                    categories: vec![MaterialCategory::Plastic],
                    helix: vec![],
                    feed_ratio: 0.10,
                    per_mm_diameter: 60.0,
                    cap_mm_min: 150.0,
                    note: "Thermoplastiques (estimation CAM du pack fablab, POM).".into(),
                },
                PlungeRule {
                    id: "downcut".into(),
                    categories: vec![],
                    helix: vec![Helix::Downcut, Helix::Compression],
                    feed_ratio: 0.10,
                    per_mm_diameter: 60.0,
                    cap_mm_min: 300.0,
                    note: "Hélice négative : les copeaux sont poussés vers le bas.".into(),
                },
                PlungeRule {
                    id: "default".into(),
                    categories: vec![],
                    helix: vec![],
                    feed_ratio: 0.15,
                    per_mm_diameter: 100.0,
                    cap_mm_min: 600.0,
                    note: "Bois / panneaux, hélice positive (estimation CAM du pack fablab)."
                        .into(),
                },
            ],
            levers: LeverPolicy {
                deeper_below_load_pct: 60.0,
                deeper_target_load_pct: 75.0,
                deeper_max_step: 1.5,
                deeper_full_width_max_ratio: 1.0,
                homothetic_max_gain: 0.3,
                homothetic_min_gain: 0.05,
                chip_step: 0.10,
                faster_below_load_pct: 55.0,
            },
            rounding: Rounding {
                rpm_step: 50.0,
                feed_step: 10.0,
                plunge_step: 1.0,
                doc_step: 0.05,
            },
        }
    }
}

impl EnginePolicy {
    /// Première règle de plongée correspondant à la matière et à l'hélice.
    pub fn plunge_rule(&self, category: MaterialCategory, helix: Helix) -> Option<&PlungeRule> {
        self.plunge_rules.iter().find(|r| {
            (r.categories.is_empty() || r.categories.contains(&category))
                && (r.helix.is_empty() || r.helix.contains(&helix))
        })
    }
}
