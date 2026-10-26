//! Échange additif de fiches de bibliothèque. Aucun remplacement, même en cas
//! de conflit d'identifiant/alias ; vérification et écriture du lot sous le
//! même verrou que les autres écritures du poste.
use crate::api::{ApiError, ReqLang};
use crate::auth::Admin;
use crate::backup::{self, Backup};
use crate::lab::{file_name, Kind};
use crate::state::AppState;
use axum::extract::{DefaultBodyLimit, Json, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use fabspeed_core::import::import_ivac_toolset;
use fabspeed_core::toml_data::{machine_document, material_document, tool_document};
use fabspeed_core::{tr, Catalog, Lang, MachineProfile, MaterialProfile, ToolSpec};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Section {
    Machines,
    Materials,
    Tools,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct Library {
    machines: Vec<MachineProfile>,
    materials: Vec<MaterialProfile>,
    tools: Vec<ToolSpec>,
}

impl Library {
    fn from_catalog(c: &Catalog, kind: Option<Section>) -> Self {
        Self {
            machines: if kind.is_none() || kind == Some(Section::Machines) {
                c.machines.clone()
            } else {
                vec![]
            },
            materials: if kind.is_none() || kind == Some(Section::Materials) {
                c.materials.clone()
            } else {
                vec![]
            },
            tools: if kind.is_none() || kind == Some(Section::Tools) {
                c.tools.clone()
            } else {
                vec![]
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportBody {
    content: String,
    kind: Option<Section>,
}

#[derive(Default, Deserialize)]
struct Options {
    #[serde(default)]
    check: bool,
    kind: Option<Section>,
}

#[derive(Serialize)]
struct Record {
    kind: Section,
    id: String,
    name: String,
    action: &'static str,
    reason: String,
}

#[derive(Default, Serialize)]
struct Report {
    added: usize,
    skipped: usize,
    records: Vec<Record>,
    issues: Vec<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/library/export", get(export)).route(
        "/library/import",
        post(import).layer(DefaultBodyLimit::max(backup::MAX_BACKUP_BYTES)),
    )
}

fn invalid(lang: Lang, detail: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_import",
        tr!(
            lang,
            "Import refusé, rien n'a été ajouté. {detail}",
            "Import refused, nothing was added. {detail}"
        ),
    )
}

fn parse(state: &AppState, text: &str, lang: Lang) -> Result<Library, ApiError> {
    let text = text.trim_start_matches('\u{feff}').trim();
    if text.starts_with('{') {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| invalid(lang, e))?;
        if value.get("format").is_some() {
            let b: Backup = serde_json::from_value(value).map_err(|e| invalid(lang, e))?;
            let dir = state.data_dir().ok_or_else(|| invalid(lang, "read_only"))?;
            let (c, _) = backup::catalog_for_import(dir, &b, lang)?;
            // Format 1: only records actually present in the saved lab, not
            // the current base used to validate the old backup.
            let mut lib = Library::from_catalog(&c, None);
            if b.version == 1 {
                lib.machines
                    .retain(|m| c.layers.lab_machines.contains(&m.id));
                lib.materials
                    .retain(|m| c.layers.lab_materials.contains(&m.id));
                lib.tools.retain(|t| c.layers.lab_tools.contains(&t.id));
            }
            return Ok(lib);
        }
        if value.get("kind").is_some() {
            return Ok(Library {
                tools: import_ivac_toolset(text, lang)
                    .map_err(|e| invalid(lang, e.message(lang)))?,
                ..Library::default()
            });
        }
        serde_json::from_value(value).map_err(|e| invalid(lang, e))
    } else {
        toml::from_str(text).map_err(|e| invalid(lang, e))
    }
}

// Include alternate identities in conflict detection, so importing a renamed
// record cannot silently change material/tool lookup for existing settings.
fn tool_keys(t: &ToolSpec) -> BTreeSet<&str> {
    std::iter::once(t.id.as_str())
        .chain(t.fabcam_identity.iter().map(|i| i.id.as_str()))
        .collect()
}
fn material_keys(m: &MaterialProfile) -> BTreeSet<&str> {
    std::iter::once(m.id.as_str())
        .chain(m.aliases.iter().map(String::as_str))
        .collect()
}

type Entry = (Kind, String, String);
fn plan(
    current: &Catalog,
    reserved: &crate::identity::Reservations,
    incoming: Library,
    kind: Option<Section>,
    lang: Lang,
) -> Result<(Report, Vec<Entry>), ApiError> {
    let mut candidate = current.clone();
    candidate.lang = lang;
    let mut report = Report::default();
    let mut entries = vec![];
    let mut record = |section, id: &str, name: &str, conflict: bool| -> Result<(), ApiError> {
        if !crate::identity::valid_id(id) {
            return Err(invalid(
                lang,
                tr!(lang, "Identifiant vide : {name}", "Empty ID: {name}"),
            ));
        }
        report.records.push(Record {
            kind: section,
            id: id.into(),
            name: name.into(),
            action: if conflict { "skip" } else { "add" },
            reason: if conflict {
                tr!(
                    lang,
                    "Identifiant ou alias déjà présent ou historiquement réservé : fiche conservée.",
                    "ID or alias already present or historically reserved: existing record kept."
                )
            } else {
                String::new()
            },
        });
        if conflict {
            report.skipped += 1;
        } else {
            report.added += 1;
        }
        Ok(())
    };
    if kind.is_none() || kind == Some(Section::Machines) {
        for m in incoming.machines {
            let conflict = reserved.contains(Kind::Machine, &m.id)
                || candidate.machines.iter().any(|x| x.id == m.id);
            record(Section::Machines, &m.id, &m.name, conflict)?;
            if !conflict {
                entries.push((Kind::Machine, m.id.clone(), machine_document(&m)));
                candidate.machines.push(m);
            }
        }
    }
    if kind.is_none() || kind == Some(Section::Materials) {
        for m in incoming.materials {
            let keys = material_keys(&m);
            let conflict = keys.iter().any(|id| reserved.contains(Kind::Material, id))
                || candidate
                    .materials
                    .iter()
                    .any(|x| !material_keys(x).is_disjoint(&keys));
            record(Section::Materials, &m.id, &m.name, conflict)?;
            if !conflict {
                entries.push((Kind::Material, m.id.clone(), material_document(&m)));
                candidate.materials.push(m);
            }
        }
    }
    if kind.is_none() || kind == Some(Section::Tools) {
        for t in incoming.tools {
            let keys = tool_keys(&t);
            let conflict = keys.iter().any(|id| reserved.contains(Kind::Tool, id))
                || candidate
                    .tools
                    .iter()
                    .any(|x| !tool_keys(x).is_disjoint(&keys));
            record(Section::Tools, &t.id, &t.name, conflict)?;
            if !conflict {
                entries.push((Kind::Tool, t.id.clone(), tool_document(&t)));
                candidate.tools.push(t);
            }
        }
    }
    let issues = candidate.validate();
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.fatal)
        .map(|i| i.message.as_str())
        .collect();
    if !errors.is_empty() {
        return Err(invalid(lang, errors.join(" · ")));
    }
    report.issues = issues
        .into_iter()
        .filter(|i| !i.fatal)
        .map(|i| i.message)
        .collect();
    Ok((report, entries))
}

async fn export(
    State(state): State<AppState>,
    _: Admin,
    Query(options): Query<Options>,
) -> Response {
    let c = state.catalog(Lang::Fr).await;
    let lib = Library::from_catalog(&c, options.kind);
    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"fabspeed-library.json\"",
            ),
        ],
        serde_json::to_string_pretty(&lib).expect("catalogue sérialisable"),
    )
        .into_response()
}

