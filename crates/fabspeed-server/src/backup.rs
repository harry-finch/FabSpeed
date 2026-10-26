//! Sauvegarde, restauration et export des données du poste.
//!
//! Une sauvegarde est **un fichier JSON** qui contient toutes les données du
//! poste telles qu'elles sont sur le disque : machines, matières, fraises,
//! réglages de référence, symptômes, traductions, et la base du lab
//! (`data/lab/` : essais, validations, fiches ajoutées ou modifiées,
//! archives). Lisible, sans dépendance, restaurable sur un autre poste. Le
//! mot de passe administrateur (`auth.toml`) et les copies de sécurité n'en
//! font jamais partie.
//!
//! Restaurer remplace toutes ces données, en trois temps : vérification
//! (chemins sûrs, fichiers lisibles, catalogue complet cohérent) ; copie des
//! données actuelles dans `data/backups/` ; échange des fichiers puis
//! rechargement, avec retour arrière en cas d'échec.
//!
//! Format 1 (première version) : base du lab seule, chemins relatifs à
//! `data/lab/`. Toujours accepté : il ne remplace alors que la base du lab.

use crate::api::{ApiError, ReqLang};
use crate::auth::Admin;
use crate::lab::now_rfc3339;
use crate::loader::load_all;
use crate::state::AppState;
use axum::extract::{DefaultBodyLimit, Json, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use fabspeed_core::{tr, Catalog, Lang, ENGINE_VERSION};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Identifiant du format de sauvegarde.
pub const FORMAT: &str = "fabspeed-lab-backup";
/// Version du format : 2 = toutes les données du poste ; 1 = base du lab seule.
pub const VERSION: u32 = 2;
/// Taille maximale d'une sauvegarde envoyée pour restauration.
pub const MAX_BACKUP_BYTES: usize = 50 * 1024 * 1024;
/// Fichiers de premier niveau du dossier de données repris dans une sauvegarde.
const DATA_FILES: [&str; 5] = [
    "dataset.toml",
    "machines.toml",
    "materials.toml",
    "advisor.toml",
    "policy.toml",
];
/// Dossiers repris dans une sauvegarde (récursivement pour `lab/`).
const DATA_DIRS: [&str; 4] = ["tools", "presets", "i18n", "lab"];
/// Extensions de fichiers reprises (fiches, bibliothèques FabCAM, notes).
const EXTENSIONS: [&str; 3] = ["toml", "json", "md"];
/// Copies de sécurité faites avant chaque restauration.
const BACKUPS_DIR: &str = "backups";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    pub format: String,
    pub version: u32,
    pub created_at: String,
    #[serde(default)]
    pub engine_version: String,
    #[serde(default)]
    pub dataset_revision: String,
    pub files: Vec<BackupFile>,
    /// Optional for legacy backups. Contains hashes/IDs only, never source identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_reservations: Option<crate::identity::Reservations>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupFile {
    /// Chemin relatif au dossier de données (format 1 : à `data/lab/`), séparé par `/`.
    pub path: String,
    pub content: String,
}

impl Backup {
    /// Fichiers avec leur chemin relatif au dossier de données, quel que soit le format.
    fn data_files(&self) -> Vec<BackupFile> {
        self.files
            .iter()
            .map(|f| BackupFile {
                path: if self.version == 1 {
                    format!("lab/{}", f.path)
                } else {
                    f.path.clone()
                },
                content: f.content.clone(),
            })
            .collect()
    }

    /// Éléments du dossier de données remplacés par la restauration.
    fn replaced(&self) -> Vec<&'static str> {
        if self.version == 1 {
            vec!["lab"]
        } else {
            DATA_FILES.iter().chain(&DATA_DIRS).copied().collect()
        }
    }
}

/// Contenu des données : nombre de fiches par type.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Counts {
    pub machines: usize,
    pub materials: usize,
    pub tools: usize,
    /// Fiches ajoutées ou modifiées au lab (machines, matières, fraises).
    pub lab_records: usize,
    pub trials: usize,
    pub validated: usize,
    pub archived: usize,
}

