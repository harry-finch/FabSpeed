//! Routes d'écriture : essais, fraises et machines du lab (`data/lab/`).
//! Réservées à l'administrateur (extracteur [`Admin`]).
//! Contrat documenté dans `docs/API.md`.

use crate::api::{ApiError, ReqLang};
use crate::auth::Admin;
use crate::lab::{now_rfc3339, trial_id, unique_id, Kind, LabError};
use crate::state::AppState;
use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, post, put};
use axum::Router;
use fabspeed_core::toml_data::{
    machine_document, material_document, tool_document, trial_document,
};
use fabspeed_core::{
    tr, Lang, MachineProfile, MaterialProfile, ToolSpec, Trial, TrialInput, ENGINE_VERSION,
};
use serde::Deserialize;
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/trials", post(add_trial))
        .route("/trials/{id}", delete(remove_trial))
        .route(
            "/trials/{id}/validation",
            post(validate_trial).delete(unvalidate_trial),
        )
        .route("/tools", post(create_tool))
        .route("/tools/{*id}", put(update_tool).delete(remove_tool))
        .route("/materials", post(create_material))
        .route(
            "/materials/{*id}",
            put(update_material).delete(remove_material),
        )
        .route("/machines", post(create_machine))
        .route(
            "/machines/{*id}",
            put(update_machine).delete(remove_machine),
        )
}

fn lab_error(e: LabError, lang: Lang) -> ApiError {
    let (status, code) = match e {
        LabError::NoDataDir => (StatusCode::SERVICE_UNAVAILABLE, "read_only"),
        LabError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
        LabError::Invalid(_) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_data"),
        LabError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "io_error"),
    };
    ApiError::new(status, code, e.message(lang))
}

/// Un texte renvoyé inchangé dans une autre langue que le français reste en
/// français dans les données : la traduction s'en charge à l'affichage.
fn keep_base<T: PartialEq + Clone>(field: &mut T, shown: &T, base: &T) {
    if field == shown {
        *field = base.clone();
    }
}

async fn add_trial(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Json(input): Json<TrialInput>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let catalog = state.catalog(lang).await;
    let recorded_at = now_rfc3339();
    let root = trial_id(&recorded_at);
    let reservations = state.reservations();
    let id = (0u64..)
        .map(|n| {
            if n == 0 {
                root.clone()
            } else {
                format!("{root}-{n}")
            }
        })
        .find(|id| !reservations.contains(Kind::Trial, id))
        .expect("available trial identity");
    let trial = Trial::from_input(input, &catalog, id.clone(), recorded_at, ENGINE_VERSION)
        .map_err(|e| ApiError::engine(e, lang))?;
    state
        .create_entry_locked(Kind::Trial, &id, trial_document(&trial), lang)
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok((StatusCode::CREATED, Json(json!({ "trial": trial }))).into_response())
}

async fn remove_trial(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    state
        .archive_entry_locked(Kind::Trial, &id, lang)
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok(Json(json!({ "archived": id })).into_response())
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct ValidationBody {
    validated_by: String,
    note: String,
}

/// Essai enregistré (textes de base, en français).
async fn stored_trial(state: &AppState, id: &str, lang: Lang) -> Result<Trial, ApiError> {
    state
        .catalog(Lang::Fr)
        .await
        .trials
        .iter()
        .find(|t| t.id == id)
        .cloned()
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "not_found",
                tr!(lang, "Essai introuvable : {id}", "Test not found: {id}"),
            )
        })
}