async fn import(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Query(options): Query<Options>,
    Json(body): Json<ImportBody>,
) -> Result<Json<Report>, ApiError> {
    let _guard = state.write_lock().lock().await;
    let lib = parse(&state, &body.content, lang)?;
    let current = state.catalog(Lang::Fr).await;
    let (report, entries) = plan(&current, &state.reservations(), lib, body.kind, lang)?;
    let root = state
        .data_dir()
        .ok_or_else(|| invalid(lang, "read_only"))?
        .join("lab");
    // Check every destination before the first write, including sanitized-name
    // collisions. create_new below also guarantees that no file is overwritten.
    let mut paths = BTreeSet::new();
    let mut files = vec![];
    for (kind, id, contents) in entries {
        let dir = match kind {
            Kind::Machine => "machines",
            Kind::Material => "materials",
            Kind::Tool => "tools",
            Kind::Trial => unreachable!(),
        };
        let path = root.join(dir).join(file_name(&id));
        if path.exists() || !paths.insert(path.clone()) {
            return Err(invalid(
                lang,
                tr!(
                    lang,
                    "Collision de fichier pour {id}",
                    "File collision for {id}"
                ),
            ));
        }
        files.push((path, contents));
    }
    if options.check || files.is_empty() {
        return Ok(Json(report));
    }
    crate::identity::safe_file(&root).map_err(|e| invalid(lang, e))?;
    for dir in ["machines", "materials", "tools"] {
        crate::identity::safe_file(&root.join(dir)).map_err(|e| invalid(lang, e))?;
    }
    let mut reserved = state.reservations();
    for r in &report.records {
        if r.action == "add" {
            let kind = match r.kind {
                Section::Machines => Kind::Machine,
                Section::Materials => Kind::Material,
                Section::Tools => Kind::Tool,
            };
            reserved.reserve(kind, &r.id);
        }
    }
    // Record imported versions before publication, so restart can recognize a
    // fully published record even if the process stopped before reload.
    for (path, text) in &files {
        let kind = match path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
        {
            Some("machines") => Kind::Machine,
            Some("materials") => Kind::Material,
            _ => Kind::Tool,
        };
        let mut c = Catalog::default();
        c.merge_toml(kind.data_kind(), "import", text)
            .map_err(|_| invalid(lang, "invalid record"))?;
        reserved.observe(&c);
    }
    state
        .retain_reservations(&reserved)
        .map_err(|e| invalid(lang, e))?;
    let mut written = vec![];
    let result: Result<(), String> = async {
        for (path, text) in &files {
            fs::create_dir_all(path.parent().expect("lab directory")).map_err(|e| e.to_string())?;
            crate::identity::create_file(path, text.as_bytes()).map_err(|e| e.to_string())?;
            written.push(path);
        }
        state.reload_locked(lang).await.map_err(|e| e.to_string())
    }
    .await;
    if let Err(e) = result {
        for path in written {
            fs::remove_file(path)
                .map_err(|rollback| invalid(lang, format!("{e}; rollback: {rollback}")))?;
        }
        return Err(invalid(lang, e));
    }
    state.publication_changed();
    Ok(Json(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    async fn call(
        state: &AppState,
        uri: &str,
        body: Option<Value>,
        admin: bool,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(if body.is_some() { "POST" } else { "GET" })
            .uri(format!("/api/v1{uri}"));
        if admin {
            req = req.header(
                "authorization",
                format!("Bearer {}", state.auth().test_session().await),
            );
        }
        let res = crate::app(state.clone())
            .oneshot(
                req.header("content-type", "application/json")
                    .body(body.map_or(Body::empty(), |b| Body::from(b.to_string())))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    fn input(lib: &Library) -> Value {
        json!({ "content": serde_json::to_string(lib).unwrap() })
    }
    fn new_records(c: &Catalog) -> Library {
        let mut lib = Library::from_catalog(c, None);
        lib.machines.truncate(1);
        lib.materials.truncate(1);
        lib.tools.truncate(1);
        lib.machines[0].id = "transfer/machine".into();
        lib.materials[0].id = "transfer/material".into();
        lib.materials[0].aliases.clear();
        lib.tools[0].id = "transfer/tool".into();
        lib.tools[0].fabcam_identity = None;
        lib
    }

    #[tokio::test]
    async fn preview_import_all_and_reimport_preserve_existing_records() {
        let (tmp, state) = crate::state::test_state();
        let before = state.catalog(Lang::Fr).await;
        // Include an existing record with deliberately changed values: it must
        // be skipped rather than silently modifying a base or lab record.
        let mut lib = new_records(&before);
        let mut changed = before.tools[0].clone();
        changed.diameter_mm *= 2.0;
        lib.tools.push(changed);
        let body = input(&lib);
        let (status, preview) = call(
            &state,
            "/library/import?check=true",
            Some(body.clone()),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{preview}");
        assert_eq!(preview["added"], 3);
        assert_eq!(preview["skipped"], 1);
        assert!(!tmp.path().join("lab").exists());
        assert_eq!(
            state.catalog(Lang::Fr).await.tools.len(),
            before.tools.len()
        );
        // A pre-existing lab record is also protected on subsequent imports.
        let (status, result) = call(&state, "/library/import", Some(body.clone()), true).await;
        assert_eq!(status, StatusCode::OK, "{result}");
        let after = state.catalog(Lang::Fr).await;
        assert_eq!(after.tools.len(), before.tools.len() + 1);
        assert_eq!(after.materials.len(), before.materials.len() + 1);
        assert_eq!(after.machines.len(), before.machines.len() + 1);
        assert_eq!(
            serde_json::to_value(&after.tools[..before.tools.len()]).unwrap(),
            serde_json::to_value(&before.tools).unwrap()
        );
        let tool_path = tmp
            .path()
            .join("lab/tools")
            .join(file_name("transfer/tool"));
        let saved = fs::read(&tool_path).unwrap();
        let mut changed_lib = new_records(&before);
        changed_lib.tools[0].diameter_mm = 99.0;
        let (_, again) = call(&state, "/library/import", Some(input(&changed_lib)), true).await;
        assert_eq!(again["added"], 0);
        assert_eq!(again["skipped"], 3);
        assert_eq!(fs::read(tool_path).unwrap(), saved);
        drop(state); // a new owner only after the original process/state releases its lease
        let reopened = AppState::from_dir(tmp.path().to_path_buf()).unwrap();
        assert!(reopened
            .catalog(Lang::Fr)
            .await
            .tool("transfer/tool")
            .is_some());
    }

    #[tokio::test]
    async fn invalid_batch_and_filename_collisions_write_nothing() {
        let (tmp, state) = crate::state::test_state();
        let before = state.catalog(Lang::Fr).await;
        let mut lib = new_records(&before);
        lib.tools[0].diameter_mm = -1.0;
        let (status, _) = call(&state, "/library/import", Some(input(&lib)), true).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!tmp.path().join("lab").exists());
        lib.tools[0].diameter_mm = 6.0;
        let mut collision = lib.tools[0].clone();
        collision.id = "transfer_tool".into();
        lib.tools.push(collision);
        let (status, _) = call(&state, "/library/import", Some(input(&lib)), true).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!tmp.path().join("lab").exists());
        // Collision with a manually named file belonging to a different ID.
        fs::create_dir_all(tmp.path().join("lab/tools")).unwrap();
        let path = tmp.path().join("lab/tools/transfer_tool.toml");
        let mut other = before.tools[0].clone();
        other.id = "another-id".into();
        let text = tool_document(&other);
        fs::write(&path, &text).unwrap();
        state.reload(Lang::Fr).await.unwrap();
        lib.tools.pop();
        let (status, _) = call(&state, "/library/import", Some(input(&lib)), true).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(fs::read_to_string(path).unwrap(), text);
        assert!(!tmp.path().join("lab/machines").exists());
    }

    #[tokio::test]
    async fn write_failure_rolls_back_the_whole_batch() {
        let (tmp, state) = crate::state::test_state();
        let before = state.catalog(Lang::Fr).await;
        let lib = new_records(&before);
        fs::create_dir_all(tmp.path().join("lab")).unwrap();
        // Failure after the machine has been written: materials/ is a file.
        fs::write(tmp.path().join("lab/materials"), "keep me").unwrap();
        let (status, _) = call(&state, "/library/import", Some(input(&lib)), true).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            fs::read_dir(tmp.path().join("lab/machines"))
                .unwrap()
                .count(),
            0
        );
        assert_eq!(
            fs::read_to_string(tmp.path().join("lab/materials")).unwrap(),
            "keep me"
        );
        assert_eq!(
            state.catalog(Lang::Fr).await.machines.len(),
            before.machines.len()
        );
    }

    #[tokio::test]
    async fn aliases_identity_duplicates_and_section_filter_are_respected() {
        let (_tmp, state) = crate::state::test_state();
        let c = state.catalog(Lang::Fr).await;
        let mut lib = new_records(&c);
        lib.materials[0].aliases = vec![c.materials[0].id.clone()];
        lib.tools[0].fabcam_identity = Some(fabspeed_core::Identity {
            id: c.tools[0].id.clone(),
            revision: "1".into(),
        });
        lib.machines.push(lib.machines[0].clone());
        let (_, report) = call(&state, "/library/import", Some(input(&lib)), true).await;
        assert_eq!(report["added"], 1, "{report}");
        assert_eq!(report["skipped"], 3);
        let mut body = input(&new_records(&c));
        body["kind"] = json!("tools");
        let (_, report) = call(&state, "/library/import", Some(body), true).await;
        assert_eq!(report["added"], 1);
        assert_eq!(report["records"][0]["kind"], "tools");
        assert_eq!(
            state.catalog(Lang::Fr).await.materials.len(),
            c.materials.len()
        );
    }

    #[tokio::test]
    async fn toml_fabcam_and_export_round_trip_are_additive() {
        let (_tmp, state) = crate::state::test_state();
        let c = state.catalog(Lang::Fr).await;
        let lib = new_records(&c);
        let text = format!(
            "{}\n{}\n{}",
            machine_document(&lib.machines[0]),
            material_document(&lib.materials[0]),
            tool_document(&lib.tools[0])
        );
        let (status, report) = call(
            &state,
            "/library/import",
            Some(json!({"content": text})),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{report}");
        assert_eq!(report["added"], 3);
        let fabcam = r#"{"kind":"toolset","payload":[{"name":"Demo upcut","kind":"flat_endmill","diameter":6,"flutes":2,"fsIdentity":{"id":"demo/upcut","revision":"1"}},{"name":"Demo ball","kind":"ball_nose","diameter":3,"flutes":2}]}"#;
        let (status, report) = call(
            &state,
            "/library/import",
            Some(json!({"content":fabcam})),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{report}");
        assert!(report["added"].as_u64().unwrap() > 0);
        let (_, report) = call(
            &state,
            "/library/import",
            Some(json!({"content":fabcam})),
            true,
        )
        .await;
        assert_eq!(report["added"], 0);
        let (_, exported) = call(&state, "/library/export", None, true).await;
        let (status, report) = call(
            &state,
            "/library/import",
            Some(json!({"content":exported.to_string()})),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{report}");
        assert_eq!(report["added"], 0);
        let (_, exported) = call(&state, "/library/export?kind=materials", None, true).await;
        assert_eq!(exported["tools"], json!([]));
        assert_eq!(exported["machines"], json!([]));
        assert!(exported["materials"].as_array().unwrap().len() > 1);
    }

    #[tokio::test]
    async fn backups_import_effective_records_without_restoring_the_post() {
        let (source_tmp, source) = crate::state::test_state();
        let c = source.catalog(Lang::Fr).await;
        let mut lib = new_records(&c);
        // A modified base record in the source backup must not replace the
        // destination's corresponding base record.
        let mut changed = c.tools[0].clone();
        changed.diameter_mm = 99.0;
        source
            .write_entry(Kind::Tool, &changed.id, tool_document(&changed), Lang::Fr)
            .await
            .unwrap();
        lib.machines.clear();
        call(&source, "/library/import", Some(input(&lib)), true).await;
        let backup = backup::make_backup(source_tmp.path(), "source").unwrap();
        let (target_tmp, target) = crate::state::test_state();
        let original_dataset = fs::read(target_tmp.path().join("dataset.toml")).unwrap();
        let body = json!({"content":serde_json::to_string(&backup).unwrap()});
        let (status, report) = call(&target, "/library/import", Some(body), true).await;
        assert_eq!(status, StatusCode::OK, "{report}");
        assert_eq!(report["added"], 2);
        assert_eq!(
            fs::read(target_tmp.path().join("dataset.toml")).unwrap(),
            original_dataset
        );
        let after = target.catalog(Lang::Fr).await;
        assert_eq!(after.tools[0].diameter_mm, c.tools[0].diameter_mm);
        assert!(after.trials.is_empty());
        assert!(!target_tmp.path().join("backups").exists());
        // Old backups contain only lab records; current base records used to
        // validate them must not appear as imported records.
        let old = Backup {
            version: 1,
            files: backup
                .files
                .into_iter()
                .filter_map(|mut f| {
                    f.path.strip_prefix("lab/").map(str::to_string).map(|p| {
                        f.path = p;
                        f
                    })
                })
                .collect(),
            ..backup
        };
        let (status, report) = call(
            &target,
            "/library/import?check=true",
            Some(json!({"content":serde_json::to_string(&old).unwrap()})),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{report}");
        assert_eq!(report["records"].as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn existing_trials_validation_policy_and_password_are_unchanged() {
        let (tmp, state) = crate::state::test_state();
        state.auth().setup("demo-password").await.unwrap();
        let c = state.catalog(Lang::Fr).await;
        let (status, out) = call(
            &state,
            "/trials",
            Some(json!({
                "machine_id": c.machines[0].id, "material_id": c.materials[0].id,
                "tool_id": c.tools[0].id, "operation":"profile", "aggressiveness":"normal",
                "workholding":"screws_clamps", "stepover_ratio":1, "rpm":15000, "feed_mm_min":1000,
                "plunge_mm_min":100, "doc_mm":1, "outcome":"good"
            })),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{out}");
        let id = out["trial"]["id"].as_str().unwrap();
        assert_eq!(
            call(
                &state,
                &format!("/trials/{id}/validation"),
                Some(json!({})),
                true
            )
            .await
            .0,
            StatusCode::OK
        );
        fs::write(
            tmp.path().join("policy.toml"),
            toml::to_string(&c.policy).unwrap(),
        )
        .unwrap();
        let paths = [
            tmp.path().join("lab/trials").join(file_name(id)),
            tmp.path().join("policy.toml"),
            tmp.path().join("auth.toml"),
        ];
        let saved: Vec<_> = paths.iter().map(|p| fs::read(p).unwrap()).collect();
        let (_, report) = call(
            &state,
            "/library/import",
            Some(input(&new_records(&c))),
            true,
        )
        .await;
        assert_eq!(report["added"], 3, "{report}");
        for (path, bytes) in paths.iter().zip(saved) {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        assert!(state.catalog(Lang::Fr).await.trials[0].validation.is_some());
        assert!(state.auth().login("demo-password").await.is_ok());
    }

    #[tokio::test]
    async fn admin_required_and_concurrent_imports_do_not_overwrite() {
        let (_tmp, state) = crate::state::test_state();
        let c = state.catalog(Lang::Fr).await;
        let body = input(&new_records(&c));
        for uri in ["/library/import", "/library/import?check=true"] {
            let (status, _) = call(&state, uri, Some(body.clone()), false).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
        }
        assert_eq!(
            call(&state, "/library/export", None, false).await.0,
            StatusCode::UNAUTHORIZED
        );
        let (a, b) = tokio::join!(
            call(&state, "/library/import", Some(body.clone()), true),
            call(&state, "/library/import", Some(body), true)
        );
        assert_eq!(a.0, StatusCode::OK, "{:?}", a.1);
        assert_eq!(b.0, StatusCode::OK, "{:?}", b.1);
        assert_eq!(
            a.1["added"].as_u64().unwrap() + b.1["added"].as_u64().unwrap(),
            3
        );
    }
}
