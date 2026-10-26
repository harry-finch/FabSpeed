//! Persistent consent and one serialized publication owner. Source and output are
//! separate durable objects: reserve -> replace -> commit source -> commit journal.
use crate::{
    catalogue::{self, Consent},
    publication_fs as disk,
    source::SourceIdentity,
    AppState,
};
use fabspeed_core::{Catalog, Lang};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Mutex, OnceLock,
    },
};
use tokio::sync::Notify;

pub const FILE: &str = ".fabapps-publication.json";
const MAX_GENERATION: u64 = 9_007_199_254_740_991;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub destination: Option<PathBuf>,
    pub consent: Consent,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Published {
    pub generation: u64,
    pub revision: String,
    pub published_at: String,
    pub destination: PathBuf,
    pub empty: bool,
    #[serde(default)]
    pub(crate) sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    snapshot: Published,
    temp: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Control {
    version: u32,
    catalogue_id: String,
    pub settings: Settings,
    high_water: u64,
    pending: Option<Pending>,
    pub last: Option<Published>,
    pub phase: String,
    pub error: Option<String>,
    dirty: bool,
    force: bool,
    blocked: Option<disk::Blocked>,
    blocked_sha256: Option<String>,
}
impl Control {
    fn fresh(s: &SourceIdentity) -> Self {
        Self {
            version: 1,
            catalogue_id: s.catalogue_id.clone(),
            settings: Settings::default(),
            high_water: 0,
            pending: None,
            last: None,
            phase: "disabled".into(),
            error: None,
            dirty: false,
            force: false,
            blocked: None,
            blocked_sha256: None,
        }
    }
}
pub struct Publisher {
    control: Mutex<Control>,
    pub(crate) hint: Mutex<Option<PathBuf>>,
    notify: Notify,
    halted: AtomicBool,
    started: AtomicBool,
    fault: AtomicU8,
    #[cfg(test)]
    pub(crate) atomic_us: std::sync::atomic::AtomicU64,
}
#[derive(Serialize)]
pub struct Status {
    pub supported_platform: bool,
    pub settings: Settings,
    pub phase: String,
    pub error: Option<String>,
    pub last: Option<Published>,
    pub default_destination: Option<PathBuf>,
    pub local_access_blocked: bool,
    pub existing_may_be_readable: bool,
}
fn error(code: &'static str) -> io::Error {
    disk::failure(code)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn validate(value: &Value) -> io::Result<()> {
    static VALIDATOR: OnceLock<jsonschema::Validator> = OnceLock::new();
    let v = VALIDATOR.get_or_init(|| {
        let schema = serde_json::from_str(include_str!(
            "../../../docs/fabapps/catalogue/v1/fabapps-catalogue-v1.schema.json"
        ))
        .expect("embedded schema JSON");
        jsonschema::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("embedded schema")
    });
    if v.is_valid(value) {
        Ok(())
    } else {
        Err(error("invalid_projection"))
    }
}
pub(crate) fn reset_clone(dir: &Path, source: &SourceIdentity) -> io::Result<()> {
    disk::atomic_private(
        &dir.join(FILE),
        &serde_json::to_vec(&Control::fresh(source))?,
    )
}
impl Publisher {
    pub(crate) fn load(dir: &Path, source: &SourceIdentity) -> Self {
        let loaded = (|| -> io::Result<Control> {
            crate::identity::safe_file(&dir.join(FILE))?;
            match fs::read(dir.join(FILE)) {
                Ok(bytes) => {
                    let c: Control =
                        serde_json::from_slice(&bytes).map_err(|_| error("unsafe_control"))?;
                    if c.version != 1
                        || c.catalogue_id != source.catalogue_id
                        || c.high_water > MAX_GENERATION
                        || c.high_water < source.generation
                    {
                        return Err(error("unsafe_control"));
                    }
                    Ok(c)
                }
                Err(e)
                    if e.kind() == io::ErrorKind::NotFound
                        && !source.publication_initialized
                        && source.generation == 0 =>
                {
                    Ok(Control::fresh(source))
                }
                Err(_) => Err(error("unsafe_control")),
            }
        })();
        let halted = loaded.is_err();
        let mut control = loaded.unwrap_or_else(|_| Control::fresh(source));
        if halted {
            control.phase = "unsafe".into();
            control.error = Some("unsafe_control".into());
        }
        Self {
            control: Mutex::new(control),
            hint: Mutex::new(None),
            notify: Notify::new(),
            halted: AtomicBool::new(halted),
            started: AtomicBool::new(false),
            fault: AtomicU8::new(0),
            #[cfg(test)]
            atomic_us: std::sync::atomic::AtomicU64::new(0),
        }
    }
    pub(crate) fn set_hint(&self, path: PathBuf) {
        *self.hint.lock().unwrap() = Some(path);
    }
    fn current(&self) -> Control {
        self.control.lock().unwrap().clone()
    }
    fn show(&self, c: &Control) {
        *self.control.lock().unwrap() = c.clone();
    }
    fn save(&self, state: &AppState, c: &Control) -> io::Result<()> {
        if let Err(e) = disk::atomic_private(
            &state.data_dir().unwrap().join(FILE),
            &serde_json::to_vec(c)?,
        ) {
            let mut failed = c.clone();
            failed.phase = "unsafe".into();
            failed.error = Some("control_unavailable".into());
            self.halted.store(true, Ordering::SeqCst);
            self.show(&failed);
            return Err(e);
        }
        self.show(c);
        Ok(())
    }
    pub fn status(&self) -> Status {
        let c = self.current();
        let blocked = c
            .blocked
            .as_ref()
            .zip(c.settings.destination.as_ref())
            .is_some_and(|(b, p)| disk::is_blocked(p, b));
        Status {
            supported_platform: disk::SUPPORTED,
            settings: c.settings,
            phase: c.phase,
            error: c.error,
            existing_may_be_readable: c.last.as_ref().is_some_and(|p| !p.empty) && !blocked,
            last: c.last,
            default_destination: self.hint.lock().unwrap().clone(),
            local_access_blocked: blocked,
        }
    }
    fn checkpoint(&self, stage: u8) -> io::Result<()> {
        if self.fault.load(Ordering::SeqCst) == stage {
            #[cfg(test)]
            if std::env::var_os("FABSPEED_PUBLICATION_CRASH_CHILD").is_some() {
                std::process::exit(86);
            }
            return Err(error("injected_io_failure"));
        }
        Ok(())
    }
    #[cfg(all(test, unix))]
    pub(crate) fn inject(&self, stage: u8) {
        self.fault.store(stage, Ordering::SeqCst);
    }
}
impl AppState {
    pub(crate) fn published_snapshot(&self) -> Option<Value> {
        let c = self.publication.current();
        let last = c.last?;
        let bytes = disk::read(&last.destination).ok()??;
        if hash(&bytes) != last.sha256 {
            return None;
        }
        serde_json::from_slice(&bytes).ok()
    }
    pub fn publication_status(&self) -> Status {
        self.publication.status()
    }
    /// Called only after a successful mutation while holding write_lock. Failure
    /// affects publication, never turns an already committed lab write into rollback.
    pub(crate) fn publication_changed(&self) {
        if !disk::SUPPORTED {
            return;
        }
        let p = &self.publication;
        if p.halted.load(Ordering::SeqCst) {
            return;
        }
        let mut c = p.current();
        if c.settings.consent.enabled || c.dirty {
            c.dirty = true;
            c.phase = if c.settings.consent.enabled {
                "outdated"
            } else {
                "withdrawal_pending"
            }
            .into();
            if p.save(self, &c).is_ok() {
                p.notify.notify_one();
            }
        }
    }
    /// A backup has no trustworthy publication-source attestation. Suspend
    /// disclosure before removing the restore transaction marker. On a crash,
    /// that marker prevents startup/publication until restore is reconciled.
    pub(crate) fn publication_restored(&self) -> io::Result<()> {
        let p = &self.publication;
        if p.halted.load(Ordering::SeqCst) {
            return Err(error("unsafe_control"));
        }
        let mut c = p.current();
        if !c.settings.consent.enabled && c.last.is_none() && c.pending.is_none() {
            return Ok(());
        }
        c.settings.consent = Consent::default();
        c.dirty = c.last.as_ref().is_some_and(|l| !l.empty) || c.pending.is_some();
        c.phase = if c.dirty {
            "withdrawal_pending"
        } else {
            "disabled"
        }
        .into();
        p.save(self, &c)?;
        p.notify.notify_one();
        Ok(())
    }
    pub async fn configure_publication(&self, settings: Settings) -> io::Result<Status> {
        disk::require_supported()?;
        let _guard = self.write_lock().lock().await;
        let p = &self.publication;
        if p.halted.load(Ordering::SeqCst) {
            return Err(error("unsafe_control"));
        }
        let mut c = p.current();
        let cat = self.catalog(Lang::Fr).await;
        check_selection(&cat, &self.source(), &settings.consent, &c.settings.consent)?;
        if settings.consent.enabled && settings.destination.is_none() {
            return Err(error("destination_required"));
        }
        if settings.destination != c.settings.destination {
            if c.pending.is_some()
                || c.last.as_ref().is_some_and(|l| !l.empty)
                || c.blocked.is_some()
            {
                return Err(error("withdraw_before_moving"));
            }
            if let Some(path) = &settings.destination {
                disk::destination(path, self.data_dir().unwrap(), true)?;
                let _lease = disk::lease(path)?;
                if disk::read(path)?.is_some() {
                    return Err(error("unexpected_snapshot"));
                }
            }
        }
        // A disable request never needs a working destination. Persist it first.
        c.settings = settings;
        c.dirty = c.settings.consent.enabled
            || c.pending.is_some()
            || c.last.as_ref().is_some_and(|l| !l.empty);
        c.phase = if c.dirty {
            if c.settings.consent.enabled {
                "pending"
            } else {
                "withdrawal_pending"
            }
        } else {
            "configured"
        }
        .into();
        c.error = None;
        p.save(self, &c)?;
        self.initialize_publisher_source()?;
        p.notify.notify_one();
        Ok(p.status())
    }
    fn initialize_publisher_source(&self) -> io::Result<()> {
        let mut source = self.source();
        if !source.publication_initialized {
            let path = self.data_dir().unwrap().join(crate::source::FILE);
            crate::identity::safe_file(&path)?;
            let actual: SourceIdentity =
                serde_json::from_slice(&fs::read(&path)?).map_err(|_| error("unsafe_control"))?;
            if actual != source {
                return Err(error("unsafe_control"));
            }
            source.publication_initialized = true;
            disk::atomic_private(
                &self.data_dir().unwrap().join(crate::source::FILE),
                &serde_json::to_vec(&source)?,
            )?;
            self.set_source(source);
        }
        Ok(())
    }
    pub async fn request_publication(&self) -> io::Result<Status> {
        disk::require_supported()?;
        let _guard = self.write_lock().lock().await;
        let p = &self.publication;
        if p.halted.load(Ordering::SeqCst) {
            return Err(error("unsafe_control"));
        }
        let mut c = p.current();
        if c.settings.destination.is_none() {
            return Err(error("destination_required"));
        }
        c.dirty = true;
        c.force = true;
        c.phase = if c.settings.consent.enabled {
            "pending"
        } else {
            "withdrawal_pending"
        }
        .into();
        p.save(self, &c)?;
        p.notify.notify_one();
        Ok(p.status())
    }
    /// One worker per AppState. Notify coalesces bursts; state is read under the
    /// same lock as lab/restore/import/consent, never captured in a queued request.
    pub fn start_publisher(&self) -> tokio::task::JoinHandle<()> {
        let state = self.clone();
        let already = self.publication.started.swap(true, Ordering::SeqCst);
        tokio::spawn(async move {
            if already {
                return;
            }
            loop {
                state.publish_once().await;
                state.publication.notify.notified().await;
            }
        })
    }
    pub async fn publish_once(&self) {
        if !disk::SUPPORTED {
            return;
        }
        let guard = self.publication_guard().await;
        if self.publication.halted.load(Ordering::SeqCst) {
            return;
        }
        let state = self.clone();
        let cat = self.catalog(Lang::Fr).await;
        // Filesystem sync and schema work must not block the HTTP runtime.
        let _ = tokio::task::spawn_blocking(move || {
            // Cancellation of the async worker must not release ownership while
            // blocking filesystem work can still replace a snapshot.
            let _guard = guard;
            state.publish_locked(&cat)
        })
        .await;
    }
    fn publish_locked(&self, cat: &Catalog) {
        let p = &self.publication;
        let mut c = p.current();
        if !c.settings.consent.enabled && !c.dirty && c.pending.is_none() {
            return;
        }
        let result = self.attempt(cat, &mut c);
        if let Err(e) = result {
            let code = match e.to_string().as_str() {
                "unexpected_snapshot"
                | "unsafe_control"
                | "unsafe_destination"
                | "unsupported_platform"
                | "unsupported_filesystem"
                | "invalid_projection"
                | "generation_exhausted"
                | "destination_busy" => e.to_string(),
                _ => "publication_io".into(),
            };
            c.dirty = true;
            c.error = Some(code.clone());
            c.phase = if !c.settings.consent.enabled {
                "withdrawal_pending"
            } else if code == "unsafe_control" || code == "unexpected_snapshot" {
                "unsafe"
            } else {
                "failed"
            }
            .into();
            tracing::warn!(code=%code, "FabApps publication incomplete");
            if !c.settings.consent.enabled {
                self.restrict_old_snapshot(&mut c);
            }
            if !p.halted.load(Ordering::SeqCst) {
                let _ = p.save(self, &c);
            }
        }
    }
    fn source_on_disk(&self, c: &Control) -> io::Result<SourceIdentity> {
        let path = self.data_dir().unwrap().join(crate::source::FILE);
        crate::identity::safe_file(&path)?;
        let s: SourceIdentity =
            serde_json::from_slice(&fs::read(path)?).map_err(|_| error("unsafe_control"))?;
        s.check().map_err(|_| error("unsafe_control"))?;
        let known = self.source();
        if s.catalogue_id != known.catalogue_id
            || s.catalogue_id != c.catalogue_id
            || !s.publication_initialized
            || s.generation > c.high_water
            || (s.generation != known.generation
                && c.pending
                    .as_ref()
                    .is_none_or(|p| p.snapshot.generation != s.generation))
        {
            return Err(error("unsafe_control"));
        }
        self.set_source(s.clone());
        Ok(s)
    }
    fn attempt(&self, cat: &Catalog, c: &mut Control) -> io::Result<()> {
        if self
            .data_dir()
            .unwrap()
            .join(crate::identity::PENDING_RESTORE)
            .exists()
        {
            return Err(error("unsafe_control"));
        }
        self.initialize_publisher_source()?;
        let mut source = self.source_on_disk(c)?;
        let path = c
            .settings
            .destination
            .clone()
            .ok_or_else(|| error("destination_required"))?;
        disk::destination(&path, self.data_dir().unwrap(), true)?;
        let _destination_lease = disk::lease(&path)?;
        self.reconcile(c, &mut source, &path)?;
        if fs::metadata(&path).is_ok_and(|m| m.permissions().readonly())
            && !c
                .blocked
                .as_ref()
                .is_some_and(|b| disk::is_blocked(&path, b))
        {
            return Err(error("publication_io"));
        }
        if !c.settings.consent.enabled && !c.dirty {
            return Ok(());
        }
        let (projection, _) = catalogue::project(cat, &source, &c.settings.consent)
            .map_err(|_| error("invalid_projection"))?;
        let revision = projection
            .revision()
            .map_err(|_| error("invalid_projection"))?;
        if !c.force
            && c.last
                .as_ref()
                .is_some_and(|l| l.destination == path && l.revision == revision)
            && c.blocked.is_none()
        {
            c.dirty = false;
            c.error = None;
            c.phase = if c.settings.consent.enabled {
                "published"
            } else {
                "withdrawn"
            }
            .into();
            return self.publication.save(self, c);
        }
        let generation = c
            .high_water
            .checked_add(1)
            .filter(|g| *g <= MAX_GENERATION)
            .ok_or_else(|| error("generation_exhausted"))?;
        let stamp = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|_| error("invalid_projection"))?;
        let value = projection
            .assemble(generation, &stamp)
            .map_err(|_| error("invalid_projection"))?;
        validate(&value)?;
        let bytes = serde_json::to_vec_pretty(&value).map_err(|_| error("invalid_projection"))?;
        if bytes.len() as u64 > disk::MAX_BYTES {
            return Err(error("invalid_projection"));
        }
        let empty = ["machines", "tools", "materials", "presets"]
            .iter()
            .all(|k| value[k].as_array().unwrap().is_empty());
        let mut tmp = tempfile::Builder::new()
            .prefix(".fabapps-publish-")
            .tempfile_in(path.parent().unwrap())?;
        c.high_water = generation;
        c.pending = Some(Pending {
            snapshot: Published {
                generation,
                revision,
                published_at: stamp,
                destination: path.clone(),
                empty,
                sha256: hash(&bytes),
            },
            temp: tmp.path().file_name().unwrap().to_string_lossy().into(),
        });
        c.phase = if c.settings.consent.enabled {
            "publishing"
        } else {
            "withdrawal_pending"
        }
        .into();
        self.publication.save(self, c)?;
        self.publication.checkpoint(1)?; // reserved, before any bytes
        #[cfg(test)]
        let atomic_start = std::time::Instant::now();
        tmp.write_all(&bytes[..bytes.len() / 2])?;
        self.publication.checkpoint(2)?; // partial temporary write / disk full
        tmp.write_all(&bytes[bytes.len() / 2..])?;
        self.publication.checkpoint(3)?; // flush failure
        tmp.as_file().sync_all()?;
        if disk::read(tmp.path())?.as_deref() != Some(&bytes) {
            return Err(error("invalid_projection"));
        }
        self.verify_visible(c, &path)?;
        self.publication.checkpoint(4)?; // replacement failure
        if fs::symlink_metadata(&path).is_ok() {
            tmp.persist(&path).map_err(|e| e.error)?;
        } else {
            tmp.persist_noclobber(&path).map_err(|e| e.error)?;
        }
        self.publication.checkpoint(5)?; // snapshot visible, metadata old
        disk::sync_dir(path.parent().unwrap())?;
        #[cfg(test)]
        self.publication
            .atomic_us
            .store(atomic_start.elapsed().as_micros() as u64, Ordering::SeqCst);
        self.commit(c, &mut source)
    }
    fn verify_visible(&self, c: &Control, path: &Path) -> io::Result<()> {
        if c.blocked
            .as_ref()
            .is_some_and(|b| disk::is_blocked(path, b))
        {
            return Ok(());
        }
        let actual = disk::read(path)?;
        match (actual, c.last.as_ref().filter(|l| l.destination == path)) {
            (None, None) => Ok(()),
            (Some(b), Some(l)) if hash(&b) == l.sha256 => Ok(()),
            _ => Err(error("unexpected_snapshot")),
        }
    }
    fn reconcile(
        &self,
        c: &mut Control,
        source: &mut SourceIdentity,
        path: &Path,
    ) -> io::Result<()> {
        if let Some(p) = c.pending.clone() {
            if p.snapshot.destination != path
                || p.snapshot.generation != c.high_water
                || p.snapshot.generation < source.generation
                || !p.temp.starts_with(".fabapps-publish-")
                || Path::new(&p.temp).components().count() != 1
            {
                return Err(error("unsafe_control"));
            }
            if !c
                .blocked
                .as_ref()
                .is_some_and(|b| disk::is_blocked(path, b))
            {
                if let Some(bytes) = disk::read(path)? {
                    if hash(&bytes) == p.snapshot.sha256 {
                        validate(
                            &serde_json::from_slice(&bytes)
                                .map_err(|_| error("unexpected_snapshot"))?,
                        )?;
                        disk::sync_dir(path.parent().unwrap())?;
                        self.commit(c, source)?;
                        // Continue with the latest consent, even if the recovered file used older consent.
                        c.dirty = true;
                        return Ok(());
                    }
                }
            }
            if c.blocked
                .as_ref()
                .is_some_and(|b| disk::is_blocked(path, b))
                && c.blocked_sha256.as_ref() == Some(&p.snapshot.sha256)
            {
                // The exact candidate was verified before this inode was blocked.
                // Keep the restriction marker while advancing only source metadata.
                let blocked = c.blocked.clone();
                let digest = c.blocked_sha256.clone();
                self.commit(c, source)?;
                c.blocked = blocked;
                c.blocked_sha256 = digest;
                c.dirty = true;
                self.publication.save(self, c)?;
                return Ok(());
            }
            if source.generation >= p.snapshot.generation {
                return Err(error("unsafe_control"));
            }
            self.verify_visible(c, path)?;
            let temp = path.with_file_name(&p.temp);
            if disk::read(&temp)?.is_some() {
                fs::remove_file(temp)?;
            }
            c.pending = None; // high_water is deliberately retained
            self.publication.save(self, c)?;
        }
        let last = c.last.as_ref().map_or(0, |l| l.generation);
        if last != source.generation || last > c.high_water {
            return Err(error("unsafe_control"));
        }
        self.verify_visible(c, path)
    }
    fn commit(&self, c: &mut Control, source: &mut SourceIdentity) -> io::Result<()> {
        let published = c
            .pending
            .as_ref()
            .ok_or_else(|| error("unsafe_control"))?
            .snapshot
            .clone();
        source.generation = published.generation;
        disk::atomic_private(
            &self.data_dir().unwrap().join(crate::source::FILE),
            &serde_json::to_vec(source)?,
        )?;
        self.set_source(source.clone());
        self.publication.checkpoint(6)?; // source committed, journal still pending
        c.last = Some(published);
        c.pending = None;
        if !c
            .blocked
            .as_ref()
            .zip(c.settings.destination.as_ref())
            .is_some_and(|(b, p)| disk::is_blocked(p, b))
        {
            c.blocked = None;
            c.blocked_sha256 = None;
        }
        c.dirty = false;
        c.force = false;
        c.error = None;
        c.dirty = !c.settings.consent.enabled && c.last.as_ref().is_some_and(|l| !l.empty);
        c.phase = if c.dirty {
            "withdrawal_pending"
        } else if c.settings.consent.enabled {
            "published"
        } else {
            "withdrawn"
        }
        .into();
        self.publication.save(self, c)
    }
    fn restrict_old_snapshot(&self, c: &mut Control) {
        let Some(path) = c.settings.destination.clone() else {
            return;
        };
        if disk::destination(&path, self.data_dir().unwrap(), false).is_err() {
            return;
        }
        let Ok(_lease) = disk::lease(&path) else {
            return;
        };
        if c.blocked
            .as_ref()
            .is_some_and(|b| disk::is_blocked(&path, b))
        {
            return;
        }
        let Ok(Some(bytes)) = disk::read(&path) else {
            return;
        };
        let digest = hash(&bytes);
        if !c
            .last
            .as_ref()
            .is_some_and(|s| s.destination == path && s.sha256 == digest)
            && !c
                .pending
                .as_ref()
                .is_some_and(|p| p.snapshot.destination == path && p.snapshot.sha256 == digest)
        {
            return;
        }
        if let Ok(id) = disk::file_identity(&path) {
            c.blocked = Some(id.clone());
            c.blocked_sha256 = Some(digest);
            // Record intent before revoking local access. A crash afterwards can
            // recognize the same inode without reopening the private old bytes.
            let _ = self.publication.save(self, c);
            let _ = disk::block(&path, &id);
        }
    }
}

pub(crate) fn check_selection(
    cat: &Catalog,
    source: &SourceIdentity,
    new: &Consent,
    old: &Consent,
) -> io::Result<()> {
    for (requested, previous, known) in [
        (
            &new.machines,
            &old.machines,
            cat.machines.iter().map(|x| &x.id).collect::<Vec<_>>(),
        ),
        (
            &new.tools,
            &old.tools,
            cat.tools.iter().map(|x| &x.id).collect(),
        ),
        (
            &new.materials,
            &old.materials,
            cat.materials.iter().map(|x| &x.id).collect(),
        ),
    ] {
        if requested
            .difference(previous)
            .any(|id| !known.contains(&id))
        {
            return Err(error("unknown_approval"));
        }
    }
    for id in new.trials.difference(&old.trials) {
        let t = cat
            .trials
            .iter()
            .find(|t| &t.id == id)
            .ok_or_else(|| error("ineligible_preset"))?;
        let mut test = Consent {
            enabled: true,
            ..Consent::default()
        };
        test.trials.insert(id.clone());
        test.machines.insert(t.machine_id.clone());
        test.tools.insert(t.tool_id.clone());
        test.materials.insert(t.material_id.clone());
        let (p, _) =
            catalogue::project(cat, source, &test).map_err(|_| error("ineligible_preset"))?;
        if p.presets.is_empty() {
            return Err(error("ineligible_preset"));
        }
    }
    Ok(())
}

#[cfg(all(test, target_os = "windows"))]
mod windows_guard_tests {
    use super::*;
    #[tokio::test]
    async fn unsupported_publication_never_writes_controls_or_destination() {
        let (data, state) = crate::state::test_state();
        let exchange = tempfile::tempdir().unwrap();
        let path = exchange.path().join(disk::NAME);
        fs::write(&path, b"foreign file: never adopt").unwrap();
        let source = fs::read(data.path().join(crate::source::FILE)).unwrap();
        for settings in [
            Settings::default(),
            Settings {
                destination: Some(path.clone()),
                consent: Consent {
                    enabled: true,
                    ..Consent::default()
                },
            },
        ] {
            assert_eq!(
                state
                    .configure_publication(settings)
                    .await
                    .unwrap_err()
                    .to_string(),
                "unsupported_platform"
            );
        }
        assert_eq!(
            state.request_publication().await.unwrap_err().to_string(),
            "unsupported_platform"
        );
        state.publication_changed();
        state.publish_once().await;
        assert!(!data.path().join(FILE).exists());
        assert_eq!(
            source,
            fs::read(data.path().join(crate::source::FILE)).unwrap()
        );
        assert_eq!(fs::read(path).unwrap(), b"foreign file: never adopt");
        assert!(!state.publication_status().supported_platform);
    }
}
