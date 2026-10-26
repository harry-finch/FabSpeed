//! Source control is intentionally outside ordinary backup/import payloads.
use argon2::password_hash::rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Write},
    path::Path,
};

pub const FILE: &str = ".fabapps-source.json";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub version: u32,
    pub catalogue_id: String,
    /// Last durably committed publication; reserved generations live in publication control.
    pub generation: u64,
    #[serde(default)]
    pub publication_initialized: bool,
}
impl SourceIdentity {
    fn fresh() -> io::Result<Self> {
        let mut bytes = [0u8; 16];
        OsRng
            .try_fill_bytes(&mut bytes)
            .map_err(|_| io::Error::other("source entropy unavailable"))?;
        Ok(Self {
            version: 1,
            catalogue_id: format!(
                "fs-{}",
                bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
            ),
            generation: 0,
            publication_initialized: false,
        })
    }
    pub fn check(&self) -> io::Result<()> {
        let id = self.catalogue_id.strip_prefix("fs-").unwrap_or("");
        if self.version != 1
            || id.len() != 32
            || !id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.generation > 9_007_199_254_740_991
        {
            return Err(io::Error::other("invalid source control metadata"));
        }
        Ok(())
    }
    // Caller owns the same directory's writer lease. Never initialize in diagnostics.
    pub(crate) fn load_or_create(dir: &Path) -> io::Result<Self> {
        let path = dir.join(FILE);
        crate::identity::safe_file(&path)?;
        match std::fs::read(&path) {
            Ok(bytes) => {
                let value: Self = serde_json::from_slice(&bytes)
                    .map_err(|_| io::Error::other("invalid source control metadata"))?;
                value.check()?;
                Ok(value)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if dir.join(crate::publication::FILE).exists() {
                    return Err(io::Error::other("source metadata missing"));
                }
                let value = Self::fresh()?;
                let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
                tmp.write_all(&serde_json::to_vec(&value)?)?;
                tmp.as_file().sync_all()?;
                tmp.persist_noclobber(path).map_err(|e| e.error)?;
                #[cfg(unix)]
                crate::publication_fs::sync_dir(dir)?;
                Ok(value)
            }
            Err(e) => Err(e),
        }
    }
    /// Explicit independent-clone operation. Application must be stopped. Acquires
    /// its own lease; ordinary restore never calls this. No resource ID is changed.
    pub fn initialize_independent_clone(dir: &Path) -> io::Result<Self> {
        let dir = &std::fs::canonicalize(dir)?;
        let _lease = crate::identity::DataLease::acquire(dir)?;
        let path = dir.join(FILE);
        crate::identity::safe_file(&path)?;
        let value = Self::fresh()?;
        let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
        tmp.write_all(&serde_json::to_vec(&value)?)?;
        tmp.as_file().sync_all()?;
        tmp.persist(path).map_err(|e| e.error)?;
        #[cfg(unix)]
        crate::publication_fs::sync_dir(dir)?;
        if dir.join(crate::publication::FILE).exists() {
            crate::publication::reset_clone(dir, &value)?;
        }
        Ok(value)
    }
}
