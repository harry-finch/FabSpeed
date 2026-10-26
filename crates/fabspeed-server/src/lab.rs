//! Écriture des fiches du lab (`data/lab/`) depuis l'application.
//!
//! Un fichier TOML par fiche (lisible, relisible dans Git), écriture atomique
//! (fichier temporaire puis renommage), puis rechargement complet du
//! catalogue. Si les données deviennent invalides, l'écriture est annulée et
//! l'ancien catalogue reste actif. Une suppression déplace la fiche dans
//! `data/lab/archive/` : rien n'est détruit.

use crate::state::AppState;
use fabspeed_core::toml_data::DataKind;
use fabspeed_core::{tr, Catalog, Lang};
use std::fs;
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Machine,
    Material,
    Tool,
    Trial,
}

impl Kind {
    pub(crate) fn dir(self) -> &'static str {
        match self {
            Kind::Machine => "machines",
            Kind::Material => "materials",
            Kind::Tool => "tools",
            Kind::Trial => "trials",
        }
    }

    pub(crate) fn data_kind(self) -> DataKind {
        match self {
            Kind::Machine => DataKind::Machines,
            Kind::Material => DataKind::Materials,
            Kind::Tool => DataKind::Tools,
            Kind::Trial => DataKind::Trials,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LabError {
    #[error("Ce serveur n'a pas de dossier de données accessible en écriture")]
    NoDataDir,
    #[error("Fiche introuvable dans data/lab : {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error("Erreur d'écriture : {0}")]
    Io(String),
}

impl LabError {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            LabError::NoDataDir => tr!(
                lang,
                "Ce serveur n'a pas de dossier de données accessible en écriture",
                "This server has no writable data folder"
            ),
            LabError::NotFound(id) => tr!(
                lang,
                "Fiche introuvable dans data/lab : {id}",
                "Record not found in data/lab: {id}"
            ),
            LabError::Invalid(m) => m.clone(),
            LabError::Io(e) => tr!(lang, "Erreur d'écriture : {e}", "Write error: {e}"),
        }
    }
}

fn io(e: std::io::Error) -> LabError {
    LabError::Io(e.to_string())
}

/// Nom de fichier sûr dérivé d'un identifiant (`lab/ma-fraise` → `lab_ma-fraise.toml`).
pub fn file_name(id: &str) -> String {
    let safe: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{safe}.toml")
}

/// Identifiant lisible : minuscules ASCII, tirets.
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        let c = match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "sans-nom".into()
    } else {
        out
    }
}

/// `lab/<slug>`, rendu unique par un suffixe numérique.
pub fn unique_id(base: &str, taken: impl Fn(&str) -> bool) -> String {
    let root = format!("lab/{}", slug(base));
    if !taken(&root) {
        return root;
    }
    (2..)
        .map(|n| format!("{root}-{n}"))
        .find(|id| !taken(id))
        .expect("suffixe libre")
}

/// Horodatage UTC RFC 3339 à la seconde.
pub fn now_rfc3339() -> String {
    let now = OffsetDateTime::now_utc();
    now.replace_nanosecond(0)
        .unwrap_or(now)
        .format(&Rfc3339)
        .unwrap_or_default()
}

/// Identifiant d'essai : `essai-AAAAMMJJ-HHMMSS-xxxx`.
pub fn trial_id(recorded_at: &str) -> String {
    let digits: String = recorded_at
        .chars()
        .filter(char::is_ascii_digit)
        .take(14)
        .collect();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!(
        "essai-{}-{}-{:04x}",
        digits.get(..8).unwrap_or(""),
        digits.get(8..).unwrap_or(""),
        (nanos >> 16) & 0xffff
    )
}

fn write_atomic(path: &Path, contents: &str) -> Result<(), LabError> {
    use std::io::Write;
    crate::identity::safe_file(path).map_err(io)?;
    let mut tmp =
        tempfile::NamedTempFile::new_in(path.parent().expect("record parent")).map_err(io)?;
    tmp.write_all(contents.as_bytes()).map_err(io)?;
    tmp.as_file().sync_all().map_err(io)?;
    tmp.persist(path).map_err(|e| io(e.error))?;
    Ok(())
}

