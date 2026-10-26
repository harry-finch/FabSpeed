//! Moteur de recommandation : pur, déterministe, sans IO.
//!
//! Chaîne de calcul (détaillée dans `docs/CALCULATION_MODEL.md`) :
//!
//! 1. Résolution du contexte (machine, matière, fraise, opération).
//! 2. Réglage testé (essai réussi) s'il correspond **exactement**, sinon modèle :
//!    rotation (Vc) → copeau cible (k_fz) → amincissement radial → avance
//!    → profondeur (k_ap) → « murs » physiques qui réduisent la profondeur.
//! 3. Valeurs imposées par l'utilisateur (jamais modifiées par le moteur).
//! 4. Évaluation commune : copeau réel, puissance, efforts, charges.
//! 5. Avertissements, leviers « aller plus vite », trace et provenance.
//!
//! Les textes sont produits dans la langue du catalogue (`Catalog::lang`) ;
//! les codes (`Warning::code`, `TraceStep::key`) restent stables.

use crate::catalog::Catalog;
use crate::i18n::Lang;
use crate::levers;
use crate::model::{
    Aggressiveness, Evidence, Helix, LabPreset, MachineProfile, MaterialCategory, MaterialProfile,
    OperationKind, ToolFamily, ToolMaterial, ToolSpec, ValueStatus, Workholding,
};
use crate::policy::EnginePolicy;
use crate::recommendation::*;
use crate::request::RecommendationRequest;
use crate::tr;
use crate::trials::{Trial, TrialOutcome};
use std::borrow::Cow;
use std::f64::consts::PI;

pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Erreurs du moteur. `Display` donne le texte français ; [`EngineError::message`]
/// le donne dans la langue demandée.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EngineError {
    #[error("Machine inconnue : {0}")]
    UnknownMachine(String),
    #[error("Matière inconnue : {0}")]
    UnknownMaterial(String),
    #[error("Fraise inconnue : {0}")]
    UnknownTool(String),
    #[error("Aucune fraise fournie (tool_id ou tool)")]
    MissingTool,
    #[error("Symptôme inconnu : {0}")]
    UnknownSymptom(String),
    /// Le texte est déjà dans la langue du catalogue.
    #[error("{0}")]
    InvalidInput(String),
}

impl EngineError {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            EngineError::UnknownMachine(id) => {
                tr!(lang, "Machine inconnue : {id}", "Unknown machine: {id}")
            }
            EngineError::UnknownMaterial(id) => {
                tr!(lang, "Matière inconnue : {id}", "Unknown material: {id}")
            }
            EngineError::UnknownTool(id) => {
                tr!(lang, "Fraise inconnue : {id}", "Unknown tool: {id}")
            }
            EngineError::MissingTool => tr!(
                lang,
                "Aucune fraise fournie (tool_id ou tool)",
                "No tool given (tool_id or tool)"
            ),
            EngineError::UnknownSymptom(id) => {
                tr!(lang, "Symptôme inconnu : {id}", "Unknown symptom: {id}")
            }
            EngineError::InvalidInput(m) => m.clone(),
        }
    }
}

/// Contexte résolu, partagé par toutes les étapes du calcul.
pub(crate) struct Ctx<'a> {
    pub catalog: &'a Catalog,
    pub req: &'a RecommendationRequest,
    pub machine: &'a MachineProfile,
    pub material: &'a MaterialProfile,
    pub tool: Cow<'a, ToolSpec>,
    pub policy: &'a EnginePolicy,
    pub lang: Lang,
}

impl<'a> Ctx<'a> {
    pub fn resolve(
        catalog: &'a Catalog,
        req: &'a RecommendationRequest,
    ) -> Result<Self, EngineError> {
        let machine = catalog
            .machine(&req.machine_id)
            .ok_or_else(|| EngineError::UnknownMachine(req.machine_id.clone()))?;
        let material = catalog
            .material(&req.material_id)
            .ok_or_else(|| EngineError::UnknownMaterial(req.material_id.clone()))?;
        let tool = match (&req.tool_id, &req.tool) {
            (Some(id), _) => Cow::Borrowed(
                catalog
                    .tool(id)
                    .ok_or_else(|| EngineError::UnknownTool(id.clone()))?,
            ),
            (None, Some(t)) => Cow::Owned(t.clone()),
            (None, None) => return Err(EngineError::MissingTool),
        };
        let ctx = Ctx {
            catalog,
            req,
            machine,
            material,
            tool,
            policy: &catalog.policy,
            lang: catalog.lang,
        };
        ctx.validate()?;
        Ok(ctx)
    }

    fn validate(&self) -> Result<(), EngineError> {
        let l = self.lang;
        let positive = |name: String, v: Option<f64>| match v {
            Some(x) if !(x.is_finite() && x > 0.0) => Err(EngineError::InvalidInput(tr!(
                l,
                "{name} doit être un nombre > 0",
                "{name} must be a number > 0"
            ))),
            _ => Ok(()),
        };
        let t = &self.tool;
        positive(
            tr!(l, "Diamètre de fraise", "Tool diameter"),
            Some(t.diameter_mm),
        )?;
        positive(
            tr!(l, "Longueur utile", "Flute length"),
            Some(t.flute_length_mm),
        )?;
        if t.flutes == Some(0) {
            return Err(EngineError::InvalidInput(tr!(
                l,
                "Nombre de dents : 1 ou plus",
                "Number of flutes: 1 or more"
            )));
        }
        let r = self.req;
        positive(tr!(l, "Profondeur à usiner", "Total depth"), r.depth_mm)?;
        positive(tr!(l, "Sortie de pince", "Stickout"), r.stickout_mm)?;
        positive(
            tr!(l, "Profondeur de passe maximale", "Maximum depth of cut"),
            r.max_doc_mm,
        )?;
        positive(tr!(l, "Longueur de coupe", "Cut length"), r.cut_length_mm)?;
        positive(
            tr!(l, "Rotation imposée", "Spindle speed override"),
            r.overrides.rpm,
        )?;
        positive(
            tr!(l, "Avance imposée", "Feed override"),
            r.overrides.feed_mm_min,
        )?;
        positive(
            tr!(l, "Plongée imposée", "Plunge override"),
            r.overrides.plunge_mm_min,
        )?;
        positive(
            tr!(l, "Profondeur de passe imposée", "Depth of cut override"),
            r.overrides.doc_mm,
        )?;
        if let Some(s) = r.stepover_ratio {
            if !(s.is_finite() && s > 0.0 && s <= 1.0) {
                return Err(EngineError::InvalidInput(tr!(
                    l,
                    "Pas latéral : entre 1 et 100 % du diamètre",
                    "Stepover: between 1 and 100% of the diameter"
                )));
            }
        }
        Ok(())
    }

    pub fn d(&self) -> f64 {
        self.tool.diameter_mm
    }

    pub fn family(&self) -> &crate::policy::FamilyPolicy {
        self.policy.families.get(self.tool.family)
    }

    /// tan(angle de pointe / 2) d'une fraise en V, si l'angle est valide.
    pub fn v_half_tan(&self) -> Option<f64> {
        self.tool
            .tip_angle_deg
            .filter(|a| a.is_finite() && *a > 0.0 && *a < 180.0)
            .map(|a| (a.to_radians() / 2.0).tan())
    }

    /// Profondeur gravée par une fraise en V : la profondeur demandée, sinon
    /// celle où le cône atteint le diamètre de la fraise.
    pub fn v_depth(&self) -> Option<f64> {
        let t = self.v_half_tan()?;
        Some(self.req.depth_mm.unwrap_or(self.d() / (2.0 * t)))
    }

    /// Diamètre qui coupe réellement pour une passe de profondeur `ap` :
    /// - hémisphérique en passe peu profonde : 2 √(ap (D − ap)) ;
    /// - en V : 2 × profondeur gravée × tan(angle / 2), plafonné à D ;
    /// - autres : D.
    pub fn cutting_diameter(&self, ap: f64) -> f64 {
        let d = self.d();
        match self.tool.family {
            ToolFamily::BallNose if ap > 0.0 && ap < d / 2.0 => 2.0 * (ap * (d - ap)).sqrt(),
            ToolFamily::VBit => match (self.v_half_tan(), self.v_depth()) {
                (Some(t), Some(depth)) => (2.0 * depth * t).min(d),
                _ => d,
            },
            _ => d,
        }
    }