/// Résultat de la vérification d'une sauvegarde (avant restauration).
#[derive(Debug, Clone, Serialize)]
pub struct RestoreCheck {
    #[serde(skip)]
    reservations: crate::identity::Reservations,
    pub created_at: String,
    pub engine_version: String,
    pub dataset_revision: String,
    /// `false` (format 1) : seule la base du lab est remplacée.
    pub full: bool,
    /// Contenu de la sauvegarde.
    pub backup: Counts,
    /// Contenu actuel (remplacé par la restauration).
    pub current: Counts,
    /// Problèmes non bloquants (ex. essai dont la fraise n'existe plus).
    pub issues: Vec<String>,
}

fn counts(c: &Catalog, files: &[BackupFile]) -> Counts {
    Counts {
        machines: c.machines.len(),
        materials: c.materials.len(),
        tools: c.tools.len(),
        lab_records: c.layers.lab_machines.len()
            + c.layers.lab_materials.len()
            + c.layers.lab_tools.len(),
        trials: c.trials.len(),
        validated: c.trials.iter().filter(|t| t.validation.is_some()).count(),
        archived: files
            .iter()
            .filter(|f| f.path.starts_with("lab/archive/"))
            .count(),
    }
}

fn kept(p: &Path) -> bool {
    p.extension()
        .and_then(|x| x.to_str())
        .is_some_and(|x| EXTENSIONS.contains(&x))
}

