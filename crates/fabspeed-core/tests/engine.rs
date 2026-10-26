//! Tests de comportement du moteur. Les identifiants (ENG-xx) sont repris
//! dans docs/VALIDATION.md.

mod common;

use common::*;
use fabspeed_core::*;

const PLY: &str = "material/wood.plywood";
const MDF: &str = "material/wood.mdf";
const ALU: &str = "material/metal.aluminium";
const EVO6: &str = "generic/flat-6-1f-up";
const DOWN6: &str = "generic/flat-6-2f-down";
const DOWN3: &str = "generic/flat-3.175-2f-down";

fn values(r: &Recommendation) -> &CuttingValues {
    r.values.as_ref().expect("valeurs")
}

fn has_warning(r: &Recommendation, code: &str) -> bool {
    r.warnings.iter().any(|w| w.code == code)
}

#[test]
fn eng_01_exact_lab_preset_is_used_as_is() {
    // Réglage de référence fictif (tests/fixtures/presets.toml) : 15 000 tr/min, 5 000 mm/min, DOC 3.
    let c = catalog_with_presets();
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.depth_mm = Some(18.0);
    let reco = recommend(&c, &r).unwrap();
    let v = values(&reco);
    assert_eq!((v.rpm, v.feed_mm_min, v.doc_max_mm), (15000.0, 5000.0, 3.0));
    assert_eq!(v.plunge_mm_min, 600.0);
    assert_eq!(v.passes, Some(6));
    assert_eq!(reco.provenance.source, SourceKind::LabPreset);
    assert_eq!(reco.provenance.evidence, Evidence::ReferenceData);
    let o = reco.origins.unwrap();
    assert_eq!(o.rpm, Origin::LabPreset);
    assert_eq!(
        o.plunge,
        Origin::Calculated,
        "la plongée n'a pas été testée"
    );
    assert!(
        reco.theoretical.is_some(),
        "comparaison avec le modèle disponible"
    );
}

#[test]
fn eng_02_feed_equation_holds() {
    let c = catalog_with_presets();
    let mut r = req(MDF, "generic/flat-8-1f-up", OperationKind::Pocket);
    r.use_presets = false;
    let reco = recommend(&c, &r).unwrap();
    let v = values(&reco);
    let m = reco.metrics.as_ref().unwrap();
    let z = 1.0;
    assert!((v.feed_mm_min - v.rpm * z * m.chipload_mm).abs() < 1e-6);
}

#[test]
fn eng_03_small_tool_is_clamped_by_spindle_max_rpm() {
    let c = catalog_with_presets();
    let mut r = req(MDF, DOWN3, OperationKind::FullSlot);
    r.use_presets = false;
    let reco = recommend(&c, &r).unwrap();
    assert_eq!(values(&reco).rpm, 18000.0);
    assert!(reco.limits.contains(&LimitingFactor::SpindleMaxRpm));
}

#[test]
fn eng_04_feed_limit_lowers_rpm_to_keep_the_chip() {
    let c = catalog_with_presets();
    let mut r = req(MDF, DOWN6, OperationKind::FullSlot);
    r.use_presets = false;
    let reco = recommend(&c, &r).unwrap();
    let v = values(&reco);
    assert!(v.feed_mm_min <= 6000.0);
    assert!(reco.limits.contains(&LimitingFactor::MachineFeed));
    assert!(
        v.rpm < 18000.0,
        "rotation abaissée pour conserver le copeau"
    );
    assert_eq!(reco.metrics.unwrap().chip_zone.state, ChipState::Ok);
}

#[test]
fn eng_05_prudent_without_prudent_preset_derives_a_shallower_pass() {
    let c = catalog_with_presets();
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.aggressiveness = Aggressiveness::Prudent;
    let reco = recommend(&c, &r).unwrap();
    let v = values(&reco);
    assert_eq!((v.rpm, v.feed_mm_min), (15000.0, 5000.0));
    assert!(v.doc_max_mm < 3.0);
    let o = reco.origins.as_ref().unwrap();
    assert_eq!(o.doc, Origin::DerivedFromPreset);
    assert_eq!(
        reco.provenance.evidence,
        Evidence::Theoretical,
        "valeur dérivée = non testée"
    );
    assert!(has_warning(&reco, "preset_derived_prudent"));
}

