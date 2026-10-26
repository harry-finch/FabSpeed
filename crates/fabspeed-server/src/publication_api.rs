//! Internal authenticated settings only. No public catalogue retrieval route.
use crate::{
    api::ApiError,
    auth::Admin,
    catalogue::{self, Consent},
    publication::{self, Settings},
    AppState,
};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use fabspeed_core::Lang;
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/fabapps/sharing", get(status).post(save))
        .route("/fabapps/sharing/preview", post(preview))
        .route("/fabapps/sharing/publish", post(publish))
}
fn failure(_: impl std::fmt::Display) -> ApiError {
    // Do not reflect paths, trial contents, schema errors or OS diagnostics.
    ApiError::new(StatusCode::CONFLICT,"publication_refused","FabApps: action refused; inspect publication status and destination. / Action refusée ; vérifiez l’état et la destination.".into())
}
async fn status(_: Admin, State(state): State<AppState>) -> Json<Value> {
    let c = state.catalog(Lang::Fr).await;
    let published = state.published_snapshot();
    let shared = |kind: &str, id: &str| {
        published
            .as_ref()
            .and_then(|p| p[kind].as_array())
            .is_some_and(|rows| rows.iter().any(|r| r["id"] == id))
    };
    let mut choices = json!({"machines":[],"tools":[],"materials":[],"trials":[]});
    for (kind, resources) in [
        (
            "machines",
            c.machines
                .iter()
                .map(catalogue::machine)
                .collect::<Vec<_>>(),
        ),
        ("tools", c.tools.iter().map(catalogue::tool).collect()),
        (
            "materials",
            c.materials.iter().map(catalogue::material).collect(),
        ),
    ] {
        choices[kind] = json!(resources
            .into_iter()
            .filter_map(Result::ok)
            .map(|r| json!({"id":r.id(),"name":r.value()["name"],"published":shared(kind,r.id())}))
            .collect::<Vec<_>>());
    }
    let mut trials = vec![];
    for t in &c.trials {
        let consent = Consent {
            enabled: true,
            machines: [t.machine_id.clone()].into(),
            tools: [t.tool_id.clone()].into(),
            materials: [t.material_id.clone()].into(),
            trials: [t.id.clone()].into(),
        };
        if let Ok((p, reasons)) = catalogue::project(&c, &state.source(), &consent) {
            let id = crate::canonical::digest("trial-reference", &json!(t.id)).unwrap_or_default();
            trials.push(json!({"id":t.id,"name":format!("Preset {}",id.get(7..19).unwrap_or("")),"eligible":!p.presets.is_empty(),"published":shared("presets",&format!("trial:{}",id.trim_start_matches("sha256:"))),"reason":reasons.first().map(|r|format!("{r:?}")),"machine":t.machine_id,"tool":t.tool_id,"material":t.material_id}));
        }
    }
    choices["trials"] = json!(trials);
    Json(json!({"status":state.publication_status(),"choices":choices}))
}
async fn save(
    _: Admin,
    State(state): State<AppState>,
    Json(settings): Json<Settings>,
) -> Result<Json<publication::Status>, ApiError> {
    Ok(Json(
        state
            .configure_publication(settings)
            .await
            .map_err(failure)?,
    ))
}
async fn publish(
    _: Admin,
    State(state): State<AppState>,
) -> Result<Json<publication::Status>, ApiError> {
    Ok(Json(state.request_publication().await.map_err(failure)?))
}
async fn preview(
    _: Admin,
    State(state): State<AppState>,
    Json(settings): Json<Settings>,
) -> Result<Json<Value>, ApiError> {
    let _guard = state.write_lock().lock().await;
    let c = state.catalog(Lang::Fr).await;
    publication::check_selection(
        &c,
        &state.source(),
        &settings.consent,
        &state.publication_status().settings.consent,
    )
    .map_err(failure)?;
    let (p, reasons) =
        catalogue::project(&c, &state.source(), &settings.consent).map_err(failure)?;
    let stamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(failure)?;
    let value = p
        .assemble(state.source().generation + 1, &stamp)
        .map_err(failure)?;
    publication::validate(&value).map_err(failure)?;
    Ok(Json(
        json!({"snapshot":value,"excluded":reasons.iter().map(|r|format!("{r:?}")).collect::<Vec<_>>(),"preview_only":true}),
    ))
}
