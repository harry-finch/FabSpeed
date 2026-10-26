//! Installation des données sur un poste (application de bureau).
//!
//! Les données de base livrées avec l'application (machines, matières,
//! fraises, symptômes, traductions) sont copiées dans le dossier de données
//! du poste au premier lancement. Quand une nouvelle version livre une autre
//! révision (`dataset.toml`), elles sont remplacées après une copie de
//! sécurité complète dans `backups/` ; la base du lab (`lab/`), le mot de
//! passe (`auth.toml`), la politique locale (`policy.toml`) et les copies de
//! sécurité ne sont jamais touchés.

use crate::backup::{make_backup, stamp};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Fichiers de base remplacés lors d'une mise à jour.
pub const BASE_FILES: [&str; 4] = [
    "dataset.toml",
    "machines.toml",
    "materials.toml",
    "advisor.toml",
];
/// Dossiers de base remplacés lors d'une mise à jour.
pub const BASE_DIRS: [&str; 3] = ["tools", "presets", "i18n"];

#[derive(Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    /// Premier lancement : données de base copiées.
    Fresh,
    /// Même révision que les données livrées : rien à faire.
    UpToDate,
    /// Données de base remplacées ; copie de sécurité des données d'avant.
    Updated {
        from: String,
        to: String,
        safety_backup: PathBuf,
    },
}

/// Révision déclarée dans `<dir>/dataset.toml`.
pub fn revision(dir: &Path) -> io::Result<String> {
    let text = fs::read_to_string(dir.join("dataset.toml"))?;
    let value: toml::Table =
        toml::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    value
        .get("revision")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "dataset.toml sans révision"))
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let p = e?.path();
        let target = to.join(p.file_name().unwrap_or_default());
        if p.is_dir() {
            copy_dir(&p, &target)?;
        } else {
            fs::copy(&p, &target)?;
        }
    }
    Ok(())
}

fn copy_base(bundled: &Path, data_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(data_dir)?;
    for f in BASE_FILES {
        let src = bundled.join(f);
        if src.is_file() {
            fs::copy(&src, data_dir.join(f))?;
        }
    }
    for d in BASE_DIRS {
        let src = bundled.join(d);
        if src.is_dir() {
            copy_dir(&src, &data_dir.join(d))?;
        }
    }
    Ok(())
}

/// Installe ou met à jour les données de base de `bundled` dans `data_dir`.
pub fn install_base(bundled: &Path, data_dir: &Path) -> io::Result<InstallOutcome> {
    let _lease = crate::identity::DataLease::acquire(data_dir)?;
    crate::identity::safe_tree(data_dir)?;
    if data_dir.join("dataset.toml").is_file() {
        // Reserve base IDs/versions BEFORE replacing the shipped data.
        let r = crate::identity::inspect(data_dir)?;
        r.save(data_dir)?;
    }
    let to = revision(bundled)?;
    let installed = data_dir
        .join("dataset.toml")
        .is_file()
        .then(|| revision(data_dir))
        .transpose()?;
    match installed {
        None => {
            copy_base(bundled, data_dir)?;
            Ok(InstallOutcome::Fresh)
        }
        Some(from) if from == to => Ok(InstallOutcome::UpToDate),
        Some(from) => {
            let backups = data_dir.join("backups");
            fs::create_dir_all(&backups)?;
            let safety_backup = backups.join(format!("avant-mise-a-jour-{}.json", stamp()));
            let backup = make_backup(data_dir, &from)?;
            fs::write(
                &safety_backup,
                serde_json::to_string_pretty(&backup).expect("sauvegarde sérialisable"),
            )?;
            for f in BASE_FILES {
                let target = data_dir.join(f);
                if target.is_file() {
                    fs::remove_file(&target)?;
                }
            }
            for d in BASE_DIRS {
                let target = data_dir.join(d);
                if target.is_dir() {
                    fs::remove_dir_all(&target)?;
                }
            }
            copy_base(bundled, data_dir)?;
            Ok(InstallOutcome::Updated {
                from,
                to,
                safety_backup,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_data() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    #[test]
    fn inst_01_first_launch_then_update_keeps_the_lab() {
        let bundled_tmp = tempfile::tempdir().unwrap();
        let bundled = bundled_tmp.path();
        copy_base(&repo_data(), bundled).unwrap();
        let station = tempfile::tempdir().unwrap();
        let data = station.path().join("data");

        // Premier lancement : données de base copiées, catalogue chargeable.
        assert_eq!(install_base(bundled, &data).unwrap(), InstallOutcome::Fresh);
        assert!(data.join("machines.toml").is_file() && data.join("tools").is_dir());
        assert!(!data.join("lab").exists(), "aucune donnée du lab livrée");
        crate::AppState::from_dir(data.clone()).unwrap();
        assert_eq!(
            install_base(bundled, &data).unwrap(),
            InstallOutcome::UpToDate
        );

        // Le poste vit : un essai, un mot de passe, une politique locale.
        fs::create_dir_all(data.join("lab/trials")).unwrap();
        fs::write(data.join("lab/trials/essai.toml"), "# essai du poste\n").unwrap();
        fs::write(data.join("auth.toml"), "admin_password_hash = \"x\"\n").unwrap();
        fs::write(data.join("policy.toml"), "# politique locale\n").unwrap();
        fs::write(
            data.join("tools/obsolete.toml"),
            "# retirée de la nouvelle version\n",
        )
        .unwrap();

        // Nouvelle version des données livrées.
        let ds = fs::read_to_string(bundled.join("dataset.toml")).unwrap();
        let old = revision(bundled).unwrap();
        fs::write(
            bundled.join("dataset.toml"),
            ds.replace(&old, "2099-01-01.1"),
        )
        .unwrap();
        let outcome = install_base(bundled, &data).unwrap();
        let InstallOutcome::Updated {
            from,
            to,
            safety_backup,
        } = outcome
        else {
            panic!("mise à jour attendue");
        };
        assert_eq!((from.as_str(), to.as_str()), (old.as_str(), "2099-01-01.1"));
        assert_eq!(revision(&data).unwrap(), "2099-01-01.1");
        // Base du lab, mot de passe et politique intacts ; ancienne base remplacée.
        assert!(data.join("lab/trials/essai.toml").is_file());
        assert!(data.join("auth.toml").is_file() && data.join("policy.toml").is_file());
        assert!(!data.join("tools/obsolete.toml").exists());
        // Copie de sécurité complète d'avant la mise à jour.
        let saved = fs::read_to_string(&safety_backup).unwrap();
        assert!(saved.contains("obsolete.toml") && saved.contains("lab/trials/essai.toml"));
        assert!(!saved.contains("admin_password_hash"));
    }
}
