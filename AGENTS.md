# Loofi SupportKit repository instructions

## Mission and supported targets

Build a trustworthy, local-first, read-only diagnostics desktop application for Windows 11 x64 and Fedora KDE 44 x64. Do not claim broader platform support without tested collectors and packaging.

## Read order

1. `AGENTS.md`
2. `docs/PRODUCT_SPEC.md`
3. `ARCHITECTURE.md` and `docs/THREAT_MODEL.md`
4. `docs/IMPLEMENTATION_PLAN.md` and `docs/PROGRESS.md`
5. Relevant schemas, rules, implementation, and tests

## Architectural boundaries

- `collector-runtime` is the only crate allowed to start external processes.
- Process definitions use fixed executable IDs and argument arrays. Never add shell execution.
- `supportkit-core` contains deterministic domain models and rules; it performs no I/O.
- `redaction-engine` transforms structured copies and never logs original matched values.
- `report-builder` accepts already-redacted evidence only.
- Tauri commands validate enum-shaped input and expose no general filesystem or process capability.
- React components call the typed bridge and contain no host diagnostics.

## Required verification

Run focused tests while changing code. Before handoff, run `just check`; for UI workflow changes also run `npm run test:e2e`; for packaging changes run `just package` on the target platform when dependencies are available.

Keep fixtures synthetic. Never commit host evidence, usernames, tokens, machine IDs, serial numbers, or network identifiers.