    /// Hauteur de fraise engagée pour une passe de profondeur `ap`. En
    /// détourage, rainure et finition, la fraise repasse dans le même sillon
    /// (ou longe tout le flanc) : la dernière passe engage toute la profondeur
    /// usinée. En poche, surfaçage et gravure, chaque passe n'engage que `ap`.
    pub fn engaged_depth(&self, ap: f64) -> f64 {
        match self.req.operation {
            OperationKind::Profile | OperationKind::FullSlot | OperationKind::Finish => {
                self.req.depth_mm.map_or(ap, |depth| depth.max(ap))
            }
            _ => ap,
        }
    }

    /// Diamètre de référence du copeau et de la passe visés : pour une fraise
    /// en V, le diamètre qui coupe, au moins une fraction de D (politique) ;
    /// diamètre plafonné par la politique de la famille (surfaçage) ; sinon D.
    pub fn reference_diameter(&self) -> f64 {
        match self.tool.family {
            ToolFamily::VBit => self
                .cutting_diameter(self.d())
                .max(self.policy.v_bit_reference_min_ratio * self.d()),
            _ => self.d().min(
                self.family()
                    .reference_diameter_cap_mm
                    .unwrap_or(f64::INFINITY),
            ),
        }
    }

    pub fn mode(&self) -> &crate::policy::ModeFactors {
        self.policy.modes.get(self.req.aggressiveness)
    }

    /// Nombre dans la langue du calcul.
    pub fn n(&self, v: f64, decimals: usize) -> String {
        self.lang.num(v, decimals)
    }

    /// Engagement radial effectif (fraction de D) pour l'opération.
    pub fn engagement_ratio(&self) -> f64 {
        let op = self.policy.operations.get(self.req.operation);
        op.engagement
            .or(self.req.stepover_ratio)
            .or(op.default_engagement)
            .unwrap_or(self.material.default_stepover)
            .clamp(0.01, 1.0)
    }

    pub fn stickout(&self) -> f64 {
        self.req
            .stickout_mm
            .unwrap_or(self.tool.flute_length_mm + self.policy.stickout_margin_mm)
    }

    /// Pénalité d'élancement : (seuil / élancement)², bornée par le plancher.
    pub fn slenderness_penalty(&self) -> f64 {
        let s = self.stickout() / self.d();
        if s > self.policy.slenderness_threshold {
            (self.policy.slenderness_threshold / s)
                .powi(2)
                .max(self.policy.slenderness_penalty_floor)
        } else {
            1.0
        }
    }

    /// Plafond haut de rotation : broche, puis fraise si connu.
    pub fn rpm_ceiling(&self) -> f64 {
        let mut hi = self.machine.rpm_max;
        if let Some(t) = self.tool.max_rpm {
            hi = hi.min(t);
        }
        hi.max(self.machine.rpm_min)
    }

    /// Avance par dent nominale (mode normal, sans facteur d'opération).
    pub fn chipload_nominal(&self) -> f64 {
        (self.reference_diameter()
            * self.material.k_fz
            * self.machine.chipload_factor
            * self.family().chipload)
            .max(self.material.fz_min_mm)
    }

    /// Puissance disponible à un régime donné (broche à couple constant sous
    /// son régime nominal).
    pub fn available_power(&self, rpm: f64) -> f64 {
        let torque_factor = match self.machine.spindle_rated_rpm {
            Some(rated) if rated > 0.0 => (rpm / rated).min(1.0),
            _ => 1.0,
        };
        self.machine.spindle_power_w * torque_factor
    }

    /// Effort admissible par la fraise ; pour une fraise en V, celui de la
    /// partie engagée, évaluée sur le diamètre de référence (pointe soutenue
    /// par le corps de la fraise, voir `v_bit_reference_min_ratio`).
    pub fn tool_force_limit(&self) -> f64 {
        let d = match self.tool.family {
            ToolFamily::VBit => self.reference_diameter(),
            _ => self.d(),
        };
        self.policy.tool_force_coeff * d.powi(2) * self.slenderness_penalty()
    }

    pub fn holding_force_limit(&self) -> f64 {
        self.policy.workholding.get(self.req.workholding)
    }

    pub fn is_cut_through(&self) -> bool {
        matches!(
            self.req.operation,
            OperationKind::Profile | OperationKind::FullSlot
        )
    }

    fn plunge_for(&self, feed: f64) -> (f64, String) {
        let rule = self
            .policy
            .plunge_rule(self.material.category, self.tool.helix)
            .expect("au moins une règle de plongée (validé au chargement)");
        let v = (rule.feed_ratio * feed)
            .min(rule.per_mm_diameter * self.d())
            .min(rule.cap_mm_min);
        (v, rule.id.clone())
    }
}

/// Facteur géométrique d'amincissement radial du copeau (non plafonné).
pub fn rct_geometric(ae_ratio: f64, threshold: f64) -> f64 {
    if ae_ratio < threshold && ae_ratio > 0.0 {
        1.0 / (2.0 * (ae_ratio - ae_ratio * ae_ratio).sqrt())
    } else {
        1.0
    }
}

fn round_down(v: f64, step: f64) -> f64 {
    if step > 0.0 {
        ((v / step) + 1e-9).floor() * step
    } else {
        v
    }
}

fn round_near(v: f64, step: f64) -> f64 {
    if step > 0.0 {
        (v / step).round() * step
    } else {
        v
    }
}

fn step(key: &str, label: String, formula: String, result: String) -> TraceStep {
    TraceStep {
        key: key.into(),
        label,
        formula,
        result,
    }
}

fn warning(code: &str, severity: Severity, message: String) -> Warning {
    Warning {
        code: code.into(),
        severity,
        message,
    }
}

/// Sortie du modèle théorique (avant valeurs imposées).
struct ModelOutput {
    rpm: f64,
    feed: f64,
    plunge: f64,
    doc_max: f64,
    limits: Vec<LimitingFactor>,
    trace: Vec<TraceStep>,
    warnings: Vec<Warning>,
}

fn push_limit(limits: &mut Vec<LimitingFactor>, l: LimitingFactor) {
    if !limits.contains(&l) {
        limits.push(l);
    }
}

