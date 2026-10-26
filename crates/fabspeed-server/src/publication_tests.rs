//! Synthetic local filesystem qualification. Never touches the repository lab.
use super::{
    publication::{self, Settings},
    AppState,
};
use crate::catalogue::Consent;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use fabspeed_core::Lang;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::{fs, path::PathBuf, time::Instant};
use tower::ServiceExt;
const MACHINE: &str = "generic-hobby-router";
const TOOL: &str = "generic/flat-6-1f-up";
const MATERIAL: &str = "material/wood.plywood";
fn setup() -> (tempfile::TempDir, tempfile::TempDir, AppState, Settings) {
    let (data, state) = crate::state::test_state();
    let exchange = tempfile::tempdir().unwrap();
    let path = fs::canonicalize(exchange.path())
        .unwrap()
        .join(crate::publication_fs::NAME);
    let settings = Settings {
        destination: Some(path),
        consent: Consent {
            enabled: true,
            machines: [MACHINE.into()].into(),
            tools: [TOOL.into()].into(),
            materials: [MATERIAL.into()].into(),
            ..Consent::default()
        },
    };
    (data, exchange, state, settings)
}
fn read(settings: &Settings) -> Value {
    serde_json::from_slice(&fs::read(settings.destination.as_ref().unwrap()).unwrap()).unwrap()
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
    let code = r.status();
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    (
        code,
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes))),
    )
}
async fn valid_trial(s: &AppState) -> String {
    let input = json!({"machine_id":MACHINE,"material_id":MATERIAL,"tool_id":TOOL,"operation":"profile","aggressiveness":"normal","workholding":"screws_clamps","stepover_ratio":1,"rpm":12000.5,"feed_mm_min":1200.25,"plunge_mm_min":300.125,"doc_mm":1.25,"outcome":"good","operator":"SECRET_OPERATOR","notes":"SECRET_NOTES"});
    let (code, v) = call(s, "POST", "/trials", input).await;
    assert!(code.is_success(), "{v}");
    let id = v["trial"]["id"].as_str().unwrap().to_string();
    let (code, v) = call(
        s,
        "POST",
        &format!("/trials/{id}/validation"),
        json!({"validated_by":"SECRET_VALIDATOR","note":"SECRET_VALIDATION_NOTE"}),
    )
    .await;
    assert!(code.is_success(), "{v}");
    id
}
fn evidence(name: &str, value: &Value) {
    if let Some(dir) = std::env::var_os("FABSPEED_PUBLICATION_EVIDENCE") {
        let dir = PathBuf::from(dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(name), serde_json::to_vec_pretty(value).unwrap()).unwrap();
    }
}
#[tokio::test]
async fn publication_default_auth_consent_and_real_byte_replay() {
    let (_data, _exchange, s, mut settings) = setup();
    assert!(!s.publication_status().settings.consent.enabled);
    assert!(s.publication_status().settings.consent.machines.is_empty());
    s.publish_once().await;
    assert!(!settings.destination.as_ref().unwrap().exists());
    for (method, path) in [
        ("GET", "/fabapps/sharing"),
        ("POST", "/fabapps/sharing"),
        ("POST", "/fabapps/sharing/preview"),
        ("POST", "/fabapps/sharing/publish"),
    ] {
        let r = crate::app(s.clone())
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(format!("/api/v1{path}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    }
    let trial = valid_trial(&s).await;
    settings.consent.trials.insert(trial.clone());
    s.configure_publication(settings.clone()).await.unwrap();
    let (_, preview) = call(
        &s,
        "POST",
        "/fabapps/sharing/preview",
        serde_json::to_value(&settings).unwrap(),
    )
    .await;
    assert_eq!(s.source().generation, 0);
    assert!(!settings.destination.as_ref().unwrap().exists());
    assert!(!preview.to_string().contains("SECRET_"));
    s.publish_once().await;
    assert_eq!(s.publication_status().phase, "published");
    let first = read(&settings);
    publication::validate(&first).unwrap();
    evidence("01-valid.json", &first);
    assert_eq!(first["presets"][0]["values"]["feed_mm_min"], 1200.25);
    assert_eq!(first["presets"].as_array().unwrap().len(), 1);
    assert!(!first.to_string().contains("SECRET_"));
    let c = s.catalog(Lang::Fr).await;
    let mut machine =
        serde_json::to_value(c.machines.iter().find(|m| m.id == MACHINE).unwrap()).unwrap();
    machine["name"] = json!("Renamed shared machine");
    assert!(call(&s, "PUT", &format!("/machines/{MACHINE}"), machine)
        .await
        .0
        .is_success());
    s.publish_once().await;
    let second = read(&settings);
    assert_eq!(second["generation"], 2);
    assert_ne!(first["revision"], second["revision"]);
    evidence("02-modified.json", &second);
    assert!(call(
        &s,
        "DELETE",
        &format!("/trials/{trial}/validation"),
        Value::Null
    )
    .await
    .0
    .is_success());
    // Deliberate republish ensures a newer full state even if prior binding was already invalidated.
    s.request_publication().await.unwrap();
    s.publish_once().await;
    let third = read(&settings);
    assert_eq!(third["generation"], 3);
    assert!(third["presets"].as_array().unwrap().is_empty());
    evidence("03-revoked.json", &third);
    settings.consent.enabled = false;
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    let fourth = read(&settings);
    assert_eq!(fourth["generation"], 4);
    assert_eq!(fourth["catalogue_id"], first["catalogue_id"]);
    for kind in ["machines", "tools", "materials", "presets"] {
        assert!(fourth[kind].as_array().unwrap().is_empty());
    }
    assert_eq!(s.publication_status().phase, "withdrawn");
    evidence("04-disabled.json", &fourth);
}
#[tokio::test]
async fn publication_dependencies_explicit_archives_and_no_future_approvals() {
    let (_data, _exchange, s, mut settings) = setup();
    let trial = valid_trial(&s).await;
    settings.consent.trials.insert(trial);
    settings.consent.tools.clear();
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    assert!(read(&settings)["presets"].as_array().unwrap().is_empty());
    assert!(read(&settings)["tools"].as_array().unwrap().is_empty());
    settings.consent.tools.insert(TOOL.into());
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    assert_eq!(read(&settings)["presets"].as_array().unwrap().len(), 1);
    settings.consent.machines.insert("nonexistent".into());
    assert!(s.configure_publication(settings.clone()).await.is_err());
    settings.consent.machines.remove("nonexistent");
    let mut t = serde_json::to_value(&s.catalog(Lang::Fr).await.tools[0]).unwrap();
    t["id"] = json!("synthetic/share-tool");
    t["reference"] = json!("");
    assert!(call(&s, "POST", "/tools", t).await.0.is_success());
    s.publish_once().await;
    assert_eq!(read(&settings)["tools"].as_array().unwrap().len(), 1);
    settings.consent.tools.insert("synthetic/share-tool".into());
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    assert_eq!(read(&settings)["tools"].as_array().unwrap().len(), 2);
    assert!(
        call(&s, "DELETE", "/tools/synthetic/share-tool", Value::Null)
            .await
            .0
            .is_success()
    );
    s.publish_once().await;
    assert_eq!(read(&settings)["tools"].as_array().unwrap().len(), 1);
    settings.consent.tools.clear();
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    assert!(read(&settings)["presets"].as_array().unwrap().is_empty());
}
#[tokio::test]
async fn publication_restart_idempotence_and_explicit_republish() {
    let (data, _exchange, s, settings) = setup();
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    let original = read(&settings);
    drop(s);
    let s = AppState::from_dir(data.path().into()).unwrap();
    s.publish_once().await;
    assert_eq!(read(&settings), original);
    assert_eq!(
        s.publication_status().settings.consent.tools,
        settings.consent.tools
    );
    s.request_publication().await.unwrap();
    s.publish_once().await;
    let newer = read(&settings);
    assert_eq!(newer["revision"], original["revision"]);
    assert_eq!(newer["generation"], 2);
    evidence("05-republish-before.json", &original);
    evidence("06-republish-after.json", &newer);
    let mut disabled = settings.clone();
    disabled.consent.enabled = false;
    s.configure_publication(disabled).await.unwrap();
    s.publish_once().await;
    let empty = read(&settings);
    drop(s);
    let s = AppState::from_dir(data.path().into()).unwrap();
    s.publish_once().await;
    assert_eq!(read(&settings), empty);
    assert!(!s.publication_status().settings.consent.enabled);
}
#[tokio::test]
async fn publication_fault_matrix_preserves_last_and_burns_reservations() {
    for stage in 1..=6 {
        let (data, _exchange, s, settings) = setup();
        s.configure_publication(settings.clone()).await.unwrap();
        s.publish_once().await;
        let good = fs::read(settings.destination.as_ref().unwrap()).unwrap();
        s.request_publication().await.unwrap();
        s.publication.inject(stage);
        s.publish_once().await;
        assert_ne!(s.publication_status().phase, "published");
        if stage < 5 {
            assert_eq!(
                fs::read(settings.destination.as_ref().unwrap()).unwrap(),
                good
            );
        }
        drop(s);
        let s = AppState::from_dir(data.path().into()).unwrap();
        s.publish_once().await;
        assert_eq!(s.publication_status().phase, "published", "stage {stage}");
        let current = read(&settings);
        assert_eq!(current["generation"], if stage < 5 { 3 } else { 2 });
        assert_eq!(
            s.source().generation,
            current["generation"].as_u64().unwrap()
        );
        assert!(
            fs::read_dir(settings.destination.as_ref().unwrap().parent().unwrap())
                .unwrap()
                .all(|p| !p
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".fabapps-publish-"))
        );
    }
}
#[tokio::test]
async fn publication_crash_child() {
    let Some(path) = std::env::var_os("FABSPEED_PUBLICATION_CRASH_CHILD") else {
        return;
    };
    let s = AppState::from_dir(path.into()).unwrap();
    s.publication.inject(
        std::env::var("FABSPEED_PUBLICATION_CRASH_STAGE")
            .unwrap()
            .parse()
            .unwrap(),
    );
    s.publish_once().await;
    panic!("child did not stop at crash point");
}
#[tokio::test]
async fn publication_process_exit_recovery() {
    for stage in [1, 2, 5, 6] {
        let (data, _exchange, s, settings) = setup();
        s.configure_publication(settings.clone()).await.unwrap();
        drop(s);
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "publication_tests::publication_crash_child",
                "--nocapture",
            ])
            .env("FABSPEED_PUBLICATION_CRASH_CHILD", data.path())
            .env("FABSPEED_PUBLICATION_CRASH_STAGE", stage.to_string())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(86),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let s = AppState::from_dir(data.path().into()).unwrap();
        s.publish_once().await;
        assert_eq!(s.publication_status().phase, "published", "stage {stage}");
        assert_eq!(read(&settings)["generation"], if stage < 5 { 2 } else { 1 });
    }
}
#[tokio::test]
async fn publication_unknown_corrupt_old_and_control_states_fail_closed() {
    for mode in 0..6 {
        let (data, _exchange, s, settings) = setup();
        s.configure_publication(settings.clone()).await.unwrap();
        s.publish_once().await;
        let first = fs::read(settings.destination.as_ref().unwrap()).unwrap();
        s.request_publication().await.unwrap();
        s.publish_once().await;
        drop(s);
        let path = settings.destination.as_ref().unwrap();
        match mode {
            0 => fs::write(path, b"truncated {").unwrap(),
            1 => fs::write(path, &first).unwrap(),
            2 => {
                let mut v = read(&settings);
                v["catalogue_id"] = json!("fs-00000000000000000000000000000000");
                fs::write(path, v.to_string()).unwrap();
            }
            3 => {
                let mut v = read(&settings);
                v["machines"][0]["name"] = json!("tampered");
                fs::write(path, v.to_string()).unwrap();
            }
            4 => fs::write(data.path().join(publication::FILE), b"corrupt").unwrap(),
            _ => fs::remove_file(data.path().join(publication::FILE)).unwrap(),
        }
        let before = fs::read(path).unwrap();
        let s = AppState::from_dir(data.path().into()).unwrap();
        s.publish_once().await;
        assert_eq!(s.publication_status().phase, "unsafe");
        assert_eq!(fs::read(path).unwrap(), before);
        assert_eq!(s.source().generation, 2);
        assert!(s.request_publication().await.is_err() || mode < 4);
    }
}
#[tokio::test]
async fn publication_destination_ownership_symlinks_unrelated_files_and_leases() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let (data, exchange, s, settings) = setup();
    let path = settings.destination.as_ref().unwrap();
    fs::write(path, b"unrelated").unwrap();
    assert!(s.configure_publication(settings.clone()).await.is_err());
    assert_eq!(fs::read(path).unwrap(), b"unrelated");
    fs::remove_file(path).unwrap();
    let target = exchange.path().join("do-not-touch");
    fs::write(&target, b"private").unwrap();
    symlink(&target, path).unwrap();
    assert!(s.configure_publication(settings.clone()).await.is_err());
    fs::remove_file(path).unwrap();
    fs::hard_link(&target, path).unwrap();
    assert!(s.configure_publication(settings.clone()).await.is_err());
    fs::remove_file(path).unwrap();
    let mut bad = settings.clone();
    bad.destination = Some(
        fs::canonicalize(data.path())
            .unwrap()
            .join(crate::publication_fs::NAME),
    );
    assert!(s.configure_publication(bad).await.is_err());
    fs::set_permissions(exchange.path(), fs::Permissions::from_mode(0o777)).unwrap();
    assert!(s.configure_publication(settings.clone()).await.is_err());
    fs::set_permissions(exchange.path(), fs::Permissions::from_mode(0o700)).unwrap();
    s.configure_publication(settings.clone()).await.unwrap();
    let (_data2, s2) = crate::state::test_state();
    s2.configure_publication(settings.clone()).await.unwrap();
    let lease = crate::publication_fs::lease(path).unwrap();
    s.publish_once().await;
    assert_eq!(
        s.publication_status().error.as_deref(),
        Some("destination_busy")
    );
    drop(lease);
    s.publish_once().await;
    let bytes = fs::read(path).unwrap();
    s2.publish_once().await;
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert_eq!(s2.publication_status().phase, "unsafe");
    assert!(AppState::from_dir(data.path().into()).is_err());
}
#[tokio::test]
async fn publication_disable_failure_blocks_known_file_then_empty_retry() {
    use std::os::unix::fs::PermissionsExt;
    for stage in [2, 5, 6] {
        let (data, _exchange, s, mut settings) = setup();
        s.configure_publication(settings.clone()).await.unwrap();
        s.publish_once().await;
        settings.consent.enabled = false;
        s.configure_publication(settings.clone()).await.unwrap();
        s.publication.inject(stage);
        s.publish_once().await;
        assert_eq!(s.publication_status().phase, "withdrawal_pending");
        assert!(!s.publication_status().settings.consent.enabled);
        assert!(s.publication_status().local_access_blocked);
        assert!(!s.publication_status().existing_may_be_readable);
        assert_eq!(
            fs::metadata(settings.destination.as_ref().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0
        );
        drop(s);
        let s = AppState::from_dir(data.path().into()).unwrap();
        s.publish_once().await;
        assert_eq!(
            s.publication_status().phase,
            "withdrawn",
            "stage {stage}: {:?}",
            s.publication_status().error
        );
        assert!(read(&settings)["machines"].as_array().unwrap().is_empty());
    }
    let (_data, exchange, s, mut settings) = setup();
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    fs::set_permissions(exchange.path(), fs::Permissions::from_mode(0o500)).unwrap();
    settings.consent.enabled = false;
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    assert_eq!(s.publication_status().phase, "withdrawal_pending");
    assert!(s.publication_status().local_access_blocked);
    fs::set_permissions(exchange.path(), fs::Permissions::from_mode(0o700)).unwrap();
    s.publish_once().await;
    assert_eq!(s.publication_status().phase, "withdrawn");
}
#[tokio::test]
async fn publication_auto_refresh_rollbacks_import_restore_and_racing_requests() {
    let (_data, _exchange, s, settings) = setup();
    s.configure_publication(settings.clone()).await.unwrap();
    let worker = s.start_publisher();
    wait_phase(&s, "published").await;
    let generation = s.source().generation;
    let (status, _) = call(&s, "PUT", &format!("/tools/{TOOL}"), json!({"id":"wrong"})).await;
    assert!(!status.is_success());
    s.publish_once().await;
    assert_eq!(s.source().generation, generation);
    let mut tool = serde_json::to_value(
        s.catalog(Lang::Fr)
            .await
            .tools
            .iter()
            .find(|t| t.id == TOOL)
            .unwrap(),
    )
    .unwrap();
    tool["name"] = json!("Modified by successful transaction");
    assert!(call(&s, "PUT", &format!("/tools/{TOOL}"), tool)
        .await
        .0
        .is_success());
    wait_generation(&s, generation + 1).await;
    let (_, backup) = call(&s, "GET", "/lab/backup", Value::Null).await;
    let before = fs::read(settings.destination.as_ref().unwrap()).unwrap();
    assert!(call(&s, "POST", "/lab/restore", backup)
        .await
        .0
        .is_success());
    s.publish_once().await;
    assert_eq!(s.publication_status().phase, "withdrawn");
    assert!(s.publication_status().settings.consent.tools.is_empty());
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    let restored = read(&settings);
    assert_eq!(
        restored["revision"],
        serde_json::from_slice::<Value>(&before).unwrap()["revision"]
    );
    let before = fs::read(settings.destination.as_ref().unwrap()).unwrap();
    let (_, library) = call(&s, "GET", "/library/export", Value::Null).await;
    assert!(call(
        &s,
        "POST",
        "/library/import",
        json!({"content":library.to_string()})
    )
    .await
    .0
    .is_success());
    s.publish_once().await;
    assert_eq!(
        fs::read(settings.destination.as_ref().unwrap()).unwrap(),
        before
    );
    let mut jobs = vec![];
    for _ in 0..32 {
        let state = s.clone();
        jobs.push(tokio::spawn(async move {
            state.request_publication().await.unwrap();
            state.publish_once().await;
        }));
    }
    for job in jobs {
        job.await.unwrap();
    }
    let mut disabled = settings.clone();
    disabled.consent.enabled = false;
    s.configure_publication(disabled).await.unwrap();
    wait_phase(&s, "withdrawn").await;
    for _ in 0..3 {
        s.publish_once().await;
    }
    assert!(read(&settings)["machines"].as_array().unwrap().is_empty());
    worker.abort();
}
async fn wait_phase(s: &AppState, phase: &str) {
    for _ in 0..200 {
        if s.publication_status().phase == phase {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!(
        "expected {phase}; got {} {:?}",
        s.publication_status().phase,
        s.publication_status().error
    );
}
async fn wait_generation(s: &AppState, min: u64) {
    for _ in 0..200 {
        if s.source().generation >= min {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("refresh timed out");
}
#[tokio::test]
async fn publication_destination_move_requires_empty_withdrawal_and_clone_resets_consent() {
    let (data, _exchange, s, mut settings) = setup();
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    let other = tempfile::tempdir().unwrap();
    let mut moved = settings.clone();
    moved.destination = Some(
        fs::canonicalize(other.path())
            .unwrap()
            .join(crate::publication_fs::NAME),
    );
    assert!(s.configure_publication(moved.clone()).await.is_err());
    settings.consent.enabled = false;
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    s.configure_publication(moved.clone()).await.unwrap();
    s.publish_once().await;
    assert_eq!(read(&moved)["generation"], 3);
    drop(s);
    let original = fs::read(moved.destination.as_ref().unwrap()).unwrap();
    crate::source::SourceIdentity::initialize_independent_clone(data.path()).unwrap();
    let s = AppState::from_dir(data.path().into()).unwrap();
    s.publish_once().await;
    assert!(!s.publication_status().settings.consent.enabled);
    assert!(s.publication_status().settings.consent.tools.is_empty());
    assert_eq!(fs::read(moved.destination.unwrap()).unwrap(), original);
}
#[tokio::test]
async fn publication_bounded_measurements() {
    // One sample per size; no claim of statistical or slow-disk performance.
    for size in [10, 1000, 10000] {
        let (data, _exchange, s, mut settings) = setup();
        let mut c = (*s.catalog(Lang::Fr).await).clone();
        let template = c.tools[0].clone();
        c.tools = (0..size)
            .map(|i| {
                let mut t = template.clone();
                t.id = format!("synthetic/{i:05}");
                t
            })
            .collect();
        settings.consent.tools = c.tools.iter().map(|t| t.id.clone()).collect();
        let start = Instant::now();
        let p = crate::catalogue::project(&c, &s.source(), &settings.consent)
            .unwrap()
            .0;
        let projection = start.elapsed();
        let value = p.assemble(1, "2026-01-01T00:00:00Z").unwrap();
        let start = Instant::now();
        publication::validate(&value).unwrap();
        let validation = start.elapsed();
        let start = Instant::now();
        let bytes = serde_json::to_vec_pretty(&value).unwrap();
        let serialization = start.elapsed();
        // Install a synthetic in-memory catalogue, never raw test data in the repository.
        s.replace_test_catalog(c.clone()).await;
        s.configure_publication(settings.clone()).await.unwrap();
        let start = Instant::now();
        s.publish_once().await;
        let total = start.elapsed();
        let atomic = s
            .publication
            .atomic_us
            .load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(s.publication_status().phase, "published");
        drop(s);
        let start = Instant::now();
        let s = AppState::from_dir(data.path().into()).unwrap();
        let startup = start.elapsed();
        s.replace_test_catalog(c).await;
        let start = Instant::now();
        s.publish_once().await;
        let recovery = start.elapsed();
        assert_eq!(read(&settings)["generation"], 1);
        println!("publication size={size} bytes={} projection_us={} validation_us={} serialization_us={} atomic_us={} total_us={} startup_us={} recovery_us={}",bytes.len(),projection.as_micros(),validation.as_micros(),serialization.as_micros(),atomic,total.as_micros(),startup.as_micros(),recovery.as_micros());
        drop(s);
    }
}

#[tokio::test]
async fn publication_private_bytes_and_changed_binding_never_escape() {
    let (_data, _exchange, s, mut settings) = setup();
    let id = valid_trial(&s).await;
    settings.consent.trials.insert(id);
    let mut c = (*s.catalog(Lang::Fr).await).clone();
    let machine = c.machines.iter_mut().find(|m| m.id == MACHINE).unwrap();
    machine.description = "SECRET_LOCAL_PATH /private/local".into();
    machine.notes = vec!["SECRET_MACHINE_NOTES".into()];
    let tool = c.tools.iter_mut().find(|t| t.id == TOOL).unwrap();
    tool.url = Some("https://SECRET_SUPPLIER_URL".into());
    tool.notes = vec!["SECRET_TOOL_NOTES".into()];
    c.meta.description = "SECRET_INTERNAL_POLICY".into();
    c.tools
        .iter_mut()
        .filter(|t| t.id != TOOL)
        .for_each(|t| t.name = "SECRET_UNAPPROVED".into());
    s.replace_test_catalog(c.clone()).await;
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    let good = read(&settings);
    assert!(!good.to_string().contains("SECRET_"));
    let (_, preview) = call(
        &s,
        "POST",
        "/fabapps/sharing/preview",
        serde_json::to_value(&settings).unwrap(),
    )
    .await;
    assert!(!preview.to_string().contains("SECRET_"));
    for kind in 0..6 {
        let mut changed = c.clone();
        match kind {
            0 => {
                changed
                    .tools
                    .iter_mut()
                    .find(|t| t.id == TOOL)
                    .unwrap()
                    .diameter_mm += 0.125
            }
            1 => {
                changed
                    .machines
                    .iter_mut()
                    .find(|m| m.id == MACHINE)
                    .unwrap()
                    .rpm_max += 1.0
            }
            2 => {
                let m = changed
                    .materials
                    .iter_mut()
                    .find(|m| m.id == MATERIAL)
                    .unwrap();
                m.supervision_required = !m.supervision_required;
            }
            3 => changed.trials[0].validation = None,
            4 => changed.trials[0].validation.as_mut().unwrap().binding = None,
            _ => changed.trials.clear(),
        }
        s.replace_test_catalog(changed).await;
        s.request_publication().await.unwrap();
        s.publish_once().await;
        assert!(
            read(&settings)["presets"].as_array().unwrap().is_empty(),
            "change {kind}"
        );
    }
    let previous = fs::read(settings.destination.as_ref().unwrap()).unwrap();
    c.tools
        .iter_mut()
        .find(|t| t.id == TOOL)
        .unwrap()
        .diameter_mm = f64::NAN;
    s.replace_test_catalog(c).await;
    s.request_publication().await.unwrap();
    s.publish_once().await;
    assert_eq!(s.publication_status().phase, "failed");
    assert_eq!(
        fs::read(settings.destination.as_ref().unwrap()).unwrap(),
        previous
    );
    assert!(!serde_json::to_string(&s.publication_status())
        .unwrap()
        .contains("SECRET_"));
}
#[tokio::test]
async fn publication_empty_enabled_and_latest_consent_wins_a_burst() {
    let (_data, _exchange, s, mut settings) = setup();
    settings.consent = Consent {
        enabled: true,
        ..Consent::default()
    };
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    assert!(read(&settings)["machines"].as_array().unwrap().is_empty());
    let worker = s.start_publisher();
    let start = Instant::now();
    let mut jobs = vec![];
    for i in 0..32 {
        let state = s.clone();
        let mut cfg = settings.clone();
        cfg.consent.enabled = i % 2 == 0;
        if i % 3 == 0 {
            cfg.consent.machines.insert(MACHINE.into());
        }
        jobs.push(tokio::spawn(async move {
            state.configure_publication(cfg).await.unwrap();
            state.publish_once().await;
        }));
    }
    for job in jobs {
        job.await.unwrap();
    }
    settings.consent.enabled = false;
    s.configure_publication(settings.clone()).await.unwrap();
    s.request_publication().await.unwrap();
    wait_phase(&s, "withdrawn").await;
    let bytes = fs::read(settings.destination.as_ref().unwrap()).unwrap();
    assert!(read(&settings)["machines"].as_array().unwrap().is_empty());
    s.publish_once().await;
    assert_eq!(
        fs::read(settings.destination.as_ref().unwrap()).unwrap(),
        bytes
    );
    println!(
        "publication mutation_burst_32_us={}",
        start.elapsed().as_micros()
    );
    worker.abort();
}
#[tokio::test]
async fn publication_control_write_failure_and_reservation_loss_do_not_reuse() {
    use std::os::unix::fs::PermissionsExt;
    let (data, _exchange, s, mut settings) = setup();
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    let good = fs::read(settings.destination.as_ref().unwrap()).unwrap();
    fs::set_permissions(data.path(), fs::Permissions::from_mode(0o500)).unwrap();
    settings.consent.enabled = false;
    assert!(s.configure_publication(settings.clone()).await.is_err());
    assert_eq!(s.publication_status().phase, "unsafe");
    s.publish_once().await;
    assert_eq!(
        fs::read(settings.destination.as_ref().unwrap()).unwrap(),
        good
    );
    fs::set_permissions(data.path(), fs::Permissions::from_mode(0o700)).unwrap();
    drop(s);
    let s = AppState::from_dir(data.path().into()).unwrap();
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    assert_eq!(s.publication_status().phase, "withdrawn");
    drop(s);
    fs::remove_file(data.path().join(crate::source::FILE)).unwrap();
    assert!(AppState::from_dir(data.path().into()).is_err());
}

#[tokio::test]
async fn publication_readonly_file_is_preserved_and_retryable() {
    use std::os::unix::fs::PermissionsExt;
    let (_data, _exchange, s, settings) = setup();
    s.configure_publication(settings.clone()).await.unwrap();
    s.publish_once().await;
    let path = settings.destination.as_ref().unwrap();
    let before = fs::read(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o400)).unwrap();
    s.request_publication().await.unwrap();
    s.publish_once().await;
    assert_eq!(s.publication_status().phase, "failed");
    assert_eq!(fs::read(path).unwrap(), before);
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    s.publish_once().await;
    assert_eq!(s.publication_status().phase, "published");
}
