//! Essais enregistrés : la base de connaissances du lab (TRI-xx, docs/VALIDATION.md).

mod common;

use common::*;
use fabspeed_core::toml_data::{machine_document, tool_document, trial_document, DataKind};
use fabspeed_core::*;

const PLY: &str = "material/wood.plywood";
const EVO6: &str = "generic/flat-6-1f-up";

fn input(op: OperationKind, outcome: TrialOutcome) -> TrialInput {
    TrialInput {
        operator: "Camille".into(),
        machine_id: SHOPBOT.into(),
        material_id: PLY.into(),
        tool_id: EVO6.into(),
        operation: op,
        aggressiveness: Aggressiveness::Normal,
        workholding: Workholding::ScrewsClamps,
        thickness_mm: Some(18.0),
        depth_mm: Some(18.0),
        stepover_ratio: 1.0,
        stickout_mm: None,
        tool_condition: Some(ToolCondition::New),
        rpm: 15000.0,
        feed_mm_min: 4000.0,
        plunge_mm_min: 400.0,
        doc_mm: 2.5,
        outcome,
        symptoms: vec![],
        notes: "Copeaux francs, bord propre.".into(),
    }
}

fn record(c: &mut Catalog, i: TrialInput, id: &str, at: &str) -> Trial {
    let t = Trial::from_input(i, c, id.into(), at.into(), ENGINE_VERSION).unwrap();
    c.trials.push(t.clone());
    t
}

#[test]
fn tri_01_good_trial_becomes_the_tested_setting() {
    let mut c = lab_catalog();
    record(
        &mut c,
        input(OperationKind::Profile, TrialOutcome::Good),
        "essai-1",
        "2026-10-22T09:00:00Z",
    );
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    r.depth_mm = Some(18.0);
    let reco = recommend(&c, &r).unwrap();
    let v = reco.values.as_ref().unwrap();
    assert_eq!(
        (v.rpm, v.feed_mm_min, v.plunge_mm_min, v.doc_max_mm),
        (15000.0, 4000.0, 400.0, 2.5)
    );
    let o = reco.origins.as_ref().unwrap();
    assert_eq!(
        (o.rpm, o.plunge, o.doc),
        (Origin::LabPreset, Origin::LabPreset, Origin::LabPreset)
    );
    assert_eq!(reco.provenance.evidence, Evidence::Tested);
    assert_eq!(
        reco.provenance.preset_label.as_deref(),
        Some("Essai du 22/10/2026 · Camille")
    );
    assert_eq!(reco.history.len(), 1);
}

#[test]
fn tri_02_scope_profile_equals_slot_but_not_pocket() {
    let mut c = lab_catalog();
    record(
        &mut c,
        input(OperationKind::Profile, TrialOutcome::Good),
        "essai-1",
        "2026-10-22T09:00:00Z",
    );
    let slot = recommend(&c, &req(PLY, EVO6, OperationKind::FullSlot)).unwrap();
    assert_eq!(slot.provenance.source, SourceKind::LabPreset);
    let pocket = recommend(&c, &req(PLY, EVO6, OperationKind::Pocket)).unwrap();
    assert_eq!(pocket.provenance.source, SourceKind::Model);
    let other_tool = recommend(
        &c,
        &req(PLY, "generic/flat-8-1f-up", OperationKind::Profile),
    )
    .unwrap();
    assert_eq!(other_tool.provenance.source, SourceKind::Model);
}

#[test]
fn tri_03_most_recent_good_trial_wins() {
    let mut c = lab_catalog();
    record(
        &mut c,
        input(OperationKind::Profile, TrialOutcome::Good),
        "essai-a",
        "2026-10-22T09:00:00Z",
    );
    let mut newer = input(OperationKind::Profile, TrialOutcome::Good);
    newer.feed_mm_min = 4500.0;
    record(&mut c, newer, "essai-b", "2026-10-23T09:00:00Z");
    let reco = recommend(&c, &req(PLY, EVO6, OperationKind::Profile)).unwrap();
    assert_eq!(reco.values.unwrap().feed_mm_min, 4500.0);
    assert_eq!(
        reco.history[0].id, "essai-b",
        "historique du plus récent au plus ancien"
    );
}

#[test]
fn tri_04_unsuccessful_trial_is_history_and_warns_on_repeat() {
    let mut c = lab_catalog();
    let mut bad = input(OperationKind::Profile, TrialOutcome::Adjust);
    bad.symptoms = vec!["burning".into()];
    record(&mut c, bad, "essai-1", "2026-10-22T09:00:00Z");
    let mut r = req(PLY, EVO6, OperationKind::Profile);
    let reco = recommend(&c, &r).unwrap();
    assert_eq!(
        reco.provenance.source,
        SourceKind::Model,
        "un essai à ajuster n'est pas une référence"
    );
    assert_eq!(reco.history.len(), 1);
    assert!(!reco
        .warnings
        .iter()
        .any(|w| w.code == "repeats_unsuccessful_trial"));
    r.overrides = Overrides {
        rpm: Some(15000.0),
        feed_mm_min: Some(4000.0),
        doc_mm: Some(2.5),
        plunge_mm_min: None,
    };
    let again = recommend(&c, &r).unwrap();
    let w = again
        .warnings
        .iter()
        .find(|w| w.code == "repeats_unsuccessful_trial")
        .unwrap();
    assert!(
        w.message.contains("22/10/2026") && w.message.contains("brûle"),
        "{}",
        w.message
    );
}