/// L'administrateur valide un essai réussi : il devient la référence du contexte.
async fn validate_trial(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Path(id): Path<String>,
    Json(body): Json<ValidationBody>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let mut trial = stored_trial(&state, &id, lang).await?;
    trial
        .validate(now_rfc3339(), &body.validated_by, &body.note, lang)
        .map_err(|e| ApiError::engine(e, lang))?;
    let catalog = state.catalog(Lang::Fr).await;
    let binding = crate::catalogue::capture(&catalog, &trial, &state.source()).map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "validation_binding",
            tr!(
                lang,
                "Liaison de validation aux ressources et au contexte refusée",
                "Validation dependency/context binding refused"
            ),
        )
    })?;
    trial.validation.as_mut().expect("validated trial").binding = Some(binding);
    state
        .write_entry_locked(Kind::Trial, &id, trial_document(&trial), lang)
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok(Json(json!({ "trial": trial })).into_response())
}

async fn unvalidate_trial(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let mut trial = stored_trial(&state, &id, lang).await?;
    trial.validation = None;
    state
        .write_entry_locked(Kind::Trial, &id, trial_document(&trial), lang)
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok(Json(json!({ "trial": trial })).into_response())
}

/// Création : identifiant `lab/<nom>` attribué par le serveur si absent.
async fn create_tool(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Json(mut tool): Json<ToolSpec>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let catalog = state.catalog(lang).await;
    if tool.id.trim().is_empty() {
        let base = if tool.reference.trim().is_empty() {
            &tool.name
        } else {
            &tool.reference
        };
        tool.id = {
            let reserved = state.reservations();
            unique_id(base, |id| reserved.contains(Kind::Tool, id))
        };
    } else if catalog.tool(&tool.id).is_some() {
        let id = &tool.id;
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "already_exists",
            tr!(
                lang,
                "La fraise {id} existe déjà",
                "Tool {id} already exists"
            ),
        ));
    }
    state
        .create_entry_locked(Kind::Tool, &tool.id, tool_document(&tool), lang)
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok((StatusCode::CREATED, Json(json!({ "tool": tool }))).into_response())
}

/// Modification : une fraise de base modifiée est enregistrée comme version du lab.
async fn update_tool(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Path(id): Path<String>,
    Json(mut tool): Json<ToolSpec>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let (shown, base) = (state.catalog(lang).await, state.catalog(Lang::Fr).await);
    let (Some(shown), Some(base)) = (shown.tool(&id), base.tool(&id)) else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "unknown_tool",
            tr!(lang, "Fraise inconnue : {id}", "Unknown tool: {id}"),
        ));
    };
    keep_base(&mut tool.name, &shown.name, &base.name);
    keep_base(&mut tool.notes, &shown.notes, &base.notes);
    tool.id = id;
    state
        .write_entry_locked(Kind::Tool, &tool.id, tool_document(&tool), lang)
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok(Json(json!({ "tool": tool })).into_response())
}

async fn remove_tool(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let catalog = state.catalog(lang).await;
    if !catalog.layers.lab_tools.contains(&id) {
        return Err(base_entry_error(
            lang,
            catalog.tool(&id).is_some(),
            &id,
            "data/tools/",
        ));
    }
    let restored_base = catalog.layers.overridden.contains(&id);
    state
        .archive_entry_locked(Kind::Tool, &id, lang)
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok(Json(json!({ "archived": id, "restored_base": restored_base })).into_response())
}

async fn create_machine(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Json(mut machine): Json<MachineProfile>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let catalog = state.catalog(lang).await;
    if machine.id.trim().is_empty() {
        machine.id = {
            let reserved = state.reservations();
            unique_id(&machine.name, |id| reserved.contains(Kind::Machine, id))
        };
    } else if catalog.machine(&machine.id).is_some() {
        let id = &machine.id;
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "already_exists",
            tr!(
                lang,
                "La machine {id} existe déjà",
                "Machine {id} already exists"
            ),
        ));
    }
    state
        .create_entry_locked(Kind::Machine, &machine.id, machine_document(&machine), lang)
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok((StatusCode::CREATED, Json(json!({ "machine": machine }))).into_response())
}

