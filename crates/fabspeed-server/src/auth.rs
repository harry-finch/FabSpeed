//! Rôles : administrateur (mot de passe) et invité.
//!
//! L'invité calcule et consulte ; seul l'administrateur écrit dans `data/lab/`
//! (essais, fraises, machines). Le mot de passe est propre au poste : son
//! empreinte Argon2 est dans `<données>/auth.toml` (jamais le mot de passe en
//! clair, fichier hors Git). Une connexion ouvre une session en mémoire,
//! identifiée par un jeton aléatoire envoyé dans `Authorization: Bearer`.
//! Redémarrer le serveur ferme toutes les sessions.

use crate::api::{ApiError, ReqLang};
use crate::state::AppState;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{FromRequestParts, Json, State};
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use fabspeed_core::{tr, Lang};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

/// Fichier du mot de passe, dans le dossier des données.
pub const AUTH_FILE: &str = "auth.toml";
/// Longueur minimale du mot de passe administrateur.
pub const MIN_PASSWORD_LEN: usize = 8;
/// Une session inactive plus longtemps est fermée.
pub const SESSION_IDLE: Duration = Duration::from_secs(8 * 3600);
/// Attente après un mauvais mot de passe (freine les essais en série).
pub const FAILED_LOGIN_DELAY: Duration = Duration::from_millis(800);

#[derive(Serialize, Deserialize)]
struct AuthFile {
    /// Empreinte Argon2id au format PHC (`$argon2id$…`).
    admin_password_hash: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    NoDataDir,
    AlreadyConfigured,
    NotConfigured,
    TooShort,
    WrongPassword,
    Io(String),
}

impl AuthError {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            AuthError::NoDataDir => tr!(
                lang,
                "Ce serveur n'a pas de dossier de données : mot de passe impossible à enregistrer",
                "This server has no data folder: the password cannot be saved"
            ),
            AuthError::AlreadyConfigured => tr!(
                lang,
                "Un mot de passe administrateur existe déjà sur ce poste",
                "An administrator password already exists on this workstation"
            ),
            AuthError::NotConfigured => tr!(
                lang,
                "Aucun mot de passe administrateur n'est encore défini sur ce poste",
                "No administrator password has been set on this workstation yet"
            ),
            AuthError::TooShort => tr!(
                lang,
                "Le mot de passe doit faire au moins {MIN_PASSWORD_LEN} caractères",
                "The password must be at least {MIN_PASSWORD_LEN} characters long"
            ),
            AuthError::WrongPassword => tr!(lang, "Mot de passe incorrect", "Wrong password"),
            AuthError::Io(e) => tr!(lang, "Erreur d'écriture : {e}", "Write error: {e}"),
        }
    }

    fn api(&self, lang: Lang) -> ApiError {
        let (status, code) = match self {
            AuthError::NoDataDir => (StatusCode::SERVICE_UNAVAILABLE, "read_only"),
            AuthError::AlreadyConfigured => (StatusCode::CONFLICT, "already_configured"),
            AuthError::NotConfigured => (StatusCode::CONFLICT, "not_configured"),
            AuthError::TooShort => (StatusCode::UNPROCESSABLE_ENTITY, "password_too_short"),
            AuthError::WrongPassword => (StatusCode::UNAUTHORIZED, "wrong_password"),
            AuthError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "io_error"),
        };
        ApiError::new(status, code, self.message(lang))
    }
}

pub struct Auth {
    file: Option<PathBuf>,
    hash: RwLock<Option<String>>,
    sessions: Mutex<HashMap<String, Instant>>,
}

impl Auth {
    /// Lit `<dir>/auth.toml` s'il existe. Sans dossier : aucun admin possible.
    pub fn load(dir: Option<&Path>) -> Result<Self, String> {
        let file = dir.map(|d| d.join(AUTH_FILE));
        let hash = match &file {
            Some(f) if f.exists() => {
                let text =
                    std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
                let parsed: AuthFile =
                    toml::from_str(&text).map_err(|e| format!("{}: {e}", f.display()))?;
                PasswordHash::new(&parsed.admin_password_hash)
                    .map_err(|e| format!("{}: {e}", f.display()))?;
                Some(parsed.admin_password_hash)
            }
            _ => None,
        };
        Ok(Auth {
            file,
            hash: RwLock::new(hash),
            sessions: Mutex::default(),
        })
    }

