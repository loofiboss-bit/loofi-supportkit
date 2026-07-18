# Progress

## Current phase

First usable vertical slice — implemented and locally validated.

## Completed

- Inspected the empty target workspace and available sibling repositories.
- Confirmed Tauri 2 command/capability and packaging conventions against current official documentation.
- Recorded product scope, architecture decisions, threat model, and phased implementation plan.
- Added the root workspace and command surface.
- Added versioned evidence, finding, rule, and support-case schemas with synthetic fixtures.
- Implemented deterministic rules, adversarial structured redaction, bounded collectors, static reports, checksummed ZIPs, and atomic redacted-case persistence.
- Implemented the Tauri 2 + React diagnostic, findings, privacy, export, history, and settings workflow.
- Added Linux/Windows CI, Dependabot, RPM/AppImage/NSIS packaging jobs, and user/developer/format/rule documentation.

## Validation performed

- `cargo test --workspace`: all unit and documentation tests passed.
- `npm test`, schema validation, and production frontend build passed.
- `just check`: formatting, Clippy, Rust/React tests, TypeScript, ESLint, schemas, and Prettier passed.
- The ignored live Fedora collector smoke test passed without elevation.
- Four Chromium/Playwright flows passed, including WCAG checks and 320/375/414/768 px overflow checks.
- The Windows collector runtime passed cross-target Clippy with warnings denied.
- Final Fedora RPM and AppImage packages built; the AppImage started successfully.

## Known limitations

- `hwmonitor-remote` and `loofi-control-center` were not available for inspection.
- Windows live collectors and Windows packaging require Windows 11 CI/hardware validation.
- Flatpak is intentionally not claimed.

## Deferred work

- Privileged collectors, repair actions, upload, remote access, accounts, cloud services, and AI diagnosis are outside the MVP.

## Next highest-priority task

Validate Windows collectors on physical Windows 11 x64 hardware, add signing, and expose cooperative scan cancellation without weakening command bounds.
