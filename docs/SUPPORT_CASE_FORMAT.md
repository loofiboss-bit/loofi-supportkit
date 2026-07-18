# Support-case format

The canonical contract is [support-case.schema.json](../schemas/support-case.schema.json). Version `1.0.0` is additive-only within the current major schema version. Readers must reject unknown major versions and may ignore additive fields after schema validation.

## Case document

The JSON document contains:

- identity, timestamp, application version, platform, and selected category;
- scan status plus a per-collector completion manifest;
- a normalized system profile and redacted evidence records;
- ordered deterministic findings with rule IDs and evidence references;
- a redaction manifest containing categories, counts, paths, and review warnings;
- local export history.

IDs referenced by a finding must match an evidence `id` in the same case. Dates use RFC 3339 UTC. The platform and category values are closed enums defined by the schema.

## ZIP bundle

The ZIP export contains:

```text
summary.html
findings.json
system-profile.json
collection-manifest.json
redaction-manifest.json
checksums.sha256
evidence/<evidence-id>.json
```

`summary.html` is static and contains no scripts, remote fonts, trackers, or external resources. `checksums.sha256` covers every payload file except itself. Entries are sorted and use a fixed ZIP timestamp so identical cases produce identical bytes.

## Compatibility and migration

Persisted cases store `schemaVersion`. The current application opens only `1.0.0`; corrupt or unsupported cases fail closed with a user-readable error. A future incompatible change must add an explicit migration, preserve the original file until migration succeeds, and increment the major schema version.

## Validation

Run:

```bash
npm run validate-schemas
cargo test -p report-builder
```

The first command validates schemas, all declarative rules, normalized evidence, and a complete synthetic support case. The second verifies escaping, offline HTML, file layout, checksums, and deterministic ZIP output.
