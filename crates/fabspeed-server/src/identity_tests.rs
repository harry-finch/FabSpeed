//! API-level regressions. All records and filesystem failures live in TempDir.
use crate::{identity, lab, AppState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use fabspeed_core::{toml_data::tool_document, Lang};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs, path::Path, time::Instant};
use tower::ServiceExt;

async fn call(s: &AppState, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    let token = s.auth().test_session().await;
    let req = Request::builder()
        .method(method)
        .uri(format!("/api/v1{path}"))
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(body.map_or(Body::empty(), |x| Body::from(x.to_string())))
        .unwrap();
    let response = crate::app(s.clone()).oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}
async fn tool(s: &AppState, id: &str) -> Value {
    let mut t = serde_json::to_value(&s.catalog(Lang::Fr).await.tools[0]).unwrap();
    t["id"] = json!(id);
    t["name"] = json!("Synthetic tool");
    t["reference"] = json!("");
    t
}
fn trial(tool: &str) -> Value {
    json!({
        "machine_id":"generic-hobby-router","material_id":"material/wood.plywood","tool_id":tool,
        "operation":"profile","aggressiveness":"normal","workholding":"screws_clamps",
        "stepover_ratio":1,"rpm":12000,"feed_mm_min":1200,"plunge_mm_min":300,"doc_mm":1,"outcome":"good"
    })
}
fn tree(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, p: &Path, out: &mut std::collections::BTreeMap<String, Vec<u8>>) {
        for e in fs::read_dir(p).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(root, &p, out)
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().to_string_lossy().into_owned(),
                    fs::read(p).unwrap(),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[tokio::test]
async fn identity_archive_restart_restore_and_no_historical_reuse() {
    let (tmp, s) = crate::state::test_state();
    let (status, t) = call(&s, "POST", "/tools", Some(tool(&s, "").await)).await;
    assert_eq!(status, StatusCode::CREATED);
    let id = t["tool"]["id"].as_str().unwrap();
    let (_, tr) = call(&s, "POST", "/trials", Some(trial(id))).await;
    let tid = tr["trial"]["id"].as_str().unwrap();
    let (_, backup) = call(&s, "GET", "/lab/backup", None).await;
    assert_eq!(
        call(&s, "DELETE", &format!("/tools/{id}"), None).await.0,
        StatusCode::OK
    );
    let archived = tree(&tmp.path().join("lab/archive"));
    let (status, new) = call(&s, "POST", "/tools", Some(tool(&s, "").await)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_ne!(new["tool"]["id"], id);
    assert_eq!(
        s.catalog(Lang::Fr)
            .await
            .trials
            .iter()
            .find(|t| t.id == tid)
            .unwrap()
            .tool_id,
        id
    );
    assert!(s.catalog(Lang::Fr).await.tested_presets().is_empty());
    assert_ne!(
        call(&s, "POST", "/tools", Some(tool(&s, id).await)).await.0,
        StatusCode::CREATED
    );
    assert_eq!(tree(&tmp.path().join("lab/archive")), archived);
    drop(s);
    let s = AppState::from_dir(tmp.path().to_path_buf()).unwrap();
    assert_ne!(
        call(&s, "POST", "/tools", Some(tool(&s, id).await)).await.0,
        StatusCode::CREATED
    );
    let (status, result) = call(&s, "POST", "/lab/restore", Some(backup)).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert!(s.catalog(Lang::Fr).await.tool(id).is_some());
    assert_eq!(s.catalog(Lang::Fr).await.tested_presets().len(), 1);
    // Restore does not erase the identities created after the backup.
    let newer = new["tool"]["id"].as_str().unwrap();
    assert_ne!(
        call(&s, "POST", "/tools", Some(tool(&s, newer).await))
            .await
            .0,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn identity_all_kinds_edit_copy_restart_and_base_override() {
    let (tmp, s) = crate::state::test_state();
    for (route, field, base) in [
        (
            "machines",
            "machine",
            serde_json::to_value(&s.catalog(Lang::Fr).await.machines[0]).unwrap(),
        ),
        (
            "materials",
            "material",
            serde_json::to_value(&s.catalog(Lang::Fr).await.materials[0]).unwrap(),
        ),
        ("tools", "tool", tool(&s, "").await),
    ] {
        let mut body = base;
        body["id"] = json!("");
        body["name"] = json!("Synthetic copy");
        if route == "materials" {
            body["aliases"] = json!([]);
        }
        let (status, out) = call(&s, "POST", &format!("/{route}"), Some(body.clone())).await;
        assert_eq!(status, StatusCode::CREATED, "{out}");
        let id = out[field]["id"].as_str().unwrap();
        let mut updated = out[field].clone();
        updated["name"] = json!("Renamed synthetic");
        let (status, out) = call(&s, "PUT", &format!("/{route}/{id}"), Some(updated)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(out[field]["id"], id);
        let (status, copy) = call(&s, "POST", &format!("/{route}"), Some(body.clone())).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_ne!(copy[field]["id"], id);
        assert_eq!(
            call(&s, "DELETE", &format!("/{route}/{id}"), None).await.0,
            StatusCode::OK
        );
        let (_, again) = call(&s, "POST", &format!("/{route}"), Some(body)).await;
        assert_ne!(again[field]["id"], id);
    }
    let before = s.catalog(Lang::Fr).await;
    drop(s);
    let s = AppState::from_dir(tmp.path().to_path_buf()).unwrap();
    let after = s.catalog(Lang::Fr).await;
    assert_eq!(before.machines, after.machines);
    assert_eq!(before.tools, after.tools);
    assert_eq!(before.materials, after.materials);
}

#[tokio::test]
async fn identity_path_collision_bad_ids_and_failed_creation_preserve_bytes() {
    let (tmp, s) = crate::state::test_state();
    let a = tool(&s, "lab/a").await;
    assert_eq!(
        call(&s, "POST", "/tools", Some(a)).await.0,
        StatusCode::CREATED
    );
    let path = tmp.path().join("lab/tools/lab_a.toml");
    let original = fs::read(&path).unwrap();
    for id in ["lab_a", "../escape", "/absolute", "x/../z", "x\\z"] {
        assert_ne!(
            call(&s, "POST", "/tools", Some(tool(&s, id).await)).await.0,
            StatusCode::CREATED
        );
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    let mut invalid = tool(&s, "invalid/tool").await;
    invalid["diameter_mm"] = json!(0);
    assert_eq!(
        call(&s, "POST", "/tools", Some(invalid)).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(!tmp.path().join("lab/tools/invalid_tool.toml").exists());
    assert_eq!(fs::read(&path).unwrap(), original);
    // Simulated interrupted temp publication: never replace or clean an unrelated temp.
    let orphan = tmp.path().join("lab/tools/unfinished.toml.tmp");
    fs::write(&orphan, "private sentinel").unwrap();
    s.reload(Lang::Fr).await.unwrap();
    assert_eq!(fs::read_to_string(orphan).unwrap(), "private sentinel");
    // Path ownership changed externally: update refuses even when the in-memory ID still exists.
    let mut foreign: fabspeed_core::ToolSpec =
        serde_json::from_value(tool(&s, "other/owner").await).unwrap();
    foreign.name = "PRIVATE OWNER".into();
    fs::write(&path, tool_document(&foreign)).unwrap();
    let bytes = fs::read(&path).unwrap();
    let (status, error) = call(&s, "PUT", "/tools/lab/a", Some(tool(&s, "lab/a").await)).await;
    assert_ne!(status, StatusCode::OK);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(!error.to_string().contains("PRIVATE OWNER"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn identity_concurrent_api_creations_trials_and_rollbacks() {
    let (tmp, s) = crate::state::test_state();
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..32 {
        let s = s.clone();
        tasks.spawn(async move { call(&s, "POST", "/tools", Some(tool(&s, "").await)).await });
    }
    let mut ids = BTreeSet::new();
    while let Some(r) = tasks.join_next().await {
        let (status, out) = r.unwrap();
        assert_eq!(status, StatusCode::CREATED, "{out}");
        assert!(ids.insert(out["tool"]["id"].as_str().unwrap().to_string()));
    }
    assert_eq!(ids.len(), 32);
    for id in &ids {
        let path = tmp.path().join("lab/tools").join(lab::file_name(id));
        assert_eq!(
            identity::record_ids(&path, lab::Kind::Tool).unwrap(),
            [id.clone()]
        );
    }
    for _ in 0..32 {
        let s = s.clone();
        tasks.spawn(async move {
            call(&s, "POST", "/trials", Some(trial("generic/flat-6-1f-up"))).await
        });
    }
    let mut trials = BTreeSet::new();
    while let Some(r) = tasks.join_next().await {
        let (status, out) = r.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert!(trials.insert(out["trial"]["id"].as_str().unwrap().to_string()));
    }
    assert_eq!(s.catalog(Lang::Fr).await.trials.len(), 32);
    for id in &trials {
        assert!(tmp
            .path()
            .join("lab/trials")
            .join(lab::file_name(id))
            .exists());
        for method in ["POST", "DELETE"] {
            let (status, out) = call(
                &s,
                method,
                &format!("/trials/{id}/validation"),
                if method == "POST" {
                    Some(json!({}))
                } else {
                    None
                },
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(out["trial"]["id"], *id);
        }
    }
    for id in [
        "same/explicit",
        "same/explicit",
        "path/a",
        "path_a",
        "bad/item",
    ] {
        let s = s.clone();
        tasks.spawn(async move {
            let mut t = tool(&s, id).await;
            if id == "bad/item" {
                t["diameter_mm"] = json!(0);
            }
            call(&s, "POST", "/tools", Some(t)).await
        });
    }
    let mut successes = 0;
    while let Some(r) = tasks.join_next().await {
        if r.unwrap().0 == StatusCode::CREATED {
            successes += 1;
        }
    }
    assert_eq!(successes, 2);
    assert_eq!(s.catalog(Lang::Fr).await.tools.len(), 11 + 32 + 2);
    let same = tmp.path().join("lab/tools/same_explicit.toml");
    assert_eq!(
        identity::record_ids(&same, lab::Kind::Tool).unwrap(),
        ["same/explicit"]
    );
    assert!(!tmp.path().join("lab/tools/bad_item.toml").exists());
}

#[tokio::test]
async fn identity_import_and_aliases_cannot_revive_reserved_ids() {
    let (_tmp, s) = crate::state::test_state();
    let body = tool(&s, "retired/tool").await;
    call(&s, "POST", "/tools", Some(body.clone())).await;
    call(&s, "DELETE", "/tools/retired/tool", None).await;
    let (status, report) = call(
        &s,
        "POST",
        "/library/import",
        Some(json!({"content":json!({"tools":[body]}).to_string()})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["added"], 0);
    assert_eq!(report["skipped"], 1);
    let mut alias = tool(&s, "new/tool").await;
    alias["fabcam_identity"] = json!({"id":"retired/tool","revision":"1"});
    assert_ne!(
        call(&s, "POST", "/tools", Some(alias)).await.0,
        StatusCode::CREATED
    );
    assert!(s.catalog(Lang::Fr).await.tool("retired/tool").is_none());
}

#[tokio::test]
async fn identity_restore_conflict_and_read_only_diagnostic() {
    let (tmp, s) = crate::state::test_state();
    call(&s, "POST", "/tools", Some(tool(&s, "known/tool").await)).await;
    let (_, mut backup) = call(&s, "GET", "/lab/backup", None).await;
    let file = backup["files"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["path"] == "lab/tools/known_tool.toml")
        .unwrap();
    file["content"] = json!(file["content"]
        .as_str()
        .unwrap()
        .replace("diameter_mm = ", "diameter_mm = 9"));
    let before = tree(tmp.path());
    assert_ne!(
        call(&s, "POST", "/lab/restore", Some(backup)).await.0,
        StatusCode::OK
    );
    assert_eq!(tree(tmp.path()), before);
    identity::inspect(tmp.path()).unwrap();
    assert_eq!(tree(tmp.path()), before);
    // Dangling references alone reserve IDs, even without archives.
    drop(s);
    let path = tmp.path().join("lab/trials");
    fs::create_dir_all(&path).unwrap();
    let mut c = crate::loader::load_catalog(tmp.path(), Lang::Fr).unwrap().0;
    let input = serde_json::from_value(trial("known/tool")).unwrap();
    let mut t = fabspeed_core::Trial::from_input(
        input,
        &c,
        "synthetic-old".into(),
        "2026-01-01T00:00:00Z".into(),
        "test",
    )
    .unwrap();
    t.tool_id = "missing/historical".into();
    c.trials.push(t.clone());
    fs::write(
        path.join("old.toml"),
        fabspeed_core::toml_data::trial_document(&t),
    )
    .unwrap();
    let s = AppState::from_dir(tmp.path().to_path_buf()).unwrap();
    assert_ne!(
        call(
            &s,
            "POST",
            "/tools",
            Some(tool(&s, "missing/historical").await)
        )
        .await
        .0,
        StatusCode::CREATED
    );
}

#[test]
fn identity_lease_child() {
    if let Ok(dir) = std::env::var("FABSPEED_TEST_LEASE_DIR") {
        assert!(identity::DataLease::acquire(Path::new(&dir)).is_err());
        assert!(AppState::from_dir(dir.into()).is_err());
    }
}
#[test]
fn identity_cross_process_lease_and_release() {
    let (tmp, lease) = crate::state::test_state();
    assert!(identity::DataLease::acquire(tmp.path()).is_err());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "identity_tests::identity_lease_child",
            "--nocapture",
        ])
        .env("FABSPEED_TEST_LEASE_DIR", tmp.path())
        .status()
        .unwrap();
    assert!(status.success());
    drop(lease);
    assert!(AppState::from_dir(tmp.path().to_path_buf()).is_ok());
}

#[tokio::test]
async fn identity_synthetic_scale_measurements() {
    for count in [10, 1000, 10000] {
        let (tmp, s) = crate::state::test_state();
        let mut template = s.catalog(Lang::Fr).await.tools[0].clone();
        drop(s);
        let mut text = String::new();
        for n in 0..count {
            template.id = format!("scale/{n}");
            text.push_str(&tool_document(&template));
        }
        fs::write(tmp.path().join("tools/scale.toml"), text).unwrap();
        let start = Instant::now();
        let s = AppState::from_dir(tmp.path().to_path_buf()).unwrap();
        let startup = start.elapsed();
        let start = Instant::now();
        let (status, record) = call(&s, "POST", "/tools", Some(tool(&s, "scale/new").await)).await;
        let create = start.elapsed();
        assert_eq!(status, StatusCode::CREATED, "{record}");
        let start = Instant::now();
        assert_eq!(
            call(&s, "DELETE", "/tools/scale/new", None).await.0,
            StatusCode::OK
        );
        let archive = start.elapsed();
        println!(
            "IDENTITY_SCALE count={count} startup_ms={} create_ms={} archive_ms={}",
            startup.as_millis(),
            create.as_millis(),
            archive.as_millis()
        );
        assert_eq!(s.catalog(Lang::Fr).await.tools.len(), count + 11);
    }
}

#[tokio::test]
async fn identity_legacy_ambiguity_diagnostic_never_repairs() {
    let (tmp, s) = crate::state::test_state();
    let original: fabspeed_core::ToolSpec =
        serde_json::from_value(tool(&s, "legacy/reused").await).unwrap();
    drop(s);
    fs::remove_file(tmp.path().join(identity::FILE)).unwrap();
    fs::create_dir_all(tmp.path().join("lab/archive/tools")).unwrap();
    fs::create_dir_all(tmp.path().join("lab/tools")).unwrap();
    fs::write(
        tmp.path().join("lab/archive/tools/old.toml"),
        tool_document(&original),
    )
    .unwrap();
    let mut different = original.clone();
    different.diameter_mm += 1.0;
    fs::write(
        tmp.path().join("lab/tools/new.toml"),
        tool_document(&different),
    )
    .unwrap();
    let before = tree(tmp.path());
    let error = identity::inspect(tmp.path()).unwrap_err().to_string();
    assert!(error.contains("legacy/reused"));
    assert!(error.contains("archived"));
    assert_eq!(tree(tmp.path()), before);
    assert!(!tmp.path().join(identity::FILE).exists());
    // Duplicate active lab IDs must not be silently merged by layer loading.
    fs::remove_file(tmp.path().join("lab/archive/tools/old.toml")).unwrap();
    fs::write(
        tmp.path().join("lab/tools/second.toml"),
        tool_document(&different),
    )
    .unwrap();
    assert!(identity::inspect(tmp.path())
        .unwrap_err()
        .to_string()
        .contains("duplicate"));
}

#[tokio::test]
async fn identity_no_clobber_permission_failure_and_symlink_refusal() {
    let (tmp, s) = crate::state::test_state();
    let p = tmp.path().join("unrelated");
    fs::write(&p, b"sentinel").unwrap();
    assert!(identity::create_file(&p, b"replacement").is_err());
    assert_eq!(fs::read(&p).unwrap(), b"sentinel");
    #[cfg(unix)]
    {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = tmp.path().join("lab/tools");
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).unwrap();
        let result = call(
            &s,
            "POST",
            "/tools",
            Some(tool(&s, "permission/refused").await),
        )
        .await;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        assert_ne!(result.0, StatusCode::CREATED);
        assert!(!dir.join("permission_refused.toml").exists());
        symlink(&p, dir.join("link_refused.toml")).unwrap();
        assert_ne!(
            call(&s, "POST", "/tools", Some(tool(&s, "link/refused").await))
                .await
                .0,
            StatusCode::CREATED
        );
        assert_eq!(fs::read(&p).unwrap(), b"sentinel");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn identity_archive_create_race_never_reassigns_trial() {
    let (_tmp, s) = crate::state::test_state();
    let (_, created) = call(&s, "POST", "/tools", Some(tool(&s, "").await)).await;
    let id = created["tool"]["id"].as_str().unwrap();
    call(&s, "POST", "/trials", Some(trial(id))).await;
    let path = format!("/tools/{id}");
    let create = async { call(&s, "POST", "/tools", Some(tool(&s, "").await)).await };
    let (deleted, created) = tokio::join!(call(&s, "DELETE", &path, None), create);
    assert_eq!(deleted.0, StatusCode::OK);
    assert_eq!(created.0, StatusCode::CREATED);
    assert_ne!(created.1["tool"]["id"], id);
    assert_eq!(s.catalog(Lang::Fr).await.trials[0].tool_id, id);
    assert!(s.catalog(Lang::Fr).await.tested_presets().is_empty());
}

#[test]
fn identity_interrupted_restore_is_fail_closed() {
    let (tmp, s) = crate::state::test_state();
    drop(s);
    fs::write(
        tmp.path().join(identity::PENDING_RESTORE),
        b"synthetic interrupted restore",
    )
    .unwrap();
    let before = tree(tmp.path());
    assert!(identity::inspect(tmp.path())
        .unwrap_err()
        .to_string()
        .contains("interrupted restore"));
    assert!(AppState::from_dir(tmp.path().to_path_buf()).is_err());
    assert_eq!(tree(tmp.path()), before);
}

#[tokio::test]
async fn identity_legacy_restore_reserves_archive_only_ids_before_next_request() {
    let (source_dir, source) = crate::state::test_state();
    call(
        &source,
        "POST",
        "/tools",
        Some(tool(&source, "old/archive-only").await),
    )
    .await;
    call(&source, "DELETE", "/tools/old/archive-only", None).await;
    let (_, mut backup) = call(&source, "GET", "/lab/backup", None).await;
    backup
        .as_object_mut()
        .unwrap()
        .remove("identity_reservations");
    let (_target_dir, target) = crate::state::test_state();
    let (status, out) = call(&target, "POST", "/lab/restore", Some(backup)).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_ne!(
        call(
            &target,
            "POST",
            "/tools",
            Some(tool(&target, "old/archive-only").await)
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert!(source_dir.path().join("lab/archive/tools").is_dir());
}

#[tokio::test]
async fn identity_diagnostic_reports_changed_trial_snapshot_without_private_content() {
    let (tmp, s) = crate::state::test_state();
    let (_, created) = call(&s, "POST", "/tools", Some(tool(&s, "snapshot/tool").await)).await;
    let mut input = trial("snapshot/tool");
    input["operator"] = json!("PRIVATE OPERATOR");
    input["notes"] = json!("PRIVATE NOTES");
    call(&s, "POST", "/trials", Some(input)).await;
    let mut changed = created["tool"].clone();
    changed["diameter_mm"] = json!(7);
    assert_eq!(
        call(&s, "PUT", "/tools/snapshot/tool", Some(changed))
            .await
            .0,
        StatusCode::OK
    );
    let before = tree(tmp.path());
    let messages = identity::diagnostics(tmp.path()).unwrap().join("\n");
    assert!(messages.contains("snapshot/tool"));
    assert!(messages.contains("geometry differs"));
    assert!(!messages.contains("PRIVATE"));
    assert_eq!(tree(tmp.path()), before);
    let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    assert!(crate::install::install_base(&bundled, tmp.path()).is_err());
    assert_eq!(tree(tmp.path()), before);
}
