//! Local identity reservations. Not a FabApps source identity or a validation record.
//! All mutations require the data-directory lease and AppState's transaction mutex.
use crate::lab::{file_name, Kind};
use fabspeed_core::{Catalog, Lang};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const FILE: &str = ".identity-reservations.json";
pub const PENDING_RESTORE: &str = ".identity-restore-pending";
const LOCK: &str = ".fabspeed-writer.lock";

/// OS lease: never unlink the lock file (another process may already have it open).
#[derive(Debug)]
pub struct DataLease {
    _file: File,
}
impl DataLease {
    pub fn acquire(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        safe_file(&dir.join(LOCK))?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join(LOCK))?;
        fs2::FileExt::try_lock_exclusive(&file).map_err(|_| {
            io::Error::other("identity: data directory already owned by a FabSpeed writer")
        })?;
        Ok(Self { _file: file })
    }
}

pub fn safe_file(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => {
            Err(io::Error::other("identity: symbolic link refused"))
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Check existing descendants before IO. External manual writers are unsupported
/// while the lease is held; this is not a defence against a malicious OS account.
pub fn safe_tree(dir: &Path) -> io::Result<()> {
    safe_file(dir)?;
    if !dir.is_dir() {
        return Ok(());
    }
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        safe_file(&p)?;
        if p.is_dir() {
            safe_tree(&p)?;
        }
    }
    Ok(())
}

pub fn valid_id(id: &str) -> bool {
    !id.trim().is_empty()
        && id.len() <= 512
        && !id.chars().any(char::is_control)
        && !id.contains('\\')
        && !id.starts_with('/')
        && id
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reservations {
    version: u32,
    /// type:id -> fingerprints of versions actually observed. Empty = reference only.
    entries: BTreeMap<String, BTreeSet<String>>,
}
impl Default for Reservations {
    fn default() -> Self {
        Self {
            version: 1,
            entries: BTreeMap::new(),
        }
    }
}
fn key(kind: Kind, id: &str) -> String {
    format!("{}:{id}", kind.dir())
}
pub fn fingerprint<T: Serialize>(value: &T) -> String {
    // Value's map ordering is deterministic; private content is not stored in the ledger.
    let value = serde_json::to_value(value).expect("serializable identity record");
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&value).expect("JSON value"))
    )
}
impl Reservations {
    pub fn read(dir: &Path) -> io::Result<Self> {
        safe_file(&dir.join(FILE))?;
        match fs::read(dir.join(FILE)) {
            Ok(bytes) => {
                let r: Self = serde_json::from_slice(&bytes)
                    .map_err(|_| io::Error::other("identity: unreadable reservation metadata"))?;
                r.check()?;
                Ok(r)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }
    pub fn check(&self) -> io::Result<()> {
        if self.version != 1
            || self.entries.iter().any(|(k, hashes)| {
                !["machines:", "materials:", "tools:", "trials:"]
                    .iter()
                    .any(|p| k.starts_with(p))
                    || hashes
                        .iter()
                        .any(|h| h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()))
            })
        {
            return Err(io::Error::other("identity: invalid reservation metadata"));
        }
        Ok(())
    }
    pub fn contains(&self, kind: Kind, id: &str) -> bool {
        self.entries.contains_key(&key(kind, id))
    }
    pub fn reserve(&mut self, kind: Kind, id: &str) {
        self.entries.entry(key(kind, id)).or_default();
    }
    pub fn record<T: Serialize>(&mut self, kind: Kind, id: &str, value: &T) {
        self.entries
            .entry(key(kind, id))
            .or_default()
            .insert(fingerprint(value));
    }
    pub fn merge(&mut self, other: &Self) {
        for (k, v) in &other.entries {
            self.entries.entry(k.clone()).or_default().extend(v.clone());
        }
    }
    pub fn save(&self, dir: &Path) -> io::Result<()> {
        safe_file(&dir.join(FILE))?;
        let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
        serde_json::to_writer_pretty(&mut tmp, self)?;
        tmp.as_file().sync_all()?;
        tmp.persist(dir.join(FILE)).map_err(|e| e.error)?;
        Ok(())
    }
    pub fn observe(&mut self, c: &Catalog) {
        for m in &c.machines {
            self.record(Kind::Machine, &m.id, m);
        }
        for m in &c.materials {
            self.record(Kind::Material, &m.id, m);
            for a in &m.aliases {
                self.reserve(Kind::Material, a);
            }
        }
        for t in &c.tools {
            self.record(Kind::Tool, &t.id, t);
            if let Some(i) = &t.fabcam_identity {
                self.reserve(Kind::Tool, &i.id);
            }
        }
        for t in &c.trials {
            self.record(Kind::Trial, &t.id, t);
            self.reserve(Kind::Machine, &t.machine_id);
            self.reserve(Kind::Material, &t.material_id);
            self.reserve(Kind::Tool, &t.tool_id);
        }
        for p in &c.presets {
            for id in &p.machines {
                self.reserve(Kind::Machine, id);
            }
            self.reserve(Kind::Material, &p.material);
            self.reserve(Kind::Tool, &p.tool);
        }
    }
    /// Restore may recover a known version, never guess that conflicting content
    /// from an independent database denotes the same resource. Backup metadata
    /// cannot itself authorize a conflict against this workstation's history.
    pub fn check_restore(&self, incoming: &Self) -> io::Result<()> {
        for (k, hashes) in &incoming.entries {
            if let Some(known) = self.entries.get(k) {
                if !hashes.is_empty() && (known.is_empty() || !hashes.is_subset(known)) {
                    return Err(io::Error::other(format!(
                        "identity: restore needs review for {k}"
                    )));
                }
            }
        }
        Ok(())
    }
}

/// Parse exactly one record per lab file; reject ambiguous ownership before IO.
pub fn record_ids(path: &Path, kind: Kind) -> io::Result<Vec<String>> {
    safe_file(path)?;
    let mut c = Catalog::default();
    c.merge_toml(
        kind.data_kind(),
        "identity record",
        &fs::read_to_string(path)?,
    )
    .map_err(|_| io::Error::other(format!("identity: unreadable {} record", kind.dir())))?;
    Ok(ids(&c, kind))
}
fn ids(c: &Catalog, k: Kind) -> Vec<String> {
    match k {
        Kind::Machine => c.machines.iter().map(|r| r.id.clone()).collect(),
        Kind::Material => c.materials.iter().map(|r| r.id.clone()).collect(),
        Kind::Tool => c.tools.iter().map(|r| r.id.clone()).collect(),
        Kind::Trial => c.trials.iter().map(|r| r.id.clone()).collect(),
    }
}
fn files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut out = vec![];
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p.extension().is_some_and(|x| x == "toml") {
            out.push(p)
        }
    }
    out.sort();
    Ok(out)
}

