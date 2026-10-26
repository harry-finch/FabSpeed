use crate::{catalogue::*, source::SourceIdentity, AppState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use fabspeed_core::{Catalog, Lang, Trial};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::{fs, time::Instant};
use tower::ServiceExt;
const DATE: &str = "2026-01-01T12:00:00Z";
fn source() -> SourceIdentity {
    SourceIdentity {
        version: 1,
        catalogue_id: "fs-0123456789abcdef0123456789abcdef".into(),
        generation: 0,
        publication_initialized: false,
    }
}
fn fixture() -> (Catalog, Consent) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let mut c = crate::loader::load_catalog(&root, Lang::Fr).unwrap().0;
    c.machines.retain(|m| m.id == "generic-hobby-router");
    c.tools.retain(|t| t.id == "generic/flat-6-1f-up");
    c.materials.retain(|m| m.id == "material/wood.plywood");
    let input=serde_json::from_value(json!({"machine_id":c.machines[0].id,"material_id":c.materials[0].id,"tool_id":c.tools[0].id,"operation":"pocket","aggressiveness":"normal","workholding":"screws_clamps","stepover_ratio":0.4,"thickness_mm":12.5,"depth_mm":4.0,"stickout_mm":20.0,"tool_condition":"new","rpm":12000.5,"feed_mm_min":1200.25,"plunge_mm_min":300.125,"doc_mm":1.25,"outcome":"good"})).unwrap();
    let mut trial = Trial::from_input(
        input,
        &c,
        "synthetic-trial".into(),
        DATE.into(),
        "synthetic",
    )
    .unwrap();
    trial
        .validate(DATE.into(), "PRIVATE_VALIDATOR", "PRIVATE_NOTE", Lang::Fr)
        .unwrap();
    trial.validation.as_mut().unwrap().binding = Some(capture(&c, &trial, &source()).unwrap());
    let consent = Consent {
        enabled: true,
        machines: [c.machines[0].id.clone()].into(),
        tools: [c.tools[0].id.clone()].into(),
        materials: [c.materials[0].id.clone()].into(),
        trials: [trial.id.clone()].into(),
    };
    c.trials = vec![trial];
    (c, consent)
}
async fn call(s: &AppState, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let token = s.auth().test_session().await;
    let r = crate::app(s.clone())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(format!("/api/v1{path}"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = r.status();
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[test]
fn catalogue_projection_privacy_eligibility_and_revisions() {
    let (mut c, mut consent) = fixture();
    let src = source();
    let before = project(&c, &src, &consent).unwrap().0.revision().unwrap();
    c.trials[0].operator = "PRIVATE_OPERATOR".into();
    c.trials[0].notes = "PRIVATE_TRIAL_LABEL".into();
    c.trials[0].tool_name = "PRIVATE_TOOL_SNAPSHOT".into();
    c.tools[0].notes = vec!["PRIVATE_TOOL_NOTES".into()];
    c.tools[0].url = Some("https://PRIVATE_SUPPLIER_URL".into());
    c.tools[0].fabcam_identity = Some(fabspeed_core::model::Identity {
        id: "PRIVATE_ALIAS".into(),
        revision: "PRIVATE_REV".into(),
    });
    c.machines[0].notes = vec!["PRIVATE_MACHINE".into()];
    c.machines[0].description = "PRIVATE_LOCAL_PATH".into();
    c.materials[0].source = "PRIVATE_SOURCE".into();
    c.materials[0].tips = vec!["PRIVATE_TIPS".into()];
    c.meta.description = "PRIVATE_POLICY_METADATA".into();
    let (p, reasons) = project(&c, &src, &consent).unwrap();
    assert!(reasons.is_empty());
    assert_eq!(p.revision().unwrap(), before);
    let public = p.assemble(1, DATE).unwrap();
    assert!(!public.to_string().contains("PRIVATE_"));
    assert_eq!(public["presets"][0]["values"]["feed_mm_min"], 1200.25);
    assert_eq!(
        project(&c, &src, &Consent::default())
            .unwrap()
            .0
            .assemble(1, DATE)
            .unwrap()["tools"],
        json!([])
    );
    consent.tools.clear();
    assert!(project(&c, &src, &consent).unwrap().0.presets.is_empty());
    let (_, consent) = fixture();
    for kind in 0..6 {
        let mut changed = c.clone();
        match kind {
            0 => changed.machines[0].rpm_max += 1.0,
            1 => changed.tools[0].diameter_mm += 0.1,
            2 => {
                changed.materials[0].supervision_required =
                    !changed.materials[0].supervision_required
            }
            3 => changed.trials[0].feed_mm_min += 0.1,
            4 => changed.trials[0].workholding = fabspeed_core::model::Workholding::Vacuum,
            _ => changed.trials[0].validation = None,
        };
        assert!(project(&changed, &src, &consent)
            .unwrap()
            .0
            .presets
            .is_empty());
    }
    let mut old = c.clone();
    old.trials[0].validation.as_mut().unwrap().binding = None;
    assert!(project(&old, &src, &consent).unwrap().0.presets.is_empty());
    old.trials[0].validation = None;
    assert!(project(&old, &src, &consent).unwrap().0.presets.is_empty());
    let mut other = src.clone();
    other.catalogue_id = "fs-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
    assert!(project(&c, &other, &consent).unwrap().0.presets.is_empty());
    let mut missing = c.clone();
    missing.machines.clear();
    assert!(project(&missing, &src, &consent)
        .unwrap()
        .0
        .presets
        .is_empty());
}
#[test]
fn catalogue_invalid_values_are_refused() {
    let (c, consent) = fixture();
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0, -1.0] {
        let mut x = c.clone();
        x.tools[0].shank_mm = Some(n);
        assert!(project(&x, &source(), &consent).is_err());
        let mut x = c.clone();
        x.machines[0].work_area_mm = Some([n, 1.0, 1.0]);
        assert!(project(&x, &source(), &consent).is_err());
        let mut x = c.clone();
        x.trials[0].stickout_mm = Some(n);
        assert!(project(&x, &source(), &consent).is_err());
    }
    let mut x = c.clone();
    x.tools.push(x.tools[0].clone());
    assert!(project(&x, &source(), &consent).is_err());
    let mut x = c.clone();
    x.trials[0].validation.as_mut().unwrap().validated_at = "PRIVATE_NOT_DATE".into();
    assert_eq!(
        project(&x, &source(), &consent).unwrap_err(),
        "invalid shared timestamp"
    );
    let mut advanced = source();
    advanced.generation = 17;
    let advanced_projection = project(&c, &advanced, &consent).unwrap().0;
    assert!(advanced_projection.assemble(17, DATE).is_err());
    assert!(advanced_projection.assemble(18, DATE).is_ok());
    let p = project(&c, &source(), &consent).unwrap().0;
    assert!(p.assemble(0, DATE).is_err());
    assert!(p.assemble(1, "bad").is_err());
}
#[test]
fn catalogue_vectors_and_synthetic_fixture() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/fabapps/catalogue/v1/canonical-vectors.json");
    let vectors: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    for v in vectors.as_array().unwrap() {
        assert_eq!(
            crate::canonical::digest(v["domain"].as_str().unwrap(), &v["value"]).unwrap(),
            v["revision"].as_str().unwrap()
        );
    }
    let (c, consent) = fixture();
    let p = project(&c, &source(), &consent).unwrap().0;
    let one = p.assemble(1, DATE).unwrap();
    let two = p.assemble(2, "2026-02-01T12:00:00Z").unwrap();
    assert_eq!(one["revision"], two["revision"]);
    if let Ok(dir) = std::env::var("FABSPEED_CONTRACT_FIXTURES") {
        fs::write(
            std::path::Path::new(&dir).join("fabapps-catalogue-v1.synthetic.json"),
            serde_json::to_string_pretty(&one).unwrap(),
        )
        .unwrap();
    } else {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/fabapps/catalogue/v1/fabapps-catalogue-v1.synthetic.json");
        let saved: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(one, saved);
    }
}
#[tokio::test]
async fn catalogue_binding_persistence_restore_and_revocation() {
    let (tmp, s) = crate::state::test_state();
    let original = s.source().clone();
    let (c, _) = fixture();
    let mut trial = c.trials[0].clone();
    trial.validation = None;
    {
        let _g = s.write_lock().lock().await;
        s.create_entry_locked(
            crate::lab::Kind::Trial,
            &trial.id,
            fabspeed_core::toml_data::trial_document(&trial),
            Lang::Fr,
        )
        .await
        .unwrap();
    }
    let uri = format!("/trials/{}/validation", trial.id);
    assert_eq!(call(&s, "POST", &uri, json!({})).await.0, StatusCode::OK);
    let bound = s.catalog(Lang::Fr).await.trials[0].clone();
    assert_eq!(
        bound
            .validation
            .as_ref()
            .unwrap()
            .binding
            .as_ref()
            .unwrap()
            .catalogue_id,
        original.catalogue_id
    );
    let (status, backup) = call(&s, "GET", "/lab/backup", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!backup.to_string().contains(crate::source::FILE));
    assert_eq!(call(&s, "DELETE", &uri, json!({})).await.0, StatusCode::OK);
    assert_eq!(
        call(&s, "POST", "/lab/restore", backup.clone()).await.0,
        StatusCode::OK
    );
    assert_eq!(s.source(), original);
    assert_eq!(
        s.catalog(Lang::Fr).await.trials[0].validation,
        bound.validation
    );
    drop(s);
    let s = AppState::from_dir(tmp.path().to_path_buf()).unwrap();
    assert_eq!(s.source(), original);
    assert_eq!(
        s.catalog(Lang::Fr).await.trials[0].validation,
        bound.validation
    );
    assert!(SourceIdentity::initialize_independent_clone(tmp.path()).is_err());
    drop(s);
    let cloned = SourceIdentity::initialize_independent_clone(tmp.path()).unwrap();
    assert_ne!(cloned.catalogue_id, original.catalogue_id);
    assert_eq!(cloned.generation, 0);
    let (destination, destination_state) = crate::state::test_state();
    drop(destination_state);
    fs::write(
        destination.path().join(crate::source::FILE),
        serde_json::to_vec(&cloned).unwrap(),
    )
    .unwrap();
    let s = AppState::from_dir(destination.path().to_path_buf()).unwrap();
    assert_eq!(
        call(&s, "POST", "/lab/restore", backup).await.0,
        StatusCode::OK
    );
    assert_eq!(s.source(), cloned);
}
#[test]
fn catalogue_scale_measurement() {
    for n in [10, 1000, 10000] {
        let (mut c, mut consent) = fixture();
        c.trials.clear();
        consent.trials.clear();
        let template = c.tools[0].clone();
        c.tools.clear();
        consent.tools.clear();
        for i in 0..n {
            let mut t = template.clone();
            t.id = format!("synthetic/{i}");
            consent.tools.insert(t.id.clone());
            c.tools.push(t);
        }
        let start = Instant::now();
        let p = project(&c, &source(), &consent).unwrap().0;
        let projected = start.elapsed();
        let start = Instant::now();
        let json = serde_json::to_vec(&p.assemble(1, DATE).unwrap()).unwrap();
        println!(
            "CATALOGUE_SCALE tools={n} projection_us={} assembly_serialization_us={} bytes={}",
            projected.as_micros(),
            start.elapsed().as_micros(),
            json.len()
        );
    }
}

#[tokio::test]
async fn catalogue_capture_waits_for_coherent_mutation_and_failure_preserves_trial() {
    let (_tmp, s) = crate::state::test_state();
    let (c, _) = fixture();
    let mut t = c.trials[0].clone();
    t.validation = None;
    {
        let _guard = s.write_lock().lock().await;
        s.create_entry_locked(
            crate::lab::Kind::Trial,
            &t.id,
            fabspeed_core::toml_data::trial_document(&t),
            Lang::Fr,
        )
        .await
        .unwrap();
    }
    let guard = s.write_lock().lock().await;
    let other = s.clone();
    let id = t.id.clone();
    let request = tokio::spawn(async move {
        call(
            &other,
            "POST",
            &format!("/trials/{id}/validation"),
            json!({}),
        )
        .await
    });
    tokio::task::yield_now().await;
    assert!(!request.is_finished());
    let catalog = s.catalog(Lang::Fr).await;
    let mut tool = catalog
        .tools
        .iter()
        .find(|r| r.id == t.tool_id)
        .unwrap()
        .clone();
    tool.diameter_mm += 0.125;
    s.write_entry_locked(
        crate::lab::Kind::Tool,
        &tool.id,
        fabspeed_core::toml_data::tool_document(&tool),
        Lang::Fr,
    )
    .await
    .unwrap();
    drop(guard);
    assert_eq!(request.await.unwrap().0, StatusCode::OK);
    let catalog = s.catalog(Lang::Fr).await;
    let recorded = &catalog.trials[0];
    assert_eq!(
        recorded
            .validation
            .as_ref()
            .unwrap()
            .binding
            .as_ref()
            .unwrap(),
        &capture(&catalog, recorded, &s.source()).unwrap()
    );
    // Failed ownership verification must not replace the original trial bytes.
    let path = s
        .data_dir()
        .unwrap()
        .join("lab/trials")
        .join(crate::lab::file_name(&t.id));
    let bytes = fs::read(&path).unwrap();
    let _guard = s.write_lock().lock().await;
    let result = s
        .write_entry_locked(
            crate::lab::Kind::Trial,
            "other-id",
            String::from_utf8(bytes.clone()).unwrap(),
            Lang::Fr,
        )
        .await;
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn catalogue_source_move_generation_corruption_and_readonly_diagnostic() {
    let (tmp, s) = crate::state::test_state();
    let mut source = s.source().clone();
    drop(s);
    source.generation = 17;
    fs::write(
        tmp.path().join(crate::source::FILE),
        serde_json::to_vec(&source).unwrap(),
    )
    .unwrap();
    let parent = tempfile::tempdir().unwrap();
    let moved = parent.path().join("moved");
    fs::rename(tmp.path(), &moved).unwrap();
    let s = AppState::from_dir(moved.clone()).unwrap();
    assert_eq!(s.source(), source);
    drop(s);
    let saved = fs::read(moved.join(crate::source::FILE)).unwrap();
    crate::identity::diagnostics(&moved).unwrap();
    assert_eq!(fs::read(moved.join(crate::source::FILE)).unwrap(), saved);
    fs::remove_file(moved.join(crate::source::FILE)).unwrap();
    crate::identity::diagnostics(&moved).unwrap();
    assert!(!moved.join(crate::source::FILE).exists());
    let s = AppState::from_dir(moved.clone()).unwrap();
    assert_ne!(s.source().catalogue_id, source.catalogue_id);
    drop(s);
    fs::write(moved.join(crate::source::FILE), b"invalid metadata").unwrap();
    assert!(AppState::from_dir(moved.clone()).is_err());
    assert_eq!(
        fs::read(moved.join(crate::source::FILE)).unwrap(),
        b"invalid metadata"
    );
    #[cfg(unix)]
    {
        fs::remove_file(moved.join(crate::source::FILE)).unwrap();
        let victim = parent.path().join("victim");
        fs::write(&victim, b"untouched").unwrap();
        std::os::unix::fs::symlink(&victim, moved.join(crate::source::FILE)).unwrap();
        assert!(SourceIdentity::initialize_independent_clone(&moved).is_err());
        assert_eq!(fs::read(victim).unwrap(), b"untouched");
    }
}

#[tokio::test]
async fn catalogue_private_sidecars_and_upgrade_do_not_change_public_identity() {
    let (tmp, s) = crate::state::test_state();
    let token = s.auth().setup("PRIVATE_AUTH_PASSWORD").await.unwrap();
    fs::write(tmp.path().join("internal.log"), b"PRIVATE_INTERNAL_LOG").unwrap();
    let c = s.catalog(Lang::Fr).await;
    let consent = Consent {
        enabled: true,
        machines: c.machines.iter().map(|m| m.id.clone()).collect(),
        tools: c.tools.iter().map(|m| m.id.clone()).collect(),
        materials: c.materials.iter().map(|m| m.id.clone()).collect(),
        trials: Default::default(),
    };
    let (p, reasons) = project(&c, &s.source(), &consent).unwrap();
    let output = p.assemble(1, DATE).unwrap().to_string();
    assert!(!output.contains("PRIVATE_") && !output.contains(&token));
    assert!(reasons.is_empty());
    let source = s.source().clone();
    let bytes = fs::read(tmp.path().join(crate::source::FILE)).unwrap();
    drop(s);
    let meta = tmp.path().join("dataset.toml");
    let mut value: toml::Table = toml::from_str(&fs::read_to_string(&meta).unwrap()).unwrap();
    value.insert(
        "revision".into(),
        toml::Value::String("synthetic-old".into()),
    );
    fs::write(meta, toml::to_string(&value).unwrap()).unwrap();
    let bundled = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    assert!(matches!(
        crate::install::install_base(&bundled, tmp.path()).unwrap(),
        crate::install::InstallOutcome::Updated { .. }
    ));
    assert_eq!(
        fs::read(tmp.path().join(crate::source::FILE)).unwrap(),
        bytes
    );
    let s = AppState::from_dir(tmp.path().to_path_buf()).unwrap();
    assert_eq!(s.source(), source);
}