/// Données du poste : fichiers de premier niveau connus et dossiers gérés
/// (récursif), dans un ordre stable. Jamais `auth.toml` ni `backups/`.
fn collect(data_dir: &Path) -> std::io::Result<Vec<BackupFile>> {
    fn rel(root: &Path, p: &Path) -> String {
        p.strip_prefix(root)
            .expect("sous le dossier de données")
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
    fn walk(root: &Path, dir: &Path, out: &mut Vec<BackupFile>) -> std::io::Result<()> {
        let mut entries: Vec<PathBuf> = match fs::read_dir(dir) {
            Ok(r) => r.filter_map(|e| e.ok().map(|e| e.path())).collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(root, &p, out)?;
            } else if kept(&p) {
                out.push(BackupFile {
                    path: rel(root, &p),
                    content: fs::read_to_string(&p)?,
                });
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    for name in DATA_FILES {
        let p = data_dir.join(name);
        if p.is_file() {
            out.push(BackupFile {
                path: name.into(),
                content: fs::read_to_string(&p)?,
            });
        }
    }
    for name in DATA_DIRS {
        walk(data_dir, &data_dir.join(name), &mut out)?;
    }
    Ok(out)
}

/// Sauvegarde de toutes les données du poste.
pub fn make_backup(data_dir: &Path, dataset_revision: &str) -> std::io::Result<Backup> {
    Ok(Backup {
        format: FORMAT.into(),
        version: VERSION,
        created_at: now_rfc3339(),
        engine_version: ENGINE_VERSION.into(),
        dataset_revision: dataset_revision.into(),
        files: collect(data_dir)?,
        identity_reservations: Some(crate::identity::inspect(data_dir)?),
    })
}

/// Chemin sûr (relatif au dossier de données) : sans `..` ni chemin absolu,
/// fichier connu de premier niveau ou fichier d'un dossier géré.
fn safe_path(p: &str) -> bool {
    let path = Path::new(p);
    let parts: Vec<_> = path.components().collect();
    if p.contains('\\') || !parts.iter().all(|c| matches!(c, Component::Normal(_))) {
        return false;
    }
    match parts.as_slice() {
        [file] => DATA_FILES.iter().any(|f| file.as_os_str() == *f),
        [dir, ..] => {
            let max_depth = if dir.as_os_str() == "lab" { 4 } else { 2 };
            DATA_DIRS.iter().any(|d| dir.as_os_str() == *d)
                && parts.len() <= max_depth
                && kept(path)
        }
        [] => false,
    }
}

fn write_files(root: &Path, files: &[BackupFile]) -> std::io::Result<()> {
    fs::create_dir_all(root)?;
    for f in files {
        let target = root.join(&f.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, &f.content)?;
    }
    Ok(())
}

/// Copie les éléments gérés de `from` qui ne sont **pas** remplacés (format 1 : données de base).
fn copy_kept(from: &Path, to: &Path, replaced: &[&str]) -> std::io::Result<()> {
    let current = collect(from)?;
    let kept: Vec<BackupFile> = current
        .into_iter()
        .filter(|f| {
            let top = f.path.split('/').next().unwrap_or_default();
            !replaced.contains(&top)
        })
        .collect();
    write_files(to, &kept)
}

fn invalid(lang: Lang, detail: String) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_backup",
        tr!(
            lang,
            "Sauvegarde refusée, rien n'a été modifié. {detail}",
            "Backup refused, nothing was changed. {detail}"
        ),
    )
}

/// Vérifie une sauvegarde sans rien modifier : elle est chargée en entier
/// dans un dossier temporaire, comme le serveur le ferait après restauration.
pub fn check(
    data_dir: &Path,
    b: &Backup,
    current: &Catalog,
    lang: Lang,
) -> Result<RestoreCheck, ApiError> {
    let (restored, issues, incoming) = catalog_and_reservations(data_dir, b, lang)?;
    let files = b.data_files();
    crate::identity::Reservations::read(data_dir)
        .map_err(|e| invalid(lang, e.to_string()))?
        .check_restore(&incoming)
        .map_err(|e| invalid(lang, e.to_string()))?;
    if let Some(r) = &b.identity_reservations {
        crate::identity::Reservations::read(data_dir)
            .map_err(|e| invalid(lang, e.to_string()))?
            .check_restore(r)
            .map_err(|e| invalid(lang, e.to_string()))?;
    }
    let current_files = collect(data_dir).map_err(|e| invalid(lang, e.to_string()))?;
    Ok(RestoreCheck {
        reservations: incoming,
        created_at: b.created_at.clone(),
        engine_version: b.engine_version.clone(),
        dataset_revision: b.dataset_revision.clone(),
        full: b.version >= 2,
        backup: counts(&restored, &files),
        current: counts(current, &current_files),
        issues,
    })
}

/// Charge une sauvegarde vérifiée dans un dossier temporaire, sans modifier le poste.
pub(crate) fn catalog_for_import(
    data_dir: &Path,
    b: &Backup,
    lang: Lang,
) -> Result<(Catalog, Vec<String>), ApiError> {
    let (catalog, issues, _) = catalog_and_reservations(data_dir, b, lang)?;
    Ok((catalog, issues))
}

fn catalog_and_reservations(
    data_dir: &Path,
    b: &Backup,
    lang: Lang,
) -> Result<(Catalog, Vec<String>, crate::identity::Reservations), ApiError> {
    if b.format != FORMAT {
        return Err(invalid(
            lang,
            tr!(
                lang,
                "Ce fichier n'est pas une sauvegarde FabSpeed.",
                "This file is not a FabSpeed backup."
            ),
        ));
    }
    if b.version > VERSION || b.version == 0 {
        let v = b.version;
        return Err(invalid(
            lang,
            tr!(
                lang,
                "Format {v} non pris en charge par cette version de FabSpeed.",
                "Format {v} is not supported by this FabSpeed version."
            ),
        ));
    }
    if let Some(r) = &b.identity_reservations {
        r.check().map_err(|e| invalid(lang, e.to_string()))?;
    }
    let files = b.data_files();
    let mut seen = std::collections::BTreeSet::new();
    if files.iter().any(|f| !seen.insert(&f.path)) {
        return Err(invalid(lang, "duplicate backup path".into()));
    }
    if let Some(f) = files.iter().find(|f| !safe_path(&f.path)) {
        let p = &f.path;
        return Err(invalid(
            lang,
            tr!(lang, "Chemin non autorisé : {p}", "Path not allowed: {p}"),
        ));
    }
    let io = |e: std::io::Error| invalid(lang, e.to_string());
    let tmp = tempfile::tempdir().map_err(io)?;
    copy_kept(data_dir, tmp.path(), &b.replaced()).map_err(io)?;
    write_files(tmp.path(), &files).map_err(io)?;
    if let Some(r) = &b.identity_reservations {
        r.save(tmp.path()).map_err(io)?;
    }
    let reservations =
        crate::identity::inspect(tmp.path()).map_err(|e| invalid(lang, e.to_string()))?;
    let loaded = load_all(tmp.path(), lang).map_err(|e| invalid(lang, e.to_string()))?;
    let restored = loaded
        .catalogs
        .get(&Lang::Fr)
        .expect("catalogue français")
        .clone();
    Ok((
        restored,
        loaded.issues.into_iter().map(|i| i.message).collect(),
        reservations,
    ))
}

pub(crate) fn stamp() -> String {
    now_rfc3339()
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .chars()
        .enumerate()
        .flat_map(|(i, c)| (i == 8).then_some('-').into_iter().chain([c]))
        .collect()
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/lab/backup", get(download))
        .route(
            "/lab/restore",
            post(restore).layer(DefaultBodyLimit::max(MAX_BACKUP_BYTES)),
        )
        .route("/trials.csv", get(trials_csv))
}

fn data_dir(state: &AppState, lang: Lang) -> Result<PathBuf, ApiError> {
    state.data_dir().map(Path::to_path_buf).ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "read_only",
            tr!(
                lang,
                "Ce serveur n'a pas de dossier de données",
                "This server has no data folder"
            ),
        )
    })
}