fn compute_model(ctx: &Ctx, z: f64) -> ModelOutput {
    let l = ctx.lang;
    let p = ctx.policy;
    let d = ctx.d();
    let mode = ctx.mode();
    let op = p.operations.get(ctx.req.operation);
    let mut limits = Vec::new();
    let mut trace = Vec::new();
    let mut warnings = Vec::new();
    let rpm_unit = tr!(l, "tr/min", "rpm");

    let fam = ctx.family();
    let ae_ratio = ctx.engagement_ratio();
    // Diamètre de référence du copeau et de la passe (V : diamètre qui coupe ;
    // surfaçage : plafonné par la politique de la famille).
    let d_ref = ctx.reference_diameter();

    // Profondeur de passe nominale et plafonds géométriques. Calculée avant la
    // rotation : pour une fraise hémisphérique, elle fixe le diamètre qui coupe.
    let slender = ctx.slenderness_penalty();
    if slender < 1.0 {
        push_limit(&mut limits, LimitingFactor::Slenderness);
    }
    let small = if d_ref < p.doc_reference_diameter_mm {
        (d_ref / p.doc_reference_diameter_mm)
            .powf(p.doc_small_tool_exponent)
            .max(p.doc_small_tool_floor)
    } else {
        1.0
    };
    let bonus = p
        .engagement_doc_bonus
        .iter()
        .find(|b| ae_ratio <= b.max_ratio)
        .map_or(1.0, |b| b.factor);
    let doc_nominal =
        d_ref * ctx.material.k_ap * small * bonus * mode.doc * op.doc * fam.doc * slender;
    let mut doc = doc_nominal.min(p.doc_max_ratio * d_ref);
    if doc > ctx.tool.flute_length_mm {
        doc = ctx.tool.flute_length_mm;
        push_limit(&mut limits, LimitingFactor::FluteLength);
    }
    if let Some(cap) = ctx.req.max_doc_mm {
        if doc > cap {
            doc = cap;
            push_limit(&mut limits, LimitingFactor::UserDocCap);
        }
    }
    if let Some(depth) = ctx.req.depth_mm {
        doc = doc.min(depth);
    }
    let doc_factors = format!(
        "{} × {} × {} × {} × {} × {} × {} × {}",
        ctx.n(d_ref, 2),
        ctx.n(ctx.material.k_ap, 2),
        ctx.n(small, 2),
        ctx.n(bonus, 2),
        ctx.n(mode.doc, 2),
        ctx.n(op.doc, 2),
        ctx.n(fam.doc, 2),
        ctx.n(slender, 2)
    );
    let (nominal, applied) = (ctx.n(doc_nominal, 2), ctx.n(doc, 2));
    let doc_step = step(
        "depth_of_cut",
        tr!(l, "Profondeur de passe visée", "Target depth of cut"),
        tr!(
            l,
            "ap = D × k_ap × petite fraise × engagement × mode × opération × type de fraise × élancement = {doc_factors}",
            "ap = D × k_ap × small tool × engagement × mode × operation × tool type × slenderness = {doc_factors}"
        ),
        tr!(l, "{nominal} mm (plafonds appliqués : {applied} mm)", "{nominal} mm (after caps: {applied} mm)"),
    );

    // Diamètre qui coupe réellement (hémisphérique peu engagée, fraise en V),
    // à la passe la plus engagée.
    let engaged = ctx.engaged_depth(doc);
    let dc = ctx.cutting_diameter(engaged);
    if dc < d - 1e-9 {
        let (dc_s, d_s) = (ctx.n(dc, 2), ctx.n(d, 2));
        let (formula, result) = match ctx.tool.family {
            ToolFamily::VBit => {
                let (depth, angle) = (
                    ctx.n(ctx.v_depth().unwrap_or_default(), 2),
                    ctx.n(ctx.tool.tip_angle_deg.unwrap_or_default(), 0),
                );
                (
                    tr!(
                        l,
                        "Dc = 2 × profondeur × tan(angle / 2) = 2 × {depth} × tan({angle}° / 2)",
                        "Dc = 2 × depth × tan(angle / 2) = 2 × {depth} × tan({angle}° / 2)"
                    ),
                    tr!(
                        l,
                        "{dc_s} mm (largeur gravée en surface, fraise de {d_s} mm)",
                        "{dc_s} mm (engraved width at the surface, {d_s} mm tool)"
                    ),
                )
            }
            _ => {
                let ap = ctx.n(engaged, 2);
                (
                    format!("Dc = 2 √(ap × (D − ap)) = 2 √({ap} × ({d_s} − {ap}))"),
                    tr!(
                        l,
                        "{dc_s} mm au lieu de {d_s} mm : rotation et avance compensées",
                        "{dc_s} mm instead of {d_s} mm: speed and feed compensated"
                    ),
                )
            }
        };
        trace.push(step(
            "cutting_diameter",
            tr!(l, "Diamètre qui coupe", "Cutting diameter"),
            formula,
            result,
        ));
    }
    // Compensation de la fraise hémisphérique, plafonnée par la politique.
    let ball_comp = if ctx.tool.family == ToolFamily::BallNose {
        (d / dc).min(p.ball_nose_max_compensation)
    } else {
        1.0
    };

    // 1. Rotation à partir de la vitesse de coupe au diamètre qui coupe, bornée par la broche.
    let vc_factor = match ctx.tool.tool_material {
        ToolMaterial::Carbide => 1.0,
        ToolMaterial::Hss => p.hss_vc_factor,
    };
    let vc = ctx.material.vc_m_min * vc_factor;
    let rpm_ideal = match ctx.tool.family {
        ToolFamily::BallNose => vc * 1000.0 / (PI * d) * ball_comp,
        _ => vc * 1000.0 / (PI * dc),
    };
    let rpm_hi = ctx.rpm_ceiling();
    let rpm_lo = ctx.machine.rpm_min;
    let mut rpm = rpm_ideal;
    if rpm > rpm_hi {
        rpm = rpm_hi;
        let tool_limited = ctx.tool.max_rpm.is_some_and(|t| t < ctx.machine.rpm_max);
        push_limit(
            &mut limits,
            if tool_limited {
                LimitingFactor::ToolMaxRpm
            } else {
                LimitingFactor::SpindleMaxRpm
            },
        );
    } else if rpm < rpm_lo {
        rpm = rpm_lo;
        push_limit(&mut limits, LimitingFactor::SpindleMinRpm);
    }
    let (ideal, bounded) = (ctx.n(rpm_ideal, 0), ctx.n(rpm, 0));
    trace.push(step(
        "spindle_speed",
        tr!(l, "Vitesse de rotation", "Spindle speed"),
        format!(
            "N = Vc × 1000 / (π × D) = {} × 1000 / (π × {})",
            ctx.n(vc, 0),
            ctx.n(if ball_comp > 1.0 { d / ball_comp } else { dc }, 2)
        ),
        if (rpm - rpm_ideal).abs() > 0.5 {
            tr!(
                l,
                "{ideal} {rpm_unit} → bornée à {bounded} {rpm_unit}",
                "{ideal} {rpm_unit} → limited to {bounded} {rpm_unit}"
            )
        } else {
            format!("{bounded} {rpm_unit}")
        },
    ));

    // 2. Épaisseur de copeau cible, proportionnelle au diamètre de référence.
    let hex_raw = d_ref
        * ctx.material.k_fz
        * ctx.machine.chipload_factor
        * mode.chipload
        * op.chipload
        * fam.chipload
        * slender;
    let hex_target = hex_raw.max(ctx.material.fz_min_mm);
    let factors = format!(
        "{} × {} × {} × {} × {} × {} × {}",
        ctx.n(d_ref, 2),
        ctx.n(ctx.material.k_fz, 3),
        ctx.n(ctx.machine.chipload_factor, 2),
        ctx.n(mode.chipload, 2),
        ctx.n(op.chipload, 2),
        ctx.n(fam.chipload, 2),
        ctx.n(slender, 2)
    );
    let (raw, target) = (ctx.n(hex_raw, 3), ctx.n(hex_target, 3));
    trace.push(step(
        "chip_target",
        tr!(l, "Épaisseur de copeau visée", "Target chip thickness"),
        tr!(
            l,
            "h = D × k_fz × machine × mode × opération × type de fraise × élancement = {factors}",
            "h = D × k_fz × machine × mode × operation × tool type × slenderness = {factors}"
        ),
        if hex_raw < ctx.material.fz_min_mm {
            tr!(
                l,
                "{raw} mm → relevée au minimum anti-frottement {target} mm",
                "{raw} mm → raised to the anti-rubbing minimum {target} mm"
            )
        } else {
            tr!(l, "{target} mm/dent", "{target} mm/tooth")
        },
    ));

    // 3. Amincissement du copeau : radial à faible engagement, axial pour une
    //    hémisphérique peu engagée. On programme plus pour obtenir le copeau visé.
    let rct = rct_geometric(ae_ratio, p.rct_threshold_ratio).min(p.rct_max_factor);
    let fz = hex_target * rct * ball_comp;
    if rct > 1.0 || ball_comp > 1.0 {
        let (r, f, k) = (ctx.n(ae_ratio, 2), ctx.n(fz, 3), ctx.n(rct * ball_comp, 2));
        trace.push(step(
            "chip_thinning",
            tr!(l, "Amincissement du copeau", "Chip thinning"),
            if ball_comp > 1.0 {
                let b = ctx.n(ball_comp, 2);
                tr!(
                    l,
                    "fz = h × radial ({r}) × hémisphérique (D / Dc = {b})",
                    "fz = h × radial ({r}) × ball nose (D / Dc = {b})"
                )
            } else {
                tr!(
                    l,
                    "fz = h / (2√(r − r²)) avec r = ae/D = {r}",
                    "fz = h / (2√(r − r²)) with r = ae/D = {r}"
                )
            },
            tr!(l, "× {k} → fz = {f} mm/dent", "× {k} → fz = {f} mm/tooth"),
        ));
    }

    // 4. Avance ; si la machine plafonne, on baisse la rotation pour garder
    //    le copeau (sinon la fraise frotte et chauffe).
    let mut feed = rpm * z * fz;
    let feed_ideal = feed;
    if feed > ctx.machine.max_feed_mm_min {
        push_limit(&mut limits, LimitingFactor::MachineFeed);
        if p.reduce_rpm_when_feed_limited {
            let rpm_keep = ctx.machine.max_feed_mm_min / (z * fz);
            rpm = rpm_keep.max(rpm_lo);
        }
        feed = ctx.machine.max_feed_mm_min.min(rpm * z * fz);
    }
    let (asked, capped, adjusted) = (ctx.n(feed_ideal, 0), ctx.n(feed, 0), ctx.n(rpm, 0));
    trace.push(step(
        "feed",
        tr!(l, "Vitesse d'avance", "Feed rate"),
        format!("Vf = N × Z × fz = {} × {} × {}", ctx.n(rpm, 0), z, ctx.n(fz, 3)),
        if feed_ideal > feed + 0.5 {
            tr!(
                l,
                "{asked} mm/min demandé, plafonné à {capped} mm/min (rotation ajustée à {adjusted} {rpm_unit})",
                "{asked} mm/min requested, capped at {capped} mm/min (spindle adjusted to {adjusted} {rpm_unit})"
            )
        } else {
            format!("{capped} mm/min")
        },
    ));

    // 5. Profondeur de passe (calculée plus haut).
    trace.push(doc_step);

    // 6. Murs physiques : puissance, rigidité machine, résistance de la
    //    fraise, maintien de pièce. Puissance et efforts sont proportionnels
    //    à ap : on réduit ap sans toucher au copeau.
    // Fraise en V : seule la largeur gravée travaille.
    let ae = (ae_ratio * d).min(if ctx.tool.family == ToolFamily::VBit {
        dc
    } else {
        d
    });
    let target = mode.load_target;
    let p_cut = |ap: f64, vf: f64| ap * ae * vf * ctx.material.kc_n_mm2 / 60_000.0;
    // Effort tangentiel à la vitesse de coupe du diamètre qui coupe.
    let force = |pc: f64| pc * 60_000.0 / (PI * dc * rpm);
    let ratios = |ap: f64, vf: f64| -> [(LimitingFactor, f64); 4] {
        let pc = p_cut(ap, vf);
        let f = force(pc);
        [
            (
                LimitingFactor::SpindlePower,
                pc / p.spindle_efficiency / (ctx.available_power(rpm) * target),
            ),
            (
                LimitingFactor::MachineRigidity,
                f / (ctx.machine.max_lateral_force_n * target),
            ),
            (
                LimitingFactor::ToolStrength,
                f / (ctx.tool_force_limit() * target),
            ),
            (
                LimitingFactor::Workholding,
                f / (ctx.holding_force_limit() * target),
            ),
        ]
    };
    let worst = |rs: &[(LimitingFactor, f64); 4]| {
        rs.iter()
            .copied()
            .fold((LimitingFactor::SpindlePower, 0.0), |a, b| {
                if b.1 > a.1 {
                    b
                } else {
                    a
                }
            })
    };
    let (factor, r) = worst(&ratios(doc, feed));
    if r > 1.0 {
        doc /= r;
        push_limit(&mut limits, factor);
        let (pct, ratio, reduced, why) = (
            ctx.n(target * 100.0, 0),
            ctx.n(r, 2),
            ctx.n(doc, 2),
            limit_label(l, factor),
        );
        trace.push(step(
            "physical_limit",
            tr!(l, "Limite physique atteinte", "Physical limit reached"),
            tr!(
                l,
                "charge visée {pct} % dépassée × {ratio}",
                "target load {pct}% exceeded × {ratio}"
            ),
            tr!(
                l,
                "profondeur réduite à {reduced} mm ({why})",
                "depth reduced to {reduced} mm ({why})"
            ),
        ));
    }
    if doc < p.doc_min_mm {
        doc = p.doc_min_mm;
        let (factor, r) = worst(&ratios(doc, feed));
        if r > 1.0 {
            feed /= r;
            push_limit(&mut limits, factor);
            let (depth, f) = (ctx.n(doc, 2), ctx.n(feed, 0));
            warnings.push(warning(
                "feed_reduced_for_load",
                Severity::Warning,
                tr!(
                    l,
                    "Même à {depth} mm de profondeur, la combinaison est trop chargée : avance réduite à {f} mm/min. Envisagez une fraise plus grosse ou une autre matière.",
                    "Even at {depth} mm depth, this combination is overloaded: feed reduced to {f} mm/min. Consider a larger tool or another material."
                ),
            ));
        }
    }

    // 7. Arrondis prudents des valeurs calculées.
    let rnd = &p.rounding;
    let rpm = round_near(rpm, rnd.rpm_step).clamp(rpm_lo, rpm_hi);
    let feed = round_down(feed, rnd.feed_step).max(rnd.feed_step);
    let doc_max = round_down(doc, rnd.doc_step).max(p.doc_min_mm);
    let (plunge, rule) = ctx.plunge_for(feed);
    let plunge = round_down(plunge, rnd.plunge_step).max(rnd.plunge_step);
    let pl = ctx.n(plunge, 0);
    trace.push(step(
        "plunge",
        tr!(l, "Vitesse de plongée", "Plunge rate"),
        tr!(
            l,
            "règle « {rule} » : min(k × Vf, k × D, plafond)",
            "rule “{rule}”: min(k × Vf, k × D, cap)"
        ),
        tr!(
            l,
            "{pl} mm/min (estimation, entrée en rampe conseillée)",
            "{pl} mm/min (estimate, ramp entry advised)"
        ),
    ));

    ModelOutput {
        rpm,
        feed,
        plunge,
        doc_max,
        limits,
        trace,
        warnings,
    }
}

