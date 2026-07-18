# Progress

## Current phase

Phase 1 — foundations.

## Completed

- Inspected the empty target workspace and available sibling repositories.
- Confirmed Tauri 2 command/capability and packaging conventions against current official documentation.
- Recorded product scope, architecture decisions, threat model, and phased implementation plan.
- Added the root workspace and command surface.

## Validation performed

- Local Node.js, npm, Rust, Cargo, Git, and just versions checked.
- No pre-existing Git worktree or source changes were present.

## Known limitations

- `hwmonitor-remote` and `loofi-control-center` were not available for inspection.
- Windows live collectors and Windows packaging require Windows 11 CI/hardware validation.
- Flatpak is intentionally not claimed.

## Deferred work

- Privileged collectors, repair actions, upload, remote access, accounts, cloud services, and AI diagnosis are outside the MVP.

## Next highest-priority task

Implement versioned schemas and the deterministic Rust domain core with fixture tests.