fn attachment(name: &str, mime: &str, body: String) -> Response {
    (
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        body,
    )
        .into_response()
}

/// Télécharge toutes les données du poste en un fichier (administrateur).
async fn download(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
) -> Result<Response, ApiError> {
    let _guard = state.write_lock().lock().await;
    let dir = data_dir(&state, lang)?;
    let revision = state.catalog(Lang::Fr).await.meta.revision.clone();
    let backup = make_backup(&dir, &revision)
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "io_error", e.to_string()))?;
    let body = serde_json::to_string_pretty(&backup).expect("sauvegarde sérialisable");
    let name = tr!(
        lang,
        "fabspeed-sauvegarde-{}.json",
        "fabspeed-backup-{}.json",
        stamp()
    );
    Ok(attachment(&name, "application/json", body))
}

#[derive(Deserialize)]
struct RestoreQuery {
    /// `true` : vérifier seulement, sans rien modifier.
    #[serde(default)]
    check: bool,
}

/// Vérifie (`?check=true`) ou restaure une sauvegarde (administrateur).
async fn restore(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Query(q): Query<RestoreQuery>,
    Json(backup): Json<Backup>,
) -> Result<Response, ApiError> {
    let dir = data_dir(&state, lang)?;
    let _guard = state.write_lock().lock().await;
    let current = state.catalog(Lang::Fr).await;
    let summary = check(&dir, &backup, &current, lang)?;
    if q.check {
        return Ok(Json(serde_json::json!({ "check": summary })).into_response());
    }
    let io = |e: std::io::Error| {
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "io_error", e.to_string())
    };
    let mut retained = state.reservations();
    retained.merge(&summary.reservations);
    if let Some(r) = &backup.identity_reservations {
        retained.merge(r);
    }
    state.retain_reservations(&retained).map_err(io)?;
    crate::identity::safe_tree(&dir).map_err(io)?;
    let stamp = stamp();

    // 1. Copie de sécurité des données actuelles (toujours complète).
    let safety = dir
        .join(BACKUPS_DIR)
        .join(format!("avant-restauration-{stamp}.json"));
    fs::create_dir_all(safety.parent().expect("dossier")).map_err(io)?;
    let previous = make_backup(&dir, &current.meta.revision).map_err(io)?;
    crate::identity::create_file(
        &safety,
        serde_json::to_string_pretty(&previous)
            .expect("sérialisable")
            .as_bytes(),
    )
    .map_err(io)?;

    // 2. Nouvelles données écrites à côté, puis échange élément par élément
    //    (même disque : renommages). Les éléments absents de la sauvegarde
    //    (ex. pas de policy.toml) disparaissent, comme dans la sauvegarde.
    let incoming_tmp = tempfile::tempdir_in(&dir).map_err(io)?;
    let old_tmp = tempfile::tempdir_in(&dir).map_err(io)?;
    let incoming = incoming_tmp.path().to_path_buf();
    write_files(&incoming, &backup.data_files()).map_err(io)?;
    let marker = dir.join(crate::identity::PENDING_RESTORE);
    crate::identity::create_file(
        &marker,
        b"Interrupted restore: retain temporary directories and safety backup for review",
    )
    .map_err(io)?;
    // Keep rollback material across process interruption; remove only after successful resolution.
    let incoming = incoming_tmp.keep();
    let old = old_tmp.keep();
    let replaced = backup.replaced();
    let mut moved_old = vec![];
    let mut installed = vec![];
    let mut swap = || -> std::io::Result<()> {
        for name in &replaced {
            if dir.join(name).exists() {
                fs::rename(dir.join(name), old.join(name))?;
                moved_old.push(*name);
            }
            if incoming.join(name).exists() {
                fs::rename(incoming.join(name), dir.join(name))?;
                installed.push(*name);
            }
        }
        Ok(())
    };
    let swapped = swap();

    // 3. Rechargement ; en cas d'échec, les données précédentes reviennent.
    let reloaded = match swapped {
        Ok(()) => state.reload_locked(lang).await.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    if let Err(e) = reloaded {
        for name in installed.iter().rev() {
            let target = dir.join(name);
            if target.is_dir() {
                fs::remove_dir_all(target).map_err(io)?;
            } else {
                fs::remove_file(target).map_err(io)?;
            }
        }
        for name in moved_old.iter().rev() {
            fs::rename(old.join(name), dir.join(name)).map_err(io)?;
        }
        fs::remove_file(&marker).map_err(io)?;
        let _ = fs::remove_dir_all(&incoming);
        let _ = fs::remove_dir_all(&old);
        state.reload_locked(lang).await.ok();
        return Err(invalid(lang, e));
    }
    state.publication_restored().map_err(io)?;
    fs::remove_file(&marker).map_err(io)?;
    let _ = fs::remove_dir_all(&incoming);
    fs::remove_dir_all(&old).map_err(io)?;
    tracing::info!(
        "données restaurées ; copie de sécurité : {}",
        safety.display()
    );
    let safety_name = format!(
        "{BACKUPS_DIR}/{}",
        safety.file_name().unwrap_or_default().to_string_lossy()
    );
    Ok(
        Json(serde_json::json!({ "restored": summary, "safety_backup": safety_name }))
            .into_response(),
    )
}