    pub async fn configured(&self) -> bool {
        self.hash.read().await.is_some()
    }

    /// Premier mot de passe du poste ; ouvre une session.
    pub async fn setup(&self, password: &str) -> Result<String, AuthError> {
        let mut hash = self.hash.write().await;
        if hash.is_some() {
            return Err(AuthError::AlreadyConfigured);
        }
        *hash = Some(self.store(password)?);
        Ok(self.open_session().await)
    }

    pub async fn login(&self, password: &str) -> Result<String, AuthError> {
        let ok = match self.hash.read().await.as_deref() {
            None => return Err(AuthError::NotConfigured),
            Some(h) => verify(password, h),
        };
        if !ok {
            tokio::time::sleep(FAILED_LOGIN_DELAY).await;
            return Err(AuthError::WrongPassword);
        }
        Ok(self.open_session().await)
    }

    /// Change le mot de passe : toutes les sessions sont fermées, une nouvelle est ouverte.
    pub async fn change(&self, current: &str, new: &str) -> Result<String, AuthError> {
        let mut hash = self.hash.write().await;
        let ok = match hash.as_deref() {
            None => return Err(AuthError::NotConfigured),
            Some(h) => verify(current, h),
        };
        if !ok {
            tokio::time::sleep(FAILED_LOGIN_DELAY).await;
            return Err(AuthError::WrongPassword);
        }
        *hash = Some(self.store(new)?);
        self.sessions.lock().await.clear();
        Ok(self.open_session().await)
    }

    /// Session valide (et prolongée) ?
    pub async fn check(&self, token: &str) -> bool {
        let mut sessions = self.sessions.lock().await;
        let now = Instant::now();
        sessions.retain(|_, last| now.duration_since(*last) < SESSION_IDLE);
        match sessions.get_mut(token) {
            Some(last) => {
                *last = now;
                true
            }
            None => false,
        }
    }

    pub async fn logout(&self, token: &str) {
        self.sessions.lock().await.remove(token);
    }

    /// Session administrateur sans mot de passe, pour les tests d'écriture.
    #[cfg(test)]
    pub async fn test_session(&self) -> String {
        self.open_session().await
    }

    async fn open_session(&self) -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        self.sessions
            .lock()
            .await
            .insert(token.clone(), Instant::now());
        token
    }

    /// Hache et écrit le mot de passe (fichier temporaire puis renommage).
    fn store(&self, password: &str) -> Result<String, AuthError> {
        if password.chars().count() < MIN_PASSWORD_LEN {
            return Err(AuthError::TooShort);
        }
        let file = self.file.as_ref().ok_or(AuthError::NoDataDir)?;
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::Io(e.to_string()))?
            .to_string();
        let doc = AuthFile {
            admin_password_hash: hash.clone(),
        };
        let text = format!(
            "# Mot de passe administrateur de ce poste (empreinte Argon2id, jamais en clair).\n\
             # Mot de passe oublié : supprimer ce fichier (ou `fabspeed --reset-admin-password`),\n\
             # redémarrer, puis en définir un nouveau depuis l'interface.\n\n{}",
            toml::to_string(&doc).map_err(|e| AuthError::Io(e.to_string()))?
        );
        let tmp = file.with_extension("toml.tmp");
        std::fs::write(&tmp, text).map_err(|e| AuthError::Io(e.to_string()))?;
        std::fs::rename(&tmp, file).map_err(|e| AuthError::Io(e.to_string()))?;
        Ok(hash)
    }
}