#[test]
fn eng_06_exact_prudent_preset_wins() {
    let c = catalog_with_presets();
    let mut r = req(MDF, DOWN3, OperationKind::Profile);
    r.aggressiveness = Aggressiveness::Prudent;
    let reco = recommend(&c, &r).unwrap();
    let v = values(&reco);
    assert_eq!((v.feed_mm_min, v.doc_max_mm), (3000.0, 1.0));
    assert_eq!(
        reco.provenance.preset_id.as_deref(),
        Some("fixture-mdf-flat3down-prudent")
    );
}

#[test]
fn eng_07_rapid_keeps_the_normal_preset_and_says_so() {
    let c = catalog_with_presets();
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.aggressiveness = Aggressiveness::Rapid;
    let reco = recommend(&c, &r).unwrap();
    assert_eq!(values(&reco).doc_max_mm, 3.0);
    assert!(has_warning(&reco, "preset_other_mode"));
}

#[test]
fn eng_08_preset_scope_is_not_stretched() {
    let c = catalog_with_presets();
    // Pas latéral au-delà du périmètre du préréglage (50 %) → modèle.
    let mut r = req(PLY, EVO6, OperationKind::Pocket);
    r.stepover_ratio = Some(0.7);
    assert_eq!(
        recommend(&c, &r).unwrap().provenance.source,
        SourceKind::Model
    );
    // Finition : non couverte par le réglage → modèle.
    let r = req(PLY, EVO6, OperationKind::Finish);
    assert_eq!(
        recommend(&c, &r).unwrap().provenance.source,
        SourceKind::Model
    );
    // Autre machine → modèle.
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.machine_id = "generic-hobby-router".into();
    assert_eq!(
        recommend(&c, &r).unwrap().provenance.source,
        SourceKind::Model
    );
    // Bois dur générique : les essais du chêne ne s'y appliquent pas.
    let r = req("material/wood.hardwood", EVO6, OperationKind::Profile);
    assert_eq!(
        recommend(&c, &r).unwrap().provenance.source,
        SourceKind::Model
    );
}

#[test]
fn eng_09_aliases_resolve_fabcam_identifiers() {
    let c = catalog_with_presets();
    let r = req("fablab-material-oak", EVO6, OperationKind::Profile);
    let reco = recommend(&c, &r).unwrap();
    assert_eq!(reco.context.material_id, "material/wood.oak");
    assert_eq!(reco.provenance.source, SourceKind::LabPreset);
}

#[test]
fn eng_10_unknown_inputs_are_explicit_errors() {
    let c = catalog_with_presets();
    let r = req("material/unobtainium", EVO6, OperationKind::Profile);
    assert!(matches!(
        recommend(&c, &r),
        Err(EngineError::UnknownMaterial(_))
    ));
    let r = req(PLY, "nope", OperationKind::Profile);
    assert!(matches!(
        recommend(&c, &r),
        Err(EngineError::UnknownTool(_))
    ));
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.tool_id = None;
    assert!(matches!(recommend(&c, &r), Err(EngineError::MissingTool)));
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.machine_id = "nope".into();
    assert!(matches!(
        recommend(&c, &r),
        Err(EngineError::UnknownMachine(_))
    ));
    let mut r = req(PLY, EVO6, OperationKind::Pocket);
    r.stepover_ratio = Some(1.5);
    assert!(matches!(
        recommend(&c, &r),
        Err(EngineError::InvalidInput(_))
    ));
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.overrides.rpm = Some(0.0);
    assert!(matches!(
        recommend(&c, &r),
        Err(EngineError::InvalidInput(_))
    ));
}

#[test]
fn eng_11_unsupported_tools_return_no_values() {
    let c = catalog_with_presets();
    let reco = recommend(
        &c,
        &req(PLY, "generic/diamond-cut-3", OperationKind::Profile),
    )
    .unwrap();
    assert_eq!(reco.status, RecommendationStatus::Unsupported);
    assert!(reco.values.is_none());
    assert_eq!(reco.warnings[0].severity, Severity::Blocking);

    let mut vbit = c.tool(EVO6).unwrap().clone();
    vbit.id = "v60".into();
    vbit.family = ToolFamily::VBit;
    let mut r = req(PLY, EVO6, OperationKind::Engraving);
    r.tool_id = None;
    r.tool = Some(vbit);
    assert_eq!(
        recommend(&c, &r).unwrap().status,
        RecommendationStatus::Unsupported
    );
}

