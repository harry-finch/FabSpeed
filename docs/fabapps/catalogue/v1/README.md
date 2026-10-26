# FabApps catalogue v1 — normative contract

FS-COMPAT-2 implements a pure producer, not a publisher. `fabapps-catalogue-v1.schema.json` and the semantic rules below jointly define validity. The JSON Schema alone cannot establish hashes, unique IDs, ordering or reference closure. `scripts/check_fabapps_catalogue.py` is an independent test/reference validator, not a FabCAM cache or import library.

## Envelope and interpretation

Format `fabapps-catalogue`, integer version 1, `complete:true`, `language:fr`. Publisher is `{application:"fabspeed", application_version:...}`. IDs are opaque; external keys are `(catalogue_id, resource_type, id)`. Names and aliases are never identity matches. Machines are profiles, tools cutter specifications and materials calculation profiles, not physical assets, individual cutters or certified batches.

All four resource arrays are required and sorted by UTF-8 ID bytes, unique within each type. Arrays may be empty. Every preset's exact machine/tool/material ID and revision must match a resource in the same envelope. No implicit dependency disclosure, aliases, omitted pages or partial snapshots. Unknown critical enums, versions, units, malformed numbers and broken references reject the whole envelope. This v1 schema is closed (`additionalProperties:false`); adding fields requires an explicitly revised compatible-reader policy or a new schema version. This is deliberately stricter than the FS-COMPAT-0 suggestion that readers may ignore informational additions.

Numbers are finite binary64 values serialized as JSON numbers, preserving fractional values without clamping/rounding. Integral control values are limited to 2^53−1. Field suffixes specify mm, mm/min, rpm, W, degrees; ratios are dimensionless. Dimensions/speeds must be positive, stepover ≤1, flute count integer 1..255, angle <180 degrees. Minimum RPM must not exceed maximum; partial lengths must fit stated overall/cutting lengths. An absent optional number is unknown, never zero or an unlimited constraint. All dates are real RFC3339 UTC dates with uppercase `T` and `Z`; leap-second representations are not accepted by this v1 profile.

A resource revision changes for every shared field, including name or reliability status. Private notes, source-note prose, supplier URL, imported FabCAM identity, aliases, internal coefficients, engine policy and operator data are excluded. Material constants remain internal calculation data; changing only those constants does not change the public material revision. No calculated result is exported as experimental evidence.

## FAB-C14N-1 revision algorithm

SHA-256 input is UTF-8 `fabapps-c14n-1`, byte 00, ASCII domain, byte 00, then the following canonical typed byte encoding. Output is `sha256:` plus 64 lowercase hexadecimal digits. Domains are `machine`, `tool`, `material`, `preset`, `catalogue`, `trial-context-values`, `trial-reference` (and `vector` for conformance vectors).

- Null: ASCII `n`. Boolean: ASCII `t` or `f`.
- Number: ASCII `d`, then eight IEEE-754 binary64 bytes, big endian. Normalize both signed zeros to positive zero. Reject non-finite inputs before JSON conversion; do not turn NaN into null. Integer inputs outside ±(2^53−1) are refused.
- String: ASCII `s`, then unsigned 64-bit big-endian UTF-8 byte length, then UTF-8 bytes. No Unicode normalization, escaping, BOM or terminator. Different Unicode sequences remain different content.
- Array: ASCII `a`, unsigned 64-bit big-endian item count, then each encoded item in order. Positional coordinates retain X/Y/Z order. Resource arrays are sorted before encoding.
- Object: ASCII `o`, unsigned 64-bit big-endian pair count; sort keys by UTF-8 bytes; encode each key as a string followed by its value. Insertion order never matters. Omitted optional fields are absent; explicit null is different and is not accepted for optional exchange numbers.

This deliberately refines the FS-COMPAT-0 canonical-JSON proposal: hashing typed binary64 bytes avoids cross-language decimal rendering differences while the transport remains ordinary JSON. `canonical-vectors.json` contains independently computed bytes and expected hashes, including fractions, extremes, UTF-8 and signed zero. Rust checks these Python-generated vectors; the independent Python reader verifies the actual Rust-generated example and rejects stale or inconsistent hashes.

Resource hash input is its whole allow-listed object **without its own `revision`**. Catalogue hash input is the whole envelope without `revision`, `generation` and `published_at`. Publisher/version, language, validation dates and resource revisions are content. Projection has no time/generation side effect; `assemble` requires a valid UTC timestamp and a generation greater than the source's persisted generation, but does not reserve it.

## Validation and consent

Every new administrator confirmation of a good trial captures `Validation.binding`: version 1, source ID, original trial ID, exact machine/tool/material `{id,revision}`, and `context_values_revision`. Capture happens under the same mutation mutex as trial lookup and persistence, using one authoritative French catalogue. The digest input is `{trial_id,recorded_at,tested:{context,values}}`. Exact context is operation, aggressiveness, workholding, stepover and known thickness/depth/stickout/tool condition. Values are RPM, feed, plunge and DOC. Unknown conditions stay unknown and never authorize broader operation families or engagement.

