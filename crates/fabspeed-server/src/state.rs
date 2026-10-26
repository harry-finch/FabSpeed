//! État partagé : catalogues (un par langue) rechargeables à chaud.

use crate::auth::Auth;
use crate::loader::{load_all, LoadError};
use fabspeed_core::{Catalog, CatalogIssue, Lang};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Clone)]
pub struct AppState {
    source: Arc<std::sync::Mutex<crate::source::SourceIdentity>>,
    pub(crate) publication: Arc<crate::publication::Publisher>,
    inner: Arc<RwLock<Loaded>>,
    data_dir: Option<PathBuf>,
    /// Une seule écriture de fiche à la fois.
    write_lock: Arc<Mutex<()>>,
    auth: Arc<Auth>,
    _lease: Arc<crate::identity::DataLease>,
    reservations: Arc<std::sync::Mutex<crate::identity::Reservations>>,
}

pub struct Loaded {
    pub catalogs: BTreeMap<Lang, Arc<Catalog>>,
    pub issues: Vec<CatalogIssue>,
}

impl Loaded {
    fn from(catalogs: BTreeMap<Lang, Catalog>, issues: Vec<CatalogIssue>) -> Self {
        Loaded {
            catalogs: catalogs
                .into_iter()
                .map(|(l, c)| (l, Arc::new(c)))
                .collect(),
            issues,
        }
    }
}

impl AppState {
    pub fn from_dir(dir: PathBuf) -> Result<Self, LoadError> {
        let dir = std::fs::canonicalize(&dir).map_err(|e| LoadError::Invalid(e.to_string()))?;
        let lease = crate::identity::DataLease::acquire(&dir)
            .map_err(|e| LoadError::Invalid(e.to_string()))?;
        let reservations =
            crate::identity::inspect(&dir).map_err(|e| LoadError::Invalid(e.to_string()))?;
        reservations
            .save(&dir)
            .map_err(|e| LoadError::Invalid(e.to_string()))?;
        let data = load_all(&dir, Lang::Fr)?;
        let auth = Auth::load(Some(&dir)).map_err(LoadError::Invalid)?;
        let source = crate::source::SourceIdentity::load_or_create(&dir)
            .map_err(|_| LoadError::Invalid("source metadata unavailable".into()))?;
        Ok(Self {
            publication: Arc::new(crate::publication::Publisher::load(&dir, &source)),
            source: Arc::new(std::sync::Mutex::new(source)),
            inner: Arc::new(RwLock::new(Loaded::from(data.catalogs, data.issues))),
            data_dir: Some(dir),
            write_lock: Arc::default(),
            auth: Arc::new(auth),
            _lease: Arc::new(lease),
            reservations: Arc::new(std::sync::Mutex::new(reservations)),
        })
    }

    /// Identité privée de la source, stable pour la durée du bail.
    pub fn source(&self) -> crate::source::SourceIdentity {
        self.source.lock().expect("source mutex").clone()
    }

    pub(crate) fn set_source(&self, source: crate::source::SourceIdentity) {
        *self.source.lock().expect("source mutex") = source;
    }

    pub fn set_exchange_hint(&self, path: PathBuf) {
        self.publication.set_hint(path);
    }

    /// Rôles : mot de passe administrateur et sessions.
    #[cfg(all(test, unix))]
    pub(crate) async fn replace_test_catalog(&self, catalog: Catalog) {
        self.inner
            .write()
            .await
            .catalogs
            .insert(Lang::Fr, Arc::new(catalog));
    }
    pub fn auth(&self) -> &Auth {
        &self.auth
    }

    pub fn data_dir(&self) -> Option<&Path> {
        self.data_dir.as_deref()
    }

    pub(crate) async fn publication_guard(&self) -> tokio::sync::OwnedMutexGuard<()> {
        self.write_lock.clone().lock_owned().await
    }
    pub fn write_lock(&self) -> &Mutex<()> {
        &self.write_lock
    }

    /// Catalogue dans la langue demandée (textes de données et messages du moteur).
    pub async fn catalog(&self, lang: Lang) -> Arc<Catalog> {
        let inner = self.inner.read().await;
        inner
            .catalogs
            .get(&lang)
            .or_else(|| inner.catalogs.get(&Lang::Fr))
            .cloned()
            .expect("catalogue français")
    }

    pub async fn issues(&self) -> Vec<CatalogIssue> {
        self.inner.read().await.issues.clone()
    }

    /// Relit `data/`. En cas d'erreur (rédigée en `lang`), l'ancien catalogue reste actif.
    pub async fn reload(&self, lang: Lang) -> Result<(), LoadError> {
        let _guard = self.write_lock.lock().await;
        if let Some(dir) = self.data_dir() {
            let r = crate::identity::inspect(dir).map_err(|e| LoadError::Invalid(e.to_string()))?;
            self.retain_reservations(&r)
                .map_err(|e| LoadError::Invalid(e.to_string()))?;
        }
        self.reload_locked(lang).await?;
        self.publication_changed();
        Ok(())
    }

    pub(crate) fn reservations(&self) -> crate::identity::Reservations {
        self.reservations.lock().expect("reservation mutex").clone()
    }

    pub(crate) fn retain_reservations(
        &self,
        incoming: &crate::identity::Reservations,
    ) -> std::io::Result<()> {
        let mut guard = self.reservations.lock().expect("reservation mutex");
        let mut merged = guard.clone();
        merged.merge(incoming);
        if let Some(dir) = self.data_dir() {
            merged.save(dir)?;
        }
        *guard = merged;
        Ok(())
    }

    // Caller holds write_lock, including allocation and rollback.
    pub(crate) async fn reload_locked(&self, lang: Lang) -> Result<(), LoadError> {
        let Some(dir) = &self.data_dir else {
            return Ok(());
        };
        let data = load_all(dir, lang)?;
        crate::identity::validate_aliases(&data.catalogs[&Lang::Fr])
            .map_err(|e| LoadError::Invalid(e.to_string()))?;
        let mut r = self.reservations();
        r.observe(&data.catalogs[&Lang::Fr]);
        self.retain_reservations(&r)
            .map_err(|e| LoadError::Invalid(e.to_string()))?;
        *self.inner.write().await = Loaded::from(data.catalogs, data.issues);
        Ok(())
    }
}

/// Données de base du dépôt copiées dans un dossier temporaire, **sans** la
/// base du lab (`data/lab/`) ni le mot de passe du poste : les tests partent
/// toujours du jeu livré, quoi que contienne la base réelle, et n'y écrivent jamais.
#[cfg(test)]
pub fn test_state() -> (tempfile::TempDir, AppState) {
    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap() {
            let p = e.unwrap().path();
            let name = p.file_name().unwrap();
            if name == "lab" || name == crate::auth::AUTH_FILE || name == ".DS_Store" {
                continue;
            }
            let target = to.join(name);
            if p.is_dir() {
                copy(&p, &target);
            } else {
                std::fs::copy(&p, &target).unwrap();
            }
        }
    }
    let tmp = tempfile::tempdir().unwrap();
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        tmp.path(),
    );
    let state = AppState::from_dir(tmp.path().to_path_buf()).unwrap();
    (tmp, state)
}