async fn update_machine(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Path(id): Path<String>,
    Json(mut machine): Json<MachineProfile>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let (shown, base) = (state.catalog(lang).await, state.catalog(Lang::Fr).await);
    let (Some(shown), Some(base)) = (shown.machine(&id), base.machine(&id)) else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "unknown_machine",
            tr!(lang, "Machine inconnue : {id}", "Unknown machine: {id}"),
        ));
    };
    keep_base(&mut machine.name, &shown.name, &base.name);
    keep_base(&mut machine.category, &shown.category, &base.category);
    keep_base(
        &mut machine.description,
        &shown.description,
        &base.description,
    );
    keep_base(&mut machine.notes, &shown.notes, &base.notes);
    for (field, src) in machine.sources.iter_mut() {
        if let (Some(s), Some(b)) = (shown.sources.get(field), base.sources.get(field)) {
            keep_base(&mut src.note, &s.note, &b.note);
        }
    }
    machine.id = id;
    state
        .write_entry_locked(Kind::Machine, &machine.id, machine_document(&machine), lang)
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok(Json(json!({ "machine": machine })).into_response())
}

async fn remove_machine(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let catalog = state.catalog(lang).await;
    if !catalog.layers.lab_machines.contains(&id) {
        return Err(base_entry_error(
            lang,
            catalog.machine(&id).is_some(),
            &id,
            "data/machines.toml",
        ));
    }
    let restored_base = catalog.layers.overridden.contains(&id);
    state
        .archive_entry_locked(Kind::Machine, &id, lang)
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok(Json(json!({ "archived": id, "restored_base": restored_base })).into_response())
}

/// Création : identifiant `lab/<nom>` attribué par le serveur si absent.
async fn create_material(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Json(mut material): Json<MaterialProfile>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let catalog = state.catalog(lang).await;
    if material.id.trim().is_empty() {
        material.id = {
            let reserved = state.reservations();
            unique_id(&material.name, |id| reserved.contains(Kind::Material, id))
        };
    } else if catalog.material(&material.id).is_some() {
        let id = &material.id;
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "already_exists",
            tr!(
                lang,
                "La matière {id} existe déjà",
                "Material {id} already exists"
            ),
        ));
    }
    state
        .create_entry_locked(
            Kind::Material,
            &material.id,
            material_document(&material),
            lang,
        )
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok((StatusCode::CREATED, Json(json!({ "material": material }))).into_response())
}

/// Modification : une matière de base modifiée est enregistrée comme version du lab.
async fn update_material(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Path(id): Path<String>,
    Json(mut material): Json<MaterialProfile>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let (shown, base) = (state.catalog(lang).await, state.catalog(Lang::Fr).await);
    // Recherche par identifiant exact : un alias ne désigne pas la fiche à écrire.
    let find = |c: &fabspeed_core::Catalog| c.materials.iter().find(|m| m.id == id).cloned();
    let (Some(shown), Some(base)) = (find(&shown), find(&base)) else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "unknown_material",
            tr!(lang, "Matière inconnue : {id}", "Unknown material: {id}"),
        ));
    };
    keep_base(&mut material.name, &shown.name, &base.name);
    keep_base(
        &mut material.description,
        &shown.description,
        &base.description,
    );
    keep_base(&mut material.tips, &shown.tips, &base.tips);
    keep_base(&mut material.source, &shown.source, &base.source);
    material.id = id;
    state
        .write_entry_locked(
            Kind::Material,
            &material.id,
            material_document(&material),
            lang,
        )
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok(Json(json!({ "material": material })).into_response())
}

async fn remove_material(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let catalog = state.catalog(lang).await;
    if !catalog.layers.lab_materials.contains(&id) {
        return Err(base_entry_error(
            lang,
            catalog.materials.iter().any(|m| m.id == id),
            &id,
            "data/materials.toml",
        ));
    }
    let restored_base = catalog.layers.overridden.contains(&id);
    state
        .archive_entry_locked(Kind::Material, &id, lang)
        .await
        .map_err(|e| lab_error(e, lang))?;
    Ok(Json(json!({ "archived": id, "restored_base": restored_base })).into_response())
}