pub(crate) fn limit_label(lang: Lang, l: LimitingFactor) -> String {
    match l {
        LimitingFactor::SpindleMaxRpm => {
            tr!(lang, "rotation max de la broche", "spindle maximum speed")
        }
        LimitingFactor::SpindleMinRpm => {
            tr!(lang, "rotation min de la broche", "spindle minimum speed")
        }
        LimitingFactor::ToolMaxRpm => tr!(lang, "rotation max de la fraise", "tool maximum speed"),
        LimitingFactor::MachineFeed => {
            tr!(lang, "avance max de la machine", "machine maximum feed")
        }
        LimitingFactor::MachinePlunge => {
            tr!(lang, "plongée max de la machine", "machine maximum plunge")
        }
        LimitingFactor::SpindlePower => tr!(lang, "puissance de la broche", "spindle power"),
        LimitingFactor::MachineRigidity => tr!(lang, "rigidité de la machine", "machine rigidity"),
        LimitingFactor::ToolStrength => tr!(lang, "résistance de la fraise", "tool strength"),
        LimitingFactor::Workholding => tr!(lang, "maintien de la pièce", "workholding"),
        LimitingFactor::FluteLength => {
            tr!(lang, "longueur utile de la fraise", "tool flute length")
        }
        LimitingFactor::UserDocCap => {
            tr!(lang, "profondeur max demandée", "requested maximum depth")
        }
        LimitingFactor::Slenderness => tr!(
            lang,
            "fraise très sortie de la pince",
            "tool sticking far out of the collet"
        ),
    }
}

/// Correspondance d'un réglage connu avec la demande.
pub(crate) enum PresetMatch {
    /// Même intention (prudent / normal / rapide).
    Exact(LabPreset),
    /// Mode prudent demandé, seul un réglage normal existe : profondeur réduite.
    DerivedPrudent(LabPreset),
    /// Autre intention demandée : le réglage existant est conservé tel quel.
    Nearest(LabPreset),
}

/// Réglages connus applicables : références chargées (`data/presets/`) et
/// essais réussis, filtrés sur le contexte exact.
pub(crate) fn find_preset(ctx: &Ctx) -> Option<PresetMatch> {
    let req = ctx.req;
    let ae_ratio = ctx.engagement_ratio();
    let catalog = ctx.catalog;
    let swept = matches!(
        req.operation,
        OperationKind::Pocket | OperationKind::Surfacing
    );
    let mut candidates: Vec<LabPreset> = catalog
        .presets
        .iter()
        .cloned()
        .chain(catalog.tested_presets())
        .filter(|p| p.machines.iter().any(|m| m == &ctx.machine.id))
        .filter(|p| {
            catalog
                .material(&p.material)
                .is_some_and(|m| m.id == ctx.material.id)
        })
        .filter(|p| catalog.tool(&p.tool).is_some_and(|t| t.id == ctx.tool.id))
        .filter(|p| p.operations.contains(&req.operation))
        .filter(|p| !swept || p.stepover_max.is_none_or(|m| ae_ratio <= m + 1e-9))
        .collect();
    // Ordre déterministe : preuve la plus forte, puis le plus récent, puis identifiant.
    candidates.sort_by(|a, b| {
        b.evidence
            .cmp(&a.evidence)
            .then_with(|| b.date.cmp(&a.date))
            .then_with(|| a.id.cmp(&b.id))
    });
    let with_mode = |a: Aggressiveness| candidates.iter().find(|p| p.aggressiveness == a).cloned();
    if let Some(p) = with_mode(req.aggressiveness) {
        return Some(PresetMatch::Exact(p));
    }
    match req.aggressiveness {
        Aggressiveness::Prudent => {
            with_mode(Aggressiveness::Normal).map(PresetMatch::DerivedPrudent)
        }
        Aggressiveness::Rapid => with_mode(Aggressiveness::Normal)
            .or_else(|| with_mode(Aggressiveness::Prudent))
            .map(PresetMatch::Nearest),
        Aggressiveness::Normal => with_mode(Aggressiveness::Prudent).map(PresetMatch::Nearest),
    }
}

