//! Lecture du dossier `data/` → `Catalog`.
//!
//! Disposition attendue (les fichiers d'un même dossier sont lus par ordre
//! alphabétique, pour un résultat déterministe) :
//!
//! ```text
//! data/
//! ├── dataset.toml            métadonnées (révision)
//! ├── machines.toml
//! ├── materials.toml
//! ├── advisor.toml            symptômes du diagnostic
//! ├── policy.toml             (optionnel) facteurs du moteur
//! ├── tools/*.toml            fraises au format FabSpeed
//! ├── tools/*.ivac-toolset.json  bibliothèques FabCAM (import automatique)
//! ├── presets/*.toml          (optionnel) réglages de référence importés
//! ├── i18n/<langue>.toml      traductions des textes (en.toml…)
//! └── lab/                    écrit par l'application (un fichier par fiche)
//!     ├── machines/*.toml     nouvelles machines, ou versions modifiées des machines de base
//!     ├── tools/*.toml        nouvelles fraises, ou versions modifiées des fraises de base
//!     ├── trials/*.toml       essais enregistrés
//!     └── archive/            fiches supprimées depuis l'application (récupérables)
//! ```

use fabspeed_core::import::import_ivac_toolset;
use fabspeed_core::localize::Translations;
use fabspeed_core::toml_data::{parse_translations, DataKind};
use fabspeed_core::{tr, Catalog, CatalogIssue, Lang};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("{path} : {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error(transparent)]
    Data(#[from] fabspeed_core::toml_data::DataError),
    #[error("{0}")]
    Import(String),
    /// Texte déjà rédigé dans la langue demandée.
    #[error("{0}")]
    Invalid(String),
}

fn read(path: &Path) -> Result<String, LoadError> {
    fs::read_to_string(path).map_err(|source| LoadError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn sorted_files(dir: &Path, suffix: &str) -> Result<Vec<PathBuf>, LoadError> {
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let entries = fs::read_dir(dir).map_err(|source| LoadError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().ends_with(suffix))
        })
        .collect();
    files.sort();
    Ok(files)
}

fn name(path: &Path) -> String {
    path.display().to_string()
}

/// Catalogue de base (français) et catalogues traduits, par langue.
pub struct LoadedData {
    pub catalogs: BTreeMap<Lang, Catalog>,
    pub issues: Vec<CatalogIssue>,
}

/// Charge le catalogue et ses traductions. `lang` : langue des messages
/// d'erreur et des problèmes de données.
pub fn load_all(dir: &Path, lang: Lang) -> Result<LoadedData, LoadError> {
    let (base, mut issues) = load_catalog(dir, lang)?;
    let translations = load_translations(dir)?;
    let mut catalogs = BTreeMap::new();
    for l in Lang::ALL {
        let tr = translations.get(&l);
        if let Some(t) = tr {
            for id in base.untranslatable_ids(t) {
                let code = l.code();
                issues.push(CatalogIssue {
                    fatal: false,
                    message: tr!(
                        lang,
                        "Traduction {code} : identifiant inconnu {id}",
                        "Translation {code}: unknown identifier {id}"
                    ),
                });
            }
        }
        catalogs.insert(l, base.localized(l, tr));
    }
    Ok(LoadedData { catalogs, issues })
}

/// Fichiers `data/i18n/<langue>.toml` présents (le français est la langue des données).
pub fn load_translations(dir: &Path) -> Result<BTreeMap<Lang, Translations>, LoadError> {
    let mut out = BTreeMap::new();
    for l in Lang::ALL.into_iter().filter(|l| *l != Lang::Fr) {
        let path = dir.join("i18n").join(format!("{}.toml", l.code()));
        if path.is_file() {
            out.insert(l, parse_translations(&name(&path), &read(&path)?)?);
        }
    }
    Ok(out)
}

/// Charge et valide le catalogue de base. Les problèmes non bloquants sont renvoyés.
pub fn load_catalog(dir: &Path, lang: Lang) -> Result<(Catalog, Vec<CatalogIssue>), LoadError> {
    let mut c = Catalog::default();
    let required = [
        (DataKind::Dataset, "dataset.toml"),
        (DataKind::Machines, "machines.toml"),
        (DataKind::Materials, "materials.toml"),
        (DataKind::Advisor, "advisor.toml"),
    ];
    for (kind, file) in required {
        let path = dir.join(file);
        c.merge_toml(kind, &name(&path), &read(&path)?)?;
    }
    let policy = dir.join("policy.toml");
    if policy.is_file() {
        c.merge_toml(DataKind::Policy, &name(&policy), &read(&policy)?)?;
    }
    for path in sorted_files(&dir.join("tools"), ".toml")? {
        c.merge_toml(DataKind::Tools, &name(&path), &read(&path)?)?;
    }
    for path in sorted_files(&dir.join("tools"), ".ivac-toolset.json")? {
        let tools = import_ivac_toolset(&read(&path)?, Lang::Fr)
            .map_err(|e| LoadError::Import(format!("{} : {e}", name(&path))))?;
        // Une fraise déjà décrite au format FabSpeed garde sa fiche détaillée.
        for t in tools {
            if c.tool(&t.id).is_none() {
                c.tools.push(t);
            }
        }
    }
    for path in sorted_files(&dir.join("presets"), ".toml")? {
        c.merge_toml(DataKind::Presets, &name(&path), &read(&path)?)?;
    }
    let lab_hash = load_lab_layer(&mut c, &dir.join("lab"))?;
    if let Some(h) = lab_hash {
        // La révision change dès que les données du lab changent : traçabilité.
        c.meta.revision = format!("{}+lab.{h}", c.meta.revision);
    }
    // Messages de validation dans la langue demandée ; le catalogue reste en français.
    c.lang = lang;
    let issues = c.validate();
    c.lang = Lang::Fr;
    let fatal: Vec<String> = issues
        .iter()
        .filter(|i| i.fatal)
        .map(|i| format!("  - {}", i.message))
        .collect();
    if !fatal.is_empty() {
        let head = tr!(lang, "Données invalides :", "Invalid data:");
        return Err(LoadError::Invalid(format!("{head}\n{}", fatal.join("\n"))));
    }
    Ok((c, issues))
}

/// Couche écrite par l'application : une fiche du lab de même identifiant
/// qu'une fiche de base la remplace. Renvoie une empreinte du contenu.
fn load_lab_layer(c: &mut Catalog, lab: &Path) -> Result<Option<String>, LoadError> {
    let mut hasher = Fnv::default();
    let mut any = false;
    for path in sorted_files(&lab.join("machines"), ".toml")? {
        let text = read(&path)?;
        hasher.feed(&name(&path), &text);
        any = true;
        let mut part = Catalog::default();
        part.merge_toml(DataKind::Machines, &name(&path), &text)?;
        for m in part.machines {
            c.layers.lab_machines.insert(m.id.clone());
            match c.machines.iter_mut().find(|x| x.id == m.id) {
                Some(slot) => {
                    c.layers.overridden.insert(m.id.clone());
                    *slot = m;
                }
                None => c.machines.push(m),
            }
        }
    }
    for path in sorted_files(&lab.join("materials"), ".toml")? {
        let text = read(&path)?;
        hasher.feed(&name(&path), &text);
        any = true;
        let mut part = Catalog::default();
        part.merge_toml(DataKind::Materials, &name(&path), &text)?;
        for m in part.materials {
            c.layers.lab_materials.insert(m.id.clone());
            match c.materials.iter_mut().find(|x| x.id == m.id) {
                Some(slot) => {
                    c.layers.overridden.insert(m.id.clone());
                    *slot = m;
                }
                None => c.materials.push(m),
            }
        }
    }
    for path in sorted_files(&lab.join("tools"), ".toml")? {
        let text = read(&path)?;
        hasher.feed(&name(&path), &text);
        any = true;
        let mut part = Catalog::default();
        part.merge_toml(DataKind::Tools, &name(&path), &text)?;
        for t in part.tools {
            c.layers.lab_tools.insert(t.id.clone());
            match c.tools.iter_mut().find(|x| x.id == t.id) {
                Some(slot) => {
                    c.layers.overridden.insert(t.id.clone());
                    *slot = t;
                }
                None => c.tools.push(t),
            }
        }
    }
    for path in sorted_files(&lab.join("trials"), ".toml")? {
        let text = read(&path)?;
        hasher.feed(&name(&path), &text);
        any = true;
        c.merge_toml(DataKind::Trials, &name(&path), &text)?;
    }
    Ok(any.then(|| format!("{:06x}", hasher.0 & 0xff_ffff)))
}

/// Empreinte FNV-1a 64 bits (déterministe, sans dépendance).
struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    fn feed(&mut self, name: &str, text: &str) {
        for b in name.bytes().chain([0]).chain(text.bytes()) {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_the_repository_data() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let (c, issues) = load_catalog(&dir, Lang::Fr).unwrap();
        assert!(issues.iter().all(|i| !i.fatal));
        // Les fraises génériques livrées ; un poste peut en avoir d'autres (hors dépôt).
        assert_eq!(
            c.tools
                .iter()
                .filter(|t| t.id.starts_with("generic/"))
                .count(),
            11
        );
        assert!(c.presets.is_empty(), "le jeu livré part du modèle seul");
        assert!(c.machine("shopbot-prs-standard-96x48-2016").is_some());
        assert!(!c.meta.revision.is_empty());
    }

    #[test]
    fn english_translations_load_and_apply() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let data = load_all(&dir, Lang::En).unwrap();
        assert!(
            data.issues
                .iter()
                .all(|i| !i.message.contains("unknown identifier")),
            "{:?}",
            data.issues
        );
        let en = &data.catalogs[&Lang::En];
        assert_eq!(en.lang, Lang::En);
        assert_eq!(
            en.material("material/wood.plywood").unwrap().name,
            "Plywood"
        );
        assert_eq!(
            en.symptom("burning").unwrap().label,
            "Smoke, the wood is burning"
        );
        let fr = &data.catalogs[&Lang::Fr];
        assert_eq!(
            fr.material("material/wood.plywood").unwrap().name,
            "Contreplaqué"
        );
        // Les valeurs numériques ne sont jamais traduites.
        assert_eq!(en.materials.len(), fr.materials.len());
        assert_eq!(
            en.material("material/wood.plywood").unwrap().k_fz,
            fr.material("material/wood.plywood").unwrap().k_fz
        );
    }

    #[test]
    fn missing_directory_is_an_explicit_error() {
        let err = load_catalog(Path::new("/nonexistent-fabspeed"), Lang::Fr).unwrap_err();
        assert!(matches!(err, LoadError::Io { .. }));
    }
}