#[test]
fn tri_05_pocket_trial_covers_smaller_stepovers_only() {
    let mut c = lab_catalog();
    let mut p = input(OperationKind::Pocket, TrialOutcome::Good);
    p.stepover_ratio = 0.3;
    record(&mut c, p, "essai-1", "2026-10-22T09:00:00Z");
    let mut r = req(PLY, EVO6, OperationKind::Pocket);
    r.stepover_ratio = Some(0.25);
    assert_eq!(
        recommend(&c, &r).unwrap().provenance.source,
        SourceKind::LabPreset
    );
    r.stepover_ratio = Some(0.4);
    assert_eq!(
        recommend(&c, &r).unwrap().provenance.source,
        SourceKind::Model
    );
}

#[test]
fn tri_06_inputs_are_validated() {
    let c = lab_catalog();
    let mut i = input(OperationKind::Profile, TrialOutcome::Good);
    i.tool_id = "nope".into();
    assert!(matches!(
        Trial::from_input(i, &c, "x".into(), "t".into(), "v"),
        Err(EngineError::UnknownTool(_))
    ));
    let mut i = input(OperationKind::Profile, TrialOutcome::Good);
    i.rpm = 0.0;
    assert!(matches!(
        Trial::from_input(i, &c, "x".into(), "t".into(), "v"),
        Err(EngineError::InvalidInput(_))
    ));
    let mut i = input(OperationKind::Profile, TrialOutcome::Good);
    i.symptoms = vec!["ghost".into()];
    assert!(matches!(
        Trial::from_input(i, &c, "x".into(), "t".into(), "v"),
        Err(EngineError::UnknownSymptom(_))
    ));
}

#[test]
fn tri_07_orphan_trials_are_ignored_not_fatal() {
    let mut c = lab_catalog();
    let mut t = record(
        &mut c,
        input(OperationKind::Profile, TrialOutcome::Good),
        "essai-1",
        "2026-10-22T09:00:00Z",
    );
    t.id = "essai-2".into();
    t.tool_id = "lab/fraise-supprimee".into();
    c.trials.push(t);
    let issues = c.validate();
    assert!(issues.iter().all(|i| !i.fatal));
    assert!(issues.iter().any(|i| i.message.contains("essai-2")));
    assert_eq!(c.tested_presets().len(), 1);
}

#[test]
fn tri_08_documents_round_trip_through_toml() {
    let mut c = lab_catalog();
    let t = record(
        &mut c,
        input(OperationKind::Pocket, TrialOutcome::Failed),
        "essai-1",
        "2026-10-22T09:00:00Z",
    );
    let tool = c.tool(EVO6).unwrap().clone();
    let machine = c.machine(SHOPBOT).unwrap().clone();

    let mut back = Catalog::default();
    back.merge_toml(DataKind::Trials, "t.toml", &trial_document(&t))
        .unwrap();
    back.merge_toml(DataKind::Tools, "o.toml", &tool_document(&tool))
        .unwrap();
    back.merge_toml(DataKind::Machines, "m.toml", &machine_document(&machine))
        .unwrap();
    assert_eq!(back.trials, vec![t]);
    assert_eq!(back.tools, vec![tool]);
    assert_eq!(back.machines, vec![machine]);
}