fn verify(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|h| {
            Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false)
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

/// La requête porte une session administrateur valide (routes ouvertes à tous
/// dont une option est réservée à l'administrateur).
pub async fn is_admin(headers: &HeaderMap, state: &AppState) -> bool {
    match bearer(headers) {
        Some(token) => state.auth().check(token).await,
        None => false,
    }
}

/// Extracteur des routes d'écriture : refuse (401 `admin_required`) sans session admin.
pub struct Admin;

impl FromRequestParts<AppState> for Admin {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let Ok(ReqLang(lang)) = ReqLang::from_request_parts(parts, state).await;
        if is_admin(&parts.headers, state).await {
            return Ok(Admin);
        }
        Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "admin_required",
            tr!(
                lang,
                "Mode invité : connectez-vous en administrateur pour enregistrer",
                "Guest mode: sign in as administrator to save"
            ),
        ))
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth", get(status))
        .route("/auth/setup", post(setup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/password", post(change_password))
}

async fn status(State(state): State<AppState>, parts: Parts) -> Json<serde_json::Value> {
    let admin = is_admin(&parts.headers, &state).await;
    Json(json!({ "configured": state.auth().configured().await, "admin": admin }))
}

#[derive(Deserialize)]
struct PasswordBody {
    password: String,
}

async fn setup(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    Json(body): Json<PasswordBody>,
) -> Result<Response, ApiError> {
    let token = state
        .auth()
        .setup(&body.password)
        .await
        .map_err(|e| e.api(lang))?;
    tracing::info!("mot de passe administrateur défini");
    Ok((StatusCode::CREATED, Json(json!({ "token": token }))).into_response())
}

async fn login(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    Json(body): Json<PasswordBody>,
) -> Result<Response, ApiError> {
    let token = state
        .auth()
        .login(&body.password)
        .await
        .map_err(|e| e.api(lang))?;
    Ok(Json(json!({ "token": token })).into_response())
}

async fn logout(State(state): State<AppState>, parts: Parts) -> Json<serde_json::Value> {
    if let Some(t) = bearer(&parts.headers) {
        state.auth().logout(t).await;
    }
    Json(json!({ "admin": false }))
}

#[derive(Deserialize)]
struct ChangeBody {
    current: String,
    new: String,
}

async fn change_password(
    State(state): State<AppState>,
    ReqLang(lang): ReqLang,
    _: Admin,
    Json(body): Json<ChangeBody>,
) -> Result<Response, ApiError> {
    let token = state
        .auth()
        .change(&body.current, &body.new)
        .await
        .map_err(|e| e.api(lang))?;
    tracing::info!("mot de passe administrateur modifié");
    Ok(Json(json!({ "token": token })).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn auth_01_setup_login_change_and_sessions() {
        let tmp = tempfile::tempdir().unwrap();
        let auth = Auth::load(Some(tmp.path())).unwrap();
        assert!(!auth.configured().await);
        assert_eq!(
            auth.login("peu importe").await,
            Err(AuthError::NotConfigured)
        );
        assert_eq!(auth.setup("court").await, Err(AuthError::TooShort));

        let t1 = auth.setup("copeau-sain").await.unwrap();
        assert!(auth.check(&t1).await);
        assert_eq!(
            auth.setup("autre-mot").await,
            Err(AuthError::AlreadyConfigured)
        );

        // Empreinte seulement, jamais le mot de passe en clair.
        let file = std::fs::read_to_string(tmp.path().join(AUTH_FILE)).unwrap();
        assert!(file.contains("$argon2id$") && !file.contains("copeau-sain"));

        assert_eq!(auth.login("mauvais").await, Err(AuthError::WrongPassword));
        let t2 = auth.login("copeau-sain").await.unwrap();
        assert_ne!(t1, t2);

        // Relu au redémarrage ; les sessions, elles, sont perdues.
        let reloaded = Auth::load(Some(tmp.path())).unwrap();
        assert!(reloaded.configured().await && !reloaded.check(&t2).await);
        assert!(reloaded.login("copeau-sain").await.is_ok());

        // Changer le mot de passe ferme les autres sessions.
        let t3 = auth.change("copeau-sain", "nouveau-copeau").await.unwrap();
        assert!(!auth.check(&t1).await && !auth.check(&t2).await && auth.check(&t3).await);
        assert!(auth.login("nouveau-copeau").await.is_ok());

        auth.logout(&t3).await;
        assert!(!auth.check(&t3).await);
    }

    #[tokio::test]
    async fn auth_02_without_data_dir_nobody_is_admin() {
        let auth = Auth::load(None).unwrap();
        assert_eq!(auth.setup("copeau-sain").await, Err(AuthError::NoDataDir));
        assert!(!auth.configured().await);
    }

    #[test]
    fn auth_03_corrupt_file_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(AUTH_FILE),
            "admin_password_hash = \"pas une empreinte\"",
        )
        .unwrap();
        assert!(Auth::load(Some(tmp.path())).is_err());
    }
}