#[test]
fn eng_12_user_overrides_are_kept_and_checked() {
    let c = catalog_with_presets();
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.overrides.feed_mm_min = Some(1000.0);
    let reco = recommend(&c, &r).unwrap();
    let v = values(&reco);
    assert_eq!(v.feed_mm_min, 1000.0);
    assert_eq!(reco.origins.as_ref().unwrap().feed, Origin::UserOverride);
    assert_eq!(
        v.plunge_mm_min, 150.0,
        "la plongée calculée suit l'avance imposée"
    );
    // 1 000 mm/min à 15 000 tr/min avec 1 dent = copeau de 0,067 mm : sain mais
    // nettement plus fin ; à 200 mm/min la fraise frotte.
    r.overrides.feed_mm_min = Some(200.0);
    let reco = recommend(&c, &r).unwrap();
    assert_eq!(
        reco.metrics.as_ref().unwrap().chip_zone.state,
        ChipState::Rubbing
    );
    assert!(has_warning(&reco, "chip_too_thin"));
    r.overrides.rpm = Some(30000.0);
    assert!(has_warning(
        &recommend(&c, &r).unwrap(),
        "override_rpm_out_of_range"
    ));
}

#[test]
fn eng_13_results_are_deterministic_and_serializable() {
    let c = catalog_with_presets();
    let mut r = req(ALU, EVO6, OperationKind::Pocket);
    r.depth_mm = Some(4.0);
    r.cut_length_mm = Some(500.0);
    let a = recommend(&c, &r).unwrap();
    let b = recommend(&c, &r).unwrap();
    assert_eq!(a, b);
    let json = serde_json::to_string(&a).unwrap();
    let back: Recommendation = serde_json::from_str(&json).unwrap();
    assert_eq!(a, back);
    let req_json = serde_json::to_string(&r).unwrap();
    let req_back: RecommendationRequest = serde_json::from_str(&req_json).unwrap();
    assert_eq!(r, req_back);
}

#[test]
fn eng_14_tool_too_short_is_blocking() {
    let c = catalog_with_presets();
    let mut r = req(PLY, DOWN3, OperationKind::Profile);
    r.depth_mm = Some(18.0);
    let reco = recommend(&c, &r).unwrap();
    let w = reco
        .warnings
        .iter()
        .find(|w| w.code == "tool_too_short")
        .unwrap();
    assert_eq!(w.severity, Severity::Blocking);
}

#[test]
fn eng_15_metals_require_supervision_and_limit_profile_thickness() {
    let c = catalog_with_presets();
    let mut r = req(ALU, EVO6, OperationKind::Profile);
    r.depth_mm = Some(12.0);
    let reco = recommend(&c, &r).unwrap();
    assert!(has_warning(&reco, "supervision_required"));
    assert!(has_warning(&reco, "profile_too_thick"));
    assert!(values(&reco).ramp_entry_advised);
}

#[test]
fn eng_16_low_radial_engagement_programs_more_feed_than_slotting() {
    let c = catalog_with_presets();
    let mut slot = req(ALU, EVO6, OperationKind::FullSlot);
    slot.use_presets = false;
    let mut pocket = slot.clone();
    pocket.operation = OperationKind::Pocket;
    pocket.stepover_ratio = Some(0.2);
    let s = recommend(&c, &slot).unwrap();
    let p = recommend(&c, &pocket).unwrap();
    assert!(values(&p).feed_mm_min > values(&s).feed_mm_min);
    assert!(values(&p).doc_max_mm >= values(&s).doc_max_mm);
    // Le copeau réel reste le même : c'est l'avance programmée qui compense.
    let (hs, hp) = (
        s.metrics.unwrap().chip_thickness_mm,
        p.metrics.unwrap().chip_thickness_mm,
    );
    assert!((hs - hp).abs() / hs < 0.05, "{hs} vs {hp}");
}

#[test]
fn eng_17_workholding_wall_reduces_depth_not_chip() {
    let mut c = catalog_with_presets();
    c.policy.workholding.tape = 1.0; // maintien volontairement très faible
    let mut screws = req(PLY, EVO6, OperationKind::FullSlot);
    screws.use_presets = false;
    screws.workholding = Workholding::ScrewsClamps;
    let mut tape = screws.clone();
    tape.workholding = Workholding::Tape;
    let a = recommend(&c, &screws).unwrap();
    let b = recommend(&c, &tape).unwrap();
    assert!(b.limits.contains(&LimitingFactor::Workholding));
    assert!(values(&b).doc_max_mm < values(&a).doc_max_mm);
    assert_eq!(values(&b).feed_mm_min, values(&a).feed_mm_min);
    assert!(b.levers.iter().any(|l| l.id == "better_workholding"));
}