#[test]
fn tri_09_validated_setting_stays_the_reference() {
    let mut c = lab_catalog();
    record(
        &mut c,
        input(OperationKind::Profile, TrialOutcome::Good),
        "essai-a",
        "2026-10-22T09:00:00Z",
    );
    let mut newer = input(OperationKind::Profile, TrialOutcome::Good);
    newer.feed_mm_min = 4500.0;
    record(&mut c, newer, "essai-b", "2026-10-23T09:00:00Z");
    c.trials[0]
        .validate(
            "2026-10-24T08:00:00Z".into(),
            " Stéphane ",
            "Bords nets sur 3 pièces.",
            Lang::Fr,
        )
        .unwrap();

    // Le réglage validé l'emporte sur l'essai réussi plus récent, qui est signalé.
    let reco = recommend(&c, &req(PLY, EVO6, OperationKind::Profile)).unwrap();
    assert_eq!(reco.values.as_ref().unwrap().feed_mm_min, 4000.0);
    assert_eq!(reco.provenance.evidence, Evidence::Validated);
    let label = reco.provenance.preset_label.as_deref().unwrap();
    assert!(label.contains("validé le 24/10/2026"), "{label}");
    assert!(reco.provenance.notes.iter().any(|n| n.contains("3 pièces")));
    assert!(reco.warnings.iter().any(|w| w.code == "newer_tested_trial"));
    assert_eq!(
        c.trials[0].validation.as_ref().unwrap().validated_by,
        "Stéphane"
    );

    // Entre deux réglages validés, le dernier validé fait référence.
    c.trials[1]
        .validate("2026-10-25T08:00:00Z".into(), "", "", Lang::Fr)
        .unwrap();
    let reco = recommend(&c, &req(PLY, EVO6, OperationKind::Profile)).unwrap();
    assert_eq!(reco.values.as_ref().unwrap().feed_mm_min, 4500.0);
    c.trials[0]
        .validate("2026-10-26T08:00:00Z".into(), "", "", Lang::Fr)
        .unwrap();
    let reco = recommend(&c, &req(PLY, EVO6, OperationKind::Profile)).unwrap();
    assert_eq!(reco.values.as_ref().unwrap().feed_mm_min, 4000.0);
    c.trials[1].validation = None;

    // Validation retirée : retour à l'essai réussi le plus récent, sans avertissement.
    c.trials[0].validation = None;
    let reco = recommend(&c, &req(PLY, EVO6, OperationKind::Profile)).unwrap();
    assert_eq!(reco.values.as_ref().unwrap().feed_mm_min, 4500.0);
    assert_eq!(reco.provenance.evidence, Evidence::Tested);
    assert!(!reco.warnings.iter().any(|w| w.code == "newer_tested_trial"));
}

#[test]
fn tri_10_only_good_trials_can_be_validated_and_validation_round_trips() {
    let mut c = lab_catalog();
    let mut failed = record(
        &mut c,
        input(OperationKind::Profile, TrialOutcome::Adjust),
        "essai-1",
        "2026-10-22T09:00:00Z",
    );
    assert!(matches!(
        failed.validate("2026-10-24T08:00:00Z".into(), "", "", Lang::En),
        Err(EngineError::InvalidInput(m)) if m.contains("Good")
    ));
    assert!(failed.validation.is_none());

    let mut good = record(
        &mut c,
        input(OperationKind::Profile, TrialOutcome::Good),
        "essai-2",
        "2026-10-22T10:00:00Z",
    );
    good.validate("2026-10-24T08:00:00Z".into(), "Stéphane", "", Lang::Fr)
        .unwrap();
    let mut back = Catalog::default();
    back.merge_toml(DataKind::Trials, "t.toml", &trial_document(&good))
        .unwrap();
    assert_eq!(back.trials, vec![good]);
}

#[test]
fn tri_11_trials_export_to_csv_in_each_language() {
    let mut c = lab_catalog();
    let mut i = input(OperationKind::Profile, TrialOutcome::Adjust);
    i.symptoms = vec!["burning".into()];
    i.notes = "Ça fume; bord \"noirci\"".into();
    record(&mut c, i, "essai-2", "2026-10-23T09:00:00Z");
    let mut good = record(
        &mut c,
        input(OperationKind::Profile, TrialOutcome::Good),
        "essai-1",
        "2026-10-22T09:00:00Z",
    );
    good.validate("2026-10-24T08:00:00Z".into(), "Stéphane", "", Lang::Fr)
        .unwrap();
    c.trials[1] = good;

    let fr = fabspeed_core::export::trials_csv(&c);
    let lines: Vec<&str> = fr.trim_start_matches('\u{feff}').lines().collect();
    assert!(fr.starts_with('\u{feff}'), "BOM pour les tableurs");
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("Identifiant;Date (UTC);Opérateur"));
    // Du plus ancien au plus récent, valeurs à la française, validation exportée.
    assert!(
        lines[1].starts_with("essai-1;2026-10-22T09:00:00Z;Camille;"),
        "{}",
        lines[1]
    );
    assert!(
        lines[1].contains(";15000;4000;400;2,50;Bon;"),
        "{}",
        lines[1]
    );
    assert!(
        lines[1].contains("2026-10-24T08:00:00Z;Stéphane"),
        "{}",
        lines[1]
    );
    // Champ contenant le séparateur et des guillemets : protégé.
    assert!(
        lines[2].contains("\"Ça fume; bord \"\"noirci\"\"\""),
        "{}",
        lines[2]
    );
    assert!(lines[2].contains(";À ajuster;"), "{}", lines[2]);
    assert!(
        lines[1].contains(";Neuve;"),
        "état de la fraise : {}",
        lines[1]
    );
    assert_eq!(
        lines[1].split(';').count(),
        lines[0].split(';').count(),
        "autant de colonnes que d'en-têtes"
    );

    let mut en_catalog = c.clone();
    en_catalog.lang = Lang::En;
    let en = fabspeed_core::export::trials_csv(&en_catalog);
    let en_lines: Vec<&str> = en.trim_start_matches('\u{feff}').lines().collect();
    assert!(en_lines[0].starts_with("Id,Date (UTC),Operator"));
    assert!(
        en_lines[1].contains(",15000,4000,400,2.50,Good,"),
        "{}",
        en_lines[1]
    );
}