Old validations lacking a binding remain historical FabSpeed validations and keep existing recommendation semantics. They are not export-eligible until explicit reconfirmation. A source mismatch, resource revision change, context/value change, missing dependency or withdrawal prevents inclusion. Invalid selected numeric data is an error, not a silently repaired preset. No new timestamps or historical bindings are manufactured. Imported static presets and unvalidated good trials never qualify.

`Consent` is an explicit caller-supplied owner-approval model, default disabled with empty ID sets. It approves machine/tool/material IDs and local trial IDs individually. Each resource approval covers its documented shared name/specification and supplier/reference fields. A preset never auto-selects dependencies. Removing an ID or disabling consent stops future inclusion. FS-COMPAT-3/4 connects this model to persisted authenticated administrator settings and local automatic publication. There is no public catalogue retrieval endpoint; see the publication protocol (compte rendu interne non publié). The pure API itself is not an authentication boundary.

Preset public ID is `trial:` followed by the SHA-256 hex of the domain-separated local trial reference; its label uses only a digest prefix. `validation.trial_id` is the full opaque digest reference. This refines FS-COMPAT-0 to avoid leaking free-form imported trial identifiers through labels or references. It stays stable for a given original trial ID. Raw trial, operator, validator, note, label, URL, path, credentials and logs are never projected. Hashes prove content equality, not publisher authenticity or physical certification. Local files edited by the account owner remain within the existing local trust boundary.

## Source, clone, restore and generation

`data/.fabapps-source.json` is private control metadata, separate from base data, lab TOML, ordinary backup payloads and library exchange. A first writable AppState creates a random 128-bit `fs-...` identity under its OS writer lease, with generation zero. Creation is create-only, synced, rejects symlinks; corrupt existing metadata refuses startup rather than silently changing identity. Restart, upgrade and intentional move retain the same metadata. Read-only `--check-identities` never initializes it. Backup v1/v2 restore and library import cannot replace it because their accepted paths exclude it.

For an **independent clone**, stop the copied application, preserve a recovery copy, then run on the copied directory:

```sh
fabspeed --data /path/to/independent-copy --initialize-independent-catalogue-source
```

This explicit action acquires the destination lease, assigns a fresh source and resets its generation to zero. Resource IDs/reservations remain unchanged. Bindings from the old source remain historical and require reconfirmation. The command cannot be combined with diagnostic/password-reset/policy commands. It does not enable sharing.

For an **intentional move or replacement takeover**, stop the original publisher permanently, transfer the entire current control metadata with the data, retain the generation high-water information, and ensure only one location is active. Ordinary backups are insufficient to transfer ownership. If the most recent control/generation state cannot be proven, initialize an independent source instead. A copied complete directory cannot be detected magically across hosts; the local advisory lease is not a distributed lock. Never run two publishers with a copied source ID.

`generation` persists the last committed publication (zero means none). FS-COMPAT-2 did not advance it. FS-COMPAT-3/4 uses a crash-safe reservation/pending-publication record under the same source lease: reserve a never-reused candidate, write/verify/sync a temporary snapshot, replace it, then persist successful generation. Recovery must reconcile a pending snapshot and must never reuse a candidate for different content. Unsuccessful reservations can cause gaps; successful publication advances the committed generation, projections do not. Ordinary restore cannot rewind control metadata. Durable replacement/recovery and permissions are implemented in FS-COMPAT-3/4; its report distinguishes native evidence and platform restrictions. Filesystem synchronization does not prove safety against every power failure. Restore suspends publication approvals because ordinary backups do not attest their publication source.

## Complete snapshots and reference-reader rules

Only an entirely valid, complete, newer generation from the same source can withdraw availability of formerly present IDs. Archive, consent withdrawal, validation withdrawal and stale binding can all cause absence; no reason is invented. Missing/corrupt/partial/incompatible files do not withdraw anything. Reject older generations; equal generation and different content is a source error; equal generation/equal content is idempotent. Offline historical copies may remain, but current qualification cannot be inferred from an unseen update. No consumer cache, tombstones or operation changes are implemented here.

## Reproduce conformance

```sh
python3 -m venv /tmp/fabspeed-contract-validator
/tmp/fabspeed-contract-validator/bin/pip install jsonschema==4.23.0
/tmp/fabspeed-contract-validator/bin/python scripts/check_fabapps_catalogue.py
cargo test -p fabspeed-server --locked catalogue_
```

For a fresh producer example, create a disposable output directory and set `FABSPEED_CONTRACT_FIXTURES` to it while running `catalogue_vectors_and_synthetic_fixture`. Without that variable Rust compares output with the checked-in fixture. The example is entirely synthetic, has one resource of each type, a bound preset and fractional rates. Test/reference code neither publishes to an exchange directory nor reserves a generation.