/// Read-only inventory; never creates metadata or locks. Do not print record content.
/// Historical semantic equivalence cannot be reconstructed from a name or geometry.
pub fn inspect(dir: &Path) -> io::Result<Reservations> {
    for name in [
        "lab",
        "tools",
        "presets",
        "machines.toml",
        "materials.toml",
        FILE,
    ] {
        safe_tree(&dir.join(name))?;
    }
    if dir.join(PENDING_RESTORE).exists() {
        return Err(io::Error::other(
            "identity: interrupted restore requires review",
        ));
    }
    let old = Reservations::read(dir)?;
    let (c, _) = crate::loader::load_catalog(dir, Lang::Fr).map_err(|e| match e {
        crate::loader::LoadError::Invalid(detail) => {
            io::Error::other(format!("identity: {detail}"))
        }
        _ => io::Error::other("identity: unreadable active catalogue; inspect data offline"),
    })?;
    validate_aliases(&c)?;
    let mut base_ids = BTreeSet::new();
    let mut found = Reservations::default();
    found.observe(&c);
    // Include the original base versions hidden by lab overrides.
    for (name, k) in [
        ("machines.toml", Kind::Machine),
        ("materials.toml", Kind::Material),
    ] {
        let mut base = Catalog::default();
        base.merge_toml(k.data_kind(), "base", &fs::read_to_string(dir.join(name))?)
            .map_err(|_| io::Error::other("identity: invalid base records"))?;
        for id in ids(&base, k) {
            base_ids.insert(key(k, &id));
        }
        found.observe(&base);
    }
    for p in files(&dir.join("tools"))? {
        let mut base = Catalog::default();
        base.merge_toml(Kind::Tool.data_kind(), "base", &fs::read_to_string(p)?)
            .map_err(|_| io::Error::other("identity: invalid base tool"))?;
        for id in ids(&base, Kind::Tool) {
            base_ids.insert(key(Kind::Tool, &id));
        }
        found.observe(&base);
    }
    for kind in [Kind::Machine, Kind::Material, Kind::Tool, Kind::Trial] {
        for id in ids(&c, kind) {
            let k = key(kind, &id);
            if !base_ids.contains(&k) {
                if let Some(known) = old.entries.get(&k) {
                    if !found.entries[&k].is_subset(known) {
                        return Err(io::Error::other(format!(
                            "identity: unrecognized historical version {}:{id:?}; review required",
                            kind.dir()
                        )));
                    }
                }
            }
        }
        let mut seen = BTreeSet::new();
        let mut paths = BTreeMap::new();
        for p in files(&dir.join("lab").join(kind.dir()))? {
            let list = record_ids(&p, kind)?;
            if list.is_empty() {
                continue;
            }
            if list.len() != 1 {
                return Err(io::Error::other(format!(
                    "identity: ambiguous {} file ownership",
                    kind.dir()
                )));
            }
            let id = &list[0];
            if !valid_id(id) || !seen.insert(id.clone()) {
                return Err(io::Error::other(format!(
                    "identity: duplicate/unsafe {} ID {id:?}",
                    kind.dir()
                )));
            }
            if let Some(other) = paths.insert(file_name(id), id.clone()) {
                return Err(io::Error::other(format!(
                    "identity: {} filename collision {other:?} / {id:?}",
                    kind.dir()
                )));
            }
        }
        for p in files(&dir.join("lab/archive").join(kind.dir()))? {
            let mut a = Catalog::default();
            a.merge_toml(kind.data_kind(), "archive", &fs::read_to_string(p)?)
                .map_err(|_| {
                    io::Error::other(format!("identity: unreadable {} archive", kind.dir()))
                })?;
            let mut archived = Reservations::default();
            archived.observe(&a);
            for id in ids(&a, kind) {
                let k = key(kind, &id);
                if ids(&c, kind).contains(&id) && !base_ids.contains(&k) {
                    let active = &found.entries[&k];
                    let history = old.entries.get(&k);
                    let prior = &archived.entries[&k];
                    if !prior.is_subset(active)
                        && !history.is_some_and(|h| prior.is_subset(h) && active.is_subset(h))
                    {
                        return Err(io::Error::other(format!(
                            "identity: potential archived ID reuse {}:{id:?}",
                            kind.dir()
                        )));
                    }
                }
            }
            found.merge(&archived);
        }
    }
    let mut out = old;
    out.merge(&found);
    Ok(out)
}

