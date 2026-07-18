# Architecture

## Decision summary

Loofi SupportKit uses a Tauri 2 shell, a React/TypeScript presentation layer, and four focused Rust domain crates. The trusted diagnostic path is deterministic and local. No renderer input can select an executable, provide process arguments, or choose an arbitrary path.

```text
React workflow
  -> narrow Tauri commands
    -> scan orchestration / atomic case store
      -> collector-runtime (reviewed read-only collectors)
      -> supportkit-core (normalized evidence + deterministic rules)
      -> redaction-engine (structured privacy transformation)
      -> report-builder (offline report + checksummed bundle)
```

## Repository map

| Path                       | Responsibility                                   |
| -------------------------- | ------------------------------------------------ |
| `apps/desktop`             | React UI and Tauri command adapter               |
| `apps/desktop/src-tauri`   | Scan orchestration, case persistence, narrow IPC |
| `crates/supportkit-core`   | Contracts, normalized evidence, rule evaluation  |
| `crates/collector-runtime` | Safe process runner and platform collectors      |
| `crates/redaction-engine`  | Structured detection, transformation, manifest   |
| `crates/report-builder`    | HTML/JSON/ZIP generation and checksums           |
| `collectors/*`             | Reviewed collector metadata and platform notes   |
| `rules/*`                  | Declarative, schema-validated rule definitions   |
| `schemas`                  | Persisted/exported JSON contracts                |
| `fixtures`                 | Synthetic deterministic test data                |

## ADR-001: Rust owns the trusted path

**Status:** Accepted.

Collection, rules, redaction, persistence, and export run in Rust. React owns only interaction and presentation. This reduces the exposed IPC surface, prevents browser code from gaining process access, and lets domain tests run without a webview.

## ADR-002: Reviewed command IDs, not command strings

**Status:** Accepted.

The collector runtime maps internal enum variants to executable paths/names and immutable bounded argument arrays. It never invokes a shell. Dynamic arguments are accepted only for validated systemd unit names discovered by a reviewed collector. Output is capped and timeouts kill the child process.

## ADR-003: Redacted case is the persistence boundary

**Status:** Accepted.

The MVP persists only the redacted case document. Raw evidence exists in memory only during a scan. Reports and exports accept redacted evidence and include a category/count manifest without original values.

## ADR-004: Browser demo is an untrusted test adapter

**Status:** Accepted.

When Tauri APIs are unavailable, the UI uses explicit synthetic fixture data. Demo mode performs no diagnostics and is visibly labelled. Production builds inside Tauri use narrow `invoke` calls.

## Data flow invariants

1. A validated problem category selects a fixed collector plan.
2. Each collector returns a structured success, skipped, partial, timeout, or failed result.
3. Normalized evidence is evaluated by versioned deterministic rules.
4. A structured copy is redacted and a redaction manifest is produced.
5. Only the redacted case is atomically persisted and passed to the report builder.
6. Export writes to a user-selected destination; no automatic upload exists.

## Prior-art inspection

The design reuses the boundary discipline, timeout conventions, deterministic tests, atomic-write patterns, narrow desktop bridge, calm navigation, and root-owned command surface observed in available `loofi-fedora-tweaks` and `swedish-secondhand-ai` checkouts. `hwmonitor-remote` and `loofi-control-center` were not present in the available workspace and could not be inspected. There is no runtime dependency on any sibling repository.
