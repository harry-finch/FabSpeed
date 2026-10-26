use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn main() {
    // Packaging must not trust .gitignore or broad resource directory globs.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    println!("cargo:rerun-if-changed=public-data.sha256");
    for line in include_str!("public-data.sha256").lines() {
        let (expected, relative) = line.split_once("  ").expect("public data manifest");
        let path = root.join(relative);
        println!("cargo:rerun-if-changed={}", path.display());
        let metadata = fs::symlink_metadata(&path).expect("public data input missing");
        assert!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "public data must be a regular file"
        );
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(fs::read(path).expect("public data readable"))
            ),
            expected,
            "public data changed: review the release manifest"
        );
    }
    tauri_build::build()
}