/// A filename is only a hint: verify stored ownership, including legacy renamed files.
fn find_file(dir: &Path, kind: Kind, id: &str) -> Result<Option<PathBuf>, LabError> {
    crate::identity::safe_tree(dir).map_err(io)?;
    let direct = dir.join(file_name(id));
    if direct.exists() && crate::identity::record_ids(&direct, kind).map_err(io)? != [id] {
        return Err(LabError::Invalid(format!(
            "identity: {} path belongs to another ID ({id:?})",
            kind.dir()
        )));
    }
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut found = None;
    for e in fs::read_dir(dir).map_err(io)? {
        let p = e.map_err(io)?.path();
        if p.extension().is_some_and(|x| x == "toml") {
            let ids = crate::identity::record_ids(&p, kind).map_err(io)?;
            if ids.iter().any(|x| x == id) {
                if ids.len() != 1 || found.is_some() {
                    return Err(LabError::Invalid(format!(
                        "identity: ambiguous {} ID {id:?}",
                        kind.dir()
                    )));
                }
                found = Some(p);
            }
        }
    }
    Ok(found)
}

impl AppState {
    fn lab_dir(&self, kind: Kind) -> Result<PathBuf, LabError> {
        let root = self.data_dir().ok_or(LabError::NoDataDir)?;
        Ok(root.join("lab").join(kind.dir()))
    }

    /// Écrit (crée ou remplace) une fiche puis recharge ; annule en cas d'échec.
    pub async fn write_entry(
        &self,
        kind: Kind,
        id: &str,
        contents: String,
        lang: Lang,
    ) -> Result<(), LabError> {
        let _guard = self.write_lock().lock().await;
        self.write_entry_locked(kind, id, contents, lang).await
    }

    pub(crate) async fn create_entry_locked(
        &self,
        kind: Kind,
        id: &str,
        contents: String,
        lang: Lang,
    ) -> Result<(), LabError> {
        if !crate::identity::valid_id(id) || self.reservations().contains(kind, id) {
            return Err(LabError::Invalid(format!(
                "identity: reserved or unsafe {} ID {id:?}",
                kind.dir()
            )));
        }
        let mut proposed = Catalog::default();
        proposed
            .merge_toml(kind.data_kind(), "creation", &contents)
            .map_err(|_| LabError::Invalid("identity: invalid record".into()))?;
        crate::identity::check_alias_changes(
            self.catalog(Lang::Fr).await.as_ref(),
            &proposed,
            &self.reservations(),
        )
        .map_err(io)?;
        let dir = self.lab_dir(kind)?;
        crate::identity::safe_lab_dir(self.data_dir().ok_or(LabError::NoDataDir)?, kind)
            .map_err(io)?;
        if find_file(&dir, kind, id)?.is_some() {
            return Err(LabError::Invalid("identity: record already exists".into()));
        }
        let mut r = self.reservations();
        r.reserve(kind, id);
        r.observe(&proposed);
        // Reserve before publication; a failed/interrupted attempt may burn an ID.
        self.retain_reservations(&r).map_err(io)?;
        fs::create_dir_all(&dir).map_err(io)?;
        let path = dir.join(file_name(id));
        crate::identity::create_file(&path, contents.as_bytes()).map_err(io)?;
        if let Err(e) = self.reload_locked(lang).await {
            fs::remove_file(&path).map_err(io)?;
            return Err(LabError::Invalid(format!(
                "identity: creation refused: {e}"
            )));
        }
        self.publication_changed();
        Ok(())
    }