/// Valeurs « candidates » avant les valeurs imposées.
struct Base {
    rpm: f64,
    feed: f64,
    plunge: f64,
    doc_max: f64,
    origins: ValueOrigins,
}

/// Point d'entrée : calcule une recommandation complète.
pub fn recommend(
    catalog: &Catalog,
    req: &RecommendationRequest,
) -> Result<Recommendation, EngineError> {
    let ctx = Ctx::resolve(catalog, req)?;
    let l = ctx.lang;
    let tool = &ctx.tool;
    let context = ResolvedContext {
        machine_id: ctx.machine.id.clone(),
        machine_name: ctx.machine.name.clone(),
        material_id: ctx.material.id.clone(),
        material_name: ctx.material.name.clone(),
        tool_id: tool.id.clone(),
        tool_name: tool.name.clone(),
        tool_diameter_mm: tool.diameter_mm,
        tool_flutes: tool.flutes,
        operation: req.operation,
        aggressiveness: req.aggressiveness,
        workholding: req.workholding,
    };
    let mut warnings = context_warnings(&ctx);
    let mut provenance = Provenance {
        source: SourceKind::Model,
        preset_id: None,
        preset_label: None,
        preset_source: None,
        evidence: Evidence::Theoretical,
        engine_version: ENGINE_VERSION.to_string(),
        dataset_revision: catalog.meta.revision.clone(),
        notes: Vec::new(),
    };

    let unsupported = match (tool.family, tool.flutes) {
        (ToolFamily::VBit, _) if ctx.v_half_tan().is_none() => Some(tr!(
            l,
            "Fraise en V : angle de pointe inconnu. Renseignez-le dans la fiche de la fraise (ex. 60°, 90°) pour calculer la largeur gravée.",
            "V-bit: unknown tip angle. Enter it in the tool record (e.g. 60°, 90°) to compute the engraved width."
        )),
        (_, None) => Some(tr!(
            l,
            "Nombre de dents inconnu (ex. denture taille diamant) : le calcul d'avance par dent n'est pas possible. Partez des réglages du fournisseur et faites un essai.",
            "Unknown number of flutes (e.g. diamond-cut teeth): chip load cannot be computed. Start from the supplier's settings and run a test."
        )),
        _ => None,
    };
    let Some(z) = tool.flutes.filter(|_| unsupported.is_none()) else {
        warnings.push(warning(
            "unsupported_tool",
            Severity::Blocking,
            unsupported.unwrap_or_default(),
        ));
        sort_warnings(&mut warnings);
        return Ok(Recommendation {
            status: RecommendationStatus::Unsupported,
            context,
            values: None,
            origins: None,
            metrics: None,
            limits: vec![],
            warnings,
            levers: vec![],
            trace: vec![],
            provenance,
            theoretical: None,
            history: Vec::new(),
        });
    };
    let z = f64::from(z);

    let model = compute_model(&ctx, z);
    let mut limits = model.limits.clone();
    let mut trace = Vec::new();
    warnings.extend(model.warnings.iter().cloned());
    let preset = if req.use_presets {
        find_preset(&ctx)
    } else {
        None
    };

    let base = match &preset {
        None => {
            trace.extend(model.trace.iter().cloned());
            provenance.evidence = Evidence::Theoretical;
            let (name, source) = (&ctx.material.name, &ctx.material.source);
            provenance.notes.push(tr!(
                l,
                "Calcul théorique à partir des constantes de la matière « {name} » ({source}).",
                "Theoretical calculation from the constants of material “{name}” ({source})."
            ));
            Base {
                rpm: model.rpm,
                feed: model.feed,
                plunge: model.plunge,
                doc_max: model.doc_max,
                origins: ValueOrigins {
                    rpm: Origin::Calculated,
                    feed: Origin::Calculated,
                    plunge: Origin::Calculated,
                    doc: Origin::Calculated,
                },
            }
        }
        Some(m) => {
            let (p, derived) = match m {
                PresetMatch::Exact(p) => (p, false),
                PresetMatch::DerivedPrudent(p) => (p, true),
                PresetMatch::Nearest(p) => (p, false),
            };
            limits.clear();
            provenance.source = SourceKind::LabPreset;
            provenance.preset_id = Some(p.id.clone());
            provenance.preset_label = Some(p.label.clone());
            provenance.preset_source = Some(p.source.clone());
            provenance.evidence = if derived {
                Evidence::Theoretical
            } else {
                p.evidence
            };
            provenance.notes.extend(p.notes.iter().cloned());
            let mut doc_max = p.doc_mm;
            let mut doc_origin = Origin::LabPreset;
            if derived {
                doc_max = round_down(p.doc_mm * ctx.mode().doc, ctx.policy.rounding.doc_step)
                    .max(ctx.policy.doc_min_mm);
                doc_origin = Origin::DerivedFromPreset;
                let (from, to) = (ctx.n(p.doc_mm, 2), ctx.n(doc_max, 2));
                warnings.push(warning(
                    "preset_derived_prudent",
                    Severity::Info,
                    tr!(
                        l,
                        "Mode prudent : profondeur du réglage testé réduite de {from} à {to} mm (rotation et avance conservées).",
                        "Cautious mode: depth of the tested setting reduced from {from} to {to} mm (speed and feed kept)."
                    ),
                ));
            }
            if let PresetMatch::Nearest(_) = m {
                let (asked, kept) = (
                    mode_label(l, req.aggressiveness),
                    mode_label(l, p.aggressiveness),
                );
                warnings.push(warning(
                    "preset_other_mode",
                    Severity::Info,
                    tr!(
                        l,
                        "Aucun réglage testé en mode « {asked} » pour ce contexte : le réglage « {kept} » est conservé. Les leviers indiquent comment accélérer prudemment.",
                        "No tested setting in “{asked}” mode for this context: the “{kept}” setting is kept. The levers show how to speed up safely."
                    ),
                ));
            }
            if doc_max > tool.flute_length_mm {
                doc_max = tool.flute_length_mm;
                push_limit(&mut limits, LimitingFactor::FluteLength);
            }
            if let Some(cap) = req.max_doc_mm {
                if doc_max > cap {
                    doc_max = cap;
                    doc_origin = Origin::DerivedFromPreset;
                    push_limit(&mut limits, LimitingFactor::UserDocCap);
                }
            }
            let (plunge, plunge_origin) = match p.plunge_mm_min {
                Some(v) => (v, Origin::LabPreset),
                None => {
                    let (v, _) = ctx.plunge_for(p.feed_mm_min);
                    (
                        round_down(v, ctx.policy.rounding.plunge_step).max(1.0),
                        Origin::Calculated,
                    )
                }
            };
            let (rpm_s, feed_s, doc_s, src) = (
                ctx.n(p.rpm, 0),
                ctx.n(p.feed_mm_min, 0),
                ctx.n(p.doc_mm, 2),
                &p.source,
            );
            trace.push(step(
                "tested_setting",
                tr!(l, "Réglage testé", "Tested setting"),
                p.label.clone(),
                tr!(
                    l,
                    "{rpm_s} tr/min · {feed_s} mm/min · passe {doc_s} mm ({src})",
                    "{rpm_s} rpm · {feed_s} mm/min · depth {doc_s} mm ({src})"
                ),
            ));
            if plunge_origin == Origin::Calculated {
                if let Some(s) = model.trace.iter().find(|s| s.key == "plunge") {
                    let pl = ctx.n(plunge, 0);
                    trace.push(TraceStep {
                        result: tr!(
                            l,
                            "{pl} mm/min (calculée, non testée)",
                            "{pl} mm/min (calculated, not tested)"
                        ),
                        ..s.clone()
                    });
                }
            }
            let mut rpm = p.rpm;
            let mut feed = p.feed_mm_min;
            if rpm > ctx.rpm_ceiling()
                || rpm < ctx.machine.rpm_min
                || feed > ctx.machine.max_feed_mm_min
            {
                rpm = rpm.clamp(ctx.machine.rpm_min, ctx.rpm_ceiling());
                feed = feed.min(ctx.machine.max_feed_mm_min);
                warnings.push(warning(
                    "preset_clamped",
                    Severity::Warning,
                    tr!(
                        l,
                        "Le réglage testé dépasse les limites du profil machine : valeurs bornées.",
                        "The tested setting exceeds the machine profile limits: values clamped."
                    ),
                ));
            }
            Base {
                rpm,
                feed,
                plunge,
                doc_max,
                origins: ValueOrigins {
                    rpm: Origin::LabPreset,
                    feed: Origin::LabPreset,
                    plunge: plunge_origin,
                    doc: doc_origin,
                },
            }
        }
    };

    // Valeurs imposées par l'utilisateur : prioritaires, jamais corrigées.
    let o = &req.overrides;
    let mut origins = base.origins;
    let pick = |v: Option<f64>, base: f64, origin: &mut Origin| {
        v.map_or(base, |x| {
            *origin = Origin::UserOverride;
            x
        })
    };
    let rpm = pick(o.rpm, base.rpm, &mut origins.rpm);
    let mut feed = pick(o.feed_mm_min, base.feed, &mut origins.feed);
    // Rotation imposée seule : l'avance (non saisie) suit pour garder la même
    // avance par dent, sinon le copeau s'amincit (frottement) ou s'épaissit.
    let feed_follows = o.rpm.is_some() && o.feed_mm_min.is_none() && base.rpm > 0.0;
    if feed_follows {
        let step = ctx.policy.rounding.feed_step;
        feed = round_down(base.feed * rpm / base.rpm, step).max(step);
        if feed > ctx.machine.max_feed_mm_min {
            feed = ctx.machine.max_feed_mm_min;
            push_limit(&mut limits, LimitingFactor::MachineFeed);
        }
        if origins.feed == Origin::LabPreset {
            origins.feed = Origin::DerivedFromPreset;
        }
        let f = ctx.n(feed, 0);
        warnings.push(warning(
            "feed_follows_rpm",
            Severity::Info,
            tr!(
                l,
                "Rotation imposée : l'avance suit ({f} mm/min) pour garder la même avance par dent. Imposez aussi l'avance pour la figer.",
                "Spindle speed set by hand: the feed follows ({f} mm/min) to keep the same chipload. Set the feed too to freeze it."
            ),
        ));
    }
    let doc_max = pick(o.doc_mm, base.doc_max, &mut origins.doc);
    let plunge = match o.plunge_mm_min {
        Some(v) => {
            origins.plunge = Origin::UserOverride;
            v
        }
        // Avance modifiée : la plongée calculée suit l'avance.
        None if (o.feed_mm_min.is_some() || feed_follows)
            && origins.plunge == Origin::Calculated =>
        {
            let (v, _) = ctx.plunge_for(feed);
            round_down(v, ctx.policy.rounding.plunge_step).max(1.0)
        }
        None => base.plunge,
    };
    let mut plunge = plunge;
    if let Some(maxp) = ctx.machine.max_plunge_mm_min {
        if plunge > maxp && origins.plunge != Origin::UserOverride {
            plunge = maxp;
            push_limit(&mut limits, LimitingFactor::MachinePlunge);
        }
    }
    if !o.is_empty() {
        warnings.push(warning(
            "user_override",
            Severity::Info,
            tr!(
                l,
                "Valeurs modifiées à la main : le moteur les vérifie mais ne les corrige pas.",
                "Values edited by hand: the engine checks them but never corrects them."
            ),
        ));
        override_limit_warnings(&ctx, rpm, feed, &mut warnings);
    }

    // Passes égalisées sur la profondeur totale.
    let (passes, doc) = match req.depth_mm {
        Some(depth) => {
            let n = (depth / doc_max - 1e-9).ceil().max(1.0);
            (Some(n as u32), depth / n)
        }
        None => (None, doc_max),
    };
    let ae_ratio = ctx.engagement_ratio();
    let values = CuttingValues {
        rpm,
        feed_mm_min: feed,
        plunge_mm_min: plunge,
        doc_mm: doc,
        doc_max_mm: doc_max,
        passes,
        depth_mm: req.depth_mm,
        stepover_mm: ae_ratio * ctx.d(),
        stepover_ratio: ae_ratio,
        ramp_entry_advised: ramp_advised(&ctx),
    };
    let metrics = evaluate(&ctx, z, &values);
    metrics_trace(&ctx, &values, &metrics, &mut trace);
    metric_warnings(&ctx, &metrics, &mut warnings);
    if provenance.source == SourceKind::Model {
        warnings.push(warning(
            "theoretical_values",
            Severity::Info,
            tr!(
                l,
                "Valeurs théoriques prudentes : faites un premier essai sur une chute et écoutez la machine.",
                "Cautious theoretical values: make a first test on scrap and listen to the machine."
            ),
        ));
    }
    let theoretical = (provenance.source == SourceKind::LabPreset).then_some(TheoreticalSummary {
        rpm: model.rpm,
        feed_mm_min: model.feed,
        plunge_mm_min: model.plunge,
        doc_mm: model.doc_max,
    });
    let history = catalog.trial_history(
        &ctx.machine.id,
        &ctx.material.id,
        &ctx.tool.id,
        req.operation,
    );
    repeat_warnings(l, &history, &values, catalog, &mut warnings);
    newer_trial_warning(l, &history, &provenance, &mut warnings);
    // Une situation bloquante se corrige avant de chercher à accélérer.
    let blocked = warnings.iter().any(|w| w.severity == Severity::Blocking);
    let levers = if blocked {
        Vec::new()
    } else {
        levers::compute(&ctx, &values, &metrics, &limits, provenance.source)
    };
    sort_warnings(&mut warnings);

    Ok(Recommendation {
        status: RecommendationStatus::Ok,
        context,
        values: Some(values),
        origins: Some(origins),
        metrics: Some(metrics),
        limits,
        warnings,
        levers,
        trace,
        provenance,
        theoretical,
        history,
    })
}

