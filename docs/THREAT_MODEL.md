# Threat model

## Assets

- Raw system evidence and identifiers.
- Redacted support-case documents and exports.
- Integrity of findings, schemas, and rule versions.
- The user's host and local case directory.

## Trust boundaries

The renderer is untrusted input. Tauri commands form the IPC boundary. The collector runtime forms the operating-system process boundary. Redaction is the export/persistence boundary. An exported bundle leaves application control and must be treated as user-approved disclosure.

## Threats and controls

| Threat                       | Control                                                                                                       |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Command injection            | No shell; internal command enum; fixed arguments; validated systemd unit names                                |
| Hung or noisy process        | Per-command timeout, kill on timeout, stdout/stderr byte caps                                                 |
| Renderer privilege expansion | Narrow Tauri commands, enum-shaped payloads, no shell/fs plugins, local capability only                       |
| Secret leakage               | Structured recursive redaction, credential detection, synthetic adversarial tests, raw evidence not persisted |
| Path traversal               | App-owned case IDs are validated as UUIDs; exports use an app-owned directory and generated UUID filename     |
| Partial/corrupt persistence  | Same-directory temporary file, flush, atomic rename, schema version checks                                    |
| Misleading diagnosis         | Deterministic versioned rules, confidence labels, evidence references, incomplete-collector disclosure        |
| Bundle tampering             | SHA-256 checksum list covering every payload file                                                             |
| Resource exhaustion          | Bounded collectors, evidence count limits, output limits, ZIP input limits                                    |
| Formula/script execution     | JSON and escaped static HTML only; no spreadsheet formats or report JavaScript                                |

## Residual risks

Heuristic redaction cannot classify every sensitive value. The privacy screen therefore lists transformed paths and warns about unclassified free text. Read-only tools can still reveal sensitive state to the application process. Windows collector behavior requires validation on Windows 11 hardware before a stable release.

## Security extension checklist

Any new collector must document its data purpose, supported platforms, executable resolution, fixed arguments, timeout, output cap, exit semantics, parser failure mode, synthetic fixture, and privacy classification. Privileged collectors require a separate threat-model revision and are out of MVP scope.