    pub(crate) async fn write_entry_locked(
        &self,
        kind: Kind,
        id: &str,
        contents: String,
        lang: Lang,
    ) -> Result<(), LabError> {
        if !crate::identity::valid_id(id) {
            return Err(LabError::Invalid("identity: unsafe ID".into()));
        }
        let current = self.catalog(Lang::Fr).await;
        let exists = match kind {
            Kind::Machine => current.machines.iter().any(|m| m.id == id),
            Kind::Material => current.materials.iter().any(|m| m.id == id),
            Kind::Tool => current.tools.iter().any(|t| t.id == id),
            Kind::Trial => current.trials.iter().any(|t| t.id == id),
        };
        if !exists {
            return Err(LabError::Invalid(
                "identity: update requires an active exact ID".into(),
            ));
        }
        let mut proposed = Catalog::default();
        proposed
            .merge_toml(kind.data_kind(), "update", &contents)
            .map_err(|_| LabError::Invalid("identity: invalid record".into()))?;
        let matches = match kind {
            Kind::Machine => proposed.machines.len() == 1 && proposed.machines[0].id == id,
            Kind::Material => proposed.materials.len() == 1 && proposed.materials[0].id == id,
            Kind::Tool => proposed.tools.len() == 1 && proposed.tools[0].id == id,
            Kind::Trial => proposed.trials.len() == 1 && proposed.trials[0].id == id,
        };
        crate::identity::check_alias_changes(&current, &proposed, &self.reservations())
            .map_err(io)?;
        if !matches {
            return Err(LabError::Invalid("identity: update ID mismatch".into()));
        }
        crate::identity::safe_lab_dir(self.data_dir().ok_or(LabError::NoDataDir)?, kind)
            .map_err(io)?;
        let dir = self.lab_dir(kind)?;
        fs::create_dir_all(&dir).map_err(io)?;
        let path = find_file(&dir, kind, id)?.unwrap_or_else(|| dir.join(file_name(id)));
        let previous = fs::read_to_string(&path).ok();
        if previous.is_some() {
            write_atomic(&path, &contents)?;
        } else {
            crate::identity::create_file(&path, contents.as_bytes()).map_err(io)?;
        }
        if let Err(e) = self.reload_locked(lang).await {
            match previous {
                Some(text) => write_atomic(&path, &text)?,
                None => fs::remove_file(&path).map_err(io)?,
            }
            return Err(LabError::Invalid(tr!(
                lang,
                "Fiche refusée, rien n'a été enregistré. {e}",
                "Record refused, nothing was saved. {e}"
            )));
        }
        self.publication_changed();
        Ok(())
    }

    /// Déplace une fiche dans `data/lab/archive/` puis recharge.
    pub(crate) async fn archive_entry_locked(
        &self,
        kind: Kind,
        id: &str,
        lang: Lang,
    ) -> Result<(), LabError> {
        let dir = self.lab_dir(kind)?;
        let path = find_file(&dir, kind, id)?.ok_or_else(|| LabError::NotFound(id.to_string()))?;
        let archive = dir
            .parent()
            .expect("data/lab")
            .join("archive")
            .join(kind.dir());
        crate::identity::safe_file(&archive).map_err(io)?;
        crate::identity::safe_file(archive.parent().expect("archive parent")).map_err(io)?;
        fs::create_dir_all(&archive).map_err(io)?;
        let stamp: String = now_rfc3339().chars().filter(char::is_ascii_digit).collect();
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let target = archive.join(format!("{stem}-{stamp}.toml"));
        crate::identity::safe_file(&target).map_err(io)?;
        // Hard link gives no-overwrite semantics. Unsupported filesystems fail closed.
        fs::hard_link(&path, &target).map_err(io)?;
        if let Err(e) = fs::remove_file(&path) {
            let _ = fs::remove_file(&target);
            return Err(io(e));
        }
        if let Err(e) = self.reload_locked(lang).await {
            fs::hard_link(&target, &path).map_err(io)?;
            fs::remove_file(&target).map_err(io)?;
            return Err(LabError::Invalid(tr!(
                lang,
                "Suppression refusée : {e}",
                "Deletion refused: {e}"
            )));
        }
        self.publication_changed();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_file_names_are_safe() {
        assert_eq!(slug("Ma fraise Ø6 — 2 dents"), "ma-fraise-6-2-dents");
        assert_eq!(slug("  "), "sans-nom");
        assert_eq!(
            file_name("generic/flat-6-1f-up"),
            "generic_flat-6-1f-up.toml"
        );
        let taken = |id: &str| id == "lab/v60" || id == "lab/v60-2";
        assert_eq!(unique_id("V60", taken), "lab/v60-3");
        let at = "2026-10-22T09:30:05Z";
        assert!(trial_id(at).starts_with("essai-20261022-093005-"));
    }
}
