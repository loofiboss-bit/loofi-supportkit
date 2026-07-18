# Product specification

## Product promise

Loofi SupportKit turns local system evidence into a privacy-reviewed support case that a non-technical user can understand and an IT technician can inspect. It is a diagnostics and communication tool, not a repair or monitoring agent.

## Supported systems

- Windows 11 x64
- Fedora KDE 44 x64

Other systems receive an unsupported-platform result. The product does not claim macOS, general Linux, older Windows, Flatpak, ARM, or remote-host support.

## Primary workflow

1. Choose one of seven problem categories.
2. Review what the read-only scan will inspect.
3. Run the scan and see completed, skipped, and failed collectors.
4. Review findings ordered by severity with progressive evidence disclosure.
5. Review privacy transformations and remaining sensitivity warnings.
6. export ZIP, offline HTML, or JSON.
7. Reopen or safely delete a locally stored case.

## Categories

Performance, network, operating-system updates, storage, drivers/graphics, failed services/background processes, and general diagnostics.

## Functional acceptance criteria

- A scan failure in one collector does not abort other collectors.
- The same normalized evidence and rule versions produce byte-equivalent finding JSON.
- Every finding links only to evidence IDs present in the case.
- Exports validate against checked-in schemas.
- ZIP entries and timestamps are deterministic for a fixed case.
- Users can inspect which redaction categories changed which evidence fields.
- The UI provides keyboard-visible focus, semantic headings, errors, empty states, and cancellation status.

## Explicit non-goals

No repairs, elevation, user-supplied commands, AI diagnosis, cloud service, account, telemetry, upload, remote access, unattended agent, sync, marketplace, plugins, mobile app, or community rule downloads.