/// Prévient si les valeurs proposées sont celles d'un essai déjà noté « à
/// ajuster » ou « échec » dans le même contexte.
fn repeat_warnings(
    l: Lang,
    history: &[Trial],
    v: &CuttingValues,
    catalog: &Catalog,
    w: &mut Vec<Warning>,
) {
    let close = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1e-9);
    let repeated = history.iter().find(|t| {
        t.outcome != TrialOutcome::Good
            && close(v.rpm, t.rpm, 0.03)
            && close(v.feed_mm_min, t.feed_mm_min, 0.03)
            && close(v.doc_mm, t.doc_mm, 0.05)
    });
    if let Some(t) = repeated {
        let seen: Vec<&str> = t
            .symptoms
            .iter()
            .filter_map(|id| catalog.symptom(id).map(|s| s.label.as_str()))
            .collect();
        let verdict = match t.outcome {
            TrialOutcome::Failed => tr!(l, "en échec", "as failed"),
            _ => tr!(l, "« à ajuster »", "“needs adjusting”"),
        };
        let date = l.date(&t.recorded_at);
        let seen = if seen.is_empty() {
            String::new()
        } else {
            format!(" ({})", seen.join(", ").to_lowercase())
        };
        w.push(warning(
            "repeats_unsuccessful_trial",
            Severity::Warning,
            tr!(
                l,
                "Ces réglages ont déjà été essayés le {date} et notés {verdict}{seen}. Consultez l'historique ou le diagnostic avant de recommencer.",
                "These settings were already tried on {date} and recorded {verdict}{seen}. Check the history or the diagnosis before trying again."
            ),
        ));
    }
}

/// Un réglage validé reste la référence même si un essai réussi plus récent
/// existe : on le signale pour que l'administrateur décide de le valider.
fn newer_trial_warning(l: Lang, history: &[Trial], p: &Provenance, w: &mut Vec<Warning>) {
    if p.evidence != Evidence::Validated {
        return;
    }
    let Some(validated) = p
        .preset_id
        .as_deref()
        .and_then(|id| id.strip_prefix("trial:"))
        .and_then(|id| history.iter().find(|t| t.id == id))
    else {
        return;
    };
    let newer = history.iter().find(|t| {
        t.outcome == TrialOutcome::Good
            && t.validation.is_none()
            && t.aggressiveness == validated.aggressiveness
            && t.recorded_at > validated.recorded_at
    });
    if let Some(t) = newer {
        let date = l.date(&t.recorded_at);
        w.push(warning(
            "newer_tested_trial",
            Severity::Info,
            tr!(
                l,
                "Un essai réussi plus récent ({date}) n'est pas validé : le réglage validé reste la référence tant que l'administrateur ne valide pas le nouveau.",
                "A more recent successful test ({date}) is not validated: the validated setting stays the reference until the administrator validates the new one."
            ),
        ));
    }
}

pub(crate) fn mode_label(lang: Lang, a: Aggressiveness) -> String {
    match a {
        Aggressiveness::Prudent => tr!(lang, "Prudent", "Cautious"),
        Aggressiveness::Normal => tr!(lang, "Normal", "Normal"),
        Aggressiveness::Rapid => tr!(lang, "Rapide", "Fast"),
    }
}

