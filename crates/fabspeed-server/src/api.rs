//! API REST v1 (JSON). Contrat documenté dans `docs/API.md`.
//!
//! Langue : paramètre `?lang=fr|en`, sinon en-tête `Accept-Language`, sinon
//! français. Elle s'applique aux textes des données (noms, conseils…), aux
//! messages du moteur et aux erreurs ; les codes restent identiques.

use crate::state::AppState;
use axum::extract::{FromRequestParts, Json, State};
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use fabspeed_core::import::import_ivac_toolset;
use fabspeed_core::{
    diagnose, recommend, tr, CatalogIssue, EngineError, Lang, RecommendationRequest, ENGINE_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/catalog", get(catalog))
        .route("/recommend", post(recommend_route))
        .route("/diagnose", post(diagnose_route))
        .route("/import/ivac-toolset", post(import_route))
        .route("/reload", post(reload))
        .merge(crate::publication_api::router())
        .merge(crate::auth::router())
        .merge(crate::backup::router())
        .merge(crate::lab_api::router())
        .merge(crate::library_transfer::router())
}

/// Langue de la requête.
pub struct ReqLang(pub Lang);

impl<S: Send + Sync> FromRequestParts<S> for ReqLang {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let from_query = parts.uri.query().and_then(|q| {
            q.split('&')
                .find_map(|kv| kv.strip_prefix("lang="))
                .map(Lang::from_tag)
        });
        let from_header = || {
            parts
                .headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok())
                .map(Lang::from_accept_language)
        };
        Ok(ReqLang(from_query.or_else(from_header).unwrap_or_default()))
    }
}

/// Erreur JSON : `{ "error": "...", "code": "..." }`.
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: String) -> Self {
        ApiError {
            status,
            code,
            message,
        }
    }

    pub fn engine(e: EngineError, lang: Lang) -> Self {
        let (status, code) = match e {
            EngineError::UnknownMachine(_) => (StatusCode::NOT_FOUND, "unknown_machine"),
            EngineError::UnknownMaterial(_) => (StatusCode::NOT_FOUND, "unknown_material"),
            EngineError::UnknownTool(_) => (StatusCode::NOT_FOUND, "unknown_tool"),
            EngineError::UnknownSymptom(_) => (StatusCode::NOT_FOUND, "unknown_symptom"),
            EngineError::MissingTool => (StatusCode::UNPROCESSABLE_ENTITY, "missing_tool"),
            EngineError::InvalidInput(_) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_input"),
        };
        ApiError {
            status,
            code,
            message: e.message(lang),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({ "error": self.message, "code": self.code })),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    engine_version: &'static str,
    dataset_revision: String,
    languages: Vec<&'static str>,
    machines: usize,
    materials: usize,
    tools: usize,
    presets: usize,
    trials: usize,
}

async fn health(State(state): State<AppState>) -> Json<Health> {
    let c = state.catalog(Lang::Fr).await;
    Json(Health {
        status: "ok",
        engine_version: ENGINE_VERSION,
        dataset_revision: c.meta.revision.clone(),
        languages: Lang::ALL.iter().map(|l| l.code()).collect(),
        machines: c.machines.len(),
        materials: c.materials.len(),
        tools: c.tools.len(),
        presets: c.presets.len(),
        trials: c.trials.len(),
    })
}

async fn catalog(State(state): State<AppState>, ReqLang(lang): ReqLang) -> Response {
    let c = state.catalog(lang).await;
    let issues = state.issues().await;
    #[derive(Serialize)]
    struct Out<'a> {
        engine_version: &'static str,
        #[serde(flatten)]
        catalog: &'a fabspeed_core::Catalog,
        issues: Vec<CatalogIssue>,
    }
    Json(Out {
        engine_version: ENGINE_VERSION,
        catalog: &c,
        issues,
    })
    .into_response()
}

async fn recommend_route(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    headers: HeaderMap,
    Json(mut req): Json<RecommendationRequest>,
) -> Result<Response, ApiError> {
    // Accélérations sur les métaux : décision de l'administrateur seulement.
    if req.supervised_speedups && !crate::auth::is_admin(&headers, &state).await {
        req.supervised_speedups = false;
    }
    let c = state.catalog(lang).await;
    Ok(Json(recommend(&c, &req).map_err(|e| ApiError::engine(e, lang))?).into_response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnoseRequest {
    request: RecommendationRequest,
    symptom_id: String,
}

async fn diagnose_route(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    Json(body): Json<DiagnoseRequest>,
) -> Result<Response, ApiError> {
    let c = state.catalog(lang).await;
    let d = diagnose(&c, &body.request, &body.symptom_id).map_err(|e| ApiError::engine(e, lang))?;
    Ok(Json(d).into_response())
}

/// Corps : le contenu brut d'un fichier `*.ivac-toolset.json`. Rien n'est
/// enregistré : les fraises sont renvoyées à l'appelant.
async fn import_route(ReqLang(lang): ReqLang, body: String) -> Result<Response, ApiError> {
    let tools = import_ivac_toolset(&body, lang).map_err(|e| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_toolset",
            e.message(lang),
        )
    })?;
    Ok(Json(json!({ "tools": tools })).into_response())
}

