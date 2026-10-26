# Licensing and attribution

Copyright © 2026 Stéphane Muller. FabSpeed's original code, documentation and
shipped data are licensed under **GPL-3.0-or-later**; the complete GPL version 3
text is in [LICENSE](LICENSE). Cargo workspace/crate metadata, npm metadata,
dataset metadata and Tauri metadata declare that licence. This decision belongs
to FabSpeed and does not depend on another FabApps application's licence.

## Included material

| Material | Origin and disposition |
| --- | --- |
| FabSpeed code and documentation | Project sources under GPL-3.0-or-later; existing notices retained. |
| Shipped machine/material/tool/advisor data and translations | Project data under GPL-3.0-or-later. On 26 October 2026 the maintainer, as the declared rights holder, expressly authorized redistribution of the included wiki-derived constants, machine values and translated instructions. The original workshop corpus and supplier records remain excluded. |
| Synthetic fixtures and examples | Explicitly fictional project test data; not real machining trials or supplier tables. |
| FabSpeed V2 logo, generated icons and five README screenshots | Maintainer confirmed ownership or permission for publication. The canonical SVG contains paths/shapes, without an embedded font or external image. |
| Inter licence notice | [SIL Open Font License 1.1](ui/public/licenses/Inter-OFL-1.1.txt), preserved verbatim from the locked `@fontsource-variable/inter` package. |
| Lucide licence notice | [ISC](ui/public/licenses/Lucide-ISC.txt), preserved verbatim from the locked `@lucide/svelte` package, including its existing copyright notices. |

The rights-holder declaration is distinct from technical validation. It does not
certify the physical safety of any cutting settings or grant rights to private
datasets subsequently imported by a user.

## Referenced dependencies

[The dependency inventory](docs/dependency-licenses.json) records the versions,
registry sources and declared licence expressions of **507 Cargo dependencies
and 74 npm packages**. Cargo.lock and ui/package-lock.json retain the precise
resolution and registry checksums/integrity. This source snapshot contains no
vendored registry packages, node_modules, compiled JavaScript, font binaries,
build cache or application binaries. Dependencies are obtained from their
registries when the reader builds the source.

Consequently, this source publication does not redistribute those 581 package
implementations. An earlier inventory's 46 missing top-level licence files
included the three FabSpeed workspace crates covered by the root LICENSE, and
43 externally fetched packages. They are not missing notices in a vendored
source tree: those packages are absent from this snapshot. Their packaging and
licence texts must still be reviewed for any future binary distribution.

In particular, MPL-2.0 dependencies include cssparser, cssparser-macros,
dtoa-short, option-ext and selectors. Their implementation sources are neither
copied nor modified here. A distributor of a build must preserve the applicable
notices and provide the corresponding covered sources as required. See
[Mozilla's MPL distribution FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/).
Do not relabel dependency code under the project's GPL declaration.

The shipped Inter/Lucide notices remain available for frontend builds. This
source inventory is **not a complete binary redistribution notice bundle**.
Before shipping executables or compiled frontend assets, inventory what is
actually included, resolve all relevant licences and attribution, and provide
the corresponding sources and notices. No binaries are distributed in this
milestone.

## Maintenance findings

The current locked dependency set has previously reported six RustSec
unmaintained-package advisories: RUSTSEC-2024-0370 (proc-macro-error) and
RUSTSEC-2025-0075, -0080, -0081, -0098 and -0100 (unic packages). These findings
remain open; no suppressions or dependency changes were introduced to hide
them. They concern maintenance and are not evidence that the source has passed
a comprehensive security certification. The default cargo-deny licence policy
was not configured; its “not explicitly allowed” output is not a finding of
licence incompatibility. Review these items before a stable binary release.
