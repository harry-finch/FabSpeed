//! Local same-directory replacement. No delete-before-rename fallback.
//! macOS/APFS and Linux local filesystems only; Windows needs a qualified ACL/replace backend.
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Component, Path},
};

pub(crate) const SUPPORTED: bool = cfg!(any(target_os = "macos", target_os = "linux"));

pub(crate) fn require_supported() -> io::Result<()> {
    if SUPPORTED {
        Ok(())
    } else {
        Err(failure("unsupported_platform"))
    }
}

pub(crate) const NAME: &str = "fabapps-catalogue-v1.json";
pub(crate) const MAX_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) fn failure(code: &'static str) -> io::Error {
    io::Error::other(code)
}
pub(crate) fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

pub(crate) fn atomic_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    crate::identity::safe_file(path)?;
    let dir = path
        .parent()
        .ok_or_else(|| failure("control_unavailable"))?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    sync_dir(dir)
}

#[cfg(unix)]
fn owned(m: &fs::Metadata, regular: bool) -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;
    // SAFETY: geteuid has no pointer arguments or side effects.
    if m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o022 != 0
        || (regular && (!m.is_file() || m.nlink() != 1))
    {
        return Err(failure("unsafe_destination"));
    }
    Ok(())
}
#[cfg(not(unix))]
fn owned(_: &fs::Metadata, _: bool) -> io::Result<()> {
    Err(failure("unsupported_platform"))
}

fn local(dir: &Path) -> io::Result<()> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::ffi::OsStrExt;
        let p = std::ffi::CString::new(dir.as_os_str().as_bytes())
            .map_err(|_| failure("unsafe_destination"))?;
        let mut info = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // SAFETY: p is terminated and info has the exact statfs ABI/layout.
        if unsafe { libc::statfs(p.as_ptr(), info.as_mut_ptr()) } != 0 {
            return Err(failure("destination_unavailable"));
        }
        let info = unsafe { info.assume_init() };
        #[cfg(target_os = "macos")]
        let supported = info.f_flags & libc::MNT_LOCAL as u32 != 0;
        #[cfg(target_os = "linux")]
        let supported = matches!(
            info.f_type as u64,
            0xef53 | 0x58465342 | 0x9123683e | 0x01021994 | 0x794c7630
        ); // ext, XFS, btrfs, tmpfs, overlay
        if supported {
            return Ok(());
        }
    }
    let _ = dir;
    Err(failure("unsupported_filesystem"))
}

/// No symlink component; one missing exchange directory may be created with mode 0700.
pub(crate) fn destination(path: &Path, private: &Path, create: bool) -> io::Result<()> {
    require_supported()?;
    if !path.is_absolute()
        || path.file_name().is_none_or(|s| s != NAME)
        || path.starts_with(private)
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(failure("unsafe_destination"));
    }
    let parent = path.parent().ok_or_else(|| failure("unsafe_destination"))?;
    let mut part = std::path::PathBuf::new();
    for c in parent.components() {
        part.push(c);
        match fs::symlink_metadata(&part) {
            Ok(m) if m.is_dir() && !m.file_type().is_symlink() => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound && part == parent && create => {
                let up = parent
                    .parent()
                    .ok_or_else(|| failure("unsafe_destination"))?;
                owned(&fs::metadata(up)?, false)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    fs::DirBuilder::new().mode(0o700).create(parent)?;
                    sync_dir(up)?;
                }
                #[cfg(not(unix))]
                {
                    return Err(failure("unsupported_platform"));
                }
            }
            _ => return Err(failure("unsafe_destination")),
        }
    }
    owned(&fs::metadata(parent)?, false)?;
    local(parent)?;
    if let Ok(m) = fs::symlink_metadata(path) {
        owned(&m, true)?;
    }
    Ok(())
}

pub(crate) fn lease(path: &Path) -> io::Result<File> {
    let lock = path.with_file_name(".fabapps-catalogue-v1.lock");
    crate::identity::safe_file(&lock)?;
    if lock.exists() {
        owned(&fs::symlink_metadata(&lock)?, true)?;
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(lock)?;
    owned(&file.metadata()?, true)?;
    file.try_lock_exclusive()
        .map_err(|_| failure("destination_busy"))?;
    Ok(file)
}

pub(crate) fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
        Ok(m) => {
            owned(&m, true)?;
            if m.len() > MAX_BYTES {
                return Err(failure("unexpected_snapshot"));
            }
            let mut bytes = Vec::new();
            File::open(path)?
                .take(MAX_BYTES + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err(failure("unexpected_snapshot"));
            }
            Ok(Some(bytes))
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Blocked {
    pub dev: u64,
    pub ino: u64,
}
pub(crate) fn file_identity(path: &Path) -> io::Result<Blocked> {
    let m = fs::symlink_metadata(path)?;
    owned(&m, true)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(Blocked {
            dev: m.dev(),
            ino: m.ino(),
        })
    }
    #[cfg(not(unix))]
    {
        Err(failure("unsupported_platform"))
    }
}
pub(crate) fn is_blocked(path: &Path, b: &Blocked) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(m) = fs::symlink_metadata(path) {
            return owned(&m, true).is_ok()
                && m.dev() == b.dev
                && m.ino() == b.ino
                && m.mode() & 0o777 == 0;
        }
    }
    let _ = (path, b);
    false
}
pub(crate) fn block(path: &Path, expected: &Blocked) -> io::Result<()> {
    let actual = file_identity(path)?;
    if actual.dev != expected.dev || actual.ino != expected.ino {
        return Err(failure("unexpected_snapshot"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file = File::open(path)?;
        file.set_permissions(fs::Permissions::from_mode(0o000))?;
        file.sync_all()?;
        sync_dir(path.parent().unwrap())
    }
    #[cfg(not(unix))]
    {
        Err(failure("unsupported_platform"))
    }
}

#[cfg(all(test, target_os = "windows"))]
#[test]
fn publication_windows_fails_closed_before_filesystem_access() {
    assert_eq!(
        destination(
            Path::new(r"C:\exchange\fabapps-catalogue-v1.json"),
            Path::new(r"C:\private"),
            false
        )
        .unwrap_err()
        .to_string(),
        "unsupported_platform"
    );
}