async fn reload(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
) -> Result<Response, ApiError> {
    state.reload(lang).await.map_err(|e| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "reload_failed",
            tr!(
                lang,
                "Rechargement refusé, données précédentes conservées. {e}",
                "Reload refused, previous data kept. {e}"
            ),
        )
    })?;
    let c = state.catalog(lang).await;
    Ok(Json(json!({ "status": "reloaded", "dataset_revision": c.meta.revision, "issues": state.issues().await }))
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// Données du dépôt en lecture seule (aucune route d'écriture appelée ici).
    /// Jeu livré (sans la base du lab), chargé en mémoire.
    fn app() -> Router {
        let (_tmp, state) = crate::state::test_state();
        Router::new().nest("/api/v1", router()).with_state(state)
    }

    async fn call_with(
        method: &str,
        uri: &str,
        lang: Option<&str>,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(l) = lang {
            req = req.header("accept-language", l);
        }
        let req = req
            .body(body.map_or(Body::empty(), |b| Body::from(b.to_string())))
            .unwrap();
        let res = app().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
        )
    }

    async fn call(
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        call_with(method, uri, None, body).await
    }

    fn plywood_request() -> serde_json::Value {
        json!({
            "machine_id": "shopbot-prs-standard-96x48-2016",
            "material_id": "material/wood.plywood",
            "tool_id": "generic/flat-6-1f-up",
            "operation": "profile",
            "depth_mm": 18.0
        })
    }

    #[tokio::test]
    async fn api_01_health() {
        let (status, body) = call("GET", "/api/v1/health", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        // Fraises génériques livrées (plus celles d'un lab, si présentes sur le poste).
        assert!(body["tools"].as_u64().unwrap() >= 11);
        assert_eq!(body["languages"], json!(["fr", "en"]));
    }

    #[tokio::test]
    async fn api_02_catalog_lists_everything() {
        let (status, body) = call("GET", "/api/v1/catalog", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!body["machines"].as_array().unwrap().is_empty());
        assert!(body["symptoms"].as_array().unwrap().len() >= 5);
        assert!(body["policy"]["modes"]["prudent"].is_object());
        assert_eq!(body["lang"], "fr");
    }

    #[tokio::test]
    async fn api_03_recommend() {
        let (status, body) = call("POST", "/api/v1/recommend", Some(plywood_request())).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(
            body["provenance"]["source"], "model",
            "le jeu livré part du modèle"
        );
        assert!(body["values"]["passes"].as_u64().unwrap() >= 6);
        assert_eq!(body["history"], json!([]));
    }

    #[tokio::test]
    async fn api_04_errors_are_json() {
        let mut req = plywood_request();
        req["material_id"] = json!("material/unknown");
        let (status, body) = call("POST", "/api/v1/recommend", Some(req)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["code"], "unknown_material");
        let (status, _) = call("POST", "/api/v1/recommend", Some(json!({ "bad": true }))).await;
        assert!(status.is_client_error());
    }

    #[tokio::test]
    async fn api_05_diagnose() {
        let body = json!({ "request": plywood_request(), "symptom_id": "burning" });
        let (status, out) = call("POST", "/api/v1/diagnose", Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{out}");
        assert!(
            out["after"]["values"]["rpm"].as_f64().unwrap()
                < out["before"]["rpm"].as_f64().unwrap()
        );
    }

    #[tokio::test]
    async fn api_06_import_fabcam_toolset() {
        let text = include_str!("../../../docs/examples/fabcam-toolset.example.json");
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/import/ivac-toolset")
            .body(Body::from(text))
            .unwrap();
        let res = app().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let out: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(out["tools"].as_array().unwrap().len(), 4);
    }

    #[tokio::test]
    async fn api_07_language_from_header_or_query() {
        // Même calcul, mêmes codes : seuls les textes changent.
        let (_, fr) = call_with(
            "POST",
            "/api/v1/recommend",
            Some("fr-FR"),
            Some(plywood_request()),
        )
        .await;
        let (_, en) = call_with(
            "POST",
            "/api/v1/recommend",
            Some("en-GB,en;q=0.9"),
            Some(plywood_request()),
        )
        .await;
        assert_eq!(fr["values"], en["values"]);
        assert_eq!(en["context"]["material_name"], "Plywood");
        assert_eq!(fr["context"]["material_name"], "Contreplaqué");
        let codes = |v: &serde_json::Value| {
            v["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w["code"].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(codes(&fr), codes(&en));
        assert!(en["trace"][0]["label"]
            .as_str()
            .unwrap()
            .contains("Spindle speed"));
        assert_eq!(en["trace"][0]["key"], fr["trace"][0]["key"]);

        let (_, cat) = call("GET", "/api/v1/catalog?lang=en", None).await;
        assert_eq!(cat["lang"], "en");
        let mut bad = plywood_request();
        bad["tool_id"] = json!("nope");
        let (_, err) = call_with("POST", "/api/v1/recommend", Some("en"), Some(bad)).await;
        assert_eq!(err["error"], "Unknown tool: nope");
    }
}
