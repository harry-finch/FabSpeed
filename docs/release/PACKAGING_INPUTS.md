# Reviewed packaging inputs

Tauri resources enumerate seven public files explicitly. No directory glob may include tools, presets or translations: ignored files are not a packaging boundary. `public-data.sha256` binds approved bytes; the desktop build refuses changed/missing/symlinked inputs. Update that manifest only with an intentional dataset review (and the dataset revision where applicable).

After bundling, run `python3 scripts/check_bundle.py <extracted-base-data>`. It compares exact names and SHA-256, rejecting links and additional files. FINAL-2 tests also place synthetic ignored records under tools/presets/i18n/lab before bundling and check they are absent. Never use real laboratory files as sentinels.

Build from a clean identified source or prepared history-free export. Keep package hashes and the source SHA outside the source tree. The Windows NSIS, macOS app/DMG and Linux deb/AppImage formats remain unchanged. No script here signs, uploads, tags or publishes.

Windows publisher availability is independent from installer generation. See [backend boundary](FINAL_2_WINDOWS_BACKEND.md). The directory chooser is still deferred: this desktop uses an external localhost webview and has no dialog plugin/IPC capability. A secure addition must bind one folder-selection command to the exact runtime window/origin without wildcard filesystem privileges. The reviewed absolute-path mechanism remains available; changing an established destination first requires withdrawal.

The embedded Inter and Lucide licence notices are shipped under frontend `licenses/`; retain them in binaries and exports. Cargo dependency licence expressions are inventoried separately during qualification; a green build is not legal clearance.

Use `python3 scripts/build_desktop.py --bundles app,dmg` on macOS (the workflow supplies its own existing target/formats). It preserves build flags and remaps builder home/source paths before Rust compilation; no account directory is redirected. Inspect resulting bytes again: remapping is not a privacy scan. This wrapper pins the same Tauri CLI and passes `--locked`.

The history-free exporter records original and exported hashes and converts Markdown links to excluded internal documents into explicit plain-text labels. Rust, frontend source, schemas and public datasets remain byte-identical to the reviewed commit.