fn base_entry_error(lang: Lang, exists: bool, id: &str, file: &str) -> ApiError {
    if exists {
        ApiError::new(
            StatusCode::CONFLICT,
            "base_entry",
            tr!(
                lang,
                "{id} est une fiche de base ({file}) : elle se retire à la main dans ce fichier.",
                "{id} is a base record ({file}): remove it by hand in that file."
            ),
        )
    } else {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "not_found",
            tr!(lang, "Fiche inconnue : {id}", "Unknown record: {id}"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use std::path::Path as FsPath;
    use tower::ServiceExt;

    /// Copie du jeu livré dans un dossier temporaire : les tests écrivent dedans.
    fn temp_state() -> (tempfile::TempDir, AppState) {
        crate::state::test_state()
    }

    /// Requête en administrateur (session ouverte pour le test).
    async fn call(
        state: &AppState,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let token = state.auth().test_session().await;
        call_as(state, Some(&token), method, uri, body).await
    }

    async fn call_as(
        state: &AppState,
        token: Option<&str>,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let app = Router::new()
            .nest("/api/v1", crate::api::router())
            .with_state(state.clone());
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(t) = token {
            req = req.header("authorization", format!("Bearer {t}"));
        }
        let req = req
            .body(body.map_or(Body::empty(), |b| Body::from(b.to_string())))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
        )
    }

    fn files(dir: &FsPath) -> Vec<String> {
        std::fs::read_dir(dir)
            .map(|r| {
                r.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn new_tool() -> serde_json::Value {
        json!({
            "id": "", "name": "Ma fraise 1/4", "family": "flat_endmill", "helix": "upcut",
            "diameter_mm": 6.35, "flutes": 2, "flute_length_mm": 22
        })
    }

    #[tokio::test]
    async fn lab_01_create_edit_and_archive_a_tool() {
        let (tmp, state) = temp_state();
        let (status, out) = call(&state, "POST", "/api/v1/tools", Some(new_tool())).await;
        assert_eq!(status, StatusCode::CREATED, "{out}");
        let id = out["tool"]["id"].as_str().unwrap().to_string();
        assert_eq!(id, "lab/ma-fraise-1-4");
        assert_eq!(
            files(&tmp.path().join("lab/tools")),
            vec!["lab_ma-fraise-1-4.toml"]
        );
        assert!(state.catalog(Lang::Fr).await.tool(&id).is_some());

        let mut edited = out["tool"].clone();
        edited["flute_length_mm"] = json!(25.0);
        let (status, _) = call(&state, "PUT", &format!("/api/v1/tools/{id}"), Some(edited)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            state
                .catalog(Lang::Fr)
                .await
                .tool(&id)
                .unwrap()
                .flute_length_mm,
            25.0
        );

        let (status, _) = call(&state, "DELETE", &format!("/api/v1/tools/{id}"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(state.catalog(Lang::Fr).await.tool(&id).is_none());
        assert_eq!(
            files(&tmp.path().join("lab/archive/tools")).len(),
            1,
            "archivée, pas détruite"
        );
    }

    #[tokio::test]
    async fn lab_02_editing_a_base_tool_creates_a_lab_version() {
        let (_tmp, state) = temp_state();
        let id = "generic/diamond-cut-3";
        let count = state.catalog(Lang::Fr).await.tools.len();
        let mut tool =
            serde_json::to_value(state.catalog(Lang::Fr).await.tool(id).unwrap()).unwrap();
        tool["flutes"] = json!(4); // dents comptées sur la fraise
        let (status, _) = call(&state, "PUT", &format!("/api/v1/tools/{id}"), Some(tool)).await;
        assert_eq!(status, StatusCode::OK);
        let c = state.catalog(Lang::Fr).await;
        assert_eq!(c.tool(id).unwrap().flutes, Some(4));
        assert!(c.layers.overridden.contains(id));
        assert_eq!(c.tools.len(), count, "remplacée, pas dupliquée");
        assert!(c.meta.revision.contains("+lab."));

        let (status, out) = call(&state, "DELETE", &format!("/api/v1/tools/{id}"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(out["restored_base"], true);
        assert_eq!(state.catalog(Lang::Fr).await.tool(id).unwrap().flutes, None);
        let (status, out) = call(&state, "DELETE", &format!("/api/v1/tools/{id}"), None).await;
        assert_eq!(status, StatusCode::CONFLICT, "{out}");
    }

    #[tokio::test]
    async fn lab_03_invalid_data_is_rolled_back() {
        let (tmp, state) = temp_state();
        let count = state.catalog(Lang::Fr).await.tools.len();
        let mut bad = new_tool();
        bad["diameter_mm"] = json!(0.0);
        let (status, out) = call(&state, "POST", "/api/v1/tools", Some(bad)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{out}");
        assert!(
            files(&tmp.path().join("lab/tools")).is_empty(),
            "rien n'est resté sur le disque"
        );
        assert_eq!(state.catalog(Lang::Fr).await.tools.len(), count);
    }

    #[tokio::test]
    async fn lab_04_good_trial_becomes_the_recommendation() {
        let (tmp, state) = temp_state();
        let trial = json!({
            "operator": "Camille",
            "machine_id": "shopbot-prs-standard-96x48-2016",
            "material_id": "material/wood.plywood",
            "tool_id": "generic/flat-6-1f-up",
            "operation": "profile", "aggressiveness": "normal", "workholding": "screws_clamps",
            "thickness_mm": 18, "depth_mm": 18, "stepover_ratio": 1,
            "rpm": 15000, "feed_mm_min": 4000, "plunge_mm_min": 400, "doc_mm": 3,
            "outcome": "good", "symptoms": [], "notes": "Bords nets."
        });
        let (status, out) = call(&state, "POST", "/api/v1/trials", Some(trial)).await;
        assert_eq!(status, StatusCode::CREATED, "{out}");
        let id = out["trial"]["id"].as_str().unwrap().to_string();
        assert!(id.starts_with("essai-"));
        assert_eq!(files(&tmp.path().join("lab/trials")).len(), 1);

        let req = json!({
            "machine_id": "shopbot-prs-standard-96x48-2016", "material_id": "material/wood.plywood",
            "tool_id": "generic/flat-6-1f-up", "operation": "profile", "depth_mm": 18
        });
        let (_, reco) = call(&state, "POST", "/api/v1/recommend", Some(req.clone())).await;
        assert_eq!(reco["provenance"]["evidence"], "tested");
        assert_eq!(reco["values"]["feed_mm_min"], 4000.0);
        assert_eq!(reco["history"].as_array().unwrap().len(), 1);

        let (status, _) = call(&state, "DELETE", &format!("/api/v1/trials/{id}"), None).await;
        assert_eq!(status, StatusCode::OK);
        let (_, reco) = call(&state, "POST", "/api/v1/recommend", Some(req)).await;
        assert_eq!(reco["provenance"]["source"], "model");
    }

    #[tokio::test]
    async fn lab_05_new_machine_is_available_for_calculation() {
        let (_tmp, state) = temp_state();
        let mut m = serde_json::to_value(
            state
                .catalog(Lang::Fr)
                .await
                .machine("generic-hobby-router")
                .unwrap(),
        )
        .unwrap();
        m["id"] = json!("");
        m["name"] = json!("Shapeoko de Dominique");
        let (status, out) = call(&state, "POST", "/api/v1/machines", Some(m)).await;
        assert_eq!(status, StatusCode::CREATED, "{out}");
        let id = out["machine"]["id"].as_str().unwrap();
        assert_eq!(id, "lab/shapeoko-de-dominique");
        let req = json!({
            "machine_id": id, "material_id": "material/wood.mdf",
            "tool_id": "generic/flat-3.175-2f-up", "operation": "pocket"
        });
        let (status, reco) = call(&state, "POST", "/api/v1/recommend", Some(req)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(reco["values"]["feed_mm_min"].as_f64().unwrap() <= 5000.0);
    }

    #[tokio::test]
    async fn lab_06_editing_in_english_keeps_french_base_texts() {
        let (_tmp, state) = temp_state();
        let id = "generic/diamond-cut-3";
        let mut tool =
            serde_json::to_value(state.catalog(Lang::En).await.tool(id).unwrap()).unwrap();
        assert_eq!(tool["name"], "Diamond-cut router bit Ø3");
        tool["flutes"] = json!(4);
        let (status, out) = call(
            &state,
            "PUT",
            &format!("/api/v1/tools/{id}?lang=en"),
            Some(tool),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{out}");
        assert_eq!(
            state.catalog(Lang::Fr).await.tool(id).unwrap().name,
            "Fraise taille diamant Ø3"
        );
        assert_eq!(
            state.catalog(Lang::En).await.tool(id).unwrap().name,
            "Diamond-cut router bit Ø3"
        );
        assert_eq!(
            state.catalog(Lang::Fr).await.tool(id).unwrap().flutes,
            Some(4)
        );

        let (status, err) = call(
            &state,
            "DELETE",
            "/api/v1/tools/generic/flat-8-1f-up?lang=en",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(
            err["error"].as_str().unwrap().contains("base record"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn lab_07_guest_cannot_write_admin_can() {
        let (tmp, state) = temp_state();
        let lab_tools = tmp.path().join("lab/tools");

        // Invité (sans jeton, ou jeton inconnu) : refus, rien sur le disque.
        for token in [None, Some("jeton-invente")] {
            let (status, err) =
                call_as(&state, token, "POST", "/api/v1/tools", Some(new_tool())).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{err}");
            assert_eq!(err["code"], "admin_required");
        }
        assert!(files(&lab_tools).is_empty());
        let (status, _) = call_as(
            &state,
            None,
            "DELETE",
            "/api/v1/tools/generic/flat-8-1f-up",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        // Lecture et calcul restent ouverts.
        let (status, auth) = call_as(&state, None, "GET", "/api/v1/auth", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(auth, json!({ "configured": false, "admin": false }));

        // Premier mot de passe : session ouverte, écriture permise.
        let pw = json!({ "password": "copeau-sain" });
        let (status, body) =
            call_as(&state, None, "POST", "/api/v1/auth/setup", Some(pw.clone())).await;
        assert_eq!(status, StatusCode::CREATED);
        let token = body["token"].as_str().unwrap().to_string();
        let (_, auth) = call_as(&state, Some(&token), "GET", "/api/v1/auth", None).await;
        assert_eq!(auth, json!({ "configured": true, "admin": true }));
        let (status, _) = call_as(
            &state,
            Some(&token),
            "POST",
            "/api/v1/tools",
            Some(new_tool()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(files(&lab_tools).len(), 1);

        // Un second « premier » mot de passe est refusé ; mauvais mot de passe : 401.
        let (status, _) = call_as(
            &state,
            None,
            "POST",
            "/api/v1/auth/setup",
            Some(json!({ "password": "pirate-123" })),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        let (status, err) = call_as(
            &state,
            None,
            "POST",
            "/api/v1/auth/login?lang=en",
            Some(json!({ "password": "pirate-123" })),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(err["error"], "Wrong password");

        // Changement de mot de passe réservé à l'admin ; l'ancien jeton est fermé.
        let change = json!({ "current": "copeau-sain", "new": "nouveau-copeau" });
        let (status, _) = call_as(
            &state,
            None,
            "POST",
            "/api/v1/auth/password",
            Some(change.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, body) = call_as(
            &state,
            Some(&token),
            "POST",
            "/api/v1/auth/password",
            Some(change),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let new_token = body["token"].as_str().unwrap().to_string();
        let (_, auth) = call_as(&state, Some(&token), "GET", "/api/v1/auth", None).await;
        assert_eq!(auth["admin"], false);

        // Déconnexion.
        call_as(
            &state,
            Some(&new_token),
            "POST",
            "/api/v1/auth/logout",
            None,
        )
        .await;
        let (status, _) = call_as(
            &state,
            Some(&new_token),
            "POST",
            "/api/v1/tools",
            Some(new_tool()),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn lab_08_admin_validates_and_unvalidates_a_trial() {
        let (tmp, state) = temp_state();
        let trial = |outcome: &str| {
            json!({
                "operator": "Camille",
                "machine_id": "shopbot-prs-standard-96x48-2016",
                "material_id": "material/wood.plywood",
                "tool_id": "generic/flat-6-1f-up",
                "operation": "profile", "aggressiveness": "normal", "workholding": "screws_clamps",
                "stepover_ratio": 1,
                "rpm": 15000, "feed_mm_min": 4000, "plunge_mm_min": 400, "doc_mm": 3,
                "outcome": outcome
            })
        };
        let (_, out) = call(&state, "POST", "/api/v1/trials", Some(trial("good"))).await;
        let good = out["trial"]["id"].as_str().unwrap().to_string();
        let (_, out) = call(&state, "POST", "/api/v1/trials", Some(trial("adjust"))).await;
        let adjust = out["trial"]["id"].as_str().unwrap().to_string();
        let body = json!({ "validated_by": "Stéphane", "note": "Trois pièces propres." });

        // Invité : refusé.
        let uri = format!("/api/v1/trials/{good}/validation");
        let (status, _) = call_as(&state, None, "POST", &uri, Some(body.clone())).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        // Un essai « à ajuster » ne se valide pas ; essai inconnu : 404.
        let (status, err) = call(
            &state,
            "POST",
            &format!("/api/v1/trials/{adjust}/validation"),
            Some(body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{err}");
        let (status, _) = call(
            &state,
            "POST",
            "/api/v1/trials/essai-fantome/validation",
            Some(body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        // Validation : écrite dans le fichier de l'essai, preuve « validated ».
        let (status, out) = call(&state, "POST", &uri, Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{out}");
        assert_eq!(out["trial"]["validation"]["validated_by"], "Stéphane");
        let file =
            std::fs::read_to_string(tmp.path().join(format!("lab/trials/{good}.toml"))).unwrap();
        assert!(file.contains("validated_by = \"Stéphane\""), "{file}");
        let req = json!({
            "machine_id": "shopbot-prs-standard-96x48-2016", "material_id": "material/wood.plywood",
            "tool_id": "generic/flat-6-1f-up", "operation": "profile"
        });
        let (_, reco) = call(&state, "POST", "/api/v1/recommend", Some(req.clone())).await;
        assert_eq!(reco["provenance"]["evidence"], "validated");

        // Retrait : retour à « tested ».
        let (status, out) = call(&state, "DELETE", &uri, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(out["trial"].get("validation").is_none());
        let (_, reco) = call(&state, "POST", "/api/v1/recommend", Some(req)).await;
        assert_eq!(reco["provenance"]["evidence"], "tested");
    }

    #[tokio::test]
    async fn lab_09_materials_can_be_created_edited_and_archived() {
        let (tmp, state) = temp_state();
        let dir = tmp.path().join("lab/materials");
        let new = json!({
            "id": "", "name": "Bambou lamellé", "category": "wood",
            "vc_m_min": 250, "k_fz": 0.03, "k_ap": 0.35, "kc_n_mm2": 45, "fz_min_mm": 0.02
        });

        // Invité refusé.
        let (status, _) =
            call_as(&state, None, "POST", "/api/v1/materials", Some(new.clone())).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(files(&dir).is_empty());

        // Création : identifiant attribué, utilisable aussitôt, signalée hors calibrage.
        let (status, out) = call(&state, "POST", "/api/v1/materials", Some(new)).await;
        assert_eq!(status, StatusCode::CREATED, "{out}");
        assert_eq!(out["material"]["id"], "lab/bambou-lamelle");
        assert_eq!(files(&dir).len(), 1);
        let req = |material: &str| {
            json!({
                "machine_id": "shopbot-prs-standard-96x48-2016", "material_id": material,
                "tool_id": "generic/flat-6-1f-up", "operation": "profile"
            })
        };
        let (status, reco) = call(
            &state,
            "POST",
            "/api/v1/recommend",
            Some(req("lab/bambou-lamelle")),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{reco}");
        let codes = |r: &serde_json::Value| -> Vec<String> {
            r["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w["code"].as_str().unwrap().to_string())
                .collect()
        };
        assert!(codes(&reco).contains(&"material_from_lab".into()));

        // Constante invalide : refusée, rien d'écrit.
        let mut bad = out["material"].clone();
        bad["k_fz"] = json!(0);
        let (status, _) = call(
            &state,
            "PUT",
            "/api/v1/materials/lab/bambou-lamelle",
            Some(bad),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        // Fiche de base modifiée (en anglais) : version du lab, textes français conservés.
        let (_, catalog) = call(&state, "GET", "/api/v1/catalog?lang=en", None).await;
        let mut ply = catalog["materials"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == "material/wood.plywood")
            .unwrap()
            .clone();
        assert_eq!(ply["name"], "Plywood");
        ply["vc_m_min"] = json!(300);
        let (status, out) = call(
            &state,
            "PUT",
            "/api/v1/materials/material/wood.plywood?lang=en",
            Some(ply),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{out}");
        assert_eq!(out["material"]["name"], "Contreplaqué");
        let (_, reco) = call(
            &state,
            "POST",
            "/api/v1/recommend",
            Some(req("material/wood.plywood")),
        )
        .await;
        assert!(codes(&reco).contains(&"material_modified".into()));
        assert_eq!(
            reco["metrics"]["cutting_speed_m_min"]
                .as_f64()
                .map(|v| v > 280.0),
            Some(true)
        );

        // Archiver la version du lab rétablit la base ; une fiche de base ne se supprime pas.
        let (status, out) = call(
            &state,
            "DELETE",
            "/api/v1/materials/material/wood.plywood",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(out["restored_base"], true);
        let (status, _) = call(
            &state,
            "DELETE",
            "/api/v1/materials/material/wood.mdf",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        let (status, _) = call(
            &state,
            "DELETE",
            "/api/v1/materials/lab/bambou-lamelle",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(files(&dir).is_empty());
    }

    #[tokio::test]
    async fn lab_10_metal_speedups_are_reserved_to_the_admin() {
        let (_tmp, state) = temp_state();
        let body = json!({
            "machine_id": "shopbot-prs-standard-96x48-2016",
            "material_id": "material/metal.aluminium",
            "tool_id": "generic/flat-6-1f-up",
            "operation": "pocket", "depth_mm": 3, "stepover_ratio": 0.2,
            "aggressiveness": "normal", "workholding": "screws_clamps",
            "supervised_speedups": true
        });
        let speedups = |out: &serde_json::Value| {
            out["levers"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|l| l["id"] != "shorter_stickout" && l["id"] != "better_workholding")
                .count()
        };
        let (status, guest) = call_as(
            &state,
            None,
            "POST",
            "/api/v1/recommend",
            Some(body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{guest}");
        assert_eq!(speedups(&guest), 0, "invité : option ignorée");
        let (_, admin) = call(&state, "POST", "/api/v1/recommend", Some(body)).await;
        assert!(speedups(&admin) > 0, "{admin}");
    }
}