fn ramp_advised(ctx: &Ctx) -> bool {
    matches!(
        ctx.material.category,
        MaterialCategory::Metal | MaterialCategory::Plastic
    ) || ctx.tool.helix == Helix::Downcut
        || ctx.req.operation == OperationKind::Pocket
}

/// Évalue des valeurs de coupe (recommandées ou imposées).
pub(crate) fn evaluate(ctx: &Ctx, z: f64, v: &CuttingValues) -> Metrics {
    let d = ctx.d();
    let p = ctx.policy;
    let dc = ctx.cutting_diameter(ctx.engaged_depth(v.doc_mm));
    let fz = v.feed_mm_min / (v.rpm * z);
    let rct = rct_geometric(v.stepover_ratio, p.rct_threshold_ratio);
    // Hémisphérique peu engagée : le copeau réel s'amincit dans le rapport Dc / D.
    let axial = if ctx.tool.family == ToolFamily::BallNose {
        dc / d
    } else {
        1.0
    };
    let hex = fz / rct * axial;
    let nominal = ctx.chipload_nominal();
    let min = ctx.material.fz_min_mm;
    let max = nominal * p.chip_thick_factor;
    let state = if hex < min {
        ChipState::Rubbing
    } else if hex > max {
        ChipState::Thick
    } else {
        ChipState::Ok
    };
    let ae = if ctx.tool.family == ToolFamily::VBit {
        v.stepover_mm.min(dc)
    } else {
        v.stepover_mm
    };
    let p_cut = v.doc_mm * ae * v.feed_mm_min * ctx.material.kc_n_mm2 / 60_000.0;
    let p_spindle = p_cut / p.spindle_efficiency;
    let p_avail = ctx.available_power(v.rpm);
    let force = p_cut * 60_000.0 / (PI * dc * v.rpm);
    let pct = |num: f64, den: f64| if den > 0.0 { num / den * 100.0 } else { 0.0 };
    let estimated_time_s = ctx.req.cut_length_mm.map(|len| {
        let passes = f64::from(v.passes.unwrap_or(1));
        let cut = passes * len / v.feed_mm_min * 60.0;
        let plunge = passes * v.doc_mm / v.plunge_mm_min * 60.0;
        cut + plunge
    });
    // Temps de coupe pour un mètre de trajectoire, toutes passes comprises.
    let time_per_metre_s = f64::from(v.passes.unwrap_or(1)) * 60_000.0 / v.feed_mm_min;
    let stickout = ctx.stickout();
    Metrics {
        chipload_mm: fz,
        chip_thickness_mm: hex,
        chip_zone: ChipZone {
            min_mm: min,
            nominal_mm: nominal,
            max_mm: max,
            state,
        },
        rct_factor: rct,
        cutting_speed_m_min: PI * dc * v.rpm / 1000.0,
        cutting_diameter_mm: dc,
        mrr_cm3_min: v.doc_mm * ae * v.feed_mm_min / 1000.0,
        cutting_power_w: p_cut,
        available_power_w: p_avail,
        cutting_force_n: force,
        loads: Loads {
            spindle_pct: pct(p_spindle, p_avail),
            machine_pct: pct(force, ctx.machine.max_lateral_force_n),
            tool_pct: pct(force, ctx.tool_force_limit()),
            holding_pct: pct(force, ctx.holding_force_limit()),
        },
        stickout_mm: stickout,
        slenderness: stickout / d,
        estimated_time_s,
        time_per_metre_s,
    }
}

fn metrics_trace(ctx: &Ctx, v: &CuttingValues, m: &Metrics, trace: &mut Vec<TraceStep>) {
    let l = ctx.lang;
    let (h, lo, hi) = (
        ctx.n(m.chip_thickness_mm, 3),
        ctx.n(m.chip_zone.min_mm, 3),
        ctx.n(m.chip_zone.max_mm, 3),
    );
    trace.push(step(
        "actual_chip",
        tr!(l, "Copeau réel", "Actual chip"),
        format!(
            "fz = Vf / (N × Z) = {} / ({} × {}) ; h = fz / {}",
            ctx.n(v.feed_mm_min, 0),
            ctx.n(v.rpm, 0),
            ctx.tool.flutes.unwrap_or(0),
            ctx.n(m.rct_factor, 2)
        ),
        tr!(
            l,
            "{h} mm/dent (zone saine {lo} – {hi} mm)",
            "{h} mm/tooth (healthy zone {lo} – {hi} mm)"
        ),
    ));
    let (pw, avail) = (ctx.n(m.cutting_power_w, 0), ctx.n(m.available_power_w, 0));
    trace.push(step(
        "cutting_power",
        tr!(l, "Puissance de coupe", "Cutting power"),
        format!(
            "P = ap × ae × Vf × kc / {k} = {} × {} × {} × {} / {k}",
            ctx.n(v.doc_mm, 2),
            ctx.n(v.stepover_mm, 2),
            ctx.n(v.feed_mm_min, 0),
            ctx.n(ctx.material.kc_n_mm2, 0),
            k = ctx.n(60_000.0, 0)
        ),
        tr!(
            l,
            "{pw} W (disponible ≈ {avail} W à ce régime)",
            "{pw} W (≈ {avail} W available at this speed)"
        ),
    ));
    let (f, fm, ft, fh) = (
        ctx.n(m.cutting_force_n, 1),
        ctx.n(ctx.machine.max_lateral_force_n, 0),
        ctx.n(ctx.tool_force_limit(), 0),
        ctx.n(ctx.holding_force_limit(), 0),
    );
    trace.push(step(
        "cutting_force",
        tr!(l, "Effort de coupe", "Cutting force"),
        format!("F = P × {} / (π × D × N)", ctx.n(60_000.0, 0)),
        tr!(
            l,
            "{f} N (machine ≈ {fm} N, fraise ≈ {ft} N, maintien ≈ {fh} N)",
            "{f} N (machine ≈ {fm} N, tool ≈ {ft} N, workholding ≈ {fh} N)"
        ),
    ));
}

