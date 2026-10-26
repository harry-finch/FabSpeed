# Source publication and platform status

This is a source preview of **0.9.0-beta.1**, with an independent initial Git
history. The implementation and numerical reference data match the qualified
software snapshot. Publication-specific changes concern documentation,
attribution and the exclusion of private/internal files and installer CI.
No stable V1 release, version tag or downloadable installer accompanies it.

## Platform status

| Platform or feature | Actual qualification |
| --- | --- |
| macOS arm64 | Automated source gates and isolated Tauri evaluation performed; internal evaluation scope. |
| macOS Intel | Native desktop acceptance remains outstanding. |
| Windows desktop | Native acceptance remains outstanding; catalogue publication is explicitly refused before filesystem mutation. |
| Linux desktop/catalogue | Unix implementation exists; native Linux acceptance remains outstanding. |
| Distribution signing | Recognized macOS/Windows signing and macOS notarization remain outstanding. |
| FabCAM | Catalogue consumption planned separately; no operational synchronization. |
| Physical cutting validation | Software and synthetic fixtures do not establish physical machining validation. |

There are no public installers or stable V1 release assets in this source
publication. Existing historical design documents describe intended or past
development work; the table above states the current qualification boundary.

## Build prerequisites

- Rust/Cargo **1.88**, with rustfmt and Clippy, selected by rust-toolchain.toml.
- Node.js **24** and npm. Use the committed lockfile with `npm ci --prefix ui`.
- Python **3.12+** and `jsonschema==4.23.0` for independent catalogue checks.
- macOS: Xcode Command Line Tools and the native SDK.
- Windows: Visual Studio C++ Build Tools, Windows SDK and WebView2.
- Linux: a C/C++ toolchain, pkg-config, WebKitGTK 4.1, GTK 3 and OpenSSL development
  libraries. The manual workflow also installs libayatana-appindicator3-dev,
  librsvg2-dev and patchelf. On Ubuntu 22.04, see its exact package command in
  [native-qualification.yml](../.github/workflows/native-qualification.yml).

Consult [Tauri's native prerequisites](https://v2.tauri.app/start/prerequisites/)
for your platform. Installing prerequisites or compiling on a target does not
constitute native acceptance of FabSpeed on that target.

Build the frontend before Cargo workspace commands because the server embeds
`ui/dist`. Follow the [README](../README.md) and [validation commands](VALIDATION.md).
Run UI experiments on a separate copy of the base dataset, never a real workshop
profile. Do not run an installer or production desktop profile as part of the
source checks.

The optional private calibration corpus is absent. The two calibration test
functions return early; report them as **NOT RUN** even if Cargo displays their
test functions as successful. Public synthetic fixtures and invariant tests
remain available. Numerical fixtures have not been altered for publication.

## Source and packaging boundary

- Original private Git metadata/history, branches, tags, raw audit evidence,
  editor state, lab datasets, supplier archives, authentication and app profiles
  are absent.
- The private wiki archive generator is excluded; public tests do not depend
  on it. Public examples are synthetic.
- Tauri resources enumerate seven exact base files, checked by
  `crates/fabspeed-desktop/public-data.sha256` and the build script. A broad
  resource glob is not acceptable. `.gitignore` is not a packaging boundary.
- The historical installer workflow is excluded. The remaining native source
  qualification workflow is manually triggered, has only `contents: read`,
  pinned actions and no artifact upload, release, deployment or package push.
  GitHub Actions are disabled for initial publication until deliberately enabled
  for a later qualification run.
- Registry dependency code and compiled output are absent. See
  [licensing and attribution](../THIRD_PARTY_NOTICES.md).

## Future source updates

Maintainers must preserve the separate public history. Never merge private
development history, copy its `.git`, use alternates, mirror it or push a private
development branch to this repository.

1. Select an explicitly qualified private source commit and record it in a
   **private** receipt. Extract its tracked tree using a safe archive operation,
   not a clone, checkout copy, shallow clone or Git bundle.
2. Apply the reviewed exclusions and public-documentation policy. Exclude all
   internal milestone reports (including future publication reports), raw
   receipts, local settings and private generators. Keep the public rights and
   platform status current. Retain copyright and third-party notices.
3. Review every changed file, including derived data, examples, translations,
   binary assets, scripts and workflows. Resolve redistribution rights; never
   silently replace numerical data to pass the review. Requalify actual software
   changes. Keep scanner findings and private file inventories outside the tree.
4. Create a new reviewed allowlist with file modes and SHA-256 hashes. Run secret,
   privacy, licence, link and independent build/contract checks on those bytes.
5. Verify the public repository's numeric identity, visibility and branch HEAD.
   In a **separate clone of this public history**, compare the approved content
   against the existing public files; apply only that reviewed content set.
   No private Git objects may be imported. Review the complete staging set.
6. Commit as a descendant of the public main branch using an approved public
   identity. Enumerate reachable objects and verify no private commits are
   present. Use a normal non-force push after review. If an accidentally included
   private blob is detected, stop: deleting it in a subsequent commit does not
   remove it from history.
7. Independently fetch/clone the updated public remote, compare its contents to
   the manifest, inspect refs and Actions logs/artifacts, and retain the exact
   source/export/public-commit mapping privately.

The maintainer's development remotes must explicitly identify the private
repository before any development push. Never rely on a rename redirect once
the former repository name has been reused. No automatic mirroring or release
deployment is configured.