#[test]
fn eng_18_levers_are_actionable_patches() {
    let c = catalog_with_presets();
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.use_presets = false;
    r.depth_mm = Some(18.0);
    let reco = recommend(&c, &r).unwrap();
    let lever = reco
        .levers
        .iter()
        .find(|l| l.id == "deeper_passes")
        .expect("levier passes");
    assert!(lever.time_gain.unwrap() > 0.0);
    let next = recommend(&c, &r.apply_patch(&lever.patch)).unwrap();
    assert!(values(&next).passes.unwrap() < values(&reco).passes.unwrap());
    assert!(next.metrics.unwrap().loads.max() <= 100.0);
}

#[test]
fn eng_19_diagnosis_adjusts_and_reevaluates() {
    let c = catalog_with_presets();
    let r = req(PLY, EVO6, OperationKind::Profile);
    let d = diagnose(&c, &r, "chatter").unwrap();
    let after = d.after.values.as_ref().unwrap();
    assert!(after.doc_max_mm < d.before.doc_max_mm);
    assert!(after.rpm < d.before.rpm);
    assert_eq!(d.after.origins.as_ref().unwrap().doc, Origin::UserOverride);
    assert!(matches!(
        diagnose(&c, &r, "nope"),
        Err(EngineError::UnknownSymptom(_))
    ));

    let burn = diagnose(&c, &r, "burning").unwrap();
    let m_before = recommend(&c, &r).unwrap().metrics.unwrap();
    let m_after = burn.after.metrics.unwrap();
    assert!(
        m_after.chip_thickness_mm > m_before.chip_thickness_mm,
        "copeau plus épais contre la brûlure"
    );
}

#[test]
fn eng_20_invariants_over_the_whole_lab_catalog() {
    let c = catalog_with_presets();
    let machine = c.machine(SHOPBOT).unwrap().clone();
    let mut count = 0;
    for material in &c.materials {
        for tool in &c.tools {
            for op in OperationKind::ALL {
                for mode in [
                    Aggressiveness::Prudent,
                    Aggressiveness::Normal,
                    Aggressiveness::Rapid,
                ] {
                    let mut r = req(&material.id, &tool.id, op);
                    r.aggressiveness = mode;
                    r.depth_mm = Some(6.0);
                    let reco = recommend(&c, &r).unwrap();
                    let Some(v) = reco.values else { continue };
                    let ctx = format!("{} / {} / {op:?} / {mode:?}", material.id, tool.id);
                    for x in [v.rpm, v.feed_mm_min, v.plunge_mm_min, v.doc_mm] {
                        assert!(x.is_finite() && x > 0.0, "{ctx}");
                    }
                    assert!(
                        v.rpm >= machine.rpm_min && v.rpm <= machine.rpm_max,
                        "{ctx}"
                    );
                    assert!(v.feed_mm_min <= machine.max_feed_mm_min, "{ctx}");
                    assert!(v.plunge_mm_min < v.feed_mm_min, "{ctx}: plongée < avance");
                    assert!(v.doc_max_mm <= tool.flute_length_mm + 1e-9, "{ctx}");
                    assert!(v.doc_mm <= v.doc_max_mm + 1e-9, "{ctx}");
                    let m = reco.metrics.unwrap();
                    if reco.provenance.source == SourceKind::Model {
                        assert!(
                            m.loads.max() <= 100.0 + 1e-6,
                            "{ctx}: charge {:.0} %",
                            m.loads.max()
                        );
                    }
                    count += 1;
                }
            }
        }
    }
    assert!(count > 500, "{count} combinaisons vérifiées");
}

#[test]
fn eng_21_catalog_validation_catches_bad_data() {
    let mut c = catalog_with_presets();
    assert!(c.validate().iter().all(|i| !i.fatal));
    c.presets[0].tool = "ghost".into();
    c.machines[0].rpm_max = 0.0;
    let fatal: Vec<_> = c.validate().into_iter().filter(|i| i.fatal).collect();
    assert!(fatal.len() >= 2, "{fatal:?}");
}