/// Tous les essais en CSV (lecture : ouvert aux invités).
async fn trials_csv(State(state): State<AppState>, ReqLang(lang): ReqLang) -> Response {
    let catalog = state.catalog(lang).await;
    let name = tr!(
        lang,
        "fabspeed-essais-{}.csv",
        "fabspeed-tests-{}.csv",
        stamp()
    );
    attachment(
        &name,
        "text/csv; charset=utf-8",
        fabspeed_core::export::trials_csv(&catalog),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    #[test]
    fn bak_01_only_known_data_paths_are_allowed() {
        for ok in [
            "machines.toml",
            "materials.toml",
            "dataset.toml",
            "tools/generic.toml",
            "tools/lab.ivac-toolset.json",
            "presets/README.md",
            "i18n/en.toml",
            "lab/trials/essai-1.toml",
            "lab/archive/trials/essai-1-2026.toml",
        ] {
            assert!(safe_path(ok), "{ok}");
        }
        for bad in [
            "auth.toml",
            "backups/x.json",
            "../machines.toml",
            "/etc/machines.toml",
            "lab/../../auth.toml",
            "tools/sub/x.toml",
            "tools/x.sh",
            "lab/a/b/c/d.toml",
            "autre/x.toml",
            "machines.toml.bak",
            "lab\\x.toml",
        ] {
            assert!(!safe_path(bad), "{bad}");
        }
    }

    async fn raw(
        state: &AppState,
        token: Option<&str>,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, String) {
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
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    fn trial() -> Value {
        json!({
            "operator": "Camille",
            "machine_id": "shopbot-prs-standard-96x48-2016",
            "material_id": "material/wood.plywood",
            "tool_id": "generic/flat-6-1f-up",
            "operation": "profile", "aggressiveness": "normal", "workholding": "screws_clamps",
            "stepover_ratio": 1, "rpm": 15000, "feed_mm_min": 4000, "plunge_mm_min": 400, "doc_mm": 3,
            "outcome": "good", "notes": "Bords nets ; \"propres\"."
        })
    }

    fn paths(b: &Value) -> Vec<String> {
        b["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["path"].as_str().unwrap().to_string())
            .collect()
    }

    #[tokio::test]
    async fn bak_02_backup_then_restore_everything_without_loss() {
        let (tmp, state) = crate::state::test_state();
        let admin = state.auth().test_session().await;
        let a = Some(admin.as_str());
        let new_tool = json!({ "id": "", "name": "Ma fraise", "family": "flat_endmill", "helix": "upcut",
            "diameter_mm": 6.35, "flutes": 2, "flute_length_mm": 22 });
        assert_eq!(
            raw(&state, a, "POST", "/api/v1/tools", Some(new_tool))
                .await
                .0,
            StatusCode::CREATED
        );
        let (_, out) = raw(&state, a, "POST", "/api/v1/trials", Some(trial())).await;
        let id = serde_json::from_str::<Value>(&out).unwrap()["trial"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let uri = format!("/api/v1/trials/{id}/validation");
        assert_eq!(
            raw(
                &state,
                a,
                "POST",
                &uri,
                Some(json!({ "validated_by": "Stéphane" }))
            )
            .await
            .0,
            StatusCode::OK
        );

        // Sauvegarde : réservée à l'admin ; machines, matières, fraises de base et base du lab, jamais le mot de passe.
        assert_eq!(
            raw(&state, None, "GET", "/api/v1/lab/backup", None).await.0,
            StatusCode::UNAUTHORIZED
        );
        state.auth().setup("copeau-sain").await.unwrap();
        let (status, text) = raw(&state, a, "GET", "/api/v1/lab/backup", None).await;
        assert_eq!(status, StatusCode::OK);
        let backup: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            (backup["format"].as_str(), backup["version"].as_u64()),
            (Some(FORMAT), Some(2))
        );
        let p = paths(&backup);
        for expected in [
            "machines.toml",
            "materials.toml",
            "advisor.toml",
            "dataset.toml",
            "tools/generic.toml",
            "i18n/en.toml",
        ] {
            assert!(p.iter().any(|x| x == expected), "{expected} absent : {p:?}");
        }
        assert!(
            p.iter().any(|x| x.starts_with("lab/tools/"))
                && p.iter().any(|x| x.starts_with("lab/trials/"))
        );
        assert!(!text.contains("argon2") && !p.iter().any(|x| x.contains("auth")));

        // Les données changent ensuite : machine de base modifiée à la main, fraise archivée, nouvel essai.
        let machines = tmp.path().join("machines.toml");
        let original = fs::read_to_string(&machines).unwrap();
        fs::write(
            &machines,
            original.replace("ShopBot PRS Standard 96×48 (2016)", "ShopBot renommée"),
        )
        .unwrap();
        state.reload(Lang::Fr).await.unwrap();
        assert_eq!(
            raw(&state, a, "DELETE", "/api/v1/tools/lab/ma-fraise", None)
                .await
                .0,
            StatusCode::OK
        );
        raw(&state, a, "POST", "/api/v1/trials", Some(trial())).await;
        let before = state.catalog(Lang::Fr).await;
        assert_eq!(before.trials.len(), 2);
        let tools = before.tools.len() as u64;

        // Vérification : résume la sauvegarde et les données actuelles, ne modifie rien.
        let (status, out) = raw(
            &state,
            a,
            "POST",
            "/api/v1/lab/restore?check=true",
            Some(backup.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{out}");
        let check: Value = serde_json::from_str(&out).unwrap();
        let c = &check["check"];
        assert_eq!(c["full"], true);
        assert_eq!(
            (
                c["backup"]["machines"].as_u64(),
                c["backup"]["materials"].as_u64(),
                c["backup"]["tools"].as_u64()
            ),
            (Some(3), Some(15), Some(tools + 1)),
            "la sauvegarde contient encore la fraise archivée depuis"
        );
        assert_eq!(
            (
                c["backup"]["trials"].as_u64(),
                c["backup"]["validated"].as_u64(),
                c["backup"]["lab_records"].as_u64()
            ),
            (Some(1), Some(1), Some(1))
        );
        assert_eq!(
            (
                c["current"]["trials"].as_u64(),
                c["current"]["archived"].as_u64(),
                c["current"]["tools"].as_u64()
            ),
            (Some(2), Some(1), Some(tools))
        );
        assert_eq!(state.catalog(Lang::Fr).await.trials.len(), 2);

        // Invité : refusé.
        assert_eq!(
            raw(
                &state,
                None,
                "POST",
                "/api/v1/lab/restore",
                Some(backup.clone())
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );

        // Restauration : tout revient, y compris la machine de base ; copie de sécurité complète.
        let (status, out) = raw(
            &state,
            a,
            "POST",
            "/api/v1/lab/restore",
            Some(backup.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{out}");
        assert_eq!(fs::read_to_string(&machines).unwrap(), original);
        let after = state.catalog(Lang::Fr).await;
        assert!(after
            .machines
            .iter()
            .any(|m| m.name == "ShopBot PRS Standard 96×48 (2016)"));
        assert_eq!(after.trials.len(), 1);
        assert_eq!(after.trials[0].id, id);
        assert_eq!(
            after.trials[0].validation.as_ref().unwrap().validated_by,
            "Stéphane"
        );
        assert!(after.tool("lab/ma-fraise").is_some());
        let safety = serde_json::from_str::<Value>(&out).unwrap()["safety_backup"]
            .as_str()
            .unwrap()
            .to_string();
        let saved: Value =
            serde_json::from_str(&fs::read_to_string(tmp.path().join(&safety)).unwrap()).unwrap();
        let sp = paths(&saved);
        assert!(
            sp.iter().any(|x| x == "machines.toml"),
            "copie de sécurité complète"
        );
        assert_eq!(
            sp.iter().filter(|x| x.starts_with("lab/")).count(),
            3,
            "2 essais + 1 fiche archivée"
        );
        assert!(saved["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["content"].as_str().unwrap().contains("ShopBot renommée")));
        // Le mot de passe du poste n'est pas touché, aucun dossier d'échange ne reste.
        assert!(state.auth().login("copeau-sain").await.is_ok());
        let leftovers: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with('.'))
            .collect();
        assert!(
            leftovers.iter().all(|n| n == crate::identity::FILE
                || n == ".fabspeed-writer.lock"
                || n == crate::source::FILE),
            "{leftovers:?}"
        );
        crate::identity::Reservations::read(tmp.path())
            .unwrap()
            .check()
            .unwrap();

        assert_eq!(
            crate::source::SourceIdentity::load_or_create(tmp.path()).unwrap(),
            state.source()
        );
        // Une nouvelle sauvegarde reproduit les mêmes fichiers (aller-retour sans perte).
        let (_, again) = raw(&state, a, "GET", "/api/v1/lab/backup", None).await;
        let again: Value = serde_json::from_str(&again).unwrap();
        assert_eq!(again["files"], backup["files"]);
    }

    #[tokio::test]
    async fn bak_03_invalid_backups_change_nothing() {
        let (tmp, state) = crate::state::test_state();
        let a = state.auth().test_session().await;
        let a = Some(a.as_str());
        raw(&state, a, "POST", "/api/v1/trials", Some(trial())).await;
        let files_before = collect(tmp.path()).unwrap();
        let (_, good) = raw(&state, a, "GET", "/api/v1/lab/backup", None).await;
        let good: Value = serde_json::from_str(&good).unwrap();
        let without = |path: &str| {
            let mut b = good.clone();
            b["files"]
                .as_array_mut()
                .unwrap()
                .retain(|f| f["path"] != path);
            b
        };

        let base = |files: Value| json!({ "format": FORMAT, "version": 2, "created_at": "x", "files": files });
        let cases = [
            json!({ "format": "autre", "version": 2, "created_at": "x", "files": [] }),
            json!({ "format": FORMAT, "version": 99, "created_at": "x", "files": [] }),
            base(json!([{ "path": "../auth.toml", "content": "" }])),
            base(json!([{ "path": "auth.toml", "content": "admin_password_hash = \"x\"" }])),
            // Sauvegarde complète sans matières : les essais et fraises y renvoient, catalogue incohérent.
            without("materials.toml"),
            // Matière du lab incohérente (constante nulle).
            json!({ "format": FORMAT, "version": 1, "created_at": "x", "files": [{ "path": "materials/x.toml", "content":
                "[[materials]]\nid = \"lab/x\"\nname = \"X\"\ncategory = \"wood\"\nvc_m_min = 0\nk_fz = 0.03\nk_ap = 0.3\nkc_n_mm2 = 40\nfz_min_mm = 0.02\n" }] }),
            json!({ "format": FORMAT, "version": 1, "created_at": "x", "files": [{ "path": "tools/x.toml", "content": "ceci n'est pas du TOML [" }] }),
        ];
        for body in cases {
            for uri in ["/api/v1/lab/restore?check=true", "/api/v1/lab/restore"] {
                let (status, out) = raw(&state, a, "POST", uri, Some(body.clone())).await;
                assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body} → {out}");
                assert!(out.contains("invalid_backup"), "{out}");
            }
        }
        assert_eq!(collect(tmp.path()).unwrap(), files_before);
        assert_eq!(state.catalog(Lang::Fr).await.trials.len(), 1);
        assert!(
            !tmp.path().join(BACKUPS_DIR).exists(),
            "pas de copie de sécurité sans restauration"
        );
    }

    #[tokio::test]
    async fn bak_04_trials_export_as_csv_for_everyone() {
        let (_tmp, state) = crate::state::test_state();
        let a = state.auth().test_session().await;
        raw(&state, Some(&a), "POST", "/api/v1/trials", Some(trial())).await;
        let (status, csv) = raw(&state, None, "GET", "/api/v1/trials.csv", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(csv.starts_with("\u{feff}Identifiant;"));
        assert_eq!(csv.lines().count(), 2);
        let (_, en) = raw(&state, None, "GET", "/api/v1/trials.csv?lang=en", None).await;
        assert!(en.starts_with("\u{feff}Id,"));
    }

    #[tokio::test]
    async fn bak_05_format_1_backups_still_restore_the_lab_only() {
        let (tmp, state) = crate::state::test_state();
        let a = state.auth().test_session().await;
        let a = Some(a.as_str());
        let (_, out) = raw(&state, a, "POST", "/api/v1/trials", Some(trial())).await;
        let trial_id = serde_json::from_str::<Value>(&out).unwrap()["trial"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let content =
            fs::read_to_string(tmp.path().join(format!("lab/trials/{trial_id}.toml"))).unwrap();
        // Sauvegarde au format 1 (première version) : chemins relatifs à data/lab/.
        let v1 = json!({ "format": FORMAT, "version": 1, "created_at": "2026-10-22T18:00:00Z",
            "files": [{ "path": format!("trials/{trial_id}.toml"), "content": content }] });
        raw(&state, a, "POST", "/api/v1/trials", Some(trial())).await;
        // Donnée de base modifiée : un format 1 ne doit pas y toucher.
        let materials = tmp.path().join("materials.toml");
        let changed = fs::read_to_string(&materials)
            .unwrap()
            .replace("name = \"MDF\"", "name = \"MDF du lab\"");
        fs::write(&materials, &changed).unwrap();
        state.reload(Lang::Fr).await.unwrap();

        let (status, out) = raw(
            &state,
            a,
            "POST",
            "/api/v1/lab/restore?check=true",
            Some(v1.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{out}");
        assert_eq!(
            serde_json::from_str::<Value>(&out).unwrap()["check"]["full"],
            false
        );
        let (status, out) = raw(&state, a, "POST", "/api/v1/lab/restore", Some(v1)).await;
        assert_eq!(status, StatusCode::OK, "{out}");
        assert_eq!(state.catalog(Lang::Fr).await.trials.len(), 1);
        assert_eq!(
            fs::read_to_string(&materials).unwrap(),
            changed,
            "données de base intactes"
        );
    }
}