fn context_warnings(ctx: &Ctx) -> Vec<Warning> {
    let l = ctx.lang;
    let mut w = Vec::new();
    let t = &ctx.tool;
    let m = ctx.material;
    let name = &m.name;
    // Constantes saisies au lab : hors du calibrage d'origine (wiki), à confirmer par un essai.
    if ctx.catalog.layers.lab_materials.contains(&m.id) {
        let (code, text) = if ctx.catalog.layers.overridden.contains(&m.id) {
            (
                "material_modified",
                tr!(
                    l,
                    "{name} : matière modifiée au lab, ses constantes sortent du calibrage d'origine. Confirmez par un essai.",
                    "{name}: material modified at the lab, its constants are outside the original calibration. Confirm with a test."
                ),
            )
        } else {
            (
                "material_from_lab",
                tr!(
                    l,
                    "{name} : matière créée au lab, constantes non calibrées. Confirmez par un essai.",
                    "{name}: material created at the lab, constants not calibrated. Confirm with a test."
                ),
            )
        };
        w.push(warning(code, Severity::Info, text));
    }
    if m.supervision_required {
        w.push(warning(
            "supervision_required",
            Severity::Warning,
            tr!(
                l,
                "{name} : toujours usiner accompagné d'un fabmanager.",
                "{name}: always machine with a fab manager present."
            ),
        ));
    }
    if t.excluded_materials
        .iter()
        .any(|id| ctx.catalog.material(id).is_some_and(|x| x.id == m.id))
    {
        w.push(warning(
            "material_excluded_by_supplier",
            Severity::Warning,
            tr!(
                l,
                "Le fournisseur déconseille cette fraise pour « {name} ».",
                "The supplier advises against this tool for “{name}”."
            ),
        ));
    }
    if let Some(min) = t.supplier_rpm_min {
        if min > ctx.machine.rpm_max {
            let (min, max) = (ctx.n(min, 0), ctx.n(ctx.machine.rpm_max, 0));
            w.push(warning(
                "spindle_too_slow_for_tool",
                Severity::Warning,
                tr!(
                    l,
                    "Le fournisseur indique au moins {min} tr/min pour cette fraise ; la broche plafonne à {max} tr/min. Restez prudent sur l'avance.",
                    "The supplier specifies at least {min} rpm for this tool; the spindle tops out at {max} rpm. Be cautious with the feed."
                ),
            ));
        }
    }
    if let Some(depth) = ctx.req.depth_mm {
        let deep_op = matches!(
            ctx.req.operation,
            OperationKind::Profile | OperationKind::FullSlot | OperationKind::Pocket
        );
        if deep_op && depth > t.flute_length_mm + 1e-9 {
            let (lu, dp) = (ctx.n(t.flute_length_mm, 1), ctx.n(depth, 1));
            w.push(warning(
                "tool_too_short",
                Severity::Blocking,
                tr!(
                    l,
                    "Fraise trop courte : longueur utile {lu} mm pour {dp} mm à usiner. Le corps de la fraise frotterait.",
                    "Tool too short: {lu} mm flute length for {dp} mm to cut. The tool body would rub."
                ),
            ));
        }
        if let Some(max) = m.max_profile_thickness_mm {
            if ctx.is_cut_through() && depth > max {
                let max = ctx.n(max, 0);
                w.push(warning(
                    "profile_too_thick",
                    Severity::Warning,
                    tr!(
                        l,
                        "{name} : ne pas détourer plus de {max} mm d'épaisseur en plein (échauffement).",
                        "{name}: do not profile more than {max} mm thick in one go (heat build-up)."
                    ),
                ));
            }
        }
    }
    match t.family {
        // Hémisphérique en 2D : fond arrondi, flancs pleine largeur, passes très fines.
        // (Une rainure à fond rond reste un usage normal.)
        ToolFamily::BallNose
            if matches!(
                ctx.req.operation,
                OperationKind::Profile | OperationKind::Pocket | OperationKind::Surfacing
            ) =>
        {
            w.push(warning(
                "ball_nose_2d",
                Severity::Warning,
                tr!(
                    l,
                    "Fraise hémisphérique : elle est faite pour le relief 3D et les rainures à fond rond. Pour détourer, vider une poche à fond plat ou surfacer, préférez une fraise droite.",
                    "Ball nose: it is meant for 3D relief and round-bottomed grooves. To profile, clear a flat-bottomed pocket or surface, prefer a flat end mill."
                ),
            ))
        }
        ToolFamily::BallNose => w.push(warning(
            "ball_nose",
            Severity::Info,
            tr!(
                l,
                "Fraise hémisphérique : en passe peu profonde, le diamètre qui coupe est plus petit ; FabSpeed compense la rotation et l'avance (plafonné). En 3D, la passe réelle dépend du relief.",
                "Ball nose: in a shallow pass the cutting diameter is smaller; FabSpeed compensates speed and feed (capped). In 3D, the real depth depends on the relief."
            ),
        )),
        ToolFamily::Compression => {
            let ap = ctx.req.max_doc_mm;
            let msg = match t.upcut_length_mm {
                Some(up) if ap.is_none_or(|ap| ap < up) => {
                    let up = ctx.n(up, 1);
                    Some(tr!(
                        l,
                        "Fraise à compression : la première passe doit dépasser la partie upcut ({up} mm) pour que le dessus reste net ; sinon seule la partie upcut travaille et arrache le dessus.",
                        "Compression bit: the first pass must go deeper than the upcut section ({up} mm) to keep the top edge clean; otherwise only the upcut section cuts and tears the top."
                    ))
                }
                Some(_) => None,
                None => Some(tr!(
                    l,
                    "Fraise à compression : la première passe doit dépasser la partie upcut (longueur sur la fiche du fournisseur) pour que le dessus reste net.",
                    "Compression bit: the first pass must go deeper than the upcut section (length on the supplier's sheet) to keep the top edge clean."
                )),
            };
            if let Some(m) = msg {
                w.push(warning("compression_first_pass", Severity::Info, m));
            }
        }
        ToolFamily::VBit | ToolFamily::Surfacing => w.push(warning(
            "tool_type_not_calibrated",
            Severity::Info,
            tr!(
                l,
                "Ce type de fraise n'est pas encore calé par des essais du lab : faites un premier passage prudent sur une chute et notez-le.",
                "This tool type is not calibrated by lab tests yet: make a first cautious pass on scrap and record it."
            ),
        )),
        _ => {}
    }
    if t.helix == Helix::Upcut
        && matches!(ctx.req.workholding, Workholding::Tape | Workholding::Vacuum)
    {
        w.push(warning(
            "upcut_lifts_part",
            Severity::Info,
            tr!(
                l,
                "Une fraise upcut tire la pièce vers le haut : vissez ou bridez si possible.",
                "An upcut tool pulls the part upwards: screw or clamp it if possible."
            ),
        ));
    }
    let unverified: Vec<&str> = [
        "rpm_min",
        "rpm_max",
        "spindle_power_w",
        "max_feed_mm_min",
        "max_lateral_force_n",
    ]
    .into_iter()
    .filter(|k| {
        ctx.machine
            .sources
            .get(*k)
            .is_some_and(|s| s.status != ValueStatus::Verified)
    })
    .collect();
    if !unverified.is_empty() {
        let (machine, fields) = (&ctx.machine.name, unverified.join(", "));
        w.push(warning(
            "machine_limits_unverified",
            Severity::Info,
            tr!(
                l,
                "Certaines limites de « {machine} » sont des estimations à vérifier ({fields}).",
                "Some limits of “{machine}” are estimates to be checked ({fields})."
            ),
        ));
    }
    w
}

fn override_limit_warnings(ctx: &Ctx, rpm: f64, feed: f64, w: &mut Vec<Warning>) {
    let l = ctx.lang;
    if rpm > ctx.rpm_ceiling() + 0.5 || rpm < ctx.machine.rpm_min - 0.5 {
        let (lo, hi) = (ctx.n(ctx.machine.rpm_min, 0), ctx.n(ctx.rpm_ceiling(), 0));
        w.push(warning(
            "override_rpm_out_of_range",
            Severity::Blocking,
            tr!(
                l,
                "Rotation hors de la plage de la broche ({lo} – {hi} tr/min).",
                "Spindle speed outside the spindle range ({lo} – {hi} rpm)."
            ),
        ));
    }
    if feed > ctx.machine.max_feed_mm_min + 0.5 {
        let max = ctx.n(ctx.machine.max_feed_mm_min, 0);
        w.push(warning(
            "override_feed_out_of_range",
            Severity::Warning,
            tr!(
                l,
                "Avance au-delà de l'avance max du profil machine ({max} mm/min).",
                "Feed above the machine profile maximum ({max} mm/min)."
            ),
        ));
    }
}

fn metric_warnings(ctx: &Ctx, m: &Metrics, w: &mut Vec<Warning>) {
    let l = ctx.lang;
    let h = ctx.n(m.chip_thickness_mm, 3);
    match m.chip_zone.state {
        ChipState::Rubbing => {
            let min = ctx.n(m.chip_zone.min_mm, 3);
            w.push(warning(
                "chip_too_thin",
                Severity::Warning,
                tr!(
                    l,
                    "Copeau trop fin ({h} mm < {min} mm) : la fraise frotte, chauffe et s'use. Augmentez l'avance ou baissez la rotation.",
                    "Chip too thin ({h} mm < {min} mm): the tool rubs, heats up and wears. Increase the feed or lower the spindle speed."
                ),
            ));
        }
        ChipState::Thick => {
            let max = ctx.n(m.chip_zone.max_mm, 3);
            w.push(warning(
                "chip_too_thick",
                Severity::Warning,
                tr!(
                    l,
                    "Copeau épais ({h} mm > {max} mm) : efforts élevés, risque de casse ou d'arrachement.",
                    "Thick chip ({h} mm > {max} mm): high forces, risk of breakage or tear-out."
                ),
            ));
        }
        ChipState::Ok => {}
    }
    let loads = &m.loads;
    let rows = [
        (
            "load_spindle",
            loads.spindle_pct,
            tr!(l, "la puissance de la broche", "spindle power"),
        ),
        (
            "load_machine",
            loads.machine_pct,
            tr!(
                l,
                "la rigidité estimée de la machine",
                "estimated machine rigidity"
            ),
        ),
        (
            "load_tool",
            loads.tool_pct,
            tr!(
                l,
                "la résistance estimée de la fraise",
                "estimated tool strength"
            ),
        ),
        (
            "load_holding",
            loads.holding_pct,
            tr!(l, "le maintien de la pièce", "workholding"),
        ),
    ];
    for (code, pct, what) in rows {
        if pct > 100.0 {
            let pct = ctx.n(pct, 0);
            w.push(warning(
                &format!("{code}_exceeded"),
                Severity::Warning,
                tr!(
                    l,
                    "{pct} % de {what} : au-delà de la limite estimée.",
                    "{pct}% of {what}: beyond the estimated limit."
                ),
            ));
        }
    }
}

fn sort_warnings(w: &mut [Warning]) {
    w.sort_by(|a, b| b.severity.cmp(&a.severity));
}