#[test]
fn eng_22_estimated_time_counts_passes() {
    let c = catalog_with_presets();
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.depth_mm = Some(18.0);
    r.cut_length_mm = Some(1000.0);
    let reco = recommend(&c, &r).unwrap();
    let t = reco.metrics.unwrap().estimated_time_s.unwrap();
    // 6 passes × 1 000 mm / 5 000 mm/min = 72 s, plus 6 plongées de 3 mm à 600 mm/min = 1,8 s.
    assert!((t - 73.8).abs() < 0.01, "{t}");
}

#[test]
fn eng_23_no_speed_up_levers_for_supervised_metals() {
    let c = catalog_with_presets();
    let mut r = req(ALU, EVO6, OperationKind::Pocket);
    r.depth_mm = Some(3.0);
    r.stepover_ratio = Some(0.2);
    let reco = recommend(&c, &r).unwrap();
    assert!(
        reco.levers
            .iter()
            .all(|l| l.id == "shorter_stickout" || l.id == "better_workholding"),
        "{:?}",
        reco.levers.iter().map(|l| &l.id).collect::<Vec<_>>()
    );
    // Le fabmanager peut les demander explicitement.
    r.supervised_speedups = true;
    let unlocked = recommend(&c, &r).unwrap();
    assert!(
        unlocked
            .levers
            .iter()
            .any(|l| l.id != "shorter_stickout" && l.id != "better_workholding"),
        "{:?}",
        unlocked.levers.iter().map(|l| &l.id).collect::<Vec<_>>()
    );
}

#[test]
fn eng_24_deeper_lever_goes_one_step_at_a_time() {
    let c = catalog_with_presets();
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.depth_mm = Some(18.0);
    let reco = recommend(&c, &r).unwrap();
    let lever = reco
        .levers
        .iter()
        .find(|l| l.id == "deeper_passes")
        .unwrap();
    let doc = lever.patch.overrides.as_ref().unwrap().doc_mm.unwrap();
    // Préréglage à 3 mm : au plus +50 % et jamais plus de 1 × D en pleine largeur.
    assert!(doc > 3.0 && doc <= 4.5 + 1e-9, "{doc}");
}

#[test]
fn eng_25_default_data_starts_from_the_model() {
    // Jeu de données livré : aucun réglage préenregistré, tout part du calcul.
    let c = lab_catalog();
    assert!(c.presets.is_empty() && c.trials.is_empty());
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.depth_mm = Some(18.0);
    let reco = recommend(&c, &r).unwrap();
    assert_eq!(reco.provenance.source, SourceKind::Model);
    assert_eq!(reco.provenance.evidence, Evidence::Theoretical);
    assert!(reco.origins.unwrap().rpm == Origin::Calculated);
}