/// Fully written temporary, then OS no-clobber publication; cleanup owns only this temp.
pub fn create_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    safe_file(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("identity: missing parent"))?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist_noclobber(path).map_err(|e| e.error)?;
    Ok(())
}

/// Alternate keys may not redirect lookup to a different logical resource.
pub(crate) fn validate_aliases(c: &Catalog) -> io::Result<()> {
    let mut materials = BTreeMap::new();
    for m in &c.materials {
        for id in std::iter::once(&m.id).chain(m.aliases.iter()) {
            if let Some(previous) = materials.insert(id, &m.id) {
                if previous != &m.id {
                    return Err(io::Error::other(format!(
                        "identity: ambiguous material key {id:?}"
                    )));
                }
            }
        }
    }
    let mut tools = BTreeMap::new();
    for t in &c.tools {
        for id in std::iter::once(&t.id).chain(t.fabcam_identity.iter().map(|i| &i.id)) {
            if let Some(previous) = tools.insert(id, &t.id) {
                if previous != &t.id {
                    return Err(io::Error::other(format!(
                        "identity: ambiguous tool key {id:?}"
                    )));
                }
            }
        }
    }
    Ok(())
}

/// An alternate key must not reactivate a historical primary ID or alias.
pub(crate) fn check_alias_changes(
    current: &Catalog,
    proposed: &Catalog,
    r: &Reservations,
) -> io::Result<()> {
    for m in &proposed.materials {
        for alias in &m.aliases {
            let previous = current.materials.iter().find(|x| x.id == m.id);
            if alias != &m.id
                && r.contains(Kind::Material, alias)
                && !previous.is_some_and(|x| x.aliases.contains(alias))
            {
                return Err(io::Error::other(format!(
                    "identity: historically reserved material alias {alias:?}"
                )));
            }
        }
    }
    for t in &proposed.tools {
        if let Some(alias) = &t.fabcam_identity {
            let previous = current.tools.iter().find(|x| x.id == t.id);
            if alias.id != t.id
                && r.contains(Kind::Tool, &alias.id)
                && previous
                    .and_then(|x| x.fabcam_identity.as_ref())
                    .is_none_or(|x| x.id != alias.id)
            {
                return Err(io::Error::other(format!(
                    "identity: historically reserved tool alias {:?}",
                    alias.id
                )));
            }
        }
    }
    Ok(())
}

/// Only controlled ancestors need checking on each request. Full archive inventory
/// is done at startup/explicit reload, never reparsed for normal CRUD allocation.
pub(crate) fn safe_lab_dir(root: &Path, kind: Kind) -> io::Result<()> {
    safe_file(&root.join("lab"))?;
    safe_file(&root.join("lab").join(kind.dir()))
}

/// Read-only diagnostic details limited to types and IDs. A changed tool snapshot
/// is a review signal, not proof that a user deliberately editing a tool reused an ID.
pub fn diagnostics(dir: &Path) -> io::Result<Vec<String>> {
    inspect(dir)?;
    let (catalog, _) = crate::loader::load_catalog(dir, Lang::Fr)
        .map_err(|_| io::Error::other("identity: unreadable catalogue"))?;
    let mut messages = Vec::new();
    for trial in &catalog.trials {
        for (kind, id, missing) in [
            (
                "machine",
                &trial.machine_id,
                catalog.machine(&trial.machine_id).is_none(),
            ),
            (
                "material",
                &trial.material_id,
                catalog.material(&trial.material_id).is_none(),
            ),
            (
                "tool",
                &trial.tool_id,
                catalog.tool(&trial.tool_id).is_none(),
            ),
        ] {
            if missing {
                messages.push(format!(
                    "identity: trial {:?} references absent {kind} {id:?} (reserved)",
                    trial.id
                ));
            }
        }
        if let Some(tool) = catalog.tool(&trial.tool_id) {
            if tool.diameter_mm != trial.tool_diameter_mm || tool.flutes != trial.tool_flutes {
                messages.push(format!("identity: trial {:?} / tool {:?}: stored geometry differs; historical equivalence requires review",trial.id,tool.id));
            }
        }
    }
    Ok(messages)
}
