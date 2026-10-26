# Field ownership

| Type | FabSpeed shared fields | Excluded from v1 | Future FabCAM local fields |
| --- | --- | --- | --- |
| Machine profile | ID, revision, approved name; RPM/feed/plunge/power/rated RPM/work area with per-field status | Controller, force/chipload coefficients, descriptions, notes/source prose | Postprocessor, coordinates, safe Z, setup and tool changes |
| Cutter specification | ID, revision, approved name, family, helix, material, diameter, flutes, lengths, angle, RPM constraints, approved supplier/reference | URL, notes, FabCAM identity, material aliases/recommendations | Holder, assembly, tool slot, physical inventory |
| Material profile | ID, revision, approved name, category, supervision flag | Coefficients, default stock dimensions, aliases, description, tips, source text | Physical batch, stock dimensions and placement |
| Validated preset | Digest-derived stable ID/label, exact dependencies/revisions, tested context, values, sanitized validation dates/basis and opaque trial reference | Operator, validator, notes, raw trial, logs, policy, credentials | Accepted/resolved operation values and their historical source snapshot |

Missing machine source note means `unknown`, never the editor's implicit `verified`. Existing statuses are declarations, not certification. Tool geometry has no structured measurement provenance; no provenance is invented. Names/supplier/reference themselves require resource-level disclosure consent. Every preset dependency must be independently selected. FabCAM keeps independent local IDs and extensions; no consumer implementation or business-data return flow is present in this milestone.