#[test]
fn eng_26_english_output_has_no_french_left() {
    use fabspeed_core::toml_data::parse_translations;
    let base = lab_catalog();
    let tr = parse_translations("en.toml", include_str!("../../../data/i18n/en.toml")).unwrap();
    assert!(
        base.untranslatable_ids(&tr).is_empty(),
        "{:?}",
        base.untranslatable_ids(&tr)
    );
    let c = base.localized(Lang::En, Some(&tr));
    // Marqueurs typiques d'un texte français resté non traduit.
    let french = [
        "é", "è", "à ", " de ", " la ", " le ", "fraise", "passe ", "tr/min",
    ];
    let mut checked = 0;
    for material in &c.materials {
        for tool in &c.tools {
            for op in OperationKind::ALL {
                for mode in [
                    Aggressiveness::Prudent,
                    Aggressiveness::Normal,
                    Aggressiveness::Rapid,
                ] {
                    let mut r = req(&material.id, &tool.id, op);
                    r.aggressiveness = mode;
                    r.depth_mm = Some(30.0);
                    r.workholding = Workholding::Tape;
                    let reco = recommend(&c, &r).unwrap();
                    let mut texts: Vec<&str> =
                        reco.warnings.iter().map(|w| w.message.as_str()).collect();
                    for l in &reco.levers {
                        texts.extend([l.title.as_str(), l.detail.as_str()]);
                        texts.extend(l.caution.as_deref());
                    }
                    for s in &reco.trace {
                        texts.extend([s.label.as_str(), s.formula.as_str(), s.result.as_str()]);
                    }
                    texts.extend(reco.provenance.notes.iter().map(String::as_str));
                    for t in texts {
                        // Les noms de fiches restent tels quels (références, Ø).
                        let t = t.replace(&reco.context.tool_name, "");
                        for f in french {
                            assert!(!t.contains(f), "« {f} » dans : {t}");
                        }
                    }
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 500);
}

#[test]
fn eng_27_lab_materials_are_flagged_outside_calibration() {
    let mut c = lab_catalog();
    let ply = "material/wood.plywood";
    let r = req(ply, "generic/flat-6-1f-up", OperationKind::Profile);
    let codes = |c: &Catalog| -> Vec<String> {
        recommend(c, &r)
            .unwrap()
            .warnings
            .into_iter()
            .map(|w| w.code)
            .collect()
    };
    assert!(!codes(&c).iter().any(|w| w.starts_with("material_")));

    // Fiche de base remplacée par une version du lab.
    c.layers.lab_materials.insert(ply.into());
    c.layers.overridden.insert(ply.into());
    assert!(codes(&c).contains(&"material_modified".to_string()));

    // Matière créée au lab.
    c.layers.overridden.clear();
    assert!(codes(&c).contains(&"material_from_lab".to_string()));
}

/// Fraise du catalogue transformée en un autre type, passée en ligne.
fn custom(c: &Catalog, family: ToolFamily, f: impl FnOnce(&mut ToolSpec)) -> ToolSpec {
    let mut t = c.tool(EVO6).unwrap().clone();
    t.id = format!("test/{family:?}");
    t.family = family;
    f(&mut t);
    t
}

fn with_tool(material: &str, tool: ToolSpec, op: OperationKind) -> RecommendationRequest {
    let mut r = req(material, EVO6, op);
    r.tool_id = None;
    r.tool = Some(tool);
    r
}

fn codes(reco: &Recommendation) -> Vec<&str> {
    reco.warnings.iter().map(|w| w.code.as_str()).collect()
}

#[test]
fn eng_28_v_bit_cuts_with_its_engraved_width() {
    let c = lab_catalog();
    let v60 = custom(&c, ToolFamily::VBit, |t| {
        t.diameter_mm = 12.0;
        t.flutes = Some(2);
        t.tip_angle_deg = Some(60.0);
    });
    let mut r = with_tool(PLY, v60.clone(), OperationKind::Engraving);
    r.depth_mm = Some(1.5);
    let reco = recommend(&c, &r).unwrap();
    assert_eq!(reco.status, RecommendationStatus::Ok);
    let m = reco.metrics.as_ref().unwrap();
    let width = 2.0 * 1.5 * (30f64).to_radians().tan();
    assert!(
        (m.cutting_diameter_mm - width).abs() < 1e-9,
        "{}",
        m.cutting_diameter_mm
    );
    // Petit diamètre qui coupe : rotation au maximum, copeau calé sur la largeur gravée.
    let v = values(&reco);
    assert_eq!(v.rpm, c.machine(SHOPBOT).unwrap().rpm_max);
    assert!(m.chip_zone.nominal_mm < 0.5 * c.material(PLY).unwrap().k_fz * 12.0);
    assert!(v.feed_mm_min > 0.0 && v.plunge_mm_min < v.feed_mm_min);
    assert!(m.loads.tool_pct <= 100.0 + 1e-9);
    assert!(reco.trace.iter().any(|s| s.key == "cutting_diameter"));
    assert!(codes(&reco).contains(&"tool_type_not_calibrated"));

    // Plus profond = plus large ; jamais plus large que la fraise.
    r.depth_mm = Some(3.0);
    let deeper = recommend(&c, &r).unwrap();
    assert!(deeper.metrics.as_ref().unwrap().cutting_diameter_mm > width);
    r.depth_mm = Some(40.0);
    r.max_doc_mm = Some(1.0);
    let full = recommend(&c, &r).unwrap();
    assert_eq!(full.metrics.as_ref().unwrap().cutting_diameter_mm, 12.0);

    // Sans angle : pas de calcul.
    let mut no_angle = v60;
    no_angle.tip_angle_deg = None;
    let blocked = recommend(&c, &with_tool(PLY, no_angle, OperationKind::Engraving)).unwrap();
    assert_eq!(blocked.status, RecommendationStatus::Unsupported);
}

#[test]
fn eng_29_ball_nose_compensates_shallow_passes_within_the_cap() {
    let c = lab_catalog();
    let ball = custom(&c, ToolFamily::BallNose, |t| {
        t.diameter_mm = 6.0;
        t.flutes = Some(2);
    });
    let mut r = with_tool(PLY, ball.clone(), OperationKind::Finish);
    r.max_doc_mm = Some(0.5);
    let reco = recommend(&c, &r).unwrap();
    let m = reco.metrics.as_ref().unwrap();
    let dc = 2.0 * (0.5f64 * 5.5).sqrt();
    assert!((m.cutting_diameter_mm - dc).abs() < 1e-9);
    // Compensation plafonnée (D / Dc ≈ 1,81 > 1,5) : le copeau réel reste dans la zone saine.
    assert_eq!(m.chip_zone.state, ChipState::Ok, "{m:?}");
    let flat = custom(&c, ToolFamily::FlatEndmill, |t| {
        t.diameter_mm = 6.0;
        t.flutes = Some(2);
    });
    let mut rf = with_tool(PLY, flat, OperationKind::Finish);
    rf.max_doc_mm = Some(0.5);
    let flat_reco = recommend(&c, &rf).unwrap();
    let (vb, vf) = (values(&reco), values(&flat_reco));
    assert!(
        vb.feed_mm_min > vf.feed_mm_min,
        "{} vs {}",
        vb.feed_mm_min,
        vf.feed_mm_min
    );
    assert!(
        vb.feed_mm_min <= vf.feed_mm_min * 1.5 * 1.5 + 10.0,
        "compensation plafonnée"
    );
    // La vitesse de coupe affichée est celle du diamètre qui coupe.
    assert!((m.cutting_speed_m_min - std::f64::consts::PI * dc * vb.rpm / 1000.0).abs() < 1e-6);

    // Passe d'au moins un rayon : pleine largeur, aucune compensation.
    r.max_doc_mm = Some(3.0);
    r.depth_mm = Some(3.0);
    r.operation = OperationKind::Pocket;
    let deep = recommend(&c, &r).unwrap();
    let dm = deep.metrics.as_ref().unwrap();
    if values(&deep).doc_mm >= 3.0 {
        assert_eq!(dm.cutting_diameter_mm, 6.0);
    }
    assert!(codes(&reco).contains(&"ball_nose"));
}

#[test]
fn eng_30_surfacing_bits_use_a_capped_reference_diameter() {
    let c = lab_catalog();
    let big = custom(&c, ToolFamily::Surfacing, |t| {
        t.diameter_mm = 50.0;
        t.flutes = Some(2);
        t.flute_length_mm = 10.0;
    });
    let reco = recommend(&c, &with_tool(PLY, big, OperationKind::Surfacing)).unwrap();
    let v = values(&reco);
    let m = reco.metrics.as_ref().unwrap();
    let k = c.material(PLY).unwrap();
    // Copeau et passe calculés comme pour une fraise de 12 mm, pas de 50 mm.
    assert!(
        m.chip_zone.nominal_mm <= 12.0 * k.k_fz + 1e-9,
        "{}",
        m.chip_zone.nominal_mm
    );
    assert!(v.doc_max_mm <= 12.0 * k.k_ap + 1e-9, "{}", v.doc_max_mm);
    assert!(v.feed_mm_min <= c.machine(SHOPBOT).unwrap().max_feed_mm_min);
    assert!(m.loads.spindle_pct <= 100.0 + 1e-9 && m.loads.machine_pct <= 100.0 + 1e-9);
    assert!(codes(&reco).contains(&"tool_type_not_calibrated"));
}

#[test]
fn eng_31_compression_bits_remind_the_first_pass_depth() {
    let c = lab_catalog();
    let known = custom(&c, ToolFamily::Compression, |t| {
        t.helix = Helix::Compression;
        t.upcut_length_mm = Some(5.0);
    });
    let mut r = with_tool(PLY, known.clone(), OperationKind::Profile);
    r.max_doc_mm = Some(3.0);
    assert!(codes(&recommend(&c, &r).unwrap()).contains(&"compression_first_pass"));
    r.max_doc_mm = Some(6.0);
    assert!(!codes(&recommend(&c, &r).unwrap()).contains(&"compression_first_pass"));
    let mut unknown = known;
    unknown.upcut_length_mm = None;
    let reco = recommend(&c, &with_tool(PLY, unknown, OperationKind::Profile)).unwrap();
    assert!(codes(&reco).contains(&"compression_first_pass"));
}

#[test]
fn eng_32_new_tool_type_messages_are_translated() {
    let mut c = lab_catalog();
    c.lang = Lang::En;
    let tools = [
        custom(&c, ToolFamily::VBit, |t| t.tip_angle_deg = Some(90.0)),
        custom(&c, ToolFamily::VBit, |t| t.tip_angle_deg = None),
        custom(&c, ToolFamily::BallNose, |_| {}),
        custom(&c, ToolFamily::Compression, |t| {
            t.upcut_length_mm = Some(5.0)
        }),
        custom(&c, ToolFamily::Surfacing, |t| t.diameter_mm = 30.0),
    ];
    for t in tools {
        let mut r = with_tool(PLY, t, OperationKind::Engraving);
        r.max_doc_mm = Some(1.0);
        let reco = recommend(&c, &r).unwrap();
        let text: Vec<String> = reco
            .warnings
            .iter()
            .map(|w| w.message.clone())
            .chain(
                reco.trace
                    .iter()
                    .flat_map(|s| [s.label.clone(), s.formula.clone(), s.result.clone()]),
            )
            .collect();
        for line in text {
            for marker in ["é", "è", "à ", " de ", "fraise", "passe ", "tr/min"] {
                assert!(!line.contains(marker), "« {marker} » dans : {line}");
            }
        }
    }
}

#[test]
fn eng_33_ball_nose_profile_engages_the_full_depth() {
    let c = lab_catalog();
    let ball = custom(&c, ToolFamily::BallNose, |t| {
        t.diameter_mm = 3.17;
        t.flutes = Some(2);
        t.flute_length_mm = 15.0;
    });
    let mut r = with_tool(PLY, ball.clone(), OperationKind::Profile);
    r.depth_mm = Some(6.0);
    r.max_doc_mm = Some(0.2);
    let reco = recommend(&c, &r).unwrap();
    let m = reco.metrics.as_ref().unwrap();
    // Dernière passe à 6 mm : toute la largeur coupe, aucune compensation,
    // copeau réel = avance par dent.
    assert_eq!(m.cutting_diameter_mm, 3.17);
    assert!((m.chip_thickness_mm - m.chipload_mm).abs() < 1e-9, "{m:?}");
    assert!(has_warning(&reco, "ball_nose_2d"));
    assert!(!has_warning(&reco, "ball_nose"));

    // Même fraise, une seule passe peu profonde : la compensation reste.
    let mut shallow = with_tool(PLY, ball, OperationKind::Finish);
    shallow.max_doc_mm = Some(0.2);
    let s = recommend(&c, &shallow).unwrap();
    assert!(s.metrics.as_ref().unwrap().cutting_diameter_mm < 3.17);
    assert!(has_warning(&s, "ball_nose") && !has_warning(&s, "ball_nose_2d"));
}

#[test]
fn eng_34_feed_follows_an_imposed_spindle_speed() {
    let c = lab_catalog();
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.depth_mm = Some(18.0);
    let before = recommend(&c, &r).unwrap();
    let vb = values(&before).clone();
    r.overrides.rpm = Some((vb.rpm * 0.8).round());
    let after = recommend(&c, &r).unwrap();
    let va = values(&after);
    let (fb, fa) = (
        before.metrics.as_ref().unwrap().chipload_mm,
        after.metrics.as_ref().unwrap().chipload_mm,
    );
    // Même avance par dent, à l'arrondi de l'avance près.
    assert!(va.feed_mm_min < vb.feed_mm_min);
    assert!((fa - fb).abs() / fb < 0.02, "{fb} → {fa}");
    let o = after.origins.as_ref().unwrap();
    assert_eq!((o.rpm, o.feed), (Origin::UserOverride, Origin::Calculated));
    assert!(has_warning(&after, "feed_follows_rpm"));

    // Avance imposée aussi : elle n'est jamais touchée.
    r.overrides.feed_mm_min = Some(vb.feed_mm_min);
    let both = recommend(&c, &r).unwrap();
    assert_eq!(values(&both).feed_mm_min, vb.feed_mm_min);
    assert!(!has_warning(&both, "feed_follows_rpm"));
}

#[test]
fn eng_35_time_per_metre_counts_passes() {
    let c = catalog_with_presets();
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.depth_mm = Some(18.0);
    let reco = recommend(&c, &r).unwrap();
    // 6 passes × 1 000 mm / 5 000 mm/min = 72 s.
    let t = reco.metrics.unwrap().time_per_metre_s;
    assert!((t - 72.0).abs() < 0.01, "{t}");
}
